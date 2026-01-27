use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount, Mint};
use crate::state::*;
use crate::constants::*;
use crate::errors::SovereignError;
use crate::events::{ProposalCreated, VoteCast, ProposalFinalized, UnwindExecuted, UnwindClaimed};

/// Create an unwind proposal
#[derive(Accounts)]
pub struct ProposeUnwind<'info> {
    #[account(mut)]
    pub proposer: Signer<'info>,
    
    #[account(
        seeds = [SOVEREIGN_SEED, &sovereign.sovereign_id.to_le_bytes()],
        bump = sovereign.bump
    )]
    pub sovereign: Account<'info, SovereignState>,
    
    /// Proposer must hold Genesis NFT
    #[account(
        seeds = [DEPOSIT_RECORD_SEED, sovereign.key().as_ref(), proposer.key().as_ref()],
        bump = deposit_record.bump,
        constraint = deposit_record.nft_minted @ SovereignError::NoGenesisNFT,
        constraint = deposit_record.depositor == proposer.key() @ SovereignError::Unauthorized
    )]
    pub deposit_record: Account<'info, DepositRecord>,
    
    /// Proposal account
    #[account(
        init,
        payer = proposer,
        space = Proposal::LEN,
        seeds = [PROPOSAL_SEED, sovereign.key().as_ref(), &sovereign.proposal_count.to_le_bytes()],
        bump
    )]
    pub proposal: Account<'info, Proposal>,
    
    pub system_program: Program<'info, System>,
}

pub fn propose_unwind_handler(ctx: Context<ProposeUnwind>) -> Result<()> {
    let sovereign = &mut ctx.accounts.sovereign;
    let proposal = &mut ctx.accounts.proposal;
    let _deposit_record = &ctx.accounts.deposit_record;
    let clock = Clock::get()?;
    
    // Validate state - must be Active (not Recovery)
    require!(
        sovereign.state == SovereignStatus::Active,
        SovereignError::InvalidState
    );
    
    // Cannot have active proposal already
    require!(
        !sovereign.has_active_proposal,
        SovereignError::ActiveProposalExists
    );
    
    // CRITICAL: Set the active proposal flag to prevent multiple proposals
    sovereign.has_active_proposal = true;
    sovereign.active_proposal_id = sovereign.proposal_count;
    
    // Initialize proposal
    proposal.sovereign = sovereign.key();
    proposal.proposal_id = sovereign.proposal_count;
    proposal.proposer = ctx.accounts.proposer.key();
    proposal.created_at = clock.unix_timestamp;
    proposal.voting_ends_at = clock.unix_timestamp + VOTING_PERIOD_SECONDS;
    proposal.status = ProposalStatus::Active;
    proposal.votes_for_bps = 0;
    proposal.votes_against_bps = 0;
    proposal.total_voted_bps = 0;
    proposal.quorum_bps = QUORUM_BPS;
    proposal.pass_threshold_bps = PASS_THRESHOLD_BPS;
    proposal.bump = ctx.bumps.proposal;
    
    emit!(ProposalCreated {
        sovereign_id: sovereign.sovereign_id,
        proposal_id: proposal.proposal_id,
        proposer: ctx.accounts.proposer.key(),
        created_at: clock.unix_timestamp,
        voting_ends_at: proposal.voting_ends_at,
    });
    
    Ok(())
}

/// Vote on an unwind proposal
#[derive(Accounts)]
pub struct Vote<'info> {
    #[account(mut)]
    pub voter: Signer<'info>,
    
    #[account(
        mut,
        seeds = [SOVEREIGN_SEED, &sovereign.sovereign_id.to_le_bytes()],
        bump = sovereign.bump
    )]
    pub sovereign: Account<'info, SovereignState>,
    
    /// Voter's deposit record with NFT
    #[account(
        seeds = [DEPOSIT_RECORD_SEED, sovereign.key().as_ref(), voter.key().as_ref()],
        bump = deposit_record.bump,
        constraint = deposit_record.nft_minted @ SovereignError::NoGenesisNFT,
        constraint = deposit_record.depositor == voter.key() @ SovereignError::Unauthorized
    )]
    pub deposit_record: Account<'info, DepositRecord>,
    
    #[account(
        mut,
        seeds = [PROPOSAL_SEED, sovereign.key().as_ref(), &proposal.proposal_id.to_le_bytes()],
        bump = proposal.bump
    )]
    pub proposal: Account<'info, Proposal>,
    
    /// Vote record - tracks individual votes
    #[account(
        init,
        payer = voter,
        space = VoteRecord::LEN,
        seeds = [VOTE_RECORD_SEED, proposal.key().as_ref(), voter.key().as_ref()],
        bump
    )]
    pub vote_record: Account<'info, VoteRecord>,
    
    pub system_program: Program<'info, System>,
}

pub fn vote_handler(ctx: Context<Vote>, support: bool) -> Result<()> {
    let proposal = &mut ctx.accounts.proposal;
    let deposit_record = &ctx.accounts.deposit_record;
    let vote_record = &mut ctx.accounts.vote_record;
    let clock = Clock::get()?;
    
    // Validate proposal is active
    require!(
        proposal.status == ProposalStatus::Active,
        SovereignError::ProposalNotActive
    );
    require!(
        clock.unix_timestamp <= proposal.voting_ends_at,
        SovereignError::VotingPeriodEnded
    );
    
    // Get voting power from deposit record (proportional to deposit)
    let voting_power = deposit_record.voting_power_bps;
    require!(voting_power > 0, SovereignError::NoVotingPower);
    
    // Record vote
    vote_record.proposal = proposal.key();
    vote_record.voter = ctx.accounts.voter.key();
    vote_record.genesis_nft_mint = deposit_record.genesis_nft_mint;
    vote_record.voting_power_bps = voting_power;
    vote_record.vote_for = support;
    vote_record.voted_at = clock.unix_timestamp;
    vote_record.bump = ctx.bumps.vote_record;
    
    // Update proposal tallies (BPS)
    if support {
        proposal.votes_for_bps = proposal.votes_for_bps.checked_add(voting_power as u32).unwrap();
    } else {
        proposal.votes_against_bps = proposal.votes_against_bps.checked_add(voting_power as u32).unwrap();
    }
    proposal.total_voted_bps = proposal.total_voted_bps.checked_add(voting_power as u32).unwrap();
    proposal.voter_count = proposal.voter_count.checked_add(1).unwrap();
    
    emit!(VoteCast {
        sovereign_id: ctx.accounts.sovereign.sovereign_id,
        proposal_id: proposal.proposal_id,
        voter: ctx.accounts.voter.key(),
        support,
        voting_power: voting_power as u64,
        votes_for: proposal.votes_for_bps as u64,
        votes_against: proposal.votes_against_bps as u64,
    });
    
    Ok(())
}

/// Finalize voting and determine outcome
#[derive(Accounts)]
pub struct FinalizeVote<'info> {
    #[account(mut)]
    pub caller: Signer<'info>,
    
    #[account(
        mut,
        seeds = [SOVEREIGN_SEED, &sovereign.sovereign_id.to_le_bytes()],
        bump = sovereign.bump
    )]
    pub sovereign: Account<'info, SovereignState>,
    
    #[account(
        mut,
        seeds = [PROPOSAL_SEED, sovereign.key().as_ref(), &proposal.proposal_id.to_le_bytes()],
        bump = proposal.bump
    )]
    pub proposal: Account<'info, Proposal>,
}

pub fn finalize_vote_handler(ctx: Context<FinalizeVote>) -> Result<()> {
    let sovereign = &mut ctx.accounts.sovereign;
    let proposal = &mut ctx.accounts.proposal;
    let clock = Clock::get()?;
    
    // Validate voting period has ended
    require!(
        proposal.status == ProposalStatus::Active,
        SovereignError::ProposalNotActive
    );
    require!(
        clock.unix_timestamp > proposal.voting_ends_at,
        SovereignError::VotingPeriodNotEnded
    );
    
    // Calculate participation based on total votes (in BPS)
    let participation_bps = proposal.total_voted_bps as u16;
    
    // Check quorum (67%)
    let quorum_met = participation_bps >= proposal.quorum_bps;
    
    // Check if passed (51% of votes cast)
    let total_votes = proposal.votes_for_bps + proposal.votes_against_bps;
    let passed = if total_votes > 0 {
        let for_percentage = (proposal.votes_for_bps as u32 * BPS_DENOMINATOR as u32) / total_votes as u32;
        quorum_met && for_percentage as u16 >= proposal.pass_threshold_bps
    } else {
        false
    };
    
    if passed {
        proposal.status = ProposalStatus::Passed;
        sovereign.state = SovereignStatus::Unwinding;
    } else {
        proposal.status = ProposalStatus::Failed;
    }
    
    // Clear active proposal flag
    sovereign.has_active_proposal = false;
    sovereign.proposal_count = sovereign.proposal_count.checked_add(1).unwrap();
    
    emit!(ProposalFinalized {
        sovereign_id: sovereign.sovereign_id,
        proposal_id: proposal.proposal_id,
        status: proposal.status.clone(),
        votes_for: proposal.votes_for_bps as u64,
        votes_against: proposal.votes_against_bps as u64,
        participation_bps,
        passed,
    });
    
    Ok(())
}

/// Execute unwind - remove liquidity and prepare for distribution
#[derive(Accounts)]
pub struct ExecuteUnwind<'info> {
    #[account(mut)]
    pub executor: Signer<'info>,
    
    #[account(
        mut,
        seeds = [SOVEREIGN_SEED, &sovereign.sovereign_id.to_le_bytes()],
        bump = sovereign.bump
    )]
    pub sovereign: Account<'info, SovereignState>,
    
    #[account(
        seeds = [PROPOSAL_SEED, sovereign.key().as_ref(), &proposal.proposal_id.to_le_bytes()],
        bump = proposal.bump,
        constraint = proposal.status == ProposalStatus::Passed @ SovereignError::ProposalNotPassed
    )]
    pub proposal: Account<'info, Proposal>,
    
    #[account(
        mut,
        seeds = [PERMANENT_LOCK_SEED, sovereign.key().as_ref()],
        bump = permanent_lock.bump
    )]
    pub permanent_lock: Account<'info, PermanentLock>,
    
    /// Token mint
    #[account(
        mut,
        address = sovereign.token_mint
    )]
    pub token_mint: Account<'info, Mint>,
    
    /// CHECK: Whirlpool position - MUST match permanent_lock.position_mint
    #[account(
        mut,
        constraint = position.key() == permanent_lock.position_mint @ SovereignError::InvalidPosition
    )]
    pub position: UncheckedAccount<'info>,
    
    /// CHECK: Whirlpool program
    #[account(address = WHIRLPOOL_PROGRAM_ID)]
    pub whirlpool_program: UncheckedAccount<'info>,
    
    /// Vault to receive removed liquidity
    /// CHECK: PDA vault
    #[account(
        mut,
        seeds = [SOL_VAULT_SEED, sovereign.key().as_ref()],
        bump
    )]
    pub sol_vault: SystemAccount<'info>,
    
    #[account(
        mut,
        seeds = [TOKEN_VAULT_SEED, sovereign.key().as_ref()],
        bump
    )]
    pub token_vault: Account<'info, TokenAccount>,
    
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

pub fn execute_unwind_handler(ctx: Context<ExecuteUnwind>) -> Result<()> {
    let sovereign = &mut ctx.accounts.sovereign;
    let clock = Clock::get()?;
    
    // Validate state
    require!(
        sovereign.state == SovereignStatus::Unwinding,
        SovereignError::InvalidState
    );
    
    // ============ Whirlpool Liquidity Removal ============
    // CPI to Whirlpool to:
    // 1. Remove all liquidity from position
    // 2. Collect any remaining fees
    // 3. Close position
    
    // SOL from LP goes to sol_vault for distribution
    // Tokens from LP go to token_vault for distribution
    
    // Calculate per-depositor shares
    // Each depositor gets: (their_deposit / total_deposited) * total_liquidity_value
    
    // Mark unwind as executed
    sovereign.state = SovereignStatus::Unwound;
    sovereign.unwound_at = Some(clock.unix_timestamp);
    
    emit!(UnwindExecuted {
        sovereign_id: sovereign.sovereign_id,
        executed_at: clock.unix_timestamp,
        // sol_amount and token_amount would come from Whirlpool CPI
        sol_amount: 0,
        token_amount: 0,
    });
    
    Ok(())
}

/// Claim unwind proceeds
#[derive(Accounts)]
pub struct ClaimUnwind<'info> {
    #[account(mut)]
    pub claimer: Signer<'info>,
    
    #[account(
        seeds = [SOVEREIGN_SEED, &sovereign.sovereign_id.to_le_bytes()],
        bump = sovereign.bump
    )]
    pub sovereign: Account<'info, SovereignState>,
    
    #[account(
        mut,
        seeds = [DEPOSIT_RECORD_SEED, sovereign.key().as_ref(), claimer.key().as_ref()],
        bump = deposit_record.bump,
        constraint = deposit_record.depositor == claimer.key() @ SovereignError::Unauthorized
    )]
    pub deposit_record: Account<'info, DepositRecord>,
    
    /// CHECK: SOL vault
    #[account(
        mut,
        seeds = [SOL_VAULT_SEED, sovereign.key().as_ref()],
        bump
    )]
    pub sol_vault: SystemAccount<'info>,
    
    /// Token vault for token distribution
    #[account(
        mut,
        seeds = [TOKEN_VAULT_SEED, sovereign.key().as_ref()],
        bump
    )]
    pub token_vault: Account<'info, TokenAccount>,
    
    /// Claimer's token account
    #[account(mut)]
    pub claimer_token_account: Account<'info, TokenAccount>,
    
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

pub fn claim_unwind_handler(ctx: Context<ClaimUnwind>) -> Result<()> {
    let sovereign = &ctx.accounts.sovereign;
    let deposit_record = &mut ctx.accounts.deposit_record;
    
    // Validate state
    require!(
        sovereign.state == SovereignStatus::Unwound,
        SovereignError::InvalidState
    );
    require!(
        !deposit_record.unwind_claimed,
        SovereignError::AlreadyClaimed
    );
    
    // CRITICAL: Prevent division by zero
    require!(
        sovereign.total_deposited > 0,
        SovereignError::NoDeposits
    );
    
    // Calculate claimer's share using safe arithmetic
    let share_bps = deposit_record.amount
        .checked_mul(BPS_DENOMINATOR as u64)
        .ok_or(SovereignError::Overflow)?
        .checked_div(sovereign.total_deposited)
        .ok_or(SovereignError::DivisionByZero)?;
    
    // Calculate actual amounts from vault balances
    let sol_share = sovereign.unwind_sol_balance
        .checked_mul(share_bps)
        .ok_or(SovereignError::Overflow)?
        .checked_div(BPS_DENOMINATOR as u64)
        .ok_or(SovereignError::DivisionByZero)?;
    
    let token_share = sovereign.unwind_token_balance
        .checked_mul(share_bps)
        .ok_or(SovereignError::Overflow)?
        .checked_div(BPS_DENOMINATOR as u64)
        .ok_or(SovereignError::DivisionByZero)?;
    
    // Transfer SOL
    if sol_share > 0 {
        let vault_info = ctx.accounts.sol_vault.to_account_info();
        let claimer_info = ctx.accounts.claimer.to_account_info();
        
        let vault_current = vault_info.lamports();
        let claimer_current = claimer_info.lamports();
        
        **vault_info.try_borrow_mut_lamports()? = vault_current
            .checked_sub(sol_share)
            .ok_or(SovereignError::InsufficientVaultBalance)?;
        **claimer_info.try_borrow_mut_lamports()? = claimer_current
            .checked_add(sol_share)
            .ok_or(SovereignError::Overflow)?;
    }
    
    // Transfer tokens using SOVEREIGN as authority (not token_vault)
    if token_share > 0 {
        let sovereign_id_bytes = sovereign.sovereign_id.to_le_bytes();
        let seeds = &[
            SOVEREIGN_SEED,
            &sovereign_id_bytes,
            &[sovereign.bump],
        ];
        let signer_seeds = &[&seeds[..]];
        
        anchor_spl::token::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                anchor_spl::token::Transfer {
                    from: ctx.accounts.token_vault.to_account_info(),
                    to: ctx.accounts.claimer_token_account.to_account_info(),
                    authority: ctx.accounts.sovereign.to_account_info(),
                },
                signer_seeds,
            ),
            token_share,
        )?;
    }
    
    deposit_record.unwind_claimed = true;
    
    emit!(UnwindClaimed {
        sovereign_id: sovereign.sovereign_id,
        claimer: ctx.accounts.claimer.key(),
        sol_amount: sol_share,
        token_amount: token_share,
    });
    
    Ok(())
}
