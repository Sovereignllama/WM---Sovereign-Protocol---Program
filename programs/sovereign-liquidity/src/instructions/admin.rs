use anchor_lang::prelude::*;
use anchor_lang::solana_program::program::invoke_signed;
use anchor_spl::token_2022::{
    Token2022,
    spl_token_2022::{
        self,
        extension::transfer_fee::instruction as transfer_fee_ix,
        instruction::{set_authority, AuthorityType},
    },
};
use anchor_spl::token_interface::Mint as MintInterface;
use crate::state::*;
use crate::constants::*;
use crate::errors::SovereignError;
use crate::events::{ProtocolFeesUpdated, FeeThresholdUpdated, FeeThresholdRenounced, SellFeeUpdated, SellFeeRenounced};

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

// ============================================================
// SELL FEE MANAGEMENT (TokenLaunch only)
// ============================================================

/// Update the sell fee for a TokenLaunch sovereign
/// Can adjust up to MAX_SELL_FEE_BPS (3%). Creator calls this.
#[derive(Accounts)]
pub struct UpdateSellFee<'info> {
    #[account(
        address = sovereign.creator @ SovereignError::NotCreator
    )]
    pub creator: Signer<'info>,
    
    #[account(
        mut,
        seeds = [SOVEREIGN_SEED, &sovereign.sovereign_id.to_le_bytes()],
        bump = sovereign.bump,
        constraint = sovereign.sovereign_type == SovereignType::TokenLaunch @ SovereignError::InvalidSovereignType,
        constraint = !sovereign.fee_control_renounced @ SovereignError::FeeControlRenounced
    )]
    pub sovereign: Account<'info, SovereignState>,
    
    /// The token mint with TransferFeeConfig
    #[account(
        mut,
        address = sovereign.token_mint
    )]
    pub token_mint: InterfaceAccount<'info, MintInterface>,
    
    pub token_program_2022: Program<'info, Token2022>,
}

pub fn update_sell_fee_handler(
    ctx: Context<UpdateSellFee>,
    new_fee_bps: u16,
) -> Result<()> {
    let sovereign = &mut ctx.accounts.sovereign;
    
    // Must be within valid range (0 to 3%)
    require!(
        new_fee_bps <= MAX_SELL_FEE_BPS,
        SovereignError::SellFeeExceedsMax
    );
    
    let old_fee = sovereign.sell_fee_bps;
    
    // Derive sovereign PDA seeds for signing
    let sovereign_id_bytes = sovereign.sovereign_id.to_le_bytes();
    let sovereign_seeds = &[
        SOVEREIGN_SEED,
        &sovereign_id_bytes,
        &[sovereign.bump],
    ];
    let sovereign_signer = &[&sovereign_seeds[..]];
    
    // Update the TransferFeeConfig on the mint
    let set_fee_ix = transfer_fee_ix::set_transfer_fee(
        &spl_token_2022::ID,
        &ctx.accounts.token_mint.key(),
        &sovereign.key(), // Transfer fee config authority
        &[],
        new_fee_bps,
        u64::MAX, // No max fee cap
    )?;
    
    invoke_signed(
        &set_fee_ix,
        &[
            ctx.accounts.token_mint.to_account_info(),
            sovereign.to_account_info(),
        ],
        sovereign_signer,
    )?;
    
    // Update sovereign state
    sovereign.sell_fee_bps = new_fee_bps;
    
    emit!(SellFeeUpdated {
        sovereign_id: sovereign.sovereign_id,
        old_fee_bps: old_fee,
        new_fee_bps,
        updated_by: ctx.accounts.creator.key(),
    });
    
    Ok(())
}

/// Permanently renounce sell fee control
/// Sets fee to 0% and removes authority - IRREVERSIBLE
/// Can only be called after recovery is complete (or anytime for FairLaunch mode)
#[derive(Accounts)]
pub struct RenounceSellFee<'info> {
    #[account(
        address = sovereign.creator @ SovereignError::NotCreator
    )]
    pub creator: Signer<'info>,
    
    #[account(
        mut,
        seeds = [SOVEREIGN_SEED, &sovereign.sovereign_id.to_le_bytes()],
        bump = sovereign.bump,
        constraint = sovereign.sovereign_type == SovereignType::TokenLaunch @ SovereignError::InvalidSovereignType,
        constraint = !sovereign.fee_control_renounced @ SovereignError::FeeControlRenounced
    )]
    pub sovereign: Account<'info, SovereignState>,
    
    /// The token mint with TransferFeeConfig
    #[account(
        mut,
        address = sovereign.token_mint
    )]
    pub token_mint: InterfaceAccount<'info, MintInterface>,
    
    pub token_program_2022: Program<'info, Token2022>,
}

pub fn renounce_sell_fee_handler(ctx: Context<RenounceSellFee>) -> Result<()> {
    let sovereign = &mut ctx.accounts.sovereign;
    
    // For FairLaunch mode, can renounce anytime
    // For other modes, must wait until recovery is complete
    if sovereign.fee_mode != FeeMode::FairLaunch {
        require!(
            sovereign.state == SovereignStatus::Active,
            SovereignError::RecoveryNotComplete
        );
    }
    
    let old_fee = sovereign.sell_fee_bps;
    
    // Derive sovereign PDA seeds for signing
    let sovereign_id_bytes = sovereign.sovereign_id.to_le_bytes();
    let sovereign_seeds = &[
        SOVEREIGN_SEED,
        &sovereign_id_bytes,
        &[sovereign.bump],
    ];
    let sovereign_signer = &[&sovereign_seeds[..]];
    
    // First, set the fee to 0
    let set_fee_ix = transfer_fee_ix::set_transfer_fee(
        &spl_token_2022::ID,
        &ctx.accounts.token_mint.key(),
        &sovereign.key(),
        &[],
        0, // Set to 0%
        0, // Max fee also 0
    )?;
    
    invoke_signed(
        &set_fee_ix,
        &[
            ctx.accounts.token_mint.to_account_info(),
            sovereign.to_account_info(),
        ],
        sovereign_signer,
    )?;
    
    // Then remove the transfer fee config authority (makes it immutable)
    // Setting authority to None means no one can ever change the fee again
    let set_authority_ix = set_authority(
        &spl_token_2022::ID,
        &ctx.accounts.token_mint.key(),
        None, // New authority = None (renounced)
        AuthorityType::TransferFeeConfig,
        &sovereign.key(), // Current authority
        &[], // No multisig signers
    )?;
    
    invoke_signed(
        &set_authority_ix,
        &[
            ctx.accounts.token_mint.to_account_info(),
            sovereign.to_account_info(),
        ],
        sovereign_signer,
    )?;
    
    // Update sovereign state
    sovereign.sell_fee_bps = 0;
    sovereign.fee_control_renounced = true;
    
    emit!(SellFeeRenounced {
        sovereign_id: sovereign.sovereign_id,
        old_fee_bps: old_fee,
        renounced_by: ctx.accounts.creator.key(),
    });
    
    Ok(())
}
