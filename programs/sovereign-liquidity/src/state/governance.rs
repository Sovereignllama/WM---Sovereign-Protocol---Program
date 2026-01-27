use anchor_lang::prelude::*;

/// Status of a governance proposal
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum ProposalStatus {
    /// Voting is active
    #[default]
    Active,
    /// Voting ended, passed, in timelock
    Passed,
    /// Voting ended, failed quorum or threshold
    Failed,
    /// Timelock expired, executed
    Executed,
    /// Cancelled by proposer or admin
    Cancelled,
}

/// Unwind proposal during Recovery phase
/// Only investors can create and vote (creator excluded)
#[account]
#[derive(Default)]
pub struct Proposal {
    /// The sovereign this proposal belongs to
    pub sovereign: Pubkey,
    
    /// Unique proposal ID within the sovereign
    pub proposal_id: u64,
    
    /// Address that created the proposal
    pub proposer: Pubkey,
    
    /// Current status
    pub status: ProposalStatus,
    
    /// Total votes for (in basis points of total shares)
    pub votes_for_bps: u32,
    
    /// Total votes against (in basis points of total shares)
    pub votes_against_bps: u32,
    
    /// Total participation (in basis points of total shares)
    pub total_voted_bps: u32,
    
    /// Number of unique voters
    pub voter_count: u32,
    
    /// Required quorum in basis points (default 6700 = 67%)
    pub quorum_bps: u16,
    
    /// Required pass threshold in basis points (default 5100 = 51%)
    pub pass_threshold_bps: u16,
    
    /// Voting period end timestamp
    pub voting_ends_at: i64,
    
    /// Timelock end timestamp (when execution is allowed)
    pub timelock_ends_at: i64,
    
    /// Timestamp when proposal was created
    pub created_at: i64,
    
    /// Timestamp when proposal was executed (0 if not executed)
    pub executed_at: i64,
    
    /// PDA bump seed
    pub bump: u8,
}

impl Proposal {
    pub const LEN: usize = 8  // discriminator
        + 32  // sovereign
        + 8   // proposal_id
        + 32  // proposer
        + 1   // status
        + 4   // votes_for_bps
        + 4   // votes_against_bps
        + 4   // total_voted_bps
        + 4   // voter_count
        + 2   // quorum_bps
        + 2   // pass_threshold_bps
        + 8   // voting_ends_at
        + 8   // timelock_ends_at
        + 8   // created_at
        + 8   // executed_at
        + 1   // bump
        + 16; // padding
    
    /// Default governance parameters
    pub fn default_quorum_bps() -> u16 { 6700 }  // 67%
    pub fn default_pass_threshold_bps() -> u16 { 5100 }  // 51%
    pub fn default_voting_period() -> i64 { 7 * 24 * 60 * 60 }  // 7 days
    pub fn default_timelock_period() -> i64 { 2 * 24 * 60 * 60 }  // 2 days
    
    /// Check if voting is still active
    pub fn is_voting_active(&self, current_time: i64) -> bool {
        self.status == ProposalStatus::Active && current_time <= self.voting_ends_at
    }
    
    /// Check if quorum is met
    pub fn is_quorum_met(&self) -> bool {
        self.total_voted_bps >= self.quorum_bps as u32
    }
    
    /// Check if proposal passed
    pub fn is_passed(&self) -> bool {
        if !self.is_quorum_met() {
            return false;
        }
        // 51% of votes cast must be FOR
        let total_votes = self.votes_for_bps + self.votes_against_bps;
        if total_votes == 0 {
            return false;
        }
        (self.votes_for_bps * 10000 / total_votes) >= self.pass_threshold_bps as u32
    }
    
    /// Check if timelock has expired (ready to execute)
    pub fn is_executable(&self, current_time: i64) -> bool {
        self.status == ProposalStatus::Passed && current_time >= self.timelock_ends_at
    }
}

/// Individual vote record to prevent double voting
#[account]
#[derive(Default)]
pub struct VoteRecord {
    /// The proposal this vote belongs to
    pub proposal: Pubkey,
    
    /// The voter's wallet address
    pub voter: Pubkey,
    
    /// Genesis NFT used for voting
    pub genesis_nft_mint: Pubkey,
    
    /// Voting power in basis points (from DepositRecord.shares_bps)
    pub voting_power_bps: u16,
    
    /// Whether voted for (true) or against (false)
    pub vote_for: bool,
    
    /// Timestamp of vote
    pub voted_at: i64,
    
    /// PDA bump seed
    pub bump: u8,
}

impl VoteRecord {
    pub const LEN: usize = 8  // discriminator
        + 32  // proposal
        + 32  // voter
        + 32  // genesis_nft_mint
        + 2   // voting_power_bps
        + 1   // vote_for
        + 8   // voted_at
        + 1   // bump
        + 8;  // padding
}
