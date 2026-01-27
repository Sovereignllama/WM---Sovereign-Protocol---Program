use anchor_lang::prelude::*;
use crate::state::*;
use crate::constants::*;
use crate::errors::SovereignError;
use crate::events::{ActivityCheckInitiated, ActivityCheckExecuted};

/// Initiate an activity check
/// Can be called by anyone to start the 90-day countdown
#[derive(Accounts)]
pub struct InitiateActivityCheck<'info> {
    #[account(mut)]
    pub initiator: Signer<'info>,
    
    #[account(
        mut,
        seeds = [SOVEREIGN_SEED, &sovereign.sovereign_id.to_le_bytes()],
        bump = sovereign.bump
    )]
    pub sovereign: Account<'info, SovereignState>,
}

pub fn initiate_activity_check_handler(ctx: Context<InitiateActivityCheck>) -> Result<()> {
    let sovereign = &mut ctx.accounts.sovereign;
    let clock = Clock::get()?;
    
    // Validate state - must be Active (not Recovery or Unwinding)
    require!(
        sovereign.state == SovereignStatus::Active,
        SovereignError::InvalidState
    );
    
    // Cannot initiate if already pending
    require!(
        sovereign.activity_check_initiated_at.is_none(),
        SovereignError::ActivityCheckAlreadyPending
    );
    
    // Cannot have active governance proposal
    require!(
        !sovereign.has_active_proposal,
        SovereignError::ActiveProposalExists
    );
    
    // CRITICAL: Enforce cooldown period after cancelled check
    // Prevents immediate re-initiation after creator cancels
    if sovereign.activity_check_last_cancelled > 0 {
        let cooldown_elapsed = clock.unix_timestamp
            .checked_sub(sovereign.activity_check_last_cancelled)
            .ok_or(SovereignError::Overflow)?;
        require!(
            cooldown_elapsed >= ACTIVITY_CHECK_COOLDOWN,
            SovereignError::ActivityCheckCooldownNotElapsed
        );
    }
    
    // Record initiation time - 90-day countdown starts now
    sovereign.activity_check_initiated_at = Some(clock.unix_timestamp);
    sovereign.activity_check_initiated = true;
    sovereign.activity_check_timestamp = clock.unix_timestamp;
    
    emit!(ActivityCheckInitiated {
        sovereign_id: sovereign.sovereign_id,
        initiator: ctx.accounts.initiator.key(),
        initiated_at: clock.unix_timestamp,
        execution_available_at: clock.unix_timestamp + ACTIVITY_CHECK_PERIOD_SECONDS,
    });
    
    Ok(())
}

/// Cancel an activity check
/// Creator can prove liveness by calling this
#[derive(Accounts)]
pub struct CancelActivityCheck<'info> {
    #[account(
        address = sovereign.creator @ SovereignError::Unauthorized
    )]
    pub creator: Signer<'info>,
    
    #[account(
        mut,
        seeds = [SOVEREIGN_SEED, &sovereign.sovereign_id.to_le_bytes()],
        bump = sovereign.bump
    )]
    pub sovereign: Account<'info, SovereignState>,
}

pub fn cancel_activity_check_handler(ctx: Context<CancelActivityCheck>) -> Result<()> {
    let sovereign = &mut ctx.accounts.sovereign;
    let clock = Clock::get()?;
    
    // Validate there's an active check
    require!(
        sovereign.activity_check_initiated_at.is_some(),
        SovereignError::NoActivityCheckPending
    );
    
    // Clear the activity check - creator has proven liveness
    sovereign.activity_check_initiated_at = None;
    sovereign.activity_check_initiated = false;
    sovereign.last_activity = clock.unix_timestamp;
    
    // CRITICAL: Record cancellation time for cooldown enforcement
    // This prevents immediate re-initiation by attackers
    sovereign.activity_check_last_cancelled = clock.unix_timestamp;
    
    Ok(())
}

/// Execute an activity check after 90 days
/// Triggers automatic unwind if creator hasn't responded
#[derive(Accounts)]
pub struct ExecuteActivityCheck<'info> {
    #[account(mut)]
    pub executor: Signer<'info>,
    
    #[account(
        mut,
        seeds = [SOVEREIGN_SEED, &sovereign.sovereign_id.to_le_bytes()],
        bump = sovereign.bump
    )]
    pub sovereign: Account<'info, SovereignState>,
}

pub fn execute_activity_check_handler(ctx: Context<ExecuteActivityCheck>) -> Result<()> {
    let sovereign = &mut ctx.accounts.sovereign;
    let clock = Clock::get()?;
    
    // Validate state
    require!(
        sovereign.state == SovereignStatus::Active,
        SovereignError::InvalidState
    );
    
    // Validate activity check was initiated
    let initiated_at = sovereign.activity_check_initiated_at
        .ok_or(SovereignError::NoActivityCheckPending)?;
    
    // Validate 90-day period has passed
    let time_elapsed = clock.unix_timestamp
        .checked_sub(initiated_at)
        .ok_or(SovereignError::Overflow)?;
    
    require!(
        time_elapsed >= ACTIVITY_CHECK_PERIOD_SECONDS,
        SovereignError::ActivityCheckPeriodNotElapsed
    );
    
    // Creator failed to respond - transition to Unwinding
    sovereign.state = SovereignStatus::Unwinding;
    sovereign.activity_check_initiated_at = None;
    sovereign.activity_check_initiated = false;
    
    emit!(ActivityCheckExecuted {
        sovereign_id: sovereign.sovereign_id,
        executor: ctx.accounts.executor.key(),
        executed_at: clock.unix_timestamp,
        days_elapsed: (time_elapsed / 86400) as u32,
    });
    
    Ok(())
}
