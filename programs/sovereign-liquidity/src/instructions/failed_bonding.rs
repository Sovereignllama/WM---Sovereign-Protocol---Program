use anchor_lang::prelude::*;
use crate::state::*;
use crate::constants::*;
use crate::errors::SovereignError;
use crate::events::{BondingFailed, FailedWithdrawal, CreatorFailedWithdrawal};

/// Mark bonding as failed if deadline passed without meeting target
#[derive(Accounts)]
pub struct MarkBondingFailed<'info> {
    #[account(mut)]
    pub caller: Signer<'info>,
    
    #[account(
        mut,
        seeds = [SOVEREIGN_SEED, &sovereign.sovereign_id.to_le_bytes()],
        bump = sovereign.bump
    )]
    pub sovereign: Account<'info, SovereignState>,
}

pub fn mark_bonding_failed_handler(ctx: Context<MarkBondingFailed>) -> Result<()> {
    let sovereign = &mut ctx.accounts.sovereign;
    let clock = Clock::get()?;
    
    // Validate state - must still be in Bonding
    require!(
        sovereign.state == SovereignStatus::Bonding,
        SovereignError::InvalidState
    );
    
    // Validate deadline has passed
    require!(
        sovereign.is_deadline_passed(clock.unix_timestamp),
        SovereignError::DeadlineNotPassed
    );
    
    // Bond target not met - mark as failed
    // Use < instead of != for safety
    require!(
        sovereign.total_deposited < sovereign.bond_target,
        SovereignError::BondTargetMet
    );
    
    // Atomic state transition
    sovereign.state = SovereignStatus::Failed;
    
    emit!(BondingFailed {
        sovereign_id: sovereign.sovereign_id,
        total_deposited: sovereign.total_deposited,
        bond_target: sovereign.bond_target,
        failed_at: clock.unix_timestamp,
    });
    
    Ok(())
}

/// Withdraw from a failed bonding (investor)
#[derive(Accounts)]
pub struct WithdrawFailed<'info> {
    #[account(mut)]
    pub depositor: Signer<'info>,
    
    #[account(
        seeds = [SOVEREIGN_SEED, &sovereign.sovereign_id.to_le_bytes()],
        bump = sovereign.bump
    )]
    pub sovereign: Account<'info, SovereignState>,
    
    #[account(
        mut,
        close = depositor,
        seeds = [DEPOSIT_RECORD_SEED, sovereign.key().as_ref(), depositor.key().as_ref()],
        bump = deposit_record.bump,
        constraint = deposit_record.depositor == depositor.key() @ SovereignError::Unauthorized
    )]
    pub deposit_record: Account<'info, DepositRecord>,
    
    /// CHECK: SOL vault
    #[account(
        mut,
        seeds = [SOL_VAULT_SEED, sovereign.key().as_ref()],
        bump
    )]
    pub sol_vault: SystemAccount<'info>,
    
    pub system_program: Program<'info, System>,
}

pub fn withdraw_failed_handler(ctx: Context<WithdrawFailed>) -> Result<()> {
    let sovereign = &ctx.accounts.sovereign;
    let deposit_record = &ctx.accounts.deposit_record;
    
    // Validate state
    require!(
        sovereign.state == SovereignStatus::Failed,
        SovereignError::InvalidState
    );
    
    // Cannot be creator (they use different instruction)
    require!(
        ctx.accounts.depositor.key() != sovereign.creator,
        SovereignError::CreatorMustUseCreatorWithdraw
    );
    
    let amount = deposit_record.amount;
    require!(amount > 0, SovereignError::NothingToWithdraw);
    
    // Verify vault has sufficient balance
    let vault_balance = ctx.accounts.sol_vault.lamports();
    require!(
        vault_balance >= amount,
        SovereignError::InsufficientVaultBalance
    );
    
    // Transfer SOL from vault to depositor using System Program CPI
    let sovereign_key = sovereign.key();
    let vault_seeds: &[&[u8]] = &[
        SOL_VAULT_SEED,
        sovereign_key.as_ref(),
        &[ctx.bumps.sol_vault],
    ];
    
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
    
    emit!(FailedWithdrawal {
        sovereign_id: sovereign.sovereign_id,
        depositor: ctx.accounts.depositor.key(),
        amount,
    });
    
    // Note: deposit_record is closed and rent returned to depositor
    
    Ok(())
}

/// Creator withdraws escrowed funds from failed bonding
#[derive(Accounts)]
pub struct WithdrawCreatorFailed<'info> {
    #[account(
        mut,
        address = sovereign.creator @ SovereignError::Unauthorized
    )]
    pub creator: Signer<'info>,
    
    #[account(
        mut,
        seeds = [SOVEREIGN_SEED, &sovereign.sovereign_id.to_le_bytes()],
        bump = sovereign.bump
    )]
    pub sovereign: Account<'info, SovereignState>,
    
    /// CHECK: SOL vault holding escrow
    #[account(
        mut,
        seeds = [SOL_VAULT_SEED, sovereign.key().as_ref()],
        bump
    )]
    pub sol_vault: SystemAccount<'info>,
    
    /// Creation fee escrow - returned to creator on failure
    #[account(
        mut,
        close = creator,
        seeds = [CREATION_FEE_ESCROW_SEED, sovereign.key().as_ref()],
        bump
    )]
    pub creation_fee_escrow: Account<'info, CreationFeeEscrow>,
    
    pub system_program: Program<'info, System>,
}

pub fn withdraw_creator_failed_handler(ctx: Context<WithdrawCreatorFailed>) -> Result<()> {
    let sovereign = &mut ctx.accounts.sovereign;
    
    // Validate state
    require!(
        sovereign.state == SovereignStatus::Failed,
        SovereignError::InvalidState
    );
    
    let escrow_amount = sovereign.creator_escrow;
    let creation_fee = ctx.accounts.creation_fee_escrow.amount;
    
    // Verify vault has sufficient balance for escrow
    if escrow_amount > 0 {
        let vault_balance = ctx.accounts.sol_vault.lamports();
        
        require!(
            vault_balance >= escrow_amount,
            SovereignError::InsufficientVaultBalance
        );
        
        // Transfer creator escrow from vault using System Program CPI
        let sovereign_key = sovereign.key();
        let vault_seeds: &[&[u8]] = &[
            SOL_VAULT_SEED,
            sovereign_key.as_ref(),
            &[ctx.bumps.sol_vault],
        ];
        
        anchor_lang::system_program::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.system_program.to_account_info(),
                anchor_lang::system_program::Transfer {
                    from: ctx.accounts.sol_vault.to_account_info(),
                    to: ctx.accounts.creator.to_account_info(),
                },
                &[vault_seeds],
            ),
            escrow_amount,
        )?;
        
        // Clear escrow amount
        sovereign.creator_escrow = 0;
    }
    
    emit!(CreatorFailedWithdrawal {
        sovereign_id: sovereign.sovereign_id,
        creator: ctx.accounts.creator.key(),
        escrow_returned: escrow_amount,
        creation_fee_returned: creation_fee,
    });
    
    // Note: creation_fee_escrow is closed and rent + fee returned to creator
    
    Ok(())
}
