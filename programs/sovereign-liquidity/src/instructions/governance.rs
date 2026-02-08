use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount, Mint};
use anchor_spl::token_interface::{
    Mint as MintInterface,
    TokenAccount as TokenAccountInterface,
    TokenInterface,
};
use anchor_lang::solana_program::program::invoke_signed;
use crate::state::*;
use crate::constants::*;
use crate::errors::SovereignError;
use crate::events::{ProposalCreated, VoteCast, ProposalFinalized, UnwindExecuted, UnwindClaimed};
use crate::samm::{self, instructions as samm_ix, cpi as samm_cpi, SammAccountDeserialize};

/// Create an unwind proposal
/// Authorization is purely via Genesis NFT possession (bearer instrument).
#[derive(Accounts)]
pub struct ProposeUnwind<'info> {
    /// Current NFT holder (bearer of the position)
    #[account(mut)]
    pub holder: Signer<'info>,
    
    #[account(
        seeds = [SOVEREIGN_SEED, &sovereign.sovereign_id.to_le_bytes()],
        bump = sovereign.bump
    )]
    pub sovereign: Account<'info, SovereignState>,
    
    /// CHECK: Original depositor wallet — used only for deposit_record PDA derivation.
    pub original_depositor: UncheckedAccount<'info>,
    
    /// Deposit record — proves a valid deposit position exists
    #[account(
        seeds = [DEPOSIT_RECORD_SEED, sovereign.key().as_ref(), original_depositor.key().as_ref()],
        bump = deposit_record.bump,
        constraint = deposit_record.nft_minted @ SovereignError::NoGenesisNFT,
    )]
    pub deposit_record: Account<'info, DepositRecord>,
    
    /// Genesis NFT token account — proves the holder possesses the position NFT
    #[account(
        constraint = nft_token_account.amount == 1 @ SovereignError::NoGenesisNFT,
        constraint = nft_token_account.mint == deposit_record.nft_mint.unwrap() @ SovereignError::WrongNFT,
        constraint = nft_token_account.owner == holder.key() @ SovereignError::Unauthorized,
    )]
    pub nft_token_account: Account<'info, TokenAccount>,
    
    /// Proposal account
    #[account(
        init,
        payer = holder,
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
    proposal.proposer = ctx.accounts.holder.key();
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
        proposer: ctx.accounts.holder.key(),
        created_at: clock.unix_timestamp,
        voting_ends_at: proposal.voting_ends_at,
    });
    
    Ok(())
}

/// Vote on an unwind proposal
/// Authorization is purely via Genesis NFT possession (bearer instrument).
/// VoteRecord is keyed by NFT mint — each position can only vote once per proposal,
/// regardless of ownership transfers.
#[derive(Accounts)]
pub struct Vote<'info> {
    /// Current NFT holder (bearer of the position)
    #[account(mut)]
    pub holder: Signer<'info>,
    
    #[account(
        mut,
        seeds = [SOVEREIGN_SEED, &sovereign.sovereign_id.to_le_bytes()],
        bump = sovereign.bump
    )]
    pub sovereign: Account<'info, SovereignState>,
    
    /// CHECK: Original depositor wallet — used only for deposit_record PDA derivation.
    pub original_depositor: UncheckedAccount<'info>,
    
    /// Voter's deposit record with NFT
    #[account(
        seeds = [DEPOSIT_RECORD_SEED, sovereign.key().as_ref(), original_depositor.key().as_ref()],
        bump = deposit_record.bump,
        constraint = deposit_record.nft_minted @ SovereignError::NoGenesisNFT,
    )]
    pub deposit_record: Account<'info, DepositRecord>,
    
    /// Genesis NFT mint — used for VoteRecord PDA derivation (one vote per NFT position)
    #[account(
        constraint = Some(nft_mint.key()) == deposit_record.nft_mint @ SovereignError::WrongNFT
    )]
    pub nft_mint: Account<'info, Mint>,
    
    /// Genesis NFT token account — proves the holder possesses the position NFT
    #[account(
        constraint = nft_token_account.amount == 1 @ SovereignError::NoGenesisNFT,
        constraint = nft_token_account.mint == nft_mint.key() @ SovereignError::WrongNFT,
        constraint = nft_token_account.owner == holder.key() @ SovereignError::Unauthorized,
    )]
    pub nft_token_account: Account<'info, TokenAccount>,
    
    #[account(
        mut,
        seeds = [PROPOSAL_SEED, sovereign.key().as_ref(), &proposal.proposal_id.to_le_bytes()],
        bump = proposal.bump
    )]
    pub proposal: Account<'info, Proposal>,
    
    /// Vote record — keyed by NFT mint to prevent double-voting after NFT transfer
    #[account(
        init,
        payer = holder,
        space = VoteRecord::LEN,
        seeds = [VOTE_RECORD_SEED, proposal.key().as_ref(), nft_mint.key().as_ref()],
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
    vote_record.voter = ctx.accounts.holder.key();
    vote_record.genesis_nft_mint = ctx.accounts.nft_mint.key();
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
        voter: ctx.accounts.holder.key(),
        support,
        voting_power: voting_power as u64,
        votes_for: proposal.votes_for_bps as u64,
        votes_against: proposal.votes_against_bps as u64,
    });
    
    Ok(())
}

/// Finalize voting and determine outcome.
/// If vote passes, snapshots SAMM pool fee_growth and starts 90-day
/// observation period. Unwind only proceeds if volume stays below threshold.
/// remaining_accounts[0] = pool_state (required when vote passes)
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
    
    #[account(
        seeds = [PERMANENT_LOCK_SEED, sovereign.key().as_ref()],
        bump = permanent_lock.bump
    )]
    pub permanent_lock: Account<'info, PermanentLock>,
}

pub fn finalize_vote_handler<'info>(
    ctx: Context<'_, '_, 'info, 'info, FinalizeVote<'info>>,
) -> Result<()> {
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
        
        // Snapshot SAMM pool fee_growth and start 90-day observation period
        // remaining_accounts[0] must be the pool_state matching permanent_lock.pool_state
        require!(
            ctx.remaining_accounts.len() >= 1,
            SovereignError::MissingSAMMAccounts
        );
        let pool_info = &ctx.remaining_accounts[0];
        require!(
            pool_info.key() == ctx.accounts.permanent_lock.pool_state,
            SovereignError::InvalidPool
        );
        
        let pool_data = pool_info.try_borrow_data()?;
        let pool = samm::PoolState::try_deserialize(&pool_data)?;
        drop(pool_data);
        
        // Snapshot fee_growth at this moment — will be compared at execution time
        sovereign.fee_growth_snapshot_a = pool.fee_growth_global_0_x64;
        sovereign.fee_growth_snapshot_b = pool.fee_growth_global_1_x64;
        sovereign.activity_check_timestamp = clock.unix_timestamp + UNWIND_OBSERVATION_PERIOD;
        sovereign.activity_check_initiated = true;
        sovereign.activity_check_initiated_at = Some(clock.unix_timestamp);
        
        // Set state to Unwinding (observation pending — execute_unwind enforces the 90-day wait)
        sovereign.state = SovereignStatus::Unwinding;
        
        // Set timelock on proposal
        proposal.timelock_ends_at = clock.unix_timestamp + UNWIND_OBSERVATION_PERIOD;
        
        msg!("Vote passed — 90-day observation period started, ends at {}", 
            sovereign.activity_check_timestamp);
        msg!("Fee growth snapshot: A={}, B={}", 
            sovereign.fee_growth_snapshot_a, sovereign.fee_growth_snapshot_b);
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

/// Execute unwind - remove liquidity, take protocol fee, prepare for distribution
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
        seeds = [PROTOCOL_STATE_SEED],
        bump = protocol_state.bump
    )]
    pub protocol_state: Box<Account<'info, ProtocolState>>,
    
    /// CHECK: Protocol treasury — receives unwind fee
    #[account(
        mut,
        address = protocol_state.treasury @ SovereignError::Unauthorized
    )]
    pub treasury: SystemAccount<'info>,
    
    #[account(
        seeds = [PROPOSAL_SEED, sovereign.key().as_ref(), &proposal.proposal_id.to_le_bytes()],
        bump = proposal.bump,
        constraint = proposal.status == ProposalStatus::Passed @ SovereignError::ProposalNotPassed
    )]
    pub proposal: Box<Account<'info, Proposal>>,
    
    #[account(
        mut,
        seeds = [PERMANENT_LOCK_SEED, sovereign.key().as_ref()],
        bump = permanent_lock.bump
    )]
    pub permanent_lock: Box<Account<'info, PermanentLock>>,
    
    /// Token mint (supports Token-2022)
    #[account(
        mut,
        address = sovereign.token_mint
    )]
    pub token_mint: Box<InterfaceAccount<'info, MintInterface>>,
    
    /// CHECK: SAMM position - MUST match permanent_lock.position_mint
    #[account(
        mut,
        constraint = position.key() == permanent_lock.position_mint @ SovereignError::InvalidPosition
    )]
    pub position: UncheckedAccount<'info>,
    
    /// CHECK: Trashbin SAMM program
    #[account(address = SAMM_PROGRAM_ID)]
    pub samm_program: UncheckedAccount<'info>,
    
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
    pub token_vault: Box<InterfaceAccount<'info, TokenAccountInterface>>,
    
    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}

/// Check volume during observation period. Returns true if threshold met (cancel unwind).
#[inline(never)]
fn check_volume_threshold(
    pool_account: &AccountInfo,
    pool_state_key: Pubkey,
    snapshot_a: u128,
    total_deposited: u64,
    position_liquidity: u128,
    threshold_bps: u64,
) -> Result<bool> {
    require!(
        pool_account.key() == pool_state_key,
        SovereignError::InvalidPool
    );
    
    let pool_data = pool_account.try_borrow_data()?;
    let pool = samm::PoolState::try_deserialize(&pool_data)?;
    drop(pool_data);
    
    let fee_delta_a = pool.fee_growth_global_0_x64.saturating_sub(snapshot_a);
    let threshold = if threshold_bps == 0 { DEFAULT_UNWIND_VOLUME_THRESHOLD_BPS as u64 } else { threshold_bps };
    
    // actual_fees = fee_delta * position_liquidity >> 64
    let actual_fees = ((fee_delta_a as u128)
        .checked_mul(position_liquidity as u128)
        .unwrap_or(0)) >> 64;
    
    let required_fees = (total_deposited as u128)
        .checked_mul(threshold as u128)
        .unwrap_or(0)
        .checked_div(BPS_DENOMINATOR as u128)
        .unwrap_or(0);
    
    msg!("Volume check — actual fees: {} GOR, required: {} GOR ({}bps of {})", 
        actual_fees, required_fees, threshold, total_deposited);
    
    Ok(actual_fees >= required_fees)
}

pub fn execute_unwind_handler<'info>(ctx: Context<'_, '_, 'info, 'info, ExecuteUnwind<'info>>) -> Result<()> {
    let sovereign = &mut ctx.accounts.sovereign;
    let permanent_lock = &mut ctx.accounts.permanent_lock;
    let protocol = &ctx.accounts.protocol_state;
    let clock = Clock::get()?;
    
    // Validate state
    require!(
        sovereign.state == SovereignStatus::Unwinding,
        SovereignError::InvalidState
    );
    require!(permanent_lock.liquidity > 0, SovereignError::NothingToWithdraw);
    
    // ============ Step 0: 90-Day Observation Period & Volume Check ============
    // After vote passes, there's a 90-day observation window.
    // If pool fee_growth during this period meets the threshold,
    // the sovereign is still viable → cancel unwind.
    
    let observation_ends_at = sovereign.activity_check_timestamp;
    require!(
        clock.unix_timestamp >= observation_ends_at,
        SovereignError::ActivityCheckPeriodNotElapsed
    );
    
    // Read current fee_growth from SAMM pool to compare against snapshot
    require!(ctx.remaining_accounts.len() >= 15, SovereignError::MissingSAMMAccounts);
    
    let volume_met = check_volume_threshold(
        &ctx.remaining_accounts[2],
        permanent_lock.pool_state,
        sovereign.fee_growth_snapshot_a,
        sovereign.total_deposited,
        permanent_lock.liquidity as u128,
        protocol.min_fee_growth_threshold as u64,
    )?;
    
    if volume_met {
        sovereign.state = SovereignStatus::Active;
        sovereign.activity_check_initiated = false;
        sovereign.activity_check_initiated_at = None;
        sovereign.activity_check_last_cancelled = clock.unix_timestamp;
        
        msg!("Volume threshold met — unwind cancelled, sovereign returned to Active");
        return Ok(());
    }
    
    msg!("Volume below threshold — proceeding with unwind");
    
    // ============ Step 1: SAMM Liquidity Removal ============
    
    // SECURITY: Pool already validated in check_volume_threshold
    
    msg!("Executing unwind via SAMM CPI - removing all liquidity...");
    
    // Read actual liquidity from SAMM personal position
    let pp_data = ctx.remaining_accounts[1].try_borrow_data()?;
    let personal_pos = samm::PersonalPositionState::try_deserialize(&pp_data)?;
    let actual_liquidity = personal_pos.liquidity;
    drop(pp_data);
    
    require!(actual_liquidity > 0, SovereignError::NothingToWithdraw);
    
    let sovereign_key = sovereign.key();
    let lock_seeds = &[
        PERMANENT_LOCK_SEED,
        sovereign_key.as_ref(),
        &[permanent_lock.bump],
    ];
    let lock_signer_seeds = &[&lock_seeds[..]];
    
    let decrease_accounts = samm_ix::DecreaseLiquidityV2Accounts {
        nft_owner: permanent_lock.to_account_info(),
        nft_account: ctx.remaining_accounts[0].clone(),
        personal_position: ctx.remaining_accounts[1].clone(),
        pool_state: ctx.remaining_accounts[2].clone(),
        protocol_position: ctx.remaining_accounts[3].clone(),
        token_vault_0: ctx.remaining_accounts[4].clone(),
        token_vault_1: ctx.remaining_accounts[5].clone(),
        tick_array_lower: ctx.remaining_accounts[6].clone(),
        tick_array_upper: ctx.remaining_accounts[7].clone(),
        recipient_token_account_0: ctx.remaining_accounts[8].clone(),
        recipient_token_account_1: ctx.remaining_accounts[9].clone(),
        token_program: ctx.accounts.token_program.to_account_info(),
        token_program_2022: ctx.remaining_accounts[10].clone(),
        memo_program: ctx.remaining_accounts[11].clone(),
        vault_0_mint: ctx.remaining_accounts[12].clone(),
        vault_1_mint: ctx.remaining_accounts[13].clone(),
        tick_array_bitmap_extension: ctx.remaining_accounts[14].clone(),
    };
    
    let _result = samm_cpi::remove_liquidity(
        &ctx.accounts.samm_program.to_account_info(),
        decrease_accounts,
        actual_liquidity,
        0, // Min amount 0 for unwind (accept any amount)
        0,
        lock_signer_seeds,
    )?;
    
    msg!("Liquidity removed from SAMM pool");
    
    // ============ Step 2: Read WGOR ATA balance & close → sol_vault ============
    let recipient_0_info = &ctx.remaining_accounts[8]; // WGOR ATA
    let recipient_1_info = &ctx.remaining_accounts[9]; // Token ATA
    
    let wgor_amount = {
        let data = recipient_0_info.try_borrow_data()?;
        u64::from_le_bytes(data[64..72].try_into().unwrap())
    };
    
    // Close WGOR ATA → all lamports to sol_vault (unwraps WGOR to native SOL)
    let close_wgor_ix = anchor_lang::solana_program::instruction::Instruction {
        program_id: anchor_spl::token::ID,
        accounts: vec![
            anchor_lang::solana_program::instruction::AccountMeta::new(
                recipient_0_info.key(), false,
            ),
            anchor_lang::solana_program::instruction::AccountMeta::new(
                ctx.accounts.sol_vault.key(), false,
            ),
            anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                permanent_lock.key(), true,
            ),
        ],
        data: vec![9u8], // SPL Token CloseAccount
    };
    invoke_signed(
        &close_wgor_ix,
        &[
            recipient_0_info.clone(),
            ctx.accounts.sol_vault.to_account_info(),
            permanent_lock.to_account_info(),
        ],
        lock_signer_seeds,
    )?;
    msg!("WGOR ATA closed → {} WGOR unwrapped to sol_vault", wgor_amount);
    
    // ============ Step 3: Protocol fee — 20% off the top ============
    let fee_bps = protocol.unwind_fee_bps;
    let protocol_fee = (wgor_amount as u128)
        .checked_mul(fee_bps as u128)
        .ok_or(SovereignError::Overflow)?
        .checked_div(BPS_DENOMINATOR as u128)
        .ok_or(SovereignError::DivisionByZero)? as u64;
    
    if protocol_fee > 0 {
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
                    to: ctx.accounts.treasury.to_account_info(),
                },
                &[vault_seeds],
            ),
            protocol_fee,
        )?;
        msg!("Protocol fee: {} GOR ({}bps) → treasury", protocol_fee, fee_bps);
    }
    
    let investor_pool = wgor_amount.saturating_sub(protocol_fee);
    
    // ============ Step 4: Calculate surplus & token redemption ============
    let investor_cap = sovereign.total_deposited;
    let surplus = investor_pool.saturating_sub(investor_cap);
    
    // Read token ATA balance
    let token_amount = {
        let data = recipient_1_info.try_borrow_data()?;
        u64::from_le_bytes(data[64..72].try_into().unwrap())
    };
    
    if sovereign.sovereign_type == SovereignType::TokenLaunch && surplus > 0 {
        // TokenLaunch: surplus goes to token holder redemption pool
        sovereign.token_redemption_pool = surplus;
        
        // Snapshot circulating tokens: total supply minus protocol-held tokens
        let tv_info = ctx.accounts.token_vault.to_account_info();
        let token_vault_amount = {
            let data = tv_info.try_borrow_data()?;
            u64::from_le_bytes(data[64..72].try_into().unwrap())
        };
        let circulating = ctx.accounts.token_mint.supply
            .saturating_sub(token_vault_amount)
            .saturating_sub(token_amount); // lock ATA tokens
        sovereign.circulating_tokens_at_unwind = circulating;
        sovereign.token_redemption_deadline = clock.unix_timestamp + TOKEN_REDEMPTION_WINDOW;
        
        msg!("TokenLaunch surplus: {} GOR → token redemption pool ({} circulating tokens, deadline: {})", 
            surplus, circulating, sovereign.token_redemption_deadline);
    } else if sovereign.sovereign_type == SovereignType::BYOToken && surplus > 0 {
        // BYO: surplus goes to protocol treasury
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
                    to: ctx.accounts.treasury.to_account_info(),
                },
                &[vault_seeds],
            ),
            surplus,
        )?;
        msg!("BYO surplus: {} GOR → treasury", surplus);
        
        sovereign.token_redemption_pool = 0;
        sovereign.circulating_tokens_at_unwind = 0;
    }
    
    // ============ Step 5: Update state ============
    // unwind_sol_balance = what's available for investors
    // For TokenLaunch: investor_pool (includes surplus sitting in vault for token redemption)
    // For BYO: investor_pool - surplus (surplus already sent to treasury)
    sovereign.unwind_sol_balance = if sovereign.sovereign_type == SovereignType::BYOToken {
        investor_pool.saturating_sub(surplus)
    } else {
        investor_pool
    };
    sovereign.unwind_token_balance = token_amount;
    
    sovereign.state = SovereignStatus::Unwound;
    sovereign.unwound_at = Some(clock.unix_timestamp);
    permanent_lock.liquidity = 0;
    
    msg!("Unwind complete — investor_pool: {}, protocol_fee: {}, surplus: {}", 
        sovereign.unwind_sol_balance, protocol_fee, surplus);
    
    emit!(UnwindExecuted {
        sovereign_id: sovereign.sovereign_id,
        executed_at: clock.unix_timestamp,
        sol_amount: wgor_amount,
        token_amount,
    });
    
    Ok(())
}

/// Claim unwind proceeds — requires Genesis NFT, which is burned on claim.
/// Authorization is purely via Genesis NFT possession (bearer instrument).
/// The NFT holder receives proportional GOR (SOL) only.
/// Sovereign tokens remain in token_vault for the creator to reclaim.
#[derive(Accounts)]
pub struct ClaimUnwind<'info> {
    /// Current NFT holder (bearer of the position)
    #[account(mut)]
    pub holder: Signer<'info>,
    
    #[account(
        seeds = [SOVEREIGN_SEED, &sovereign.sovereign_id.to_le_bytes()],
        bump = sovereign.bump
    )]
    pub sovereign: Account<'info, SovereignState>,
    
    /// CHECK: Original depositor wallet — used only for deposit_record PDA derivation.
    pub original_depositor: UncheckedAccount<'info>,
    
    #[account(
        mut,
        seeds = [DEPOSIT_RECORD_SEED, sovereign.key().as_ref(), original_depositor.key().as_ref()],
        bump = deposit_record.bump,
    )]
    pub deposit_record: Account<'info, DepositRecord>,
    
    /// Genesis NFT mint — will be burned
    #[account(
        mut,
        constraint = nft_mint.key() == deposit_record.nft_mint.unwrap() @ SovereignError::WrongNFT
    )]
    pub nft_mint: Account<'info, Mint>,
    
    /// Genesis NFT token account — must hold exactly 1, will be burned
    #[account(
        mut,
        constraint = nft_token_account.amount == 1 @ SovereignError::NoGenesisNFT,
        constraint = nft_token_account.mint == nft_mint.key() @ SovereignError::WrongNFT,
        constraint = nft_token_account.owner == holder.key() @ SovereignError::Unauthorized,
    )]
    pub nft_token_account: Account<'info, TokenAccount>,
    
    /// CHECK: SOL vault
    #[account(
        mut,
        seeds = [SOL_VAULT_SEED, sovereign.key().as_ref()],
        bump
    )]
    pub sol_vault: SystemAccount<'info>,
    
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
    
    // Genesis NFT must have been minted
    require!(deposit_record.nft_minted, SovereignError::NFTNotMinted);
    
    // ---- Burn the Genesis NFT (one-time redemption of LP position) ----
    anchor_spl::token::burn(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            anchor_spl::token::Burn {
                mint: ctx.accounts.nft_mint.to_account_info(),
                from: ctx.accounts.nft_token_account.to_account_info(),
                authority: ctx.accounts.holder.to_account_info(),
            },
        ),
        1,
    )?;
    msg!("Genesis NFT burned for unwind claim");
    
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
    
    // Calculate GOR share from unwind SOL balance, capped at original deposit
    let sol_share = sovereign.unwind_sol_balance
        .checked_mul(share_bps)
        .ok_or(SovereignError::Overflow)?
        .checked_div(BPS_DENOMINATOR as u64)
        .ok_or(SovereignError::DivisionByZero)?
        .min(deposit_record.amount);
    
    // Transfer GOR from sol_vault to holder
    if sol_share > 0 {
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
                    to: ctx.accounts.holder.to_account_info(),
                },
                &[vault_seeds],
            ),
            sol_share,
        )?;
    }
    
    deposit_record.unwind_claimed = true;
    
    emit!(UnwindClaimed {
        sovereign_id: sovereign.sovereign_id,
        claimer: ctx.accounts.holder.key(),
        sol_amount: sol_share,
        token_amount: 0,
    });
    
    Ok(())
}
