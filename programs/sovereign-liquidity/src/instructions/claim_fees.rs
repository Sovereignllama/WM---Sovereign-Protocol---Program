use anchor_lang::prelude::*;
use anchor_spl::token::Token;
use anchor_lang::prelude::InterfaceAccount;
use crate::state::*;
use crate::constants::*;
use crate::errors::SovereignError;
use crate::events::{FeesClaimed, RecoveryComplete, PoolRestricted};
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
    
    /// Fee destination for SOL fees
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
    
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

pub fn handler<'info>(ctx: Context<'_, '_, 'info, 'info, ClaimFees<'info>>) -> Result<()> {
    let sovereign = &mut ctx.accounts.sovereign;
    let protocol = &ctx.accounts.protocol_state;
    let creator_tracker = &mut ctx.accounts.creator_fee_tracker;
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
    // [0] nft_account - Position NFT token account
    // [1] personal_position - Personal position state (writable)
    // [2] pool_state - Pool state (writable)
    // [3] protocol_position - Protocol position state (writable)
    // [4] token_vault_0 - Pool token vault A (writable)
    // [5] token_vault_1 - Pool token vault B (writable)
    // [6] tick_array_lower - Lower tick array (writable)
    // [7] tick_array_upper - Upper tick array (writable)
    // [8] recipient_token_account_0 - Recipient for token A fees (writable)
    // [9] recipient_token_account_1 - Recipient for token B fees (writable)
    // [10] token_program_2022 - Token 2022 program (optional)
    // [11] memo_program - Memo program (optional)
    // [12] vault_0_mint - Vault 0 mint
    // [13] vault_1_mint - Vault 1 mint
    
    let (sol_fees_collected, token_fees_collected) = if ctx.remaining_accounts.len() >= 14 {
        // SECURITY: Validate pool_state matches the sovereign's stored pool_state
        // This prevents attackers from passing arbitrary pool accounts
        require!(
            ctx.remaining_accounts[2].key() == ctx.accounts.permanent_lock.pool_state,
            SovereignError::InvalidPool
        );
        
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
        let result = samm_cpi::collect_fees(
            &ctx.accounts.samm_program.to_account_info(),
            decrease_accounts,
            lock_signer_seeds,
        )?;
        
        msg!("Fees collected - SOL: {}, Token: {}", result.amount_0, result.amount_1);
        
        (result.amount_0, result.amount_1)
    } else {
        // Simplified flow without SAMM CPI (test mode)
        msg!("SAMM accounts not provided - skipping CPI (test mode)");
        (0u64, 0u64)
    };
    
    // ============ Fee Distribution Logic ============
    // Based on FeeMode:
    // - CreatorRevenue: Creator gets up to fee_threshold_bps, rest to investors
    // - RecoveryBoost: All fees to recovery until complete
    // - FairLaunch: All fees to investors, no creator share
    
    let mut creator_fee_share: u64 = 0;
    let mut investor_fee_share: u64 = sol_fees_collected;
    
    if sovereign.state == SovereignStatus::Recovery {
        // During recovery: ALL fees go to recovery regardless of fee mode
        // This accelerates return of investor principal
        investor_fee_share = sol_fees_collected;
        creator_fee_share = 0;
        
        // Track recovery progress
        sovereign.total_recovered = sovereign.total_recovered
            .checked_add(sol_fees_collected)
            .unwrap();
        
        // Check if recovery is complete
        if sovereign.total_recovered >= sovereign.recovery_target {
            // Recovery complete - transition to Active
            sovereign.state = SovereignStatus::Active;
            
            // Unlock the pool via SAMM CPI (remove LP restrictions)
            // This allows external LPs to enter the pool
            if ctx.remaining_accounts.len() >= 14 {
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
    } else if sovereign.state == SovereignStatus::Active {
        // Post-recovery fee distribution based on fee mode
        match sovereign.fee_mode {
            FeeMode::CreatorRevenue => {
                // Creator gets up to fee_threshold_bps
                let max_creator_share = sol_fees_collected
                    .checked_mul(sovereign.fee_threshold_bps as u64).unwrap()
                    .checked_div(BPS_DENOMINATOR as u64).unwrap();
                
                // If creator has renounced fees (fee_threshold_bps = 0), they get nothing
                if sovereign.fee_threshold_bps > 0 && !creator_tracker.threshold_renounced {
                    creator_fee_share = max_creator_share;
                    investor_fee_share = sol_fees_collected.checked_sub(creator_fee_share).unwrap();
                }
            },
            FeeMode::RecoveryBoost => {
                // All fees continue to investors (was for recovery acceleration)
                creator_fee_share = 0;
                investor_fee_share = sol_fees_collected;
            },
            FeeMode::FairLaunch => {
                // All fees to investors
                creator_fee_share = 0;
                investor_fee_share = sol_fees_collected;
            },
        }
    }
    
    // Update creator fee tracker
    if creator_fee_share > 0 {
        creator_tracker.total_earned = creator_tracker.total_earned
            .checked_add(creator_fee_share)
            .unwrap();
        creator_tracker.pending_withdrawal = creator_tracker.pending_withdrawal
            .checked_add(creator_fee_share)
            .unwrap();
    }
    
    // Protocol fee (taken from total before distribution) - using unwind_fee_bps for fee collection
    let protocol_fee = sol_fees_collected
        .checked_mul(protocol.unwind_fee_bps as u64).unwrap()
        .checked_div(BPS_DENOMINATOR as u64).unwrap();
    
    // Investor share is reduced by protocol fee
    investor_fee_share = investor_fee_share.saturating_sub(protocol_fee);
    
    // Update sovereign tracking
    sovereign.total_fees_collected = sovereign.total_fees_collected
        .checked_add(sol_fees_collected)
        .unwrap();
    
    emit!(FeesClaimed {
        sovereign_id: sovereign.sovereign_id,
        sol_fees: sol_fees_collected,
        token_fees: token_fees_collected,
        creator_share: creator_fee_share,
        investor_share: investor_fee_share,
        protocol_share: protocol_fee,
        total_recovered: sovereign.total_recovered,
        recovery_target: sovereign.recovery_target,
    });
    
    Ok(())
}

/// Claim individual depositor's share of fees
#[derive(Accounts)]
pub struct ClaimDepositorFees<'info> {
    #[account(mut)]
    pub depositor: Signer<'info>,
    
    #[account(
        seeds = [SOVEREIGN_SEED, &sovereign.sovereign_id.to_le_bytes()],
        bump = sovereign.bump
    )]
    pub sovereign: Account<'info, SovereignState>,
    
    #[account(
        mut,
        seeds = [DEPOSIT_RECORD_SEED, sovereign.key().as_ref(), depositor.key().as_ref()],
        bump = deposit_record.bump,
        constraint = deposit_record.depositor == depositor.key() @ SovereignError::Unauthorized
    )]
    pub deposit_record: Account<'info, DepositRecord>,
    
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
        let vault_info = ctx.accounts.fee_vault.to_account_info();
        let vault_balance = vault_info.lamports();
        
        require!(
            vault_balance >= claimable,
            SovereignError::InsufficientVaultBalance
        );
        
        let depositor_info = ctx.accounts.depositor.to_account_info();
        
        // Atomic update: first update claimed amount, then transfer
        // This order prevents reentrancy-style exploits
        deposit_record.fees_claimed = deposit_record.fees_claimed
            .checked_add(claimable)
            .ok_or(SovereignError::Overflow)?;
        
        // Transfer from fee vault to depositor
        let vault_current = vault_info.lamports();
        let depositor_current = depositor_info.lamports();
        
        **vault_info.try_borrow_mut_lamports()? = vault_current
            .checked_sub(claimable)
            .ok_or(SovereignError::InsufficientVaultBalance)?;
        **depositor_info.try_borrow_mut_lamports()? = depositor_current
            .checked_add(claimable)
            .ok_or(SovereignError::Overflow)?;
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
    
    // Transfer from fee vault to creator
    let vault_info = ctx.accounts.fee_vault.to_account_info();
    let creator_info = ctx.accounts.creator.to_account_info();
    
    **vault_info.try_borrow_mut_lamports()? = vault_info
        .lamports()
        .checked_sub(amount)
        .ok_or(SovereignError::InsufficientVaultBalance)?;
    **creator_info.try_borrow_mut_lamports()? = creator_info
        .lamports()
        .checked_add(amount)
        .ok_or(SovereignError::Overflow)?;
    
    Ok(())
}

// ============================================================
// HARVEST TRANSFER FEES (Token-2022 TransferFeeConfig)
// ============================================================

use anchor_spl::token_2022::{
    Token2022,
    spl_token_2022::{
        self,
        extension::transfer_fee::instruction as transfer_fee_ix,
    },
};
use anchor_spl::token_interface::{Mint as MintInterface, TokenAccount as TokenAccountInterface};
use anchor_lang::solana_program::program::invoke_signed;
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
