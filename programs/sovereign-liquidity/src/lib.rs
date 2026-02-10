use anchor_lang::prelude::*;

pub mod constants;
pub mod errors;
pub mod events;
pub mod instructions;
pub mod samm;
pub mod state;

use instructions::*;

declare_id!("2LPPAG7UhVop1RiRBh8oZtjzMoJ9St9WV4nY7JwmoNbA");

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

    /// Create Token-2022 mint for a TokenLaunch sovereign
    /// Must be called after create_sovereign for TokenLaunch types
    pub fn create_token(
        ctx: Context<CreateToken>,
        params: CreateTokenParams,
    ) -> Result<()> {
        instructions::create_token::handler(ctx, params)
    }

    /// Deposit SOL during bonding phase
    pub fn deposit(ctx: Context<Deposit>, amount: u64) -> Result<()> {
        instructions::deposit::handler(ctx, amount)
    }

    /// Withdraw SOL during bonding phase (investors only)
    pub fn withdraw(ctx: Context<Withdraw>, amount: u64) -> Result<()> {
        instructions::withdraw::handler(ctx, amount)
    }

    /// Finalize sovereign step 1: Create SAMM pool
    /// Called after bond target is met (state = Finalizing)
    pub fn finalize_create_pool(ctx: Context<FinalizeCreatePool>) -> Result<()> {
        instructions::finalize::finalize_create_pool_handler(ctx)
    }

    /// Finalize sovereign step 2: Add liquidity to SAMM pool
    /// Called after pool is created (state = PoolCreated)
    pub fn finalize_add_liquidity<'info>(
        ctx: Context<'_, '_, 'info, 'info, FinalizeAddLiquidity<'info>>,
    ) -> Result<()> {
        instructions::finalize::finalize_add_liquidity_handler(ctx)
    }

    /// Mint Genesis NFT to a depositor after finalization
    pub fn mint_genesis_nft(ctx: Context<MintGenesisNFT>) -> Result<()> {
        instructions::finalize::mint_genesis_nft_handler(ctx)
    }

    // ============ Fee Management ============
    
    /// Collect fees from SAMM position
    pub fn claim_fees<'info>(ctx: Context<'_, '_, 'info, 'info, ClaimFees<'info>>) -> Result<()> {
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

    /// Harvest withheld transfer fees from Token-2022 token accounts
    /// Fees are collected from TransferFeeConfig extension
    pub fn harvest_transfer_fees<'info>(
        ctx: Context<'_, '_, 'info, 'info, HarvestTransferFees<'info>>,
    ) -> Result<()> {
        instructions::claim_fees::harvest_transfer_fees_handler(ctx)
    }

    /// Swap recovery tokens (harvested transfer fees) to SOL via SAMM
    /// This converts Token-2022 sell fees into GOR for investor recovery
    pub fn swap_recovery_tokens<'info>(
        ctx: Context<'_, '_, 'info, 'info, SwapRecoveryTokens<'info>>,
    ) -> Result<()> {
        instructions::claim_fees::swap_recovery_tokens_handler(ctx)
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

    /// Finalize voting after period ends.
    /// If passed, snapshots SAMM fee_growth and starts 90-day observation.
    /// remaining_accounts[0] = pool_state (required when vote passes)
    pub fn finalize_vote<'info>(ctx: Context<'_, '_, 'info, 'info, FinalizeVote<'info>>) -> Result<()> {
        instructions::governance::finalize_vote_handler(ctx)
    }

    /// Execute unwind after proposal passes
    pub fn execute_unwind<'info>(ctx: Context<'_, '_, 'info, 'info, ExecuteUnwind<'info>>) -> Result<()> {
        instructions::governance::execute_unwind_handler(ctx)
    }

    /// Claim proceeds from unwound sovereign
    pub fn claim_unwind(ctx: Context<ClaimUnwind>) -> Result<()> {
        instructions::governance::claim_unwind_handler(ctx)
    }

    // ============ Activity Check (deprecated — use governance unwind) ============
    // Activity check instructions removed. Unwind observation is now
    // integrated into the governance vote flow (finalize_vote + execute_unwind).

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
        new_unwind_fee_bps: Option<u16>,
        new_volume_threshold_bps: Option<u16>,
    ) -> Result<()> {
        instructions::admin::update_protocol_fees_handler(
            ctx,
            new_creation_fee_bps,
            new_min_fee_lamports,
            new_min_deposit,
            new_min_bond_target,
            new_unwind_fee_bps,
            new_volume_threshold_bps,
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

    // ============ Sell Fee Management (TokenLaunch) ============
    
    /// Lower the sell fee (can only decrease, never increase)
    pub fn update_sell_fee(ctx: Context<UpdateSellFee>, new_fee_bps: u16) -> Result<()> {
        instructions::admin::update_sell_fee_handler(ctx, new_fee_bps)
    }

    /// Permanently renounce sell fee control (sets to 0%, irreversible)
    /// Only after recovery is complete (or anytime for FairLaunch mode)
    pub fn renounce_sell_fee(ctx: Context<RenounceSellFee>) -> Result<()> {
        instructions::admin::renounce_sell_fee_handler(ctx)
    }

    // ============ Emergency Functions ============

    /// Emergency unlock - transitions sovereign to EmergencyUnlocked state
    /// Callable by protocol authority or sovereign creator from ANY state
    pub fn emergency_unlock(ctx: Context<EmergencyUnlock>) -> Result<()> {
        instructions::emergency::emergency_unlock_handler(ctx)
    }

    /// Emergency withdraw for investors - reclaim deposited GOR
    /// If Genesis NFT was minted, pass nft_mint and nft_token_account as remaining_accounts
    pub fn emergency_withdraw<'info>(
        ctx: Context<'_, '_, 'info, 'info, EmergencyWithdraw<'info>>,
    ) -> Result<()> {
        instructions::emergency::emergency_withdraw_handler(ctx)
    }

    /// Emergency withdraw for creator - reclaim escrow and creation fee
    pub fn emergency_withdraw_creator(ctx: Context<EmergencyWithdrawCreator>, burn_tokens: bool) -> Result<()> {
        instructions::emergency::emergency_withdraw_creator_handler(ctx, burn_tokens)
    }

    /// Emergency remove liquidity from SAMM pool
    /// Only callable by protocol authority when sovereign is EmergencyUnlocked
    /// Must be called before emergency_withdraw on post-finalization sovereigns
    pub fn emergency_remove_liquidity<'info>(
        ctx: Context<'_, '_, 'info, 'info, EmergencyRemoveLiquidity<'info>>,
    ) -> Result<()> {
        instructions::emergency::emergency_remove_liquidity_handler(ctx)
    }

    /// Emergency token redemption - external token holders burn sovereign tokens
    /// to receive proportional share of surplus GOR from the LP unwind.
    /// Available when token_redemption_pool > 0 (unwind_sol_balance > total_deposited).
    pub fn emergency_token_redemption(
        ctx: Context<EmergencyTokenRedemption>,
    ) -> Result<()> {
        instructions::emergency::emergency_token_redemption_handler(ctx)
    }

    /// Sweep unclaimed redemption pool GOR to treasury after 30-day window expires.
    /// Only callable by protocol authority.
    pub fn sweep_redemption_pool(
        ctx: Context<SweepRedemptionPool>,
    ) -> Result<()> {
        instructions::emergency::sweep_redemption_pool_handler(ctx)
    }
}
