use anchor_lang::prelude::*;
use anchor_spl::token_interface::{Mint, TokenAccount};
use spl_transfer_hook_interface::instruction::ExecuteInstruction;
use spl_tlv_account_resolution::{
    account::ExtraAccountMeta,
    seeds::Seed,
    state::ExtraAccountMetaList,
};
use crate::state::*;
use crate::constants::*;
use crate::errors::SovereignError;
use crate::events::TransferHookExecuted;

/// Extra account metas PDA seed
pub const EXTRA_ACCOUNT_METAS_SEED: &[u8] = b"extra-account-metas";

/// Initialize the extra account metas for the transfer hook
/// This tells Token-2022 which additional accounts to pass to our hook
#[derive(Accounts)]
pub struct InitializeExtraAccountMetas<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    
    /// The token mint this hook is for
    pub mint: InterfaceAccount<'info, Mint>,
    
    /// The sovereign state for this token
    #[account(
        seeds = [SOVEREIGN_SEED, &sovereign.sovereign_id.to_le_bytes()],
        bump = sovereign.bump,
        constraint = sovereign.token_mint == mint.key() @ SovereignError::InvalidMint
    )]
    pub sovereign: Account<'info, SovereignState>,
    
    /// Extra account metas PDA - stores the list of extra accounts needed by the hook
    /// CHECK: Will be initialized with ExtraAccountMetaList
    #[account(
        init,
        payer = payer,
        space = ExtraAccountMetaList::size_of(1).unwrap(), // 1 extra account: sovereign
        seeds = [EXTRA_ACCOUNT_METAS_SEED, mint.key().as_ref()],
        bump
    )]
    pub extra_account_metas: UncheckedAccount<'info>,
    
    pub system_program: Program<'info, System>,
}

/// Transfer hook execute - called automatically by Token-2022 on every transfer
/// This is where we implement sell fee logic
#[derive(Accounts)]
pub struct TransferHookExecute<'info> {
    /// Source token account
    pub source: InterfaceAccount<'info, TokenAccount>,
    
    /// The token mint
    pub mint: InterfaceAccount<'info, Mint>,
    
    /// Destination token account
    pub destination: InterfaceAccount<'info, TokenAccount>,
    
    /// CHECK: Source token account authority, validated by Token-2022 transfer hook interface
    pub authority: AccountInfo<'info>,
    
    /// Extra account metas PDA
    /// CHECK: Validated by Token-2022
    #[account(
        seeds = [EXTRA_ACCOUNT_METAS_SEED, mint.key().as_ref()],
        bump
    )]
    pub extra_account_metas: UncheckedAccount<'info>,
    
    /// Sovereign state - passed as extra account
    /// We read sell_fee_bps, fee_mode, and state from here
    #[account(
        seeds = [SOVEREIGN_SEED, &sovereign.sovereign_id.to_le_bytes()],
        bump = sovereign.bump,
        constraint = sovereign.token_mint == mint.key() @ SovereignError::InvalidMint
    )]
    pub sovereign: Account<'info, SovereignState>,
}

/// Initialize extra account metas for the transfer hook
/// Must be called after create_token to set up the hook properly
pub fn initialize_extra_account_metas_handler(
    ctx: Context<InitializeExtraAccountMetas>,
) -> Result<()> {
    let sovereign = &ctx.accounts.sovereign;
    
    // Define the extra accounts the hook needs
    // We need the sovereign state to read fee configuration
    let extra_metas = [
        // Sovereign state PDA - derived from SOVEREIGN_SEED + sovereign_id
        ExtraAccountMeta::new_with_seeds(
            &[
                Seed::Literal {
                    bytes: SOVEREIGN_SEED.to_vec(),
                },
                Seed::Literal {
                    bytes: sovereign.sovereign_id.to_le_bytes().to_vec(),
                },
            ],
            false, // is_signer
            false, // is_writable (we only read)
        )?,
    ];
    
    // Write the extra account metas to the PDA
    let account_info = ctx.accounts.extra_account_metas.to_account_info();
    let mut data = account_info.try_borrow_mut_data()?;
    ExtraAccountMetaList::init::<ExecuteInstruction>(&mut data, &extra_metas)?;
    
    Ok(())
}

/// Transfer hook execute handler
/// Called automatically by Token-2022 on every transfer
/// 
/// For sells (transfers to AMM pools), we calculate and route fees based on:
/// - sovereign.sell_fee_bps (0-300 = 0-3%)
/// - sovereign.fee_mode (CreatorRevenue, RecoveryBoost, FairLaunch)
/// - sovereign.state (Recovery vs Active)
pub fn transfer_hook_execute_handler(
    ctx: Context<TransferHookExecute>,
    amount: u64,
) -> Result<()> {
    let sovereign = &ctx.accounts.sovereign;
    let destination = &ctx.accounts.destination;
    
    // Skip if no sell fee configured
    if sovereign.sell_fee_bps == 0 {
        return Ok(());
    }
    
    // Check if this is a sell (transfer to a pool/AMM)
    // We detect sells by checking if destination is a pool token account
    // For now, we check if destination owner is not a regular wallet
    // In production, you'd check against known AMM program IDs
    let is_sell = is_likely_amm_destination(&destination.owner);
    
    if !is_sell {
        // Not a sell, no fee
        return Ok(());
    }
    
    // Calculate fee
    let fee_amount = calculate_fee(amount, sovereign.sell_fee_bps);
    
    if fee_amount == 0 {
        return Ok(());
    }
    
    // NOTE: Transfer hooks cannot modify the transfer amount or redirect tokens
    // directly. The fee collection happens through a different mechanism:
    //
    // Option 1: Use TransferFeeConfig extension (simpler, built into Token-2022)
    // Option 2: Use a separate "collect fees" instruction after transfers
    //
    // For SLP, we'll use TransferFeeConfig extension which handles fee
    // collection automatically. This hook is for additional validation/tracking.
    //
    // The actual fee routing (to creator vs recovery pool) happens in claim_fees
    
    emit!(TransferHookExecuted {
        sovereign_id: sovereign.sovereign_id,
        mint: ctx.accounts.mint.key(),
        source: ctx.accounts.source.key(),
        destination: ctx.accounts.destination.key(),
        amount,
        fee_amount,
        is_sell,
        fee_mode: sovereign.fee_mode,
    });
    
    Ok(())
}

/// Check if destination is likely an AMM pool
/// This is a heuristic - in production, maintain a list of known AMM program IDs
fn is_likely_amm_destination(owner: &Pubkey) -> bool {
    // Known AMM programs on Solana/Gorbagana
    // Add more as needed
    let amm_programs = [
        // Raydium CLMM (Trashbin SAMM on Gorbagana)
        pubkey!("WTzkPUoprVx7PDc1tfKA5sS7k1ynCgU89WtwZhksHX5"),
        // Raydium AMM v4
        pubkey!("675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8"),
        // Orca Whirlpool
        pubkey!("whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc"),
    ];
    
    amm_programs.contains(owner)
}

/// Calculate fee amount based on basis points
fn calculate_fee(amount: u64, fee_bps: u16) -> u64 {
    (amount as u128 * fee_bps as u128 / BPS_100_PERCENT as u128) as u64
}

/// Fallback instruction for transfer hook interface compliance
/// Token-2022 may call this for certain operations
#[derive(Accounts)]
pub struct TransferHookFallback<'info> {
    /// CHECK: Fallback accepts any accounts
    pub account: UncheckedAccount<'info>,
}

pub fn transfer_hook_fallback_handler(_ctx: Context<TransferHookFallback>) -> Result<()> {
    // No-op fallback
    Ok(())
}
