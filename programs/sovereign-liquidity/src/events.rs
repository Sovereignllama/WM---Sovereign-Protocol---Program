use anchor_lang::prelude::*;
use crate::state::{SovereignType, FeeMode, ProposalStatus};

// ============================================================
// SOVEREIGN LIFECYCLE EVENTS
// ============================================================

#[event]
pub struct SovereignCreated {
    pub sovereign_id: u64,
    pub creator: Pubkey,
    pub token_mint: Pubkey,
    pub sovereign_type: SovereignType,
    pub bond_target: u64,
    pub bond_deadline: i64,
    pub token_supply_deposited: u64,
    pub creation_fee_escrowed: u64,
    pub sell_fee_bps: u16,
    pub fee_mode: FeeMode,
}

#[event]
pub struct SovereignFinalized {
    pub sovereign_id: u64,
    pub total_deposited: u64,
    pub token_supply: u64,
    pub lp_tokens: u64,
    pub recovery_target: u64,
    pub finalized_at: i64,
}

#[event]
pub struct BondingFailed {
    pub sovereign_id: u64,
    pub total_deposited: u64,
    pub bond_target: u64,
    pub failed_at: i64,
}

// ============================================================
// DEPOSIT EVENTS
// ============================================================

#[event]
pub struct InvestorDeposited {
    pub sovereign_id: u64,
    pub depositor: Pubkey,
    pub amount: u64,
    pub total_deposited: u64,
    pub depositor_count: u32,
}

#[event]
pub struct InvestorWithdrew {
    pub sovereign_id: u64,
    pub depositor: Pubkey,
    pub amount: u64,
    pub remaining_deposit: u64,
    pub total_deposited: u64,
}

#[event]
pub struct CreatorEscrowed {
    pub sovereign_id: u64,
    pub creator: Pubkey,
    pub amount: u64,
    pub total_escrowed: u64,
}

#[event]
pub struct CreatorMarketBuyExecuted {
    pub sovereign_id: u64,
    pub creator: Pubkey,
    pub sol_amount: u64,
    pub tokens_received: u64,
}

// ============================================================
// FEE EVENTS
// ============================================================

#[event]
pub struct FeesClaimed {
    pub sovereign_id: u64,
    pub sol_fees: u64,
    pub token_fees: u64,
    pub creator_share: u64,
    pub investor_share: u64,
    pub protocol_share: u64,
    pub total_recovered: u64,
    pub recovery_target: u64,
}

#[event]
pub struct RecoveryComplete {
    pub sovereign_id: u64,
    pub total_recovered: u64,
    pub recovery_target: u64,
    pub completed_at: i64,
}

#[event]
pub struct PoolRestricted {
    pub sovereign_id: u64,
    pub restricted: bool,
}

// ============================================================
// GENESIS NFT EVENTS
// ============================================================

#[event]
pub struct GenesisNFTMinted {
    pub sovereign_id: u64,
    pub depositor: Pubkey,
    pub nft_mint: Pubkey,
    pub voting_power_bps: u16,
    pub deposit_amount: u64,
}

// ============================================================
// GOVERNANCE EVENTS
// ============================================================

#[event]
pub struct ProposalCreated {
    pub sovereign_id: u64,
    pub proposal_id: u64,
    pub proposer: Pubkey,
    pub created_at: i64,
    pub voting_ends_at: i64,
}

#[event]
pub struct VoteCast {
    pub sovereign_id: u64,
    pub proposal_id: u64,
    pub voter: Pubkey,
    pub support: bool,
    pub voting_power: u64,
    pub votes_for: u64,
    pub votes_against: u64,
}

#[event]
pub struct ProposalFinalized {
    pub sovereign_id: u64,
    pub proposal_id: u64,
    pub status: ProposalStatus,
    pub votes_for: u64,
    pub votes_against: u64,
    pub participation_bps: u16,
    pub passed: bool,
}

#[event]
pub struct UnwindExecuted {
    pub sovereign_id: u64,
    pub executed_at: i64,
    pub sol_amount: u64,
    pub token_amount: u64,
}

#[event]
pub struct UnwindClaimed {
    pub sovereign_id: u64,
    pub claimer: Pubkey,
    pub sol_amount: u64,
    pub token_amount: u64,
}

// ============================================================
// ACTIVITY CHECK EVENTS
// ============================================================

#[event]
pub struct ActivityCheckInitiated {
    pub sovereign_id: u64,
    pub initiator: Pubkey,
    pub initiated_at: i64,
    pub execution_available_at: i64,
}

#[event]
pub struct ActivityCheckExecuted {
    pub sovereign_id: u64,
    pub executor: Pubkey,
    pub executed_at: i64,
    pub days_elapsed: u32,
}

// ============================================================
// FAILED BONDING EVENTS
// ============================================================

#[event]
pub struct FailedWithdrawal {
    pub sovereign_id: u64,
    pub depositor: Pubkey,
    pub amount: u64,
}

#[event]
pub struct CreatorFailedWithdrawal {
    pub sovereign_id: u64,
    pub creator: Pubkey,
    pub escrow_returned: u64,
    pub creation_fee_returned: u64,
}

// ============================================================
// PROTOCOL ADMIN EVENTS
// ============================================================

#[event]
pub struct ProtocolInitialized {
    pub authority: Pubkey,
    pub treasury: Pubkey,
}

#[event]
pub struct ProtocolFeesUpdated {
    pub creation_fee_bps: u16,
    pub min_fee_lamports: u64,
    pub min_deposit: u64,
    pub min_bond_target: u64,
}

#[event]
pub struct FeeThresholdUpdated {
    pub sovereign_id: u64,
    pub old_threshold_bps: u16,
    pub new_threshold_bps: u16,
}

#[event]
pub struct FeeThresholdRenounced {
    pub sovereign_id: u64,
    pub old_threshold_bps: u16,
    pub renounced_by: Pubkey,
}
