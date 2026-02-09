use anchor_lang::prelude::*;
use anchor_lang::solana_program::program::invoke_signed;
use anchor_spl::token::{self, Token, TokenAccount, Mint, MintTo, SyncNative, Approve};
use anchor_spl::token_interface::{
    Mint as MintInterface,
    TokenAccount as TokenAccountInterface,
    TokenInterface,
    transfer_checked,
    TransferChecked,
};
use anchor_spl::associated_token::AssociatedToken;
use crate::state::*;
use crate::constants::*;
use crate::errors::SovereignError;
use crate::events::*;
use crate::samm::{self, instructions as samm_ix, cpi as samm_cpi};

// ============================================================
// STEP 1: CREATE POOL
// ============================================================

/// Create the SAMM pool for a finalized sovereign.
/// This is step 1 of the two-step finalization process.
///
/// Prerequisites:
/// - Sovereign must be in `Finalizing` state (bond target met)
///
/// After success:
/// - Pool is created on SAMM with initial price
/// - Sovereign transitions to `PoolCreated` state
/// - Pool state address stored on sovereign
#[derive(Accounts)]
pub struct FinalizeCreatePool<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(
        seeds = [PROTOCOL_STATE_SEED],
        bump = protocol_state.bump
    )]
    pub protocol_state: Box<Account<'info, ProtocolState>>,

    #[account(
        mut,
        seeds = [SOVEREIGN_SEED, &sovereign.sovereign_id.to_le_bytes()],
        bump = sovereign.bump
    )]
    pub sovereign: Box<Account<'info, SovereignState>>,

    /// The sovereign's token mint (Token-2022)
    #[account(address = sovereign.token_mint)]
    pub token_mint: Box<InterfaceAccount<'info, MintInterface>>,

    /// WGOR native mint
    /// CHECK: Validated against constant
    #[account(address = WGOR_MINT)]
    pub wgor_mint: Box<InterfaceAccount<'info, MintInterface>>,

    /// SOL vault holding deposits (used to calculate initial price)
    #[account(
        seeds = [SOL_VAULT_SEED, sovereign.key().as_ref()],
        bump
    )]
    pub sol_vault: SystemAccount<'info>,

    /// Token vault holding sovereign tokens (used to calculate initial price)
    #[account(
        seeds = [TOKEN_VAULT_SEED, sovereign.key().as_ref()],
        bump
    )]
    pub token_vault: Box<InterfaceAccount<'info, TokenAccountInterface>>,

    // ============ SAMM Accounts ============

    /// CHECK: Trashbin SAMM program
    #[account(address = SAMM_PROGRAM_ID)]
    pub samm_program: UncheckedAccount<'info>,

    /// CHECK: AMM configuration account on SAMM — validated against sovereign.amm_config
    #[account(address = sovereign.amm_config)]
    pub amm_config: UncheckedAccount<'info>,

    /// CHECK: Pool state PDA - derived from ["pool", amm_config, token_mint_0, token_mint_1]
    #[account(mut)]
    pub pool_state: UncheckedAccount<'info>,

    /// CHECK: Token vault 0 PDA on SAMM - derived from ["pool_vault", pool_state, token_mint_0]
    #[account(mut)]
    pub samm_token_vault_0: UncheckedAccount<'info>,

    /// CHECK: Token vault 1 PDA on SAMM - derived from ["pool_vault", pool_state, token_mint_1]
    #[account(mut)]
    pub samm_token_vault_1: UncheckedAccount<'info>,

    /// CHECK: Observation state PDA - derived from ["observation", pool_state]
    #[account(mut)]
    pub observation_state: UncheckedAccount<'info>,

    /// CHECK: Tick array bitmap extension PDA
    #[account(mut)]
    pub tick_array_bitmap: UncheckedAccount<'info>,

    /// Token program for WGOR (legacy SPL Token)
    pub token_program: Program<'info, Token>,

    /// Token program for sovereign token (Token-2022)
    pub token_program_2022: Interface<'info, TokenInterface>,

    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

pub fn finalize_create_pool_handler(ctx: Context<FinalizeCreatePool>) -> Result<()> {
    let sovereign = &mut ctx.accounts.sovereign;
    let protocol = &ctx.accounts.protocol_state;
    let clock = Clock::get()?;

    // ---- Validations ----
    require!(!protocol.paused, SovereignError::ProtocolPaused);
    require!(
        sovereign.state == SovereignStatus::Finalizing,
        SovereignError::InvalidState
    );
    require!(
        sovereign.total_deposited >= sovereign.bond_target,
        SovereignError::BondTargetNotMet
    );
    require!(sovereign.total_deposited > 0, SovereignError::NoDeposits);
    require!(
        sovereign.pool_state == Pubkey::default(),
        SovereignError::PoolAlreadyCreated
    );

    // ---- Determine token ordering ----
    // Canonical order: lower pubkey = token_0, higher pubkey = token_1
    let wgor_key = ctx.accounts.wgor_mint.key();
    let token_key = ctx.accounts.token_mint.key();
    let (mint_0, mint_1, is_swapped) = samm_cpi::sort_mints(&wgor_key, &token_key);
    // sort_mints(&wgor, &token) returns is_swapped=false when wgor < token (wgor IS mint_0)
    // So wgor_is_0 = !is_swapped
    let wgor_is_0 = !is_swapped;

    msg!(
        "Creating pool: mint_0={}, mint_1={}, wgor_is_token_0={}",
        mint_0,
        mint_1,
        wgor_is_0
    );

    // ---- Calculate initial sqrt price ----
    // For CLMM: price = token_1_amount / token_0_amount
    // If WGOR is token_0: price = sovereign_tokens / wgor_amount
    // If WGOR is token_1: price = wgor_amount / sovereign_tokens
    let sol_amount = ctx.accounts.sol_vault.lamports();
    let token_amount = ctx.accounts.token_vault.amount;

    // Calculate LP allocation
    let lp_tokens = if sovereign.sovereign_type == SovereignType::TokenLaunch {
        token_amount
            .checked_mul(LP_ALLOCATION_BPS as u64)
            .unwrap()
            .checked_div(BPS_DENOMINATOR as u64)
            .unwrap()
    } else {
        token_amount
    };

    // Account for Token-2022 transfer fee when computing the pool price.
    // The tokens that actually reach the LP position = lp_tokens - fee.
    // The pool price must reflect this post-fee amount.
    let tokens_for_price = if sovereign.sell_fee_bps > 0 {
        let fee = lp_tokens
            .checked_mul(sovereign.sell_fee_bps as u64).unwrap()
            .checked_add(9999).unwrap()
            .checked_div(10000).unwrap();
        lp_tokens.checked_sub(fee).unwrap()
    } else {
        lp_tokens
    };

    let price = if wgor_is_0 {
        // price = token_1_per_token_0 = tokens / sol
        tokens_for_price as f64 / sol_amount as f64
    } else {
        // price = token_1_per_token_0 = sol / tokens
        sol_amount as f64 / tokens_for_price as f64
    };

    let sqrt_price_x64 = samm_cpi::price_to_sqrt_price_x64(price);

    // ---- Determine token programs for each side ----
    let (token_program_0, token_program_1, mint_0_info, mint_1_info) = if wgor_is_0 {
        (
            ctx.accounts.token_program.to_account_info(),    // WGOR = legacy SPL
            ctx.accounts.token_program_2022.to_account_info(), // sovereign = Token-2022
            ctx.accounts.wgor_mint.to_account_info(),
            ctx.accounts.token_mint.to_account_info(),
        )
    } else {
        (
            ctx.accounts.token_program_2022.to_account_info(), // sovereign = Token-2022
            ctx.accounts.token_program.to_account_info(),      // WGOR = legacy SPL
            ctx.accounts.token_mint.to_account_info(),
            ctx.accounts.wgor_mint.to_account_info(),
        )
    };

    // ---- CPI: Create Pool ----
    let create_pool_accounts = samm_ix::CreatePoolAccounts {
        pool_creator: ctx.accounts.payer.to_account_info(),
        amm_config: ctx.accounts.amm_config.to_account_info(),
        pool_state: ctx.accounts.pool_state.to_account_info(),
        token_mint_0: mint_0_info,
        token_mint_1: mint_1_info,
        token_vault_0: ctx.accounts.samm_token_vault_0.to_account_info(),
        token_vault_1: ctx.accounts.samm_token_vault_1.to_account_info(),
        observation_state: ctx.accounts.observation_state.to_account_info(),
        tick_array_bitmap: ctx.accounts.tick_array_bitmap.to_account_info(),
        token_program_0,
        token_program_1,
        system_program: ctx.accounts.system_program.to_account_info(),
        rent: ctx.accounts.rent.to_account_info(),
    };

    #[cfg(not(any(feature = "localnet", feature = "devnet")))]
    {
        samm_cpi::create_pool(
            &ctx.accounts.samm_program.to_account_info(),
            create_pool_accounts,
            sqrt_price_x64,
            1u64, // open_time in the past = pool immediately tradeable
            &[], // payer signs naturally, no PDA seeds needed
        )?;
    }

    #[cfg(any(feature = "localnet", feature = "devnet"))]
    {
        msg!("DEVNET: Skipping SAMM create_pool CPI (test mode)");
        // Suppress unused variable warnings in test mode
        let _ = create_pool_accounts;
        let _ = sqrt_price_x64;
    }

    // ---- Update sovereign state ----
    sovereign.pool_state = ctx.accounts.pool_state.key();
    sovereign.pool_restricted = true;
    sovereign.total_supply = token_amount;
    sovereign.state = SovereignStatus::PoolCreated;

    emit!(SammPoolCreated {
        sovereign_id: sovereign.sovereign_id,
        pool_state: ctx.accounts.pool_state.key(),
        token_mint_0: mint_0,
        token_mint_1: mint_1,
        sqrt_price_x64,
        created_at: clock.unix_timestamp,
    });

    msg!("Pool created: {}", ctx.accounts.pool_state.key());
    Ok(())
}

// ============================================================
// STEP 2: ADD LIQUIDITY (Open Position)
// ============================================================

/// Add initial liquidity to the SAMM pool and transition to Recovery.
/// This is step 2 of the two-step finalization process.
///
/// Prerequisites:
/// - Sovereign must be in `PoolCreated` state
/// - Frontend must create:
///   1. WGOR ATA for permanent_lock PDA
///   2. Sovereign token ATA for permanent_lock PDA (Token-2022)
///   3. Fresh Keypair for position NFT mint (passed as signer)
///   4. ATA of permanent_lock for position NFT mint
///
/// After success:
/// - SOL wrapped to WGOR and deposited in pool
/// - Sovereign tokens deposited in pool
/// - Full-range position created, owned by permanent_lock
/// - Sovereign transitions to `Recovery` state
#[derive(Accounts)]
pub struct FinalizeAddLiquidity<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(
        seeds = [PROTOCOL_STATE_SEED],
        bump = protocol_state.bump
    )]
    pub protocol_state: Box<Account<'info, ProtocolState>>,

    #[account(
        mut,
        seeds = [SOVEREIGN_SEED, &sovereign.sovereign_id.to_le_bytes()],
        bump = sovereign.bump
    )]
    pub sovereign: Box<Account<'info, SovereignState>>,

    /// Token mint for the sovereign token (Token-2022)
    #[account(
        address = sovereign.token_mint
    )]
    pub token_mint: Box<InterfaceAccount<'info, MintInterface>>,

    /// WGOR native mint
    #[account(address = WGOR_MINT)]
    pub wgor_mint: Box<InterfaceAccount<'info, MintInterface>>,

    /// SOL vault holding deposits
    /// CHECK: PDA that holds SOL
    #[account(
        mut,
        seeds = [SOL_VAULT_SEED, sovereign.key().as_ref()],
        bump
    )]
    pub sol_vault: SystemAccount<'info>,

    /// Sovereign's token vault (Token-2022)
    #[account(
        mut,
        seeds = [TOKEN_VAULT_SEED, sovereign.key().as_ref()],
        bump
    )]
    pub token_vault: Box<InterfaceAccount<'info, TokenAccountInterface>>,

    /// Permanent lock PDA (initialized in this instruction)
    #[account(
        init_if_needed,
        payer = payer,
        space = PermanentLock::LEN,
        seeds = [PERMANENT_LOCK_SEED, sovereign.key().as_ref()],
        bump
    )]
    pub permanent_lock: Box<Account<'info, PermanentLock>>,

    // ---- Token accounts for permanent_lock (created by frontend) ----

    /// WGOR token account owned by permanent_lock PDA
    /// CHECK: Frontend creates this ATA before calling instruction
    #[account(mut)]
    pub lock_wgor_account: UncheckedAccount<'info>,

    /// Sovereign token account owned by permanent_lock PDA (Token-2022)
    /// CHECK: Frontend creates this ATA before calling instruction
    #[account(mut)]
    pub lock_token_account: UncheckedAccount<'info>,

    // ---- SAMM Accounts ----

    /// CHECK: Trashbin SAMM program
    #[account(address = SAMM_PROGRAM_ID)]
    pub samm_program: UncheckedAccount<'info>,

    /// CHECK: Pool state (must match sovereign.pool_state)
    #[account(
        mut,
        address = sovereign.pool_state
    )]
    pub pool_state: UncheckedAccount<'info>,

    /// Position NFT mint - fresh Keypair created by frontend
    #[account(mut)]
    pub position_nft_mint: Signer<'info>,

    /// CHECK: ATA of permanent_lock for position_nft_mint (created by SAMM)
    #[account(mut)]
    pub position_nft_account: UncheckedAccount<'info>,

    /// CHECK: Metaplex metadata account for position NFT
    #[account(mut)]
    pub metadata_account: UncheckedAccount<'info>,

    /// CHECK: Protocol position state PDA on SAMM
    #[account(mut)]
    pub protocol_position: UncheckedAccount<'info>,

    /// CHECK: Lower tick array PDA on SAMM
    #[account(mut)]
    pub tick_array_lower: UncheckedAccount<'info>,

    /// CHECK: Upper tick array PDA on SAMM
    #[account(mut)]
    pub tick_array_upper: UncheckedAccount<'info>,

    /// CHECK: Personal position state PDA on SAMM
    #[account(mut)]
    pub personal_position: UncheckedAccount<'info>,

    /// CHECK: SAMM pool token vault 0
    #[account(mut)]
    pub samm_token_vault_0: UncheckedAccount<'info>,

    /// CHECK: SAMM pool token vault 1
    #[account(mut)]
    pub samm_token_vault_1: UncheckedAccount<'info>,

    /// CHECK: Observation state on SAMM
    #[account(mut)]
    pub observation_state: UncheckedAccount<'info>,

    /// CHECK: Vault 0 mint (for SAMM validation)
    pub vault_0_mint: UncheckedAccount<'info>,

    /// CHECK: Vault 1 mint (for SAMM validation)
    pub vault_1_mint: UncheckedAccount<'info>,

    /// CHECK: Tick array bitmap extension PDA on SAMM
    /// Required for full-range positions that overflow the default bitmap
    #[account(mut)]
    pub tick_array_bitmap_extension: UncheckedAccount<'info>,

    /// CHECK: Metaplex Token Metadata program
    #[account(address = METAPLEX_PROGRAM_ID)]
    pub metadata_program: UncheckedAccount<'info>,

    // ---- Standard Programs ----

    /// Legacy SPL Token program (for WGOR)
    pub token_program: Program<'info, Token>,

    /// Token-2022 program (for sovereign token)
    pub token_program_2022: Interface<'info, TokenInterface>,

    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

pub fn finalize_add_liquidity_handler<'info>(
    ctx: Context<'_, '_, 'info, 'info, FinalizeAddLiquidity<'info>>,
) -> Result<()> {
    let sovereign_key = ctx.accounts.sovereign.key();
    let sovereign = &mut ctx.accounts.sovereign;
    let protocol = &ctx.accounts.protocol_state;
    let clock = Clock::get()?;

    // ---- Validations ----
    require!(!protocol.paused, SovereignError::ProtocolPaused);
    require!(
        sovereign.state == SovereignStatus::PoolCreated,
        SovereignError::InvalidState
    );
    require!(
        sovereign.pool_state != Pubkey::default(),
        SovereignError::PoolNotCreated
    );

    // ---- Signer seeds ----
    let sovereign_id_bytes = sovereign.sovereign_id.to_le_bytes();
    let sovereign_seeds = &[
        SOVEREIGN_SEED,
        &sovereign_id_bytes,
        &[sovereign.bump],
    ];
    let sovereign_signer_seeds = &[&sovereign_seeds[..]];

    let lock_seeds = &[
        PERMANENT_LOCK_SEED,
        sovereign_key.as_ref(),
        &[ctx.bumps.permanent_lock],
    ];
    let lock_signer_seeds = &[&lock_seeds[..]];

    // ---- Calculate amounts ----
    let sol_amount = ctx.accounts.sol_vault.lamports();
    let token_amount = ctx.accounts.token_vault.amount;

    // LP token allocation
    let lp_tokens = if sovereign.sovereign_type == SovereignType::TokenLaunch {
        token_amount
            .checked_mul(LP_ALLOCATION_BPS as u64)
            .unwrap()
            .checked_div(BPS_DENOMINATOR as u64)
            .unwrap()
    } else {
        token_amount
    };

    msg!(
        "Adding liquidity: {} SOL (as WGOR), {} tokens",
        sol_amount,
        lp_tokens
    );

    // ---- Step 1: Wrap SOL to WGOR ----
    // Transfer SOL from sol_vault PDA → lock_wgor_account
    // Then sync_native to update token balance

    let sol_vault_bump = ctx.bumps.sol_vault;
    let sol_vault_seeds = &[
        SOL_VAULT_SEED,
        sovereign_key.as_ref(),
        &[sol_vault_bump],
    ];
    let sol_vault_signer = &[&sol_vault_seeds[..]];

    // Transfer SOL from sol_vault to the WGOR token account
    let transfer_sol_ix = anchor_lang::solana_program::system_instruction::transfer(
        &ctx.accounts.sol_vault.key(),
        &ctx.accounts.lock_wgor_account.key(),
        sol_amount,
    );
    invoke_signed(
        &transfer_sol_ix,
        &[
            ctx.accounts.sol_vault.to_account_info(),
            ctx.accounts.lock_wgor_account.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
        ],
        sol_vault_signer,
    )?;

    // Sync native to update the WGOR token balance
    token::sync_native(CpiContext::new(
        ctx.accounts.token_program.to_account_info(),
        SyncNative {
            account: ctx.accounts.lock_wgor_account.to_account_info(),
        },
    ))?;

    msg!("Wrapped {} lamports to WGOR", sol_amount);

    // ---- Step 2: Transfer sovereign tokens to permanent_lock's account ----
    let token_decimals = ctx.accounts.token_mint.decimals;

    transfer_checked(
        CpiContext::new_with_signer(
            ctx.accounts.token_program_2022.to_account_info(),
            TransferChecked {
                from: ctx.accounts.token_vault.to_account_info(),
                mint: ctx.accounts.token_mint.to_account_info(),
                to: ctx.accounts.lock_token_account.to_account_info(),
                authority: sovereign.to_account_info(),
            },
            sovereign_signer_seeds,
        ),
        lp_tokens,
        token_decimals,
    )?;

    msg!("Transferred {} tokens to permanent lock", lp_tokens);

    // ---- Account for Token-2022 transfer fee ----
    // When sell_fee_bps > 0, Token-2022's transfer_checked withholds a fee at the
    // destination. The lock_token_account's usable balance is lp_tokens minus the
    // withheld fee. We must use the post-fee amount for SAMM LP and approvals.
    let tokens_in_lock = if sovereign.sell_fee_bps > 0 {
        let fee = lp_tokens
            .checked_mul(sovereign.sell_fee_bps as u64).unwrap()
            .checked_add(9999).unwrap()  // ceiling division to match Token-2022
            .checked_div(10000).unwrap();
        let received = lp_tokens.checked_sub(fee).unwrap();
        msg!("Transfer fee: {} bps, fee={}, usable tokens in lock={}", sovereign.sell_fee_bps, fee, received);
        received
    } else {
        lp_tokens
    };

    // ---- Step 3: Determine token ordering for position ----
    let wgor_key = ctx.accounts.wgor_mint.key();
    let token_key = ctx.accounts.token_mint.key();
    let (_mint_0, _mint_1, is_swapped) = samm_cpi::sort_mints(&wgor_key, &token_key);
    let wgor_is_0 = !is_swapped;

    // Set token accounts in correct order — use tokens_in_lock (post-fee) for the token side
    let (token_account_0, token_account_1, amount_0, amount_1) = if wgor_is_0 {
        (
            ctx.accounts.lock_wgor_account.to_account_info(),
            ctx.accounts.lock_token_account.to_account_info(),
            sol_amount,
            tokens_in_lock,
        )
    } else {
        (
            ctx.accounts.lock_token_account.to_account_info(),
            ctx.accounts.lock_wgor_account.to_account_info(),
            tokens_in_lock,
            sol_amount,
        )
    };

    // ---- Step 4: Let SAMM compute liquidity from token amounts ----
    // Instead of computing liquidity ourselves (f64 math diverges from SAMM's
    // Q64.64 fixed-point), we pass liquidity=0 with base_flag so the SAMM
    // calculates optimal liquidity. We compute from the sovereign token side
    // to maximize token utilization per spec ("100% tokens to LP").
    //
    // base_flag=true  → compute L from amount_0_max
    // base_flag=false → compute L from amount_1_max
    //
    // We apply a tiny 0.01% safety margin on the base (token) side so the
    // SAMM's computed need for the other side stays within what we have.
    // For a correctly-priced pool, dust is ≤ 0.01% of each side.
    let (base_flag, amount_0_for_lp, amount_1_for_lp) = if wgor_is_0 {
        // tokens are amount_1 → compute L from amount_1 → base_flag=false
        let token_adj = amount_1.checked_mul(9999).unwrap().checked_div(10000).unwrap();
        (Some(false), amount_0, token_adj)
    } else {
        // tokens are amount_0 → compute L from amount_0 → base_flag=true
        let token_adj = amount_0.checked_mul(9999).unwrap().checked_div(10000).unwrap();
        (Some(true), token_adj, amount_1)
    };

    // Estimate liquidity for record-keeping (not used by SAMM)
    let price = if wgor_is_0 {
        tokens_in_lock as f64 / sol_amount as f64
    } else {
        sol_amount as f64 / tokens_in_lock as f64
    };
    let sqrt_price_x64 = samm_cpi::price_to_sqrt_price_x64(price);
    let liquidity_estimate = samm_cpi::calculate_full_range_liquidity(
        amount_0,
        amount_1,
        sqrt_price_x64,
    );

    // ---- Step 5: Approve wallet payer as delegate on token accounts ----
    // The SAMM's OpenPositionV2 uses the `payer` account for both:
    //   1. Paying rent (System Transfer - requires account with NO data)
    //   2. Authorizing token transfers from token_account_0/token_account_1
    // Since permanent_lock is already initialized (carries data), it cannot
    // be used as the SAMM payer. Instead, we approve the wallet payer as a
    // delegate on both token accounts so the SAMM can use the wallet for both.

    // Approve payer as delegate on WGOR account (legacy SPL Token)
    token::approve(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            Approve {
                to: ctx.accounts.lock_wgor_account.to_account_info(),
                delegate: ctx.accounts.payer.to_account_info(),
                authority: ctx.accounts.permanent_lock.to_account_info(),
            },
            lock_signer_seeds,
        ),
        sol_amount,
    )?;

    // Approve payer as delegate on sovereign token account (Token-2022)
    // Use raw instruction since anchor_spl::token::approve is typed to legacy Token program
    let approve_token_ix = anchor_lang::solana_program::instruction::Instruction {
        program_id: ctx.accounts.token_program_2022.key(),
        accounts: vec![
            anchor_lang::solana_program::instruction::AccountMeta::new(
                ctx.accounts.lock_token_account.key(), false,
            ),
            anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                ctx.accounts.payer.key(), false,
            ),
            anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                ctx.accounts.permanent_lock.key(), true,
            ),
        ],
        data: {
            let mut buf = Vec::with_capacity(9);
            buf.push(4u8); // SPL Token Approve instruction index
            buf.extend_from_slice(&tokens_in_lock.to_le_bytes());
            buf
        },
    };
    invoke_signed(
        &approve_token_ix,
        &[
            ctx.accounts.lock_token_account.to_account_info(),
            ctx.accounts.payer.to_account_info(),
            ctx.accounts.permanent_lock.to_account_info(),
            ctx.accounts.token_program_2022.to_account_info(),
        ],
        lock_signer_seeds,
    )?;

    msg!("Approved wallet payer as delegate on token accounts");

    // ---- Step 6: CPI to SAMM open_position_v2 ----
    // Wallet payer pays rent; token transfers use delegate approval
    #[cfg(not(any(feature = "localnet", feature = "devnet")))]
    {
        let open_position_accounts = samm_ix::OpenPositionV2Accounts {
            payer: ctx.accounts.payer.to_account_info(),
            position_nft_owner: ctx.accounts.permanent_lock.to_account_info(),
            position_nft_mint: ctx.accounts.position_nft_mint.to_account_info(),
            position_nft_account: ctx.accounts.position_nft_account.to_account_info(),
            metadata_account: ctx.accounts.metadata_account.to_account_info(),
            pool_state: ctx.accounts.pool_state.to_account_info(),
            protocol_position: ctx.accounts.protocol_position.to_account_info(),
            tick_array_lower: ctx.accounts.tick_array_lower.to_account_info(),
            tick_array_upper: ctx.accounts.tick_array_upper.to_account_info(),
            personal_position: ctx.accounts.personal_position.to_account_info(),
            token_account_0,
            token_account_1,
            token_vault_0: ctx.accounts.samm_token_vault_0.to_account_info(),
            token_vault_1: ctx.accounts.samm_token_vault_1.to_account_info(),
            rent: ctx.accounts.rent.to_account_info(),
            system_program: ctx.accounts.system_program.to_account_info(),
            token_program: ctx.accounts.token_program.to_account_info(),
            associated_token_program: ctx.accounts.associated_token_program.to_account_info(),
            metadata_program: ctx.accounts.metadata_program.to_account_info(),
            token_program_2022: ctx.accounts.token_program_2022.to_account_info(),
            vault_0_mint: ctx.accounts.vault_0_mint.to_account_info(),
            vault_1_mint: ctx.accounts.vault_1_mint.to_account_info(),
            tick_array_bitmap_extension: ctx.accounts.tick_array_bitmap_extension.to_account_info(),
        };

        // No PDA signer seeds needed - wallet payer is already a transaction signer
        // Pass liquidity=0 with base_flag to let the SAMM compute optimal liquidity
        // from the sovereign token side (maximizes token utilization)
        samm_cpi::open_position_full_range(
            &ctx.accounts.samm_program.to_account_info(),
            open_position_accounts,
            0,                // let SAMM compute liquidity
            amount_0_for_lp,  // max amount_0 (full if GOR, 0.01%-reduced if tokens)
            amount_1_for_lp,  // max amount_1 (full if GOR, 0.01%-reduced if tokens)
            samm::tick::DEFAULT_TICK_SPACING,
            base_flag,        // compute L from whichever side is the sovereign token
            &[], // wallet + position_nft_mint are already outer tx signers
        )?;

        msg!("SAMM position created (liquidity computed by SAMM)");
    }

    #[cfg(any(feature = "localnet", feature = "devnet"))]
    {
        msg!("DEVNET: Skipping SAMM open_position CPI (test mode)");
    }

    // ---- Step 7: Revoke delegate approvals for security ----
    // Revoke delegate on WGOR account
    let revoke_wgor_ix = anchor_lang::solana_program::instruction::Instruction {
        program_id: ctx.accounts.token_program.key(),
        accounts: vec![
            anchor_lang::solana_program::instruction::AccountMeta::new(
                ctx.accounts.lock_wgor_account.key(), false,
            ),
            anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                ctx.accounts.permanent_lock.key(), true,
            ),
        ],
        data: vec![5u8], // SPL Token Revoke instruction index
    };
    invoke_signed(
        &revoke_wgor_ix,
        &[
            ctx.accounts.lock_wgor_account.to_account_info(),
            ctx.accounts.permanent_lock.to_account_info(),
            ctx.accounts.token_program.to_account_info(),
        ],
        lock_signer_seeds,
    )?;

    // Revoke delegate on sovereign token account
    let revoke_token_ix = anchor_lang::solana_program::instruction::Instruction {
        program_id: ctx.accounts.token_program_2022.key(),
        accounts: vec![
            anchor_lang::solana_program::instruction::AccountMeta::new(
                ctx.accounts.lock_token_account.key(), false,
            ),
            anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                ctx.accounts.permanent_lock.key(), true,
            ),
        ],
        data: vec![5u8], // SPL Token Revoke instruction index
    };
    invoke_signed(
        &revoke_token_ix,
        &[
            ctx.accounts.lock_token_account.to_account_info(),
            ctx.accounts.permanent_lock.to_account_info(),
            ctx.accounts.token_program_2022.to_account_info(),
        ],
        lock_signer_seeds,
    )?;

    // ---- Step 8: Initialize permanent lock ----
    let permanent_lock = &mut ctx.accounts.permanent_lock;
    permanent_lock.sovereign = sovereign_key;
    permanent_lock.pool_state = sovereign.pool_state;
    permanent_lock.position_mint = ctx.accounts.position_nft_mint.key();
    permanent_lock.position_token_account = ctx.accounts.position_nft_account.key();
    permanent_lock.liquidity = liquidity_estimate;
    permanent_lock.tick_lower_index = samm::tick::MIN_TICK;
    permanent_lock.tick_upper_index = samm::tick::MAX_TICK;
    permanent_lock.unwound = false;
    permanent_lock.created_at = clock.unix_timestamp;
    permanent_lock.bump = ctx.bumps.permanent_lock;

    // Derive and store position PDA
    let (position_pda, _) = Pubkey::find_program_address(
        &[
            SAMM_POSITION_SEED,
            ctx.accounts.position_nft_mint.key().as_ref(),
        ],
        &SAMM_PROGRAM_ID,
    );
    permanent_lock.position = position_pda;

    // ---- Step 7: Update sovereign state ----
    sovereign.position_mint = ctx.accounts.position_nft_mint.key();
    sovereign.recovery_target = sovereign.total_deposited;
    sovereign.total_recovered = 0;
    sovereign.finalized_at = clock.unix_timestamp;

    if sovereign.recovery_target == 0 {
        sovereign.state = SovereignStatus::Active;
    } else {
        sovereign.state = SovereignStatus::Recovery;
        emit!(PoolRestricted {
            sovereign_id: sovereign.sovereign_id,
            restricted: true,
        });
    }

    emit!(LiquidityAdded {
        sovereign_id: sovereign.sovereign_id,
        pool_state: sovereign.pool_state,
        position_nft_mint: ctx.accounts.position_nft_mint.key(),
        liquidity: liquidity_estimate,
        amount_0,
        amount_1,
    });

    emit!(SovereignFinalized {
        sovereign_id: sovereign.sovereign_id,
        total_deposited: sovereign.total_deposited,
        token_supply: sovereign.total_supply,
        lp_tokens,
        recovery_target: sovereign.recovery_target,
        finalized_at: clock.unix_timestamp,
    });

    // ---- Creator market buy (placeholder) ----
    if sovereign.creator_escrow > 0 {
        emit!(CreatorMarketBuyExecuted {
            sovereign_id: sovereign.sovereign_id,
            creator: sovereign.creator,
            sol_amount: sovereign.creator_escrow,
            tokens_received: 0, // TODO: Execute actual swap
        });
        sovereign.creator_escrow = 0;
    }

    msg!("Sovereign finalized successfully. State: Recovery");
    Ok(())
}

// ============================================================
// MINT GENESIS NFT
// ============================================================

/// Mint Genesis NFT to a depositor after finalization
/// Must be called for each depositor after finalization
#[derive(Accounts)]
pub struct MintGenesisNFT<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(
        seeds = [SOVEREIGN_SEED, &sovereign.sovereign_id.to_le_bytes()],
        bump = sovereign.bump
    )]
    pub sovereign: Account<'info, SovereignState>,

    #[account(
        mut,
        seeds = [DEPOSIT_RECORD_SEED, sovereign.key().as_ref(), depositor.key().as_ref()],
        bump = deposit_record.bump
    )]
    pub deposit_record: Account<'info, DepositRecord>,

    /// CHECK: The depositor who will receive the NFT
    pub depositor: UncheckedAccount<'info>,

    /// Genesis NFT mint for this specific depositor
    #[account(
        init,
        payer = payer,
        mint::decimals = 0,
        mint::authority = sovereign,
        seeds = [GENESIS_NFT_MINT_SEED, sovereign.key().as_ref(), depositor.key().as_ref()],
        bump
    )]
    pub nft_mint: Account<'info, Mint>,

    /// NFT token account for the depositor
    #[account(
        init_if_needed,
        payer = payer,
        associated_token::mint = nft_mint,
        associated_token::authority = depositor
    )]
    pub nft_token_account: Account<'info, TokenAccount>,

    /// CHECK: Metaplex metadata account
    #[account(mut)]
    pub metadata_account: UncheckedAccount<'info>,

    /// CHECK: Metaplex program
    #[account(address = METAPLEX_PROGRAM_ID)]
    pub metadata_program: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

pub fn mint_genesis_nft_handler(ctx: Context<MintGenesisNFT>) -> Result<()> {
    let sovereign = &ctx.accounts.sovereign;
    let deposit_record = &mut ctx.accounts.deposit_record;

    // Validate state - must be Recovery or Active (post-finalization)
    require!(
        sovereign.state == SovereignStatus::Recovery
            || sovereign.state == SovereignStatus::Active,
        SovereignError::InvalidState
    );
    require!(!deposit_record.nft_minted, SovereignError::NFTAlreadyMinted);
    require!(deposit_record.amount > 0, SovereignError::ZeroDeposit);

    // Calculate voting power based on deposit share
    let voting_power = deposit_record
        .amount
        .checked_mul(BPS_DENOMINATOR as u64)
        .unwrap()
        .checked_div(sovereign.total_deposited)
        .unwrap();

    require!(
        voting_power <= u16::MAX as u64,
        SovereignError::Overflow
    );

    // Mint the NFT
    let sovereign_id_bytes = sovereign.sovereign_id.to_le_bytes();
    let seeds = &[
        SOVEREIGN_SEED,
        &sovereign_id_bytes,
        &[sovereign.bump],
    ];
    let signer_seeds = &[&seeds[..]];

    token::mint_to(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            MintTo {
                mint: ctx.accounts.nft_mint.to_account_info(),
                to: ctx.accounts.nft_token_account.to_account_info(),
                authority: sovereign.to_account_info(),
            },
            signer_seeds,
        ),
        1, // NFT amount is always 1
    )?;

    // ---- Create Metaplex metadata for secondary market display ----
    // Encodes sovereign name + share percentage for marketplace UIs.
    // The depositor's address is encoded in the URI so buyers on
    // the secondary market can derive the deposit_record PDA.
    let nft_name = format!(
        "{} Genesis #{}", 
        &sovereign.name[..sovereign.name.len().min(20)],
        sovereign.sovereign_id,
    );
    let nft_symbol = String::from("GNFT");
    // URI encodes essential lookup data as query params for off-chain consumers
    let nft_uri = format!(
        "https://sovereign.protocol/nft?s={}&d={}&bp={}",
        sovereign.key(),
        ctx.accounts.depositor.key(),
        voting_power,
    );
    
    // Build Metaplex CreateMetadataAccountV3 CPI manually
    // Instruction index 33 = CreateMetadataAccountV3
    let metadata_program_id = ctx.accounts.metadata_program.key();
    let create_metadata_ix = {
        use anchor_lang::solana_program::instruction::{AccountMeta, Instruction};
        
        let mut data = Vec::new();
        // Discriminator for CreateMetadataAccountV3
        data.push(33u8);
        
        // Serialize DataV2 using Borsh
        // name: String
        let name_bytes = nft_name.as_bytes();
        data.extend_from_slice(&(name_bytes.len() as u32).to_le_bytes());
        data.extend_from_slice(name_bytes);
        // symbol: String
        let symbol_bytes = nft_symbol.as_bytes();
        data.extend_from_slice(&(symbol_bytes.len() as u32).to_le_bytes());
        data.extend_from_slice(symbol_bytes);
        // uri: String
        let uri_bytes = nft_uri.as_bytes();
        data.extend_from_slice(&(uri_bytes.len() as u32).to_le_bytes());
        data.extend_from_slice(uri_bytes);
        // seller_fee_basis_points: u16
        data.extend_from_slice(&0u16.to_le_bytes());
        // creators: Option<Vec<Creator>> = None
        data.push(0u8);
        // collection: Option<Collection> = None
        data.push(0u8);
        // uses: Option<Uses> = None
        data.push(0u8);
        // is_mutable: bool
        data.push(1u8);
        // collection_details: Option<CollectionDetails> = None
        data.push(0u8);
        
        Instruction {
            program_id: metadata_program_id,
            accounts: vec![
                AccountMeta::new(ctx.accounts.metadata_account.key(), false),
                AccountMeta::new_readonly(ctx.accounts.nft_mint.key(), false),
                AccountMeta::new_readonly(sovereign.key(), true),  // mint authority (PDA signer)
                AccountMeta::new(ctx.accounts.payer.key(), true),
                AccountMeta::new_readonly(sovereign.key(), true),  // update authority (PDA signer)
                AccountMeta::new_readonly(ctx.accounts.system_program.key(), false),
                AccountMeta::new_readonly(ctx.accounts.rent.key(), false),
            ],
            data,
        }
    };
    
    invoke_signed(
        &create_metadata_ix,
        &[
            ctx.accounts.metadata_account.to_account_info(),
            ctx.accounts.nft_mint.to_account_info(),
            sovereign.to_account_info(),
            ctx.accounts.payer.to_account_info(),
            sovereign.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
            ctx.accounts.rent.to_account_info(),
        ],
        signer_seeds,
    )?;
    msg!("Metaplex metadata created for Genesis NFT");

    deposit_record.nft_minted = true;
    deposit_record.nft_mint = Some(ctx.accounts.nft_mint.key());
    deposit_record.voting_power_bps = voting_power as u16;
    deposit_record.shares_bps = voting_power as u16;

    emit!(GenesisNFTMinted {
        sovereign_id: sovereign.sovereign_id,
        depositor: ctx.accounts.depositor.key(),
        nft_mint: ctx.accounts.nft_mint.key(),
        voting_power_bps: voting_power as u16,
        deposit_amount: deposit_record.amount,
    });

    Ok(())
}
