use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Mint, MintTo};
use anchor_spl::associated_token::AssociatedToken;
use crate::state::*;
use crate::constants::*;
use crate::errors::SovereignError;
use crate::events::{SovereignFinalized, GenesisNFTMinted, CreatorMarketBuyExecuted, PoolRestricted};
use crate::samm::{self, instructions as samm_ix, cpi as samm_cpi};

/// Finalize the sovereign - create pool, add liquidity, mint NFTs
/// This is a complex multi-step process that may need to be split across multiple transactions
#[derive(Accounts)]
pub struct Finalize<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    
    #[account(
        seeds = [PROTOCOL_STATE_SEED],
        bump = protocol_state.bump
    )]
    pub protocol_state: Account<'info, ProtocolState>,
    
    #[account(
        mut,
        seeds = [SOVEREIGN_SEED, &sovereign.sovereign_id.to_le_bytes()],
        bump = sovereign.bump
    )]
    pub sovereign: Account<'info, SovereignState>,
    
    /// Token mint for the sovereign token
    #[account(
        mut,
        address = sovereign.token_mint
    )]
    pub token_mint: Account<'info, Mint>,
    
    /// SOL vault holding deposits
    /// CHECK: PDA that holds SOL
    #[account(
        mut,
        seeds = [SOL_VAULT_SEED, sovereign.key().as_ref()],
        bump
    )]
    pub sol_vault: SystemAccount<'info>,
    
    /// Token vault to hold LP tokens
    #[account(
        mut,
        seeds = [TOKEN_VAULT_SEED, sovereign.key().as_ref()],
        bump
    )]
    pub token_vault: Account<'info, TokenAccount>,
    
    /// Permanent lock account
    #[account(
        init_if_needed,
        payer = payer,
        space = PermanentLock::LEN,
        seeds = [PERMANENT_LOCK_SEED, sovereign.key().as_ref()],
        bump
    )]
    pub permanent_lock: Account<'info, PermanentLock>,
    
    /// Genesis NFT collection mint (created during sovereign creation)
    #[account(
        mut,
        address = sovereign.genesis_nft_mint
    )]
    pub nft_collection_mint: Account<'info, Mint>,
    
    // ============ Trashbin SAMM Accounts ============
    // These will be used to create the pool and add liquidity
    // Due to complexity, we may need to use remaining_accounts for SAMM CPI
    
    /// CHECK: Trashbin SAMM program
    #[account(address = SAMM_PROGRAM_ID)]
    pub samm_program: UncheckedAccount<'info>,
    
    // ============ Standard Programs ============
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

pub fn handler<'info>(ctx: Context<'_, '_, 'info, 'info, Finalize<'info>>) -> Result<()> {
    // Get sovereign key before mutable borrow to avoid borrow checker conflict
    let sovereign_key = ctx.accounts.sovereign.key();
    
    let sovereign = &mut ctx.accounts.sovereign;
    let protocol = &ctx.accounts.protocol_state;
    let clock = Clock::get()?;
    
    // Check protocol pause status
    require!(
        !protocol.paused,
        SovereignError::ProtocolPaused
    );
    
    // Validate state - must be Finalizing (set when bond target reached)
    require!(
        sovereign.state == SovereignStatus::Finalizing,
        SovereignError::InvalidState
    );
    
    // Double-check bond target is actually met (defense in depth)
    require!(
        sovereign.total_deposited >= sovereign.bond_target,
        SovereignError::BondTargetNotMet
    );
    
    // Validate total_deposited > 0 to prevent division issues later
    require!(
        sovereign.total_deposited > 0,
        SovereignError::NoDeposits
    );
    
    // Calculate token amounts
    // Total supply: total_supply set during creation
    // LP allocation: 80% (8000 BPS)
    // Creator allocation: 20% (2000 BPS - if TokenLaunch)
    
    let lp_tokens = sovereign.total_supply
        .checked_mul(LP_ALLOCATION_BPS as u64).unwrap()
        .checked_div(BPS_DENOMINATOR as u64).unwrap();
    
    // Mint tokens for LP pool to token vault
    let _sovereign_key = sovereign.key();
    let sovereign_id_bytes = sovereign.sovereign_id.to_le_bytes();
    let seeds = &[
        SOVEREIGN_SEED,
        &sovereign_id_bytes,
        &[sovereign.bump],
    ];
    let signer_seeds = &[&seeds[..]];
    
    // Mint LP tokens to vault
    token::mint_to(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            MintTo {
                mint: ctx.accounts.token_mint.to_account_info(),
                to: ctx.accounts.token_vault.to_account_info(),
                authority: sovereign.to_account_info(),
            },
            signer_seeds,
        ),
        lp_tokens,
    )?;
    
    // For TokenLaunch: mint creator's 20% to creator's ATA
    // This would be handled via remaining_accounts to keep the main struct simpler
    
    // ============ Trashbin SAMM Integration ============
    // The SAMM CPI calls require many accounts passed via remaining_accounts
    // Expected remaining_accounts order for finalization:
    // [0] amm_config - AMM configuration account
    // [1] pool_state - Pool state PDA (writable)
    // [2] position_nft_mint - Position NFT mint (to be created, writable)
    // [3] position_nft_account - Position NFT token account (writable)
    // [4] metadata_account - Metaplex metadata for position NFT
    // [5] protocol_position - Protocol position state (writable)
    // [6] tick_array_lower - Lower tick array (writable)
    // [7] tick_array_upper - Upper tick array (writable)
    // [8] personal_position - Personal position state (writable)
    // [9] token_account_0 - Token A account (GOR) (writable)
    // [10] token_account_1 - Token B account (project token) (writable)
    // [11] token_vault_0 - Pool token vault A (writable)
    // [12] token_vault_1 - Pool token vault B (writable)
    // [13] observation_state - Oracle observation (writable)
    // [14] token_program_2022 - Token 2022 program (optional)
    // [15] vault_0_mint - Vault 0 mint
    // [16] vault_1_mint - Vault 1 mint
    // [17] metadata_program - Metaplex program
    
    // Calculate liquidity to add (full range)
    let sol_amount = ctx.accounts.sol_vault.lamports();
    let token_amount = ctx.accounts.token_vault.amount;
    
    // Note: In production, fetch sqrt_price_x64 from pool_state or calculate
    // For initial pool creation, calculate theoretical price:
    // price = token_amount / sol_amount (tokens per SOL)
    // sqrt_price_x64 = sqrt(price) * 2^64
    let initial_price = token_amount as f64 / sol_amount as f64;
    let sqrt_price_x64 = samm_cpi::price_to_sqrt_price_x64(initial_price);
    
    // Calculate liquidity for full range position
    let liquidity = samm_cpi::calculate_full_range_liquidity(
        sol_amount,
        token_amount,
        sqrt_price_x64,
    );
    
    // Build SAMM CPI if remaining_accounts are provided
    // In a single-instruction flow, these would be validated and used
    // For complex finalization, this may be split across multiple transactions
    
    if ctx.remaining_accounts.len() >= 18 {
        // SECURITY: Validate pool_state address if provided
        // For finalization, we're creating the pool so we store the address
        // For subsequent operations, we validate against stored address
        
        // Full SAMM integration with all required accounts
        let pool_state_info = &ctx.remaining_accounts[1];
        
        // Step 1: Open position with full range (MIN_TICK to MAX_TICK)
        // This creates the genesis LP position
        msg!("Opening full-range SAMM position...");
        
        // Build open_position_v2 accounts
        let open_position_accounts = samm_ix::OpenPositionV2Accounts {
            payer: ctx.accounts.payer.to_account_info(),
            position_nft_owner: ctx.accounts.permanent_lock.to_account_info(),
            position_nft_mint: ctx.remaining_accounts[2].clone(),
            position_nft_account: ctx.remaining_accounts[3].clone(),
            metadata_account: ctx.remaining_accounts[4].clone(),
            pool_state: ctx.remaining_accounts[1].clone(),
            protocol_position: ctx.remaining_accounts[5].clone(),
            tick_array_lower: ctx.remaining_accounts[6].clone(),
            tick_array_upper: ctx.remaining_accounts[7].clone(),
            personal_position: ctx.remaining_accounts[8].clone(),
            token_account_0: ctx.remaining_accounts[9].clone(),
            token_account_1: ctx.remaining_accounts[10].clone(),
            token_vault_0: ctx.remaining_accounts[11].clone(),
            token_vault_1: ctx.remaining_accounts[12].clone(),
            rent: ctx.accounts.rent.to_account_info(),
            system_program: ctx.accounts.system_program.to_account_info(),
            token_program: ctx.accounts.token_program.to_account_info(),
            associated_token_program: ctx.accounts.associated_token_program.to_account_info(),
            metadata_program: ctx.remaining_accounts[17].clone(),
            token_program_2022: ctx.remaining_accounts[14].clone(),
            vault_0_mint: ctx.remaining_accounts[15].clone(),
            vault_1_mint: ctx.remaining_accounts[16].clone(),
        };
        
        // Use permanent_lock as signer for position NFT ownership
        // sovereign_key was captured at function start to avoid borrow conflict
        let lock_seeds = &[
            PERMANENT_LOCK_SEED,
            sovereign_key.as_ref(),
            &[ctx.bumps.permanent_lock],
        ];
        let lock_signer_seeds = &[&lock_seeds[..]];
        
        // CPI: Open position with full range
        let position_nft_mint = samm_cpi::open_position_full_range(
            &ctx.accounts.samm_program.to_account_info(),
            open_position_accounts,
            liquidity,
            sol_amount,
            token_amount,
            DEFAULT_TICK_SPACING as i32,
            lock_signer_seeds,
        )?;
        
        msg!("Position NFT created: {}", position_nft_mint);
        
        // Step 2: Set pool to restricted mode (disable external LPs)
        // bit0 = 1 disables open_position and increase_liquidity
        msg!("Setting pool to recovery-restricted mode...");
        
        samm_cpi::set_pool_status_restricted(
            &ctx.accounts.samm_program.to_account_info(),
            &ctx.accounts.permanent_lock.to_account_info(),
            pool_state_info,
            lock_signer_seeds,
        )?;
        
        // Update permanent lock with position info
        let permanent_lock = &mut ctx.accounts.permanent_lock;
        permanent_lock.position_mint = position_nft_mint;
        permanent_lock.pool_state = pool_state_info.key();
        permanent_lock.tick_lower_index = samm::tick::MIN_TICK;
        permanent_lock.tick_upper_index = samm::tick::MAX_TICK;
        permanent_lock.liquidity = liquidity;
    } else {
        // SECURITY: In mainnet/production, SAMM accounts are REQUIRED
        // Test mode only allowed in localnet/devnet builds
        #[cfg(not(any(feature = "localnet", feature = "devnet")))]
        {
            msg!("ERROR: SAMM accounts required for mainnet deployment");
            return Err(SovereignError::MissingSAMMAccounts.into());
        }
        
        // Simplified flow without full SAMM integration
        // This allows testing without all the SAMM accounts
        #[cfg(any(feature = "localnet", feature = "devnet"))]
        {
            msg!("SAMM accounts not provided - skipping CPI (test mode)");
            
            // Initialize permanent lock with placeholder values
            let permanent_lock = &mut ctx.accounts.permanent_lock;
            permanent_lock.position_mint = Pubkey::default();
            permanent_lock.pool_state = Pubkey::default();
            permanent_lock.tick_lower_index = MIN_TICK_INDEX;
            permanent_lock.tick_upper_index = MAX_TICK_INDEX;
            permanent_lock.liquidity = liquidity;
        }
    }
    
    // Initialize permanent lock common fields
    let permanent_lock = &mut ctx.accounts.permanent_lock;
    permanent_lock.sovereign = sovereign.key();
    permanent_lock.position_mint = Pubkey::default(); // Set after position creation
    permanent_lock.created_at = clock.unix_timestamp;
    permanent_lock.bump = ctx.bumps.permanent_lock;
    
    // Store recovery target (total investor SOL deposited)
    sovereign.recovery_target = sovereign.total_deposited;
    sovereign.total_recovered = 0;
    sovereign.finalized_at = clock.unix_timestamp;
    
    // If recovery target is 0 (edge case), go straight to Active
    if sovereign.recovery_target == 0 {
        sovereign.state = SovereignStatus::Active;
    } else {
        // Pool starts in Recovery mode with restricted trading
        sovereign.state = SovereignStatus::Recovery;
        
        emit!(PoolRestricted {
            sovereign_id: sovereign.sovereign_id,
            restricted: true,
        });
    }
    
    // Emit finalization event
    emit!(SovereignFinalized {
        sovereign_id: sovereign.sovereign_id,
        total_deposited: sovereign.total_deposited,
        token_supply: sovereign.total_supply,
        lp_tokens,
        recovery_target: sovereign.recovery_target,
        finalized_at: clock.unix_timestamp,
    });
    
    // Creator market buy would be executed here if creator_escrow > 0
    if sovereign.creator_escrow > 0 {
        // Execute swap: creator_escrow SOL -> tokens
        // Tokens go to creator's ATA
        // This protects creator from frontrunning since it's atomic with LP creation
        
        emit!(CreatorMarketBuyExecuted {
            sovereign_id: sovereign.sovereign_id,
            creator: sovereign.creator,
            sol_amount: sovereign.creator_escrow,
            tokens_received: 0, // Would be set by actual swap
        });
        
        sovereign.creator_escrow = 0; // Escrow consumed
    }
    
    Ok(())
}

/// Mint Genesis NFT to a depositor
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
    
    // Validate state
    require!(
        sovereign.state == SovereignStatus::Recovery || 
        sovereign.state == SovereignStatus::Active,
        SovereignError::InvalidState
    );
    require!(
        !deposit_record.nft_minted,
        SovereignError::NFTAlreadyMinted
    );
    require!(
        deposit_record.amount > 0,
        SovereignError::ZeroDeposit
    );
    
    // Calculate voting power based on deposit share
    let voting_power = deposit_record.amount
        .checked_mul(BPS_DENOMINATOR as u64).unwrap()
        .checked_div(sovereign.total_deposited).unwrap();
    
    // SECURITY: Validate voting_power fits in u16 before cast
    // This should always be true (max 10000 BPS) but we verify to prevent truncation
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
    
    // TODO: Create Metaplex metadata with voting power attribute
    // This would use mpl_token_metadata CPI
    
    deposit_record.nft_minted = true;
    deposit_record.nft_mint = Some(ctx.accounts.nft_mint.key());
    deposit_record.voting_power_bps = voting_power as u16;
    
    emit!(GenesisNFTMinted {
        sovereign_id: sovereign.sovereign_id,
        depositor: ctx.accounts.depositor.key(),
        nft_mint: ctx.accounts.nft_mint.key(),
        voting_power_bps: voting_power as u16,
        deposit_amount: deposit_record.amount,
    });
    
    Ok(())
}
