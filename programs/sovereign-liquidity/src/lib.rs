use anchor_lang::prelude::*;

pub mod constants;
pub mod errors;
pub mod events;
pub mod instructions;
pub mod state;

use instructions::*;

declare_id!("SLPsoL1qNqFhGKLgABFhgErrrYoRb4t4HKLd6VCRYQL");

#[program]
pub mod sovereign_liquidity {
    use super::*;

    // ============ Protocol Initialization ============
    
    /// Initialize the protocol state (one-time setup)
    pub fn initialize_protocol(ctx: Context<InitializeProtocol>) -> Result<()> {
        instructions::initialize_protocol::handler(ctx)
    }

    // ============ Sovereign Lifecycle ============
    
    /// Create a new sovereign (token launch or BYO token)
    pub fn create_sovereign(
        ctx: Context<CreateSovereign>,
        params: CreateSovereignParams,
    ) -> Result<()> {
        instructions::create_sovereign::handler(ctx, params)
    }

    /// Deposit SOL during bonding phase
    pub fn deposit(ctx: Context<Deposit>, amount: u64) -> Result<()> {
        instructions::deposit::handler(ctx, amount)
    }

    /// Withdraw SOL during bonding phase (investors only)
    pub fn withdraw(ctx: Context<Withdraw>, amount: u64) -> Result<()> {
        instructions::withdraw::handler(ctx, amount)
    }

    /// Finalize sovereign after bond target is met
    /// Creates Whirlpool, adds liquidity, mints NFTs
    pub fn finalize(ctx: Context<Finalize>) -> Result<()> {
        instructions::finalize::handler(ctx)
    }

    /// Mint Genesis NFT to a depositor after finalization
    pub fn mint_genesis_nft(ctx: Context<MintGenesisNFT>) -> Result<()> {
        instructions::finalize::mint_genesis_nft_handler(ctx)
    }

    // ============ Fee Management ============
    
    /// Collect fees from Whirlpool position
    pub fn claim_fees(ctx: Context<ClaimFees>) -> Result<()> {
        instructions::claim_fees::handler(ctx)
    }

    /// Claim depositor's share of accumulated fees
    pub fn claim_depositor_fees(ctx: Context<ClaimDepositorFees>) -> Result<()> {
        instructions::claim_fees::claim_depositor_fees_handler(ctx)
    }

    /// Creator withdraws earned fees
    pub fn withdraw_creator_fees(ctx: Context<WithdrawCreatorFees>) -> Result<()> {
        instructions::claim_fees::withdraw_creator_fees_handler(ctx)
    }

    // ============ Governance ============
    
    /// Propose to unwind the sovereign (Genesis NFT holders)
    pub fn propose_unwind(ctx: Context<ProposeUnwind>) -> Result<()> {
        instructions::governance::propose_unwind_handler(ctx)
    }

    /// Vote on an unwind proposal
    pub fn vote(ctx: Context<Vote>, support: bool) -> Result<()> {
        instructions::governance::vote_handler(ctx, support)
    }

    /// Finalize voting after period ends
    pub fn finalize_vote(ctx: Context<FinalizeVote>) -> Result<()> {
        instructions::governance::finalize_vote_handler(ctx)
    }

    /// Execute unwind after proposal passes
    pub fn execute_unwind(ctx: Context<ExecuteUnwind>) -> Result<()> {
        instructions::governance::execute_unwind_handler(ctx)
    }

    /// Claim proceeds from unwound sovereign
    pub fn claim_unwind(ctx: Context<ClaimUnwind>) -> Result<()> {
        instructions::governance::claim_unwind_handler(ctx)
    }

    // ============ Activity Check ============
    
    /// Initiate activity check (90-day countdown)
    pub fn initiate_activity_check(ctx: Context<InitiateActivityCheck>) -> Result<()> {
        instructions::activity_check::initiate_activity_check_handler(ctx)
    }

    /// Creator cancels activity check (proves liveness)
    pub fn cancel_activity_check(ctx: Context<CancelActivityCheck>) -> Result<()> {
        instructions::activity_check::cancel_activity_check_handler(ctx)
    }

    /// Execute activity check after 90 days
    pub fn execute_activity_check(ctx: Context<ExecuteActivityCheck>) -> Result<()> {
        instructions::activity_check::execute_activity_check_handler(ctx)
    }

    // ============ Failed Bonding ============
    
    /// Mark bonding as failed if deadline passed
    pub fn mark_bonding_failed(ctx: Context<MarkBondingFailed>) -> Result<()> {
        instructions::failed_bonding::mark_bonding_failed_handler(ctx)
    }

    /// Investor withdraws from failed bonding
    pub fn withdraw_failed(ctx: Context<WithdrawFailed>) -> Result<()> {
        instructions::failed_bonding::withdraw_failed_handler(ctx)
    }

    /// Creator withdraws escrow from failed bonding
    pub fn withdraw_creator_failed(ctx: Context<WithdrawCreatorFailed>) -> Result<()> {
        instructions::failed_bonding::withdraw_creator_failed_handler(ctx)
    }

    // ============ Admin Functions ============
    
    /// Update protocol fee parameters
    pub fn update_protocol_fees(
        ctx: Context<UpdateProtocolFees>,
        new_creation_fee_bps: Option<u16>,
        new_min_fee_lamports: Option<u64>,
        new_min_deposit: Option<u64>,
        new_min_bond_target: Option<u64>,
    ) -> Result<()> {
        instructions::admin::update_protocol_fees_handler(
            ctx,
            new_creation_fee_bps,
            new_min_fee_lamports,
            new_min_deposit,
            new_min_bond_target,
        )
    }

    /// Transfer protocol authority
    pub fn transfer_protocol_authority(ctx: Context<TransferProtocolAuthority>) -> Result<()> {
        instructions::admin::transfer_protocol_authority_handler(ctx)
    }

    /// Update creator's fee threshold (can only decrease)
    pub fn update_fee_threshold(
        ctx: Context<UpdateFeeThreshold>,
        new_threshold_bps: u16,
    ) -> Result<()> {
        instructions::admin::update_fee_threshold_handler(ctx, new_threshold_bps)
    }

    /// Permanently renounce fee threshold (irreversible)
    pub fn renounce_fee_threshold(ctx: Context<RenounceFeeThreshold>) -> Result<()> {
        instructions::admin::renounce_fee_threshold_handler(ctx)
    }

    /// Pause/unpause protocol (emergency)
    pub fn set_protocol_paused(ctx: Context<SetProtocolPaused>, paused: bool) -> Result<()> {
        instructions::admin::set_protocol_paused_handler(ctx, paused)
    }
}
