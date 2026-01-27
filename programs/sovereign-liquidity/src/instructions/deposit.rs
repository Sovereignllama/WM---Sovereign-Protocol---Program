use anchor_lang::prelude::*;
use crate::state::*;
use crate::constants::*;
use crate::errors::SovereignError;
use crate::events::{InvestorDeposited, CreatorEscrowed};

#[derive(Accounts)]
pub struct Deposit<'info> {
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
    
    /// Deposit record - initialized if new depositor
    #[account(
        init_if_needed,
        payer = depositor,
        space = DepositRecord::LEN,
        seeds = [DEPOSIT_RECORD_SEED, sovereign.key().as_ref(), depositor.key().as_ref()],
        bump
    )]
    pub deposit_record: Account<'info, DepositRecord>,
    
    /// SOL vault to hold deposits during bonding
    /// CHECK: PDA that holds SOL
    #[account(
        mut,
        seeds = [SOL_VAULT_SEED, sovereign.key().as_ref()],
        bump
    )]
    pub sol_vault: SystemAccount<'info>,
    
    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<Deposit>, amount: u64) -> Result<()> {
    let sovereign = &mut ctx.accounts.sovereign;
    let deposit_record = &mut ctx.accounts.deposit_record;
    let protocol = &ctx.accounts.protocol_state;
    let clock = Clock::get()?;
    
    // Check protocol pause status
    require!(
        !protocol.paused,
        SovereignError::ProtocolPaused
    );
    
    // Validate state - use atomic check to prevent race conditions
    // Must be Bonding AND not already at/past target
    require!(
        sovereign.state == SovereignStatus::Bonding,
        SovereignError::InvalidState
    );
    
    // Double-check target hasn't been reached (prevents race condition)
    require!(
        sovereign.total_deposited < sovereign.bond_target,
        SovereignError::BondingComplete
    );
    
    require!(
        !sovereign.is_deadline_passed(clock.unix_timestamp),
        SovereignError::DeadlinePassed
    );
    require!(amount > 0, SovereignError::ZeroDeposit);
    require!(
        amount >= protocol.min_deposit,
        SovereignError::DepositTooSmall
    );
    
    let is_creator = ctx.accounts.depositor.key() == sovereign.creator;
    
    if is_creator {
        // CREATOR DEPOSIT: Goes to escrow for market buy (NOT LP)
        let max_creator_buy = sovereign.max_creator_buy_in();
        require!(
            sovereign.creator_escrow.checked_add(amount).unwrap() <= max_creator_buy,
            SovereignError::CreatorDepositExceedsMax
        );
        
        // Transfer SOL to vault (will be used for market buy later)
        let transfer_ix = anchor_lang::system_program::Transfer {
            from: ctx.accounts.depositor.to_account_info(),
            to: ctx.accounts.sol_vault.to_account_info(),
        };
        anchor_lang::system_program::transfer(
            CpiContext::new(
                ctx.accounts.system_program.to_account_info(),
                transfer_ix,
            ),
            amount,
        )?;
        
        sovereign.creator_escrow = sovereign.creator_escrow.checked_add(amount).unwrap();
        
        emit!(CreatorEscrowed {
            sovereign_id: sovereign.sovereign_id,
            creator: ctx.accounts.depositor.key(),
            amount,
            total_escrowed: sovereign.creator_escrow,
        });
    } else {
        // INVESTOR DEPOSIT: Counts toward bond target
        
        // Re-check bond target (defense against concurrent transactions)
        let remaining_to_target = sovereign.bond_target
            .checked_sub(sovereign.total_deposited)
            .ok_or(SovereignError::BondingComplete)?;
        
        require!(
            remaining_to_target > 0,
            SovereignError::BondingComplete
        );
        
        // Cap deposit to remaining amount needed (prevents over-bonding)
        let actual_amount = std::cmp::min(amount, remaining_to_target);
        
        // Calculate refund if user sent excess
        let _refund_amount = amount.checked_sub(actual_amount).unwrap_or(0);
        
        // If refund needed, we don't transfer the excess (it stays with user)
        // The user only sends actual_amount via SOL transfer below
        
        // Validate actual_amount meets minimum (after capping)
        require!(
            actual_amount >= protocol.min_deposit || remaining_to_target < protocol.min_deposit,
            SovereignError::DepositTooSmall
        );
        
        // Transfer SOL to vault (only actual_amount)
        let transfer_ix = anchor_lang::system_program::Transfer {
            from: ctx.accounts.depositor.to_account_info(),
            to: ctx.accounts.sol_vault.to_account_info(),
        };
        anchor_lang::system_program::transfer(
            CpiContext::new(
                ctx.accounts.system_program.to_account_info(),
                transfer_ix,
            ),
            actual_amount,
        )?;
        
        // Initialize deposit record if new
        if deposit_record.amount == 0 {
            deposit_record.sovereign = sovereign.key();
            deposit_record.depositor = ctx.accounts.depositor.key();
            deposit_record.deposited_at = clock.unix_timestamp;
            deposit_record.bump = ctx.bumps.deposit_record;
            sovereign.depositor_count = sovereign.depositor_count.checked_add(1).unwrap();
        }
        
        deposit_record.amount = deposit_record.amount.checked_add(actual_amount).unwrap();
        sovereign.total_deposited = sovereign.total_deposited.checked_add(actual_amount).unwrap();
        
        emit!(InvestorDeposited {
            sovereign_id: sovereign.sovereign_id,
            depositor: ctx.accounts.depositor.key(),
            amount: actual_amount,
            total_deposited: sovereign.total_deposited,
            depositor_count: sovereign.depositor_count,
        });
    }
    
    // ATOMIC state transition: Check if bond target is now met
    // This happens immediately to prevent race conditions
    if sovereign.total_deposited >= sovereign.bond_target {
        // Transition to Finalizing immediately to block new deposits
        sovereign.state = SovereignStatus::Finalizing;
    }
    
    Ok(())
}
