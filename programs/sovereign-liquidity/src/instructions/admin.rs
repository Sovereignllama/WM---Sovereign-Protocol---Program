use anchor_lang::prelude::*;
use crate::state::*;
use crate::constants::*;
use crate::errors::SovereignError;
use crate::events::{ProtocolFeesUpdated, FeeThresholdUpdated, FeeThresholdRenounced};

/// Update protocol-level fee parameters
/// Only callable by protocol authority
#[derive(Accounts)]
pub struct UpdateProtocolFees<'info> {
    #[account(
        address = protocol_state.authority @ SovereignError::Unauthorized
    )]
    pub authority: Signer<'info>,
    
    #[account(
        mut,
        seeds = [PROTOCOL_STATE_SEED],
        bump = protocol_state.bump
    )]
    pub protocol_state: Account<'info, ProtocolState>,
}

pub fn update_protocol_fees_handler(
    ctx: Context<UpdateProtocolFees>,
    new_creation_fee_bps: Option<u16>,
    new_min_fee_lamports: Option<u64>,
    new_min_deposit: Option<u64>,
    new_min_bond_target: Option<u64>,
) -> Result<()> {
    let protocol = &mut ctx.accounts.protocol_state;
    
    // Update creation fee (max 10% = 1000 BPS)
    if let Some(fee_bps) = new_creation_fee_bps {
        require!(fee_bps <= MAX_CREATION_FEE_BPS, SovereignError::FeeTooHigh);
        protocol.creation_fee_bps = fee_bps;
    }
    
    // Update min fee lamports
    if let Some(fee) = new_min_fee_lamports {
        protocol.min_fee_lamports = fee;
    }
    
    // Update minimum deposit
    if let Some(min) = new_min_deposit {
        require!(min > 0, SovereignError::InvalidAmount);
        protocol.min_deposit = min;
    }
    
    // Update min bond target
    if let Some(min) = new_min_bond_target {
        require!(min > 0, SovereignError::InvalidAmount);
        protocol.min_bond_target = min;
    }
    
    emit!(ProtocolFeesUpdated {
        creation_fee_bps: protocol.creation_fee_bps,
        min_fee_lamports: protocol.min_fee_lamports,
        min_deposit: protocol.min_deposit,
        min_bond_target: protocol.min_bond_target,
    });
    
    Ok(())
}

/// Transfer protocol authority to new address
#[derive(Accounts)]
pub struct TransferProtocolAuthority<'info> {
    #[account(
        address = protocol_state.authority @ SovereignError::Unauthorized
    )]
    pub authority: Signer<'info>,
    
    /// CHECK: New authority address
    pub new_authority: UncheckedAccount<'info>,
    
    #[account(
        mut,
        seeds = [PROTOCOL_STATE_SEED],
        bump = protocol_state.bump
    )]
    pub protocol_state: Account<'info, ProtocolState>,
}

pub fn transfer_protocol_authority_handler(ctx: Context<TransferProtocolAuthority>) -> Result<()> {
    let protocol = &mut ctx.accounts.protocol_state;
    protocol.authority = ctx.accounts.new_authority.key();
    Ok(())
}

/// Update creator's fee threshold for a specific sovereign
/// Only callable by sovereign creator
#[derive(Accounts)]
pub struct UpdateFeeThreshold<'info> {
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
    
    #[account(
        mut,
        seeds = [CREATOR_FEE_TRACKER_SEED, sovereign.key().as_ref()],
        bump
    )]
    pub creator_fee_tracker: Account<'info, CreatorFeeTracker>,
}

pub fn update_fee_threshold_handler(
    ctx: Context<UpdateFeeThreshold>,
    new_threshold_bps: u16,
) -> Result<()> {
    let sovereign = &mut ctx.accounts.sovereign;
    let tracker = &ctx.accounts.creator_fee_tracker;
    
    // Cannot update if already renounced
    require!(
        !tracker.threshold_renounced,
        SovereignError::FeeThresholdRenounced
    );
    
    // Cannot increase threshold, only decrease
    require!(
        new_threshold_bps <= sovereign.fee_threshold_bps,
        SovereignError::CannotIncreaseFeeThreshold
    );
    
    // Must be within valid range (0 = no fees, max = DEFAULT_FEE_THRESHOLD_BPS)
    require!(
        new_threshold_bps <= DEFAULT_FEE_THRESHOLD_BPS,
        SovereignError::InvalidFeeThreshold
    );
    
    let old_threshold = sovereign.fee_threshold_bps;
    sovereign.fee_threshold_bps = new_threshold_bps;
    
    emit!(FeeThresholdUpdated {
        sovereign_id: sovereign.sovereign_id,
        old_threshold_bps: old_threshold,
        new_threshold_bps,
    });
    
    Ok(())
}

/// Permanently renounce fee threshold - sets to 0 forever
/// This is irreversible!
#[derive(Accounts)]
pub struct RenounceFeeThreshold<'info> {
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
    
    #[account(
        mut,
        seeds = [CREATOR_FEE_TRACKER_SEED, sovereign.key().as_ref()],
        bump
    )]
    pub creator_fee_tracker: Account<'info, CreatorFeeTracker>,
}

pub fn renounce_fee_threshold_handler(ctx: Context<RenounceFeeThreshold>) -> Result<()> {
    let sovereign = &mut ctx.accounts.sovereign;
    let tracker = &mut ctx.accounts.creator_fee_tracker;
    
    // Cannot renounce twice
    require!(
        !tracker.threshold_renounced,
        SovereignError::FeeThresholdAlreadyRenounced
    );
    
    let old_threshold = sovereign.fee_threshold_bps;
    
    // Set threshold to 0 permanently
    sovereign.fee_threshold_bps = 0;
    tracker.threshold_renounced = true;
    
    emit!(FeeThresholdRenounced {
        sovereign_id: sovereign.sovereign_id,
        old_threshold_bps: old_threshold,
        renounced_by: ctx.accounts.creator.key(),
    });
    
    Ok(())
}

/// Pause/unpause the protocol (emergency use)
#[derive(Accounts)]
pub struct SetProtocolPaused<'info> {
    #[account(
        address = protocol_state.authority @ SovereignError::Unauthorized
    )]
    pub authority: Signer<'info>,
    
    #[account(
        mut,
        seeds = [PROTOCOL_STATE_SEED],
        bump = protocol_state.bump
    )]
    pub protocol_state: Account<'info, ProtocolState>,
}

pub fn set_protocol_paused_handler(ctx: Context<SetProtocolPaused>, paused: bool) -> Result<()> {
    let protocol = &mut ctx.accounts.protocol_state;
    protocol.paused = paused;
    Ok(())
}
