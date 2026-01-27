use anchor_lang::prelude::*;
use crate::state::ProtocolState;
use crate::constants::*;
use crate::errors::SovereignError;
use crate::events::ProtocolInitialized;

#[derive(Accounts)]
pub struct InitializeProtocol<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    
    #[account(
        init,
        payer = authority,
        space = ProtocolState::LEN,
        seeds = [PROTOCOL_STATE_SEED],
        bump
    )]
    pub protocol_state: Account<'info, ProtocolState>,
    
    /// CHECK: Treasury wallet to receive protocol fees - must not be zero address
    #[account(
        constraint = treasury.key() != Pubkey::default() @ SovereignError::InvalidTreasury
    )]
    pub treasury: UncheckedAccount<'info>,
    
    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<InitializeProtocol>) -> Result<()> {
    let protocol = &mut ctx.accounts.protocol_state;
    
    protocol.authority = ctx.accounts.authority.key();
    protocol.treasury = ctx.accounts.treasury.key();
    
    // Set default fee parameters
    protocol.creation_fee_bps = ProtocolState::default_creation_fee_bps();
    protocol.min_fee_lamports = ProtocolState::default_min_fee_lamports();
    protocol.governance_unwind_fee_lamports = ProtocolState::default_governance_unwind_fee();
    protocol.unwind_fee_bps = ProtocolState::default_unwind_fee_bps();
    
    // Set BYO token settings
    protocol.byo_min_supply_bps = ProtocolState::default_byo_min_supply_bps();
    
    // Set protocol limits
    protocol.min_bond_target = ProtocolState::default_min_bond_target();
    protocol.min_deposit = ProtocolState::default_min_deposit();
    protocol.auto_unwind_period = ProtocolState::default_auto_unwind_period();
    
    // Set activity check threshold
    protocol.min_fee_growth_threshold = ProtocolState::default_min_fee_growth_threshold();
    protocol.fee_threshold_renounced = false;
    
    // Initialize statistics
    protocol.sovereign_count = 0;
    protocol.total_fees_collected = 0;
    
    protocol.bump = ctx.bumps.protocol_state;
    
    emit!(ProtocolInitialized {
        authority: protocol.authority,
        treasury: protocol.treasury,
    });
    
    Ok(())
}
