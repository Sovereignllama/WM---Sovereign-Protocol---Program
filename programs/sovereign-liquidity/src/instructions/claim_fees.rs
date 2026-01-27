use anchor_lang::prelude::*;
use anchor_spl::token::Token;
use crate::state::*;
use crate::constants::*;
use crate::errors::SovereignError;
use crate::events::{FeesClaimed, RecoveryComplete, PoolRestricted};

/// Claim fees from the Whirlpool position
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
    
    /// CHECK: Whirlpool position account - MUST match permanent_lock.position_mint
    #[account(
        mut,
        constraint = position.key() == permanent_lock.position_mint @ SovereignError::InvalidPosition
    )]
    pub position: UncheckedAccount<'info>,
    
    /// CHECK: Token vault A (SOL/WSOL side) - validated via Whirlpool CPI
    #[account(mut)]
    pub token_vault_a: UncheckedAccount<'info>,
    
    /// CHECK: Token vault B (token side) - validated via Whirlpool CPI
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
    
    /// CHECK: Whirlpool program
    #[account(address = WHIRLPOOL_PROGRAM_ID)]
    pub whirlpool_program: UncheckedAccount<'info>,
    
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<ClaimFees>) -> Result<()> {
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
    
    // ============ Whirlpool Fee Collection ============
    // This would be a CPI to the Whirlpool program to collect fees
    // from the position. For now, we simulate the flow.
    
    // In production:
    // 1. CPI to whirlpool::collect_fees
    // 2. Get SOL and token amounts collected
    // 3. SOL goes to fee_vault for distribution
    // 4. Tokens get handled based on fee mode
    
    let sol_fees_collected: u64 = 0; // Would come from Whirlpool CPI
    let token_fees_collected: u64 = 0; // Would come from Whirlpool CPI
    
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
            
            // Unlock the pool (remove restrictions)
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
    
    tracker.pending_withdrawal = 0;
    tracker.total_claimed = tracker.total_claimed.checked_add(amount).unwrap();
    
    Ok(())
}
