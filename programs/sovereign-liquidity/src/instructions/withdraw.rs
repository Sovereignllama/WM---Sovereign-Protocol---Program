use anchor_lang::prelude::*;
use anchor_lang::solana_program::rent::Rent;
use crate::state::*;
use crate::constants::*;
use crate::errors::SovereignError;
use crate::events::InvestorWithdrew;

#[derive(Accounts)]
pub struct Withdraw<'info> {
    #[account(mut)]
    pub depositor: Signer<'info>,
    
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
        mut,
        seeds = [DEPOSIT_RECORD_SEED, sovereign.key().as_ref(), depositor.key().as_ref()],
        bump = deposit_record.bump,
        constraint = deposit_record.depositor == depositor.key() @ SovereignError::Unauthorized
    )]
    pub deposit_record: Account<'info, DepositRecord>,
    
    /// CHECK: PDA that holds SOL
    #[account(
        mut,
        seeds = [SOL_VAULT_SEED, sovereign.key().as_ref()],
        bump
    )]
    pub sol_vault: SystemAccount<'info>,
    
    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<Withdraw>, amount: u64) -> Result<()> {
    let sovereign = &mut ctx.accounts.sovereign;
    let deposit_record = &mut ctx.accounts.deposit_record;
    let protocol = &ctx.accounts.protocol_state;
    let _clock = Clock::get()?;
    
    // Check protocol pause status
    require!(
        !protocol.paused,
        SovereignError::ProtocolPaused
    );
    
    // Validate state - only during Bonding phase
    require!(
        sovereign.state == SovereignStatus::Bonding,
        SovereignError::InvalidState
    );
    require!(amount > 0, SovereignError::ZeroWithdraw);
    require!(
        deposit_record.amount >= amount,
        SovereignError::InsufficientDepositBalance
    );
    
    // Validate vault has sufficient balance (prevents accounting mismatch exploits)
    let vault_balance = ctx.accounts.sol_vault.lamports();
    require!(
        vault_balance >= amount,
        SovereignError::InsufficientVaultBalance
    );
    
    // Creator cannot withdraw escrow during bonding
    require!(
        ctx.accounts.depositor.key() != sovereign.creator,
        SovereignError::CreatorCannotWithdrawDuringBonding
    );
    
    // Transfer SOL from vault to depositor using System Program CPI with PDA signer
    let sovereign_key = sovereign.key();
    let vault_seeds: &[&[u8]] = &[
        SOL_VAULT_SEED,
        sovereign_key.as_ref(),
        &[ctx.bumps.sol_vault],
    ];
    
    // Safe transfer: ensure vault maintains rent exemption
    let rent = Rent::get()?;
    let min_rent = rent.minimum_balance(0);
    
    // Ensure vault retains minimum rent-exempt balance
    let available_balance = ctx.accounts.sol_vault.lamports().saturating_sub(min_rent);
    require!(
        available_balance >= amount,
        SovereignError::InsufficientVaultBalance
    );
    
    anchor_lang::system_program::transfer(
        CpiContext::new_with_signer(
            ctx.accounts.system_program.to_account_info(),
            anchor_lang::system_program::Transfer {
                from: ctx.accounts.sol_vault.to_account_info(),
                to: ctx.accounts.depositor.to_account_info(),
            },
            &[vault_seeds],
        ),
        amount,
    )?;
    
    // Update deposit record
    deposit_record.amount = deposit_record.amount.checked_sub(amount).unwrap();
    sovereign.total_deposited = sovereign.total_deposited.checked_sub(amount).unwrap();
    
    // If fully withdrawn, decrement depositor count
    if deposit_record.amount == 0 {
        sovereign.depositor_count = sovereign.depositor_count.checked_sub(1).unwrap();
    }
    
    emit!(InvestorWithdrew {
        sovereign_id: sovereign.sovereign_id,
        depositor: ctx.accounts.depositor.key(),
        amount,
        remaining_deposit: deposit_record.amount,
        total_deposited: sovereign.total_deposited,
    });
    
    Ok(())
}
