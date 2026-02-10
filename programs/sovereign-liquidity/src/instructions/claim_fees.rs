use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount, spl_token};
use anchor_lang::prelude::InterfaceAccount;
use anchor_spl::token_2022::{
    Token2022,
    spl_token_2022::{
        self,
        extension::transfer_fee::instruction as transfer_fee_ix,
    },
};
use anchor_spl::token_interface::{Mint as MintInterface, TokenAccount as TokenAccountInterface};
use anchor_lang::solana_program::program::invoke_signed;
use crate::state::*;
use crate::constants::*;
use crate::errors::SovereignError;
use crate::events::{FeesClaimed, RecoveryComplete, PoolRestricted, SellFeeRenounced, RecoveryTokensSwapped};
use crate::samm::{instructions as samm_ix, cpi as samm_cpi};

/// Claim fees from the Trashbin SAMM position
/// Fees are distributed to depositors and track recovery progress
#[derive(Accounts)]
pub struct ClaimFees<'info> {
    #[account(mut)]
    pub claimer: Signer<'info>,
    
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
    
    #[account(
        seeds = [PERMANENT_LOCK_SEED, sovereign.key().as_ref()],
        bump = permanent_lock.bump
    )]
    pub permanent_lock: Account<'info, PermanentLock>,
    
    /// CHECK: SAMM position account - validated via position NFT ownership by permanent_lock
    /// The position is derived from the position_mint NFT held by permanent_lock PDA
    #[account(mut)]
    pub position: UncheckedAccount<'info>,
    
    /// CHECK: Token vault 0 (GOR/WGOR side) - validated via SAMM CPI
    #[account(mut)]
    pub token_vault_a: UncheckedAccount<'info>,
    
    /// CHECK: Token vault 1 (token side) - validated via SAMM CPI
    #[account(mut)]
    pub token_vault_b: UncheckedAccount<'info>,
    
    /// Fee destination for GOR fees
    /// CHECK: PDA that collects fees
    #[account(
        mut,
        seeds = [SOL_VAULT_SEED, sovereign.key().as_ref()],
        bump
    )]
    pub fee_vault: SystemAccount<'info>,
    
    /// Creator fee tracker
    #[account(
        mut,
        seeds = [CREATOR_FEE_TRACKER_SEED, sovereign.key().as_ref()],
        bump
    )]
    pub creator_fee_tracker: Account<'info, CreatorFeeTracker>,
    
    /// CHECK: Trashbin SAMM program
    #[account(address = SAMM_PROGRAM_ID)]
    pub samm_program: UncheckedAccount<'info>,
    
    /// Token mint — needed for FairLaunch auto-renounce on recovery completion
    /// Optional: pass system_program if sovereign has no transfer fee
    /// CHECK: Validated against sovereign.token_mint when used
    #[account(mut)]
    pub token_mint: UncheckedAccount<'info>,
    
    pub token_program: Program<'info, Token>,
    pub token_program_2022: Program<'info, Token2022>,
    pub system_program: Program<'info, System>,
}

pub fn handler<'info>(ctx: Context<'_, '_, 'info, 'info, ClaimFees<'info>>) -> Result<()> {
    let sovereign = &mut ctx.accounts.sovereign;
    let protocol = &ctx.accounts.protocol_state;
    let _creator_tracker = &ctx.accounts.creator_fee_tracker;
    let clock = Clock::get()?;
    
    // Check protocol pause status
    require!(
        !protocol.paused,
        SovereignError::ProtocolPaused
    );
    
    // Validate state - fees can be claimed during Recovery or Active
    require!(
        sovereign.state == SovereignStatus::Recovery || 
        sovereign.state == SovereignStatus::Active,
        SovereignError::InvalidState
    );
    
    // ============ Trashbin SAMM Fee Collection ============
    // CPI to SAMM decrease_liquidity_v2 with liquidity=0 (collects fees only)
    // Required remaining_accounts order:
    // [0]  nft_account - Position NFT token account
    // [1]  personal_position - Personal position state (writable)
    // [2]  pool_state - Pool state (writable)
    // [3]  protocol_position - Protocol position state (writable)
    // [4]  token_vault_0 - Pool token vault A (writable)
    // [5]  token_vault_1 - Pool token vault B (writable)
    // [6]  tick_array_lower - Lower tick array (writable)
    // [7]  tick_array_upper - Upper tick array (writable)
    // [8]  recipient_token_account_0 - Recipient for token A fees (writable)
    // [9]  recipient_token_account_1 - Recipient for token B fees (writable)
    // [10] token_program_2022 - Token 2022 program
    // [11] memo_program - Memo program
    // [12] vault_0_mint - Vault 0 mint
    // [13] vault_1_mint - Vault 1 mint
    // [14] tick_array_bitmap_extension
    //
    // --- Token fee routing (optional, index 15+) ---
    // [15] amm_config           - SAMM AMM config (for swap path)
    // [16] observation_state    - SAMM observation state (for swap path)
    // [17] creator_token_ata    - Creator's Token-2022 ATA (for creator/active paths)
    // [18+] swap_tick_arrays    - Tick arrays for token→WGOR swap
    
    let (sol_fees_collected, token_fees_collected) = if ctx.remaining_accounts.len() >= 15 {
        // SECURITY: Validate pool_state matches the sovereign's stored pool_state
        // This prevents attackers from passing arbitrary pool accounts
        require!(
            ctx.remaining_accounts[2].key() == ctx.accounts.permanent_lock.pool_state,
            SovereignError::InvalidPool
        );
        
        // Determine which recipient is WGOR vs project token based on vault mints
        let vault_0_mint_key = ctx.remaining_accounts[12].key();
        let _vault_1_mint_key = ctx.remaining_accounts[13].key();
        let wgor_is_0 = vault_0_mint_key == WGOR_MINT;
        
        let wgor_recipient_idx: usize = if wgor_is_0 { 8 } else { 9 };
        let token_recipient_idx: usize = if wgor_is_0 { 9 } else { 8 };
        
        // Snapshot token recipient ATA balance before CPI (for tracking token fees)
        let token_balance_before = {
            let data = ctx.remaining_accounts[token_recipient_idx].try_borrow_data()?;
            if data.len() >= 72 {
                u64::from_le_bytes(data[64..72].try_into().unwrap())
            } else {
                0u64
            }
        };
        
        // Full SAMM CPI for fee collection
        msg!("Collecting fees via SAMM CPI...");
        
        // Build decrease_liquidity_v2 accounts (with liquidity=0 for fee collection only)
        let decrease_accounts = samm_ix::DecreaseLiquidityV2Accounts {
            nft_owner: ctx.accounts.permanent_lock.to_account_info(),
            nft_account: ctx.remaining_accounts[0].clone(),
            personal_position: ctx.remaining_accounts[1].clone(),
            pool_state: ctx.remaining_accounts[2].clone(),
            protocol_position: ctx.remaining_accounts[3].clone(),
            token_vault_0: ctx.remaining_accounts[4].clone(),
            token_vault_1: ctx.remaining_accounts[5].clone(),
            tick_array_lower: ctx.remaining_accounts[6].clone(),
            tick_array_upper: ctx.remaining_accounts[7].clone(),
            recipient_token_account_0: ctx.remaining_accounts[8].clone(),
            recipient_token_account_1: ctx.remaining_accounts[9].clone(),
            token_program: ctx.accounts.token_program.to_account_info(),
            token_program_2022: ctx.remaining_accounts[10].clone(),
            memo_program: ctx.remaining_accounts[11].clone(),
            vault_0_mint: ctx.remaining_accounts[12].clone(),
            vault_1_mint: ctx.remaining_accounts[13].clone(),
            tick_array_bitmap_extension: ctx.remaining_accounts[14].clone(),
        };
        
        // Use permanent_lock as signer (it owns the position NFT)
        let sovereign_key = sovereign.key();
        let lock_seeds = &[
            PERMANENT_LOCK_SEED,
            sovereign_key.as_ref(),
            &[ctx.accounts.permanent_lock.bump],
        ];
        let lock_signer_seeds = &[&lock_seeds[..]];
        
        // CPI: Collect fees (decrease_liquidity_v2 with liquidity=0)
        samm_cpi::collect_fees(
            &ctx.accounts.samm_program.to_account_info(),
            decrease_accounts,
            lock_signer_seeds,
        )?;
        
        // Read TOTAL WGOR ATA balance after harvest CPI.
        // This captures both newly harvested fees AND any leftover from previous calls.
        let wgor_balance_after_harvest = {
            let data = ctx.remaining_accounts[wgor_recipient_idx].try_borrow_data()?;
            if data.len() >= 72 {
                u64::from_le_bytes(data[64..72].try_into().unwrap())
            } else {
                0u64
            }
        };
        
        // Read TOTAL token ATA balance after harvest (includes any leftovers)
        let token_total_balance = {
            let data = ctx.remaining_accounts[token_recipient_idx].try_borrow_data()?;
            if data.len() >= 72 {
                u64::from_le_bytes(data[64..72].try_into().unwrap())
            } else {
                0u64
            }
        };
        let token_collected = token_total_balance.saturating_sub(token_balance_before);
        
        msg!("WGOR in ATA: {}, Token in ATA: {} (new: {})", 
            wgor_balance_after_harvest, token_total_balance, token_collected);
        
        // ============ Token Fee Routing ============
        // Route the token portion based on fee_mode + sovereign state.
        // The WGOR portion always goes to investors (handled below by closing WGOR ATA).
        //
        // Routing:
        //   RecoveryBoost + Recovery  → swap tokens → WGOR ATA (added to GOR for investors)
        //   FairLaunch + Recovery     → swap tokens → WGOR ATA (added to GOR for investors)
        //   RecoveryBoost + Active    → transfer tokens → creator's ATA
        //   FairLaunch + Active       → burn tokens (deflationary, benefits holders)
        //   CreatorRevenue (any)      → transfer tokens → creator's ATA
        
        if token_total_balance > 0 && ctx.remaining_accounts.len() >= 18 {
            let fee_mode = sovereign.fee_mode;
            let is_recovery = sovereign.state == SovereignStatus::Recovery;
            
            let swap_to_investors = is_recovery && 
                (fee_mode == FeeMode::RecoveryBoost || fee_mode == FeeMode::FairLaunch);
            let burn_tokens = !is_recovery && fee_mode == FeeMode::FairLaunch;
            let send_to_creator = fee_mode == FeeMode::CreatorRevenue || 
                (!is_recovery && fee_mode == FeeMode::RecoveryBoost);
            
            if swap_to_investors {
                // ---- SWAP PATH: token → WGOR via SAMM CPI ----
                // permanent_lock signs (owns the token ATA as input)
                msg!("Swapping {} tokens → WGOR for investor recovery...", token_total_balance);
                
                // Determine SAMM vault ordering for the swap
                // Input = sovereign token, Output = WGOR
                let (input_vault, output_vault) = if wgor_is_0 {
                    // mint0=WGOR, mint1=token → vault_0=WGOR, vault_1=token
                    // Input is token (vault_1), output is WGOR (vault_0)
                    (ctx.remaining_accounts[5].clone(), ctx.remaining_accounts[4].clone())
                } else {
                    // mint0=token, mint1=WGOR → vault_0=token, vault_1=WGOR
                    (ctx.remaining_accounts[4].clone(), ctx.remaining_accounts[5].clone())
                };
                
                let (input_mint, output_mint) = if wgor_is_0 {
                    (ctx.remaining_accounts[13].clone(), ctx.remaining_accounts[12].clone())
                } else {
                    (ctx.remaining_accounts[12].clone(), ctx.remaining_accounts[13].clone())
                };
                
                // Collect tick arrays for swap from remaining_accounts[18+]
                let swap_tick_arrays: Vec<AccountInfo<'info>> = ctx.remaining_accounts[18..].to_vec();
                
                let swap_accounts = samm_ix::SwapV2Accounts {
                    payer: ctx.accounts.permanent_lock.to_account_info(),
                    amm_config: ctx.remaining_accounts[15].clone(),
                    pool_state: ctx.remaining_accounts[2].clone(),
                    input_token_account: ctx.remaining_accounts[token_recipient_idx].clone(),
                    output_token_account: ctx.remaining_accounts[wgor_recipient_idx].clone(),
                    input_vault,
                    output_vault,
                    observation_state: ctx.remaining_accounts[16].clone(),
                    token_program: ctx.accounts.token_program.to_account_info(),
                    token_program_2022: ctx.remaining_accounts[10].clone(),
                    memo_program: ctx.remaining_accounts[11].clone(),
                    input_vault_mint: input_mint,
                    output_vault_mint: output_mint,
                };
                
                samm_cpi::swap_exact_input(
                    &ctx.accounts.samm_program.to_account_info(),
                    swap_accounts,
                    token_total_balance,
                    0, // min_amount_out — permissionless, protocol token, no MEV risk
                    0, // no sqrt_price_limit
                    swap_tick_arrays,
                    lock_signer_seeds,
                )?;
                
                msg!("Token → WGOR swap complete");
                
            } else if burn_tokens {
                // ---- BURN PATH: FairLaunch + Active → burn tokens ----
                msg!("Burning {} tokens (FairLaunch deflationary)...", token_total_balance);
                
                let burn_ix = spl_token_2022::instruction::burn(
                    &spl_token_2022::ID,
                    &ctx.remaining_accounts[token_recipient_idx].key(),
                    &ctx.accounts.token_mint.key(),
                    &ctx.accounts.permanent_lock.key(),
                    &[],
                    token_total_balance,
                )?;
                
                invoke_signed(
                    &burn_ix,
                    &[
                        ctx.remaining_accounts[token_recipient_idx].clone(),
                        ctx.accounts.token_mint.to_account_info(),
                        ctx.accounts.permanent_lock.to_account_info(),
                    ],
                    lock_signer_seeds,
                )?;
                
                msg!("Burned {} tokens", token_total_balance);
                
            } else if send_to_creator {
                // ---- CREATOR PATH: CreatorRevenue or RecoveryBoost+Active → creator ATA ----
                msg!("Transferring {} tokens to creator...", token_total_balance);
                
                // Read decimals from raw mint data (offset 44 in SPL Mint layout)
                let mint_decimals = {
                    let mint_data = ctx.accounts.token_mint.try_borrow_data()?;
                    mint_data[44]
                };
                
                let transfer_ix = spl_token_2022::instruction::transfer_checked(
                    &spl_token_2022::ID,
                    &ctx.remaining_accounts[token_recipient_idx].key(),
                    &ctx.accounts.token_mint.key(),
                    &ctx.remaining_accounts[17].key(), // creator_token_ata
                    &ctx.accounts.permanent_lock.key(),
                    &[],
                    token_total_balance,
                    mint_decimals,
                )?;
                
                invoke_signed(
                    &transfer_ix,
                    &[
                        ctx.remaining_accounts[token_recipient_idx].clone(),
                        ctx.accounts.token_mint.to_account_info(),
                        ctx.remaining_accounts[17].clone(), // creator_token_ata
                        ctx.accounts.permanent_lock.to_account_info(),
                    ],
                    lock_signer_seeds,
                )?;
                
                msg!("Transferred {} tokens to creator", token_total_balance);
            }
        } else if token_total_balance > 0 {
            msg!("Token routing accounts not provided — tokens remain in permanent_lock ATA");
        }
        
        // ============ Solvency-Protected GOR Extraction ============
        // Read the FINAL WGOR balance (includes harvest GOR + any tokens swapped to WGOR).
        let wgor_final_balance = {
            let data = ctx.remaining_accounts[wgor_recipient_idx].try_borrow_data()?;
            if data.len() >= 72 {
                u64::from_le_bytes(data[64..72].try_into().unwrap())
            } else {
                0u64
            }
        };
        
        // ---- Principal Protection Invariant ----
        // Compute the maximum extractable GOR while preserving the property:
        //   "If all tokens were sold back into the pool, GOR reserve ≥ bond_target"
        //
        // Formula: e_max = max(0, x_final - (x0 * S_total / y_final) * 1.001)
        // Where:
        //   x_final = current GOR in pool vault (WGOR balance)
        //   y_final = current token in pool vault
        //   x0 = sovereign.bond_target (original investor principal)
        //   S_total = token mint supply
        //   1.001 = 0.1% safety buffer for CLMM rounding
        
        let wgor_vault_idx: usize = if wgor_is_0 { 4 } else { 5 };
        let token_vault_idx: usize = if wgor_is_0 { 5 } else { 4 };
        
        let x_final: u128 = {
            let data = ctx.remaining_accounts[wgor_vault_idx].try_borrow_data()?;
            if data.len() >= 72 {
                u64::from_le_bytes(data[64..72].try_into().unwrap()) as u128
            } else { 0u128 }
        };
        let y_final: u128 = {
            let data = ctx.remaining_accounts[token_vault_idx].try_borrow_data()?;
            if data.len() >= 72 {
                u64::from_le_bytes(data[64..72].try_into().unwrap()) as u128
            } else { 0u128 }
        };
        let s_total: u128 = {
            let mint_data = ctx.accounts.token_mint.try_borrow_data()?;
            if mint_data.len() >= 44 {
                u64::from_le_bytes(mint_data[36..44].try_into().unwrap()) as u128
            } else { 0u128 }
        };
        let x0: u128 = sovereign.bond_target as u128;
        
        // Minimum GOR that must remain in pool: (x0 * S_total / y_final) * 1.001
        let min_reserve: u128 = if y_final > 0 {
            x0.checked_mul(s_total).unwrap_or(u128::MAX)
                .checked_div(y_final).unwrap_or(0)
                .checked_mul(1001).unwrap_or(u128::MAX)
                .checked_div(1000).unwrap_or(u128::MAX)
        } else {
            u128::MAX // No tokens in pool → don't extract anything
        };
        
        let e_max: u64 = if x_final > min_reserve {
            (x_final - min_reserve) as u64
        } else {
            0u64
        };
        
        let extractable = std::cmp::min(wgor_final_balance, e_max);
        let return_to_pool = wgor_final_balance - extractable;
        
        msg!("Solvency check: x_final={}, y_final={}, s_total={}, x0={}, min_reserve={}, e_max={}, extractable={}, return_to_pool={}",
            x_final, y_final, s_total, x0, min_reserve, e_max, extractable, return_to_pool);
        
        if wgor_final_balance > 0 {
            // Return excess WGOR to pool vault to preserve principal
            if return_to_pool > 0 {
                let return_ix = spl_token::instruction::transfer(
                    &spl_token::ID,
                    &ctx.remaining_accounts[wgor_recipient_idx].key(),
                    &ctx.remaining_accounts[wgor_vault_idx].key(),
                    &ctx.accounts.permanent_lock.key(),
                    &[],
                    return_to_pool,
                )?;
                
                invoke_signed(
                    &return_ix,
                    &[
                        ctx.remaining_accounts[wgor_recipient_idx].clone(),
                        ctx.remaining_accounts[wgor_vault_idx].clone(),
                        ctx.accounts.permanent_lock.to_account_info(),
                    ],
                    lock_signer_seeds,
                )?;
                
                msg!("Returned {} WGOR to pool vault (principal protection)", return_to_pool);
            }
            
            // Close WGOR ATA → remaining extractable goes to fee_vault as native GOR
            let close_ix = spl_token::instruction::close_account(
                &spl_token::ID,
                &ctx.remaining_accounts[wgor_recipient_idx].key(),
                &ctx.accounts.fee_vault.key(),
                &ctx.accounts.permanent_lock.key(),
                &[],
            )?;
            
            invoke_signed(
                &close_ix,
                &[
                    ctx.remaining_accounts[wgor_recipient_idx].clone(),
                    ctx.accounts.fee_vault.to_account_info(),
                    ctx.accounts.permanent_lock.to_account_info(),
                ],
                lock_signer_seeds,
            )?;
            
            msg!("WGOR ATA closed → {} lamports to fee_vault (solvency-protected)", extractable);
        }
        
        (extractable, token_collected)
    } else {
        // Simplified flow without SAMM CPI (test mode)
        msg!("SAMM accounts not provided - skipping CPI (test mode)");
        (0u64, 0u64)
    };
    
    // ============ SAMM Trading Fee Distribution ============
    // SAMM LP trading fees ALWAYS go 100% to investors (GOR portion).
    // Token portion is routed based on fee_mode above.
    
    let investor_fee_share: u64 = sol_fees_collected;
    
    if sovereign.state == SovereignStatus::Recovery {
        // Track recovery progress
        sovereign.total_recovered = sovereign.total_recovered
            .checked_add(sol_fees_collected)
            .unwrap();
        
        // Check if recovery is complete
        if sovereign.total_recovered >= sovereign.recovery_target {
            // Recovery complete - transition to Active
            sovereign.state = SovereignStatus::Active;
            sovereign.recovery_complete = true;
            
            // Unlock the pool via SAMM CPI (remove LP restrictions)
            // This allows external LPs to enter the pool
            if ctx.remaining_accounts.len() >= 15 {
                let pool_state_info = &ctx.remaining_accounts[2];
                let sovereign_key = sovereign.key();
                let lock_seeds = &[
                    PERMANENT_LOCK_SEED,
                    sovereign_key.as_ref(),
                    &[ctx.accounts.permanent_lock.bump],
                ];
                let lock_signer_seeds = &[&lock_seeds[..]];
                
                samm_cpi::set_pool_status_unrestricted(
                    &ctx.accounts.samm_program.to_account_info(),
                    &ctx.accounts.permanent_lock.to_account_info(),
                    pool_state_info,
                    lock_signer_seeds,
                )?;
                
                msg!("Pool restrictions removed - external LPs can now enter");
            }
            
            // FairLaunch: auto-renounce sell fee on recovery completion
            // Sets transfer fee to 0% and marks as renounced
            if sovereign.fee_mode == FeeMode::FairLaunch && !sovereign.fee_control_renounced {
                let old_fee = sovereign.sell_fee_bps;
                let sovereign_id_bytes = sovereign.sovereign_id.to_le_bytes();
                let sov_seeds = &[
                    SOVEREIGN_SEED,
                    &sovereign_id_bytes[..],
                    &[sovereign.bump],
                ];
                let sov_signer = &[&sov_seeds[..]];
                
                // Set transfer fee to 0
                let set_fee_ix = transfer_fee_ix::set_transfer_fee(
                    &spl_token_2022::ID,
                    &ctx.accounts.token_mint.key(),
                    &sovereign.key(),
                    &[],
                    0, // 0% fee
                    0, // max fee = 0
                )?;
                
                invoke_signed(
                    &set_fee_ix,
                    &[
                        ctx.accounts.token_mint.to_account_info(),
                        sovereign.to_account_info(),
                    ],
                    sov_signer,
                )?;
                
                sovereign.sell_fee_bps = 0;
                sovereign.fee_control_renounced = true;
                
                emit!(SellFeeRenounced {
                    sovereign_id: sovereign.sovereign_id,
                    old_fee_bps: old_fee,
                    renounced_by: ctx.accounts.claimer.key(),
                });
                
                msg!("FairLaunch: sell fee auto-renounced to 0%");
            }
            
            emit!(PoolRestricted {
                sovereign_id: sovereign.sovereign_id,
                restricted: false,
            });
            
            emit!(RecoveryComplete {
                sovereign_id: sovereign.sovereign_id,
                total_recovered: sovereign.total_recovered,
                recovery_target: sovereign.recovery_target,
                completed_at: clock.unix_timestamp,
            });
        }
    }
    // Active state: SAMM trading fees still go 100% to investors, no special logic needed
    
    // Update sovereign tracking
    sovereign.total_fees_collected = sovereign.total_fees_collected
        .checked_add(sol_fees_collected)
        .unwrap();
    
    emit!(FeesClaimed {
        sovereign_id: sovereign.sovereign_id,
        sol_fees: sol_fees_collected,
        token_fees: token_fees_collected,
        creator_share: 0,
        investor_share: investor_fee_share,
        protocol_share: 0,
        total_recovered: sovereign.total_recovered,
        recovery_target: sovereign.recovery_target,
    });
    
    Ok(())
}

/// Claim individual depositor's share of fees
/// Authorization is purely via Genesis NFT possession (bearer instrument).
/// The NFT holder — not necessarily the original depositor — receives fees.
#[derive(Accounts)]
pub struct ClaimDepositorFees<'info> {
    /// Current NFT holder (bearer of the position)
    #[account(mut)]
    pub holder: Signer<'info>,
    
    #[account(
        seeds = [SOVEREIGN_SEED, &sovereign.sovereign_id.to_le_bytes()],
        bump = sovereign.bump
    )]
    pub sovereign: Account<'info, SovereignState>,
    
    /// CHECK: Original depositor wallet — used only for deposit_record PDA derivation.
    /// Verified implicitly by PDA seed match.
    pub original_depositor: UncheckedAccount<'info>,
    
    #[account(
        mut,
        seeds = [DEPOSIT_RECORD_SEED, sovereign.key().as_ref(), original_depositor.key().as_ref()],
        bump = deposit_record.bump,
    )]
    pub deposit_record: Account<'info, DepositRecord>,
    
    /// Genesis NFT token account — proves the holder possesses the position NFT
    #[account(
        constraint = nft_token_account.amount == 1 @ SovereignError::NoGenesisNFT,
        constraint = nft_token_account.mint == deposit_record.nft_mint.unwrap() @ SovereignError::WrongNFT,
        constraint = nft_token_account.owner == holder.key() @ SovereignError::Unauthorized,
    )]
    pub nft_token_account: Account<'info, TokenAccount>,
    
    /// CHECK: Fee vault holding accumulated fees
    #[account(
        mut,
        seeds = [SOL_VAULT_SEED, sovereign.key().as_ref()],
        bump
    )]
    pub fee_vault: SystemAccount<'info>,
    
    pub system_program: Program<'info, System>,
}

pub fn claim_depositor_fees_handler(ctx: Context<ClaimDepositorFees>) -> Result<()> {
    let sovereign = &ctx.accounts.sovereign;
    let deposit_record = &mut ctx.accounts.deposit_record;
    
    // Validate state
    require!(
        sovereign.state == SovereignStatus::Recovery || 
        sovereign.state == SovereignStatus::Active,
        SovereignError::InvalidState
    );
    
    // Genesis NFT must have been minted
    require!(deposit_record.nft_minted, SovereignError::NFTNotMinted);
    
    // CRITICAL: Prevent division by zero
    require!(
        sovereign.total_deposited > 0,
        SovereignError::NoDeposits
    );
    
    // Calculate depositor's share based on their proportion of total deposits
    // Using safe arithmetic to prevent overflow
    let depositor_share_bps = deposit_record.amount
        .checked_mul(BPS_DENOMINATOR as u64)
        .ok_or(SovereignError::Overflow)?
        .checked_div(sovereign.total_deposited)
        .ok_or(SovereignError::DivisionByZero)? as u16;
    
    // Fee Index Pattern: Calculate claimable based on global index
    // claimable = (global_fees * share_bps / 10000) - already_claimed
    // This prevents double-claiming even with concurrent transactions
    
    let total_share = sovereign.total_fees_collected
        .checked_mul(depositor_share_bps as u64)
        .ok_or(SovereignError::Overflow)?
        .checked_div(BPS_DENOMINATOR as u64)
        .ok_or(SovereignError::DivisionByZero)?;
    
    let claimable = total_share
        .checked_sub(deposit_record.fees_claimed)
        .unwrap_or(0);
    
    if claimable > 0 {
        // Verify vault has sufficient balance
        let vault_balance = ctx.accounts.fee_vault.lamports();
        
        require!(
            vault_balance >= claimable,
            SovereignError::InsufficientVaultBalance
        );
        
        // Atomic update: first update claimed amount, then transfer
        // This order prevents reentrancy-style exploits
        deposit_record.fees_claimed = deposit_record.fees_claimed
            .checked_add(claimable)
            .ok_or(SovereignError::Overflow)?;
        
        // Transfer from fee vault to holder using System Program CPI
        let sovereign_key = sovereign.key();
        let vault_seeds: &[&[u8]] = &[
            SOL_VAULT_SEED,
            sovereign_key.as_ref(),
            &[ctx.bumps.fee_vault],
        ];
        
        anchor_lang::system_program::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.system_program.to_account_info(),
                anchor_lang::system_program::Transfer {
                    from: ctx.accounts.fee_vault.to_account_info(),
                    to: ctx.accounts.holder.to_account_info(),
                },
                &[vault_seeds],
            ),
            claimable,
        )?;
    }
    
    Ok(())
}

/// Creator withdraws their earned fees
#[derive(Accounts)]
pub struct WithdrawCreatorFees<'info> {
    #[account(
        mut,
        address = sovereign.creator @ SovereignError::Unauthorized
    )]
    pub creator: Signer<'info>,
    
    #[account(
        seeds = [SOVEREIGN_SEED, &sovereign.sovereign_id.to_le_bytes()],
        bump = sovereign.bump
    )]
    pub sovereign: Account<'info, SovereignState>,
    
    #[account(
        mut,
        seeds = [CREATOR_FEE_TRACKER_SEED, sovereign.key().as_ref()],
        bump
    )]
    pub creator_fee_tracker: Account<'info, CreatorFeeTracker>,
    
    /// CHECK: Fee vault holding creator fees
    #[account(
        mut,
        seeds = [SOL_VAULT_SEED, sovereign.key().as_ref()],
        bump
    )]
    pub fee_vault: SystemAccount<'info>,
    
    pub system_program: Program<'info, System>,
}

pub fn withdraw_creator_fees_handler(ctx: Context<WithdrawCreatorFees>) -> Result<()> {
    let tracker = &mut ctx.accounts.creator_fee_tracker;
    let _sovereign = &ctx.accounts.sovereign;
    
    require!(
        tracker.pending_withdrawal > 0,
        SovereignError::NothingToClaim
    );
    
    let amount = tracker.pending_withdrawal;
    
    // SECURITY: Update state BEFORE transfer (checks-effects-interactions pattern)
    // This prevents potential reentrancy-style exploits
    tracker.pending_withdrawal = 0;
    tracker.total_claimed = tracker.total_claimed.checked_add(amount).unwrap();
    
    // Transfer from fee vault to creator using System Program CPI
    let sovereign_key = _sovereign.key();
    let vault_seeds: &[&[u8]] = &[
        SOL_VAULT_SEED,
        sovereign_key.as_ref(),
        &[ctx.bumps.fee_vault],
    ];
    
    anchor_lang::system_program::transfer(
        CpiContext::new_with_signer(
            ctx.accounts.system_program.to_account_info(),
            anchor_lang::system_program::Transfer {
                from: ctx.accounts.fee_vault.to_account_info(),
                to: ctx.accounts.creator.to_account_info(),
            },
            &[vault_seeds],
        ),
        amount,
    )?;
    
    Ok(())
}

// ============================================================
// HARVEST TRANSFER FEES (Token-2022 TransferFeeConfig)
// ============================================================

use crate::events::TransferFeesHarvested;

/// Harvest withheld transfer fees from token accounts
/// These fees were automatically collected by Token-2022's TransferFeeConfig extension
/// 
/// Fee routing based on FeeMode:
/// - CreatorRevenue: Always to creator (even during recovery)
/// - RecoveryBoost: To recovery pool during recovery, then to creator
/// - FairLaunch: To recovery pool during recovery, then auto-renounce (no fees)
#[derive(Accounts)]
pub struct HarvestTransferFees<'info> {
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
    
    /// The token mint with TransferFeeConfig
    #[account(
        mut,
        address = sovereign.token_mint
    )]
    pub token_mint: InterfaceAccount<'info, MintInterface>,
    
    /// Creator's token account - receives fees in CreatorRevenue mode
    /// or after recovery in RecoveryBoost mode
    #[account(
        mut,
        token::mint = token_mint,
    )]
    pub creator_token_account: InterfaceAccount<'info, TokenAccountInterface>,
    
    /// Recovery pool token account - receives fees during recovery
    /// (except in CreatorRevenue mode)
    #[account(
        mut,
        token::mint = token_mint,
        seeds = [TOKEN_VAULT_SEED, sovereign.key().as_ref()],
        bump
    )]
    pub recovery_token_vault: InterfaceAccount<'info, TokenAccountInterface>,
    
    /// Creator fee tracker
    #[account(
        mut,
        seeds = [CREATOR_FEE_TRACKER_SEED, sovereign.key().as_ref()],
        bump
    )]
    pub creator_fee_tracker: Account<'info, CreatorFeeTracker>,
    
    pub token_program_2022: Program<'info, Token2022>,
}

/// Harvest transfer fees from multiple token accounts
/// remaining_accounts: List of token accounts to harvest from
pub fn harvest_transfer_fees_handler<'info>(
    ctx: Context<'_, '_, 'info, 'info, HarvestTransferFees<'info>>,
) -> Result<()> {
    let sovereign = &mut ctx.accounts.sovereign;
    let protocol = &ctx.accounts.protocol_state;
    let _creator_tracker = &mut ctx.accounts.creator_fee_tracker;
    
    // Check protocol pause status
    require!(
        !protocol.paused,
        SovereignError::ProtocolPaused
    );
    
    // Can harvest during Recovery or Active
    require!(
        sovereign.state == SovereignStatus::Recovery || 
        sovereign.state == SovereignStatus::Active,
        SovereignError::InvalidState
    );
    
    // Only TokenLaunch sovereigns have transfer fees
    require!(
        sovereign.sovereign_type == SovereignType::TokenLaunch,
        SovereignError::InvalidSovereignType
    );
    
    // Derive sovereign PDA seeds for signing
    let sovereign_id_bytes = sovereign.sovereign_id.to_le_bytes();
    let sovereign_seeds = &[
        SOVEREIGN_SEED,
        &sovereign_id_bytes,
        &[sovereign.bump],
    ];
    let sovereign_signer = &[&sovereign_seeds[..]];
    
    // Collect source accounts from remaining_accounts
    let sources: Vec<&AccountInfo> = ctx.remaining_accounts.iter().collect();
    
    if sources.is_empty() {
        return Ok(()); // Nothing to harvest
    }
    
    // Determine fee destination based on fee_mode and current state
    let (fee_destination, to_creator) = match sovereign.fee_mode {
        FeeMode::CreatorRevenue => {
            // Creator ALWAYS gets fees, even during recovery
            (ctx.accounts.creator_token_account.to_account_info(), true)
        }
        FeeMode::RecoveryBoost => {
            if sovereign.state == SovereignStatus::Recovery {
                // During recovery: fees go to recovery pool
                (ctx.accounts.recovery_token_vault.to_account_info(), false)
            } else {
                // After recovery: fees go to creator
                (ctx.accounts.creator_token_account.to_account_info(), true)
            }
        }
        FeeMode::FairLaunch => {
            if sovereign.state == SovereignStatus::Recovery {
                // During recovery: fees go to recovery pool
                (ctx.accounts.recovery_token_vault.to_account_info(), false)
            } else {
                // After recovery: should be renounced, but if not, still go to recovery
                // (FairLaunch should auto-renounce on recovery complete)
                (ctx.accounts.recovery_token_vault.to_account_info(), false)
            }
        }
    };
    
    // Build harvest instruction - collect withheld fees to mint first
    let source_key_refs: Vec<&Pubkey> = sources.iter().map(|a| a.key).collect();
    
    let harvest_ix = transfer_fee_ix::harvest_withheld_tokens_to_mint(
        &spl_token_2022::ID,
        &ctx.accounts.token_mint.key(),
        &source_key_refs,
    )?;
    
    // Build account infos for CPI
    let mut account_infos = vec![ctx.accounts.token_mint.to_account_info()];
    for source in sources {
        account_infos.push(source.clone());
    }
    
    invoke_signed(
        &harvest_ix,
        &account_infos,
        sovereign_signer,
    )?;
    
    // Now withdraw the fees from mint to the determined destination
    let withdraw_ix = transfer_fee_ix::withdraw_withheld_tokens_from_mint(
        &spl_token_2022::ID,
        &ctx.accounts.token_mint.key(),
        &fee_destination.key(),
        &sovereign.key(), // Withdraw authority
        &[],
    )?;
    
    invoke_signed(
        &withdraw_ix,
        &[
            ctx.accounts.token_mint.to_account_info(),
            fee_destination,
            sovereign.to_account_info(),
        ],
        sovereign_signer,
    )?;
    
    // Track fees harvested
    // Note: We don't know exact amount here without reading account before/after
    // The frontend should calculate this from transaction logs
    
    emit!(TransferFeesHarvested {
        sovereign_id: sovereign.sovereign_id,
        fee_mode: sovereign.fee_mode,
        to_creator,
        source_count: source_key_refs.len() as u32,
    });
    
    msg!("Transfer fees harvested - to_creator: {}, fee_mode: {:?}", to_creator, sovereign.fee_mode);
    
    Ok(())
}

// ============================================================
// SWAP RECOVERY TOKENS → GOR (via SAMM CPI)
// ============================================================

/// Swap sovereign tokens from the recovery token vault into GOR (via SAMM).
/// This is step 2 of the recovery fee flow:
///   1. harvestTransferFees — collects Token-2022 withheld fees into recovery_token_vault
///   2. swapRecoveryTokens — swaps those tokens to GOR via SAMM and adds to fee_vault
///
/// Only callable during Recovery when fee_mode is RecoveryBoost or FairLaunch.
/// Anyone can call this (permissionless — benefits all depositors).
///
/// remaining_accounts order:
///   [0]  amm_config           — SAMM AMM config (readonly)
///   [1]  pool_state           — SAMM pool state (writable)
///   [2]  samm_input_vault     — SAMM pool vault for sovereign token (writable)
///   [3]  samm_output_vault    — SAMM pool vault for WGOR (writable)
///   [4]  observation_state    — SAMM observation state (writable)
///   [5]  token_program_2022   — Token-2022 program (readonly)
///   [6]  memo_program         — Memo program (readonly)
///   [7]  wgor_mint            — WGOR mint address (readonly)
///   [8..N] tick_arrays        — SAMM tick arrays for swap path (writable)
#[derive(Accounts)]
pub struct SwapRecoveryTokens<'info> {
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
    
    #[account(
        seeds = [PERMANENT_LOCK_SEED, sovereign.key().as_ref()],
        bump = permanent_lock.bump
    )]
    pub permanent_lock: Account<'info, PermanentLock>,
    
    /// Token mint (sovereign's Token-2022 mint)
    #[account(
        address = sovereign.token_mint
    )]
    pub token_mint: InterfaceAccount<'info, MintInterface>,
    
    /// Recovery token vault — holds harvested transfer fees (Token-2022)
    /// Owned by the sovereign PDA
    #[account(
        mut,
        token::mint = token_mint,
        seeds = [TOKEN_VAULT_SEED, sovereign.key().as_ref()],
        bump
    )]
    pub recovery_token_vault: InterfaceAccount<'info, TokenAccountInterface>,
    
    /// WGOR ATA for the sovereign PDA — intermediate account for swap output
    /// Created by the caller before this instruction (use createAssociatedTokenAccountIdempotent)
    /// CHECK: Validated as WGOR ATA owned by sovereign PDA
    #[account(
        mut,
        constraint = sovereign_wgor_ata.owner == sovereign.key() @ SovereignError::Unauthorized,
    )]
    pub sovereign_wgor_ata: Account<'info, TokenAccount>,
    
    /// Fee vault (sol_vault PDA) — destination for unwrapped GOR
    /// CHECK: PDA that collects fees for investors
    #[account(
        mut,
        seeds = [SOL_VAULT_SEED, sovereign.key().as_ref()],
        bump
    )]
    pub fee_vault: SystemAccount<'info>,
    
    /// CHECK: Trashbin SAMM program
    #[account(address = SAMM_PROGRAM_ID)]
    pub samm_program: UncheckedAccount<'info>,
    
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

pub fn swap_recovery_tokens_handler<'info>(
    ctx: Context<'_, '_, 'info, 'info, SwapRecoveryTokens<'info>>,
) -> Result<()> {
    let sovereign = &mut ctx.accounts.sovereign;
    let protocol = &ctx.accounts.protocol_state;
    let permanent_lock = &ctx.accounts.permanent_lock;
    
    // Checks
    require!(!protocol.paused, SovereignError::ProtocolPaused);
    require!(
        sovereign.state == SovereignStatus::Recovery,
        SovereignError::InvalidState
    );
    require!(
        sovereign.sovereign_type == SovereignType::TokenLaunch,
        SovereignError::InvalidSovereignType
    );
    // Only RecoveryBoost and FairLaunch route tokens to investors
    require!(
        sovereign.fee_mode == FeeMode::RecoveryBoost || 
        sovereign.fee_mode == FeeMode::FairLaunch,
        SovereignError::InvalidFeeMode
    );
    
    // Validate pool_state matches sovereign's stored pool
    require!(
        ctx.remaining_accounts.len() >= 9,
        SovereignError::InsufficientAccounts
    );
    require!(
        ctx.remaining_accounts[1].key() == permanent_lock.pool_state,
        SovereignError::InvalidPool
    );
    
    // Read token vault balance — this is what we'll swap
    let swap_amount = ctx.accounts.recovery_token_vault.amount;
    if swap_amount == 0 {
        msg!("No tokens in recovery vault to swap");
        return Ok(());
    }
    
    // Derive sovereign PDA signer
    let sovereign_id_bytes = sovereign.sovereign_id.to_le_bytes();
    let sovereign_seeds = &[
        SOVEREIGN_SEED,
        &sovereign_id_bytes[..],
        &[sovereign.bump],
    ];
    let sovereign_signer = &[&sovereign_seeds[..]];
    
    // Collect tick arrays from remaining_accounts[8..]
    let tick_arrays: Vec<AccountInfo<'info>> = ctx.remaining_accounts[8..].to_vec();
    
    // Build SAMM swap accounts
    let swap_accounts = samm_ix::SwapV2Accounts {
        payer: sovereign.to_account_info(),                        // Sovereign PDA signs (owns token vault)
        amm_config: ctx.remaining_accounts[0].clone(),             // AMM config
        pool_state: ctx.remaining_accounts[1].clone(),             // Pool state
        input_token_account: ctx.accounts.recovery_token_vault.to_account_info(), // Token vault (input)
        output_token_account: ctx.accounts.sovereign_wgor_ata.to_account_info(), // WGOR ATA (output)
        input_vault: ctx.remaining_accounts[2].clone(),            // SAMM vault for token
        output_vault: ctx.remaining_accounts[3].clone(),           // SAMM vault for WGOR
        observation_state: ctx.remaining_accounts[4].clone(),      // Observation state
        token_program: ctx.accounts.token_program.to_account_info(),
        token_program_2022: ctx.remaining_accounts[5].clone(),     // Token-2022 program
        memo_program: ctx.remaining_accounts[6].clone(),           // Memo program
        input_vault_mint: ctx.accounts.token_mint.to_account_info(), // Sovereign token mint
        output_vault_mint: ctx.remaining_accounts[7].clone(),      // WGOR mint
    };
    
    // Execute swap: tokens → WGOR (min_amount_out = 0 for now, slippage handled by caller)
    samm_cpi::swap_exact_input(
        &ctx.accounts.samm_program.to_account_info(),
        swap_accounts,
        swap_amount,
        0, // min_amount_out — accept any output (permissionless call, no MEV risk for protocol tokens)
        0, // sqrt_price_limit — 0 means no limit
        tick_arrays,
        sovereign_signer,
    )?;
    
    msg!("Swapped {} tokens for WGOR via SAMM", swap_amount);
    
    // Read WGOR balance received
    ctx.accounts.sovereign_wgor_ata.reload()?;
    let wgor_amount = ctx.accounts.sovereign_wgor_ata.amount;
    
    if wgor_amount > 0 {
        // Close WGOR ATA → unwraps WGOR to native GOR → lamports go to fee_vault
        let close_wgor_ix = anchor_lang::solana_program::instruction::Instruction {
            program_id: anchor_spl::token::ID,
            accounts: vec![
                anchor_lang::solana_program::instruction::AccountMeta::new(
                    ctx.accounts.sovereign_wgor_ata.key(), false,
                ),
                anchor_lang::solana_program::instruction::AccountMeta::new(
                    ctx.accounts.fee_vault.key(), false,
                ),
                anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                    sovereign.key(), true,
                ),
            ],
            data: vec![9u8], // SPL Token CloseAccount instruction discriminator
        };
        
        invoke_signed(
            &close_wgor_ix,
            &[
                ctx.accounts.sovereign_wgor_ata.to_account_info(),
                ctx.accounts.fee_vault.to_account_info(),
                sovereign.to_account_info(),
            ],
            sovereign_signer,
        )?;
        
        msg!("WGOR ATA closed → {} WGOR unwrapped to fee_vault", wgor_amount);
        
        // Update recovery tracking
        sovereign.total_recovered = sovereign.total_recovered
            .checked_add(wgor_amount)
            .unwrap();
        sovereign.total_fees_collected = sovereign.total_fees_collected
            .checked_add(wgor_amount)
            .unwrap();
        
        // Check if recovery is now complete
        if sovereign.total_recovered >= sovereign.recovery_target {
            sovereign.state = SovereignStatus::Active;
            sovereign.recovery_complete = true;
            
            msg!("Recovery complete! Transitioning to Active state");
            
            emit!(RecoveryComplete {
                sovereign_id: sovereign.sovereign_id,
                total_recovered: sovereign.total_recovered,
                recovery_target: sovereign.recovery_target,
                completed_at: Clock::get()?.unix_timestamp,
            });
        }
    }
    
    emit!(RecoveryTokensSwapped {
        sovereign_id: sovereign.sovereign_id,
        tokens_swapped: swap_amount,
        sol_received: wgor_amount,
        total_recovered: sovereign.total_recovered,
        recovery_target: sovereign.recovery_target,
    });
    
    Ok(())
}
