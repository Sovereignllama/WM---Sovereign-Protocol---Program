use anchor_lang::prelude::*;

/// Type of token launch
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum SovereignType {
    /// Protocol creates new Token-2022 with transfer hooks for sell tax
    #[default]
    TokenLaunch,
    /// Creator brings existing SPL/Token-2022 token
    BYOToken,
}

/// Fee distribution mode for Token Launcher
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum FeeMode {
    /// Fees always go to creator after recovery
    #[default]
    CreatorRevenue,
    /// Fees boost recovery, then go to creator
    RecoveryBoost,
    /// Fees boost recovery, then set to 0% (must be renounced)
    FairLaunch,
}

/// Current state of the sovereign lifecycle
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum SovereignStatus {
    /// Initial state - accepting deposits
    #[default]
    Bonding,
    /// Bond target met - ready to finalize
    Finalizing,
    /// LP created - in recovery phase (100% fees to investors)
    Recovery,
    /// Recovery complete - LP permanently locked, pool unrestricted
    Active,
    /// Unwind in progress - liquidity being removed
    Unwinding,
    /// Unwind complete - claim period active
    Unwound,
    /// Bonding failed - refund period active
    Failed,
}

/// Main sovereign state account - one per token launch
#[account]
#[derive(Default)]
pub struct SovereignState {
    // ============================================================
    // IDENTIFICATION
    // ============================================================
    
    /// Unique identifier for this sovereign
    pub sovereign_id: u64,
    
    /// Creator's wallet address
    pub creator: Pubkey,
    
    /// Token mint address (created or BYO)
    pub token_mint: Pubkey,
    
    /// Type of launch (TokenLaunch or BYOToken)
    pub sovereign_type: SovereignType,
    
    /// Current lifecycle state
    pub state: SovereignStatus,
    
    // ============================================================
    // BONDING CONFIGURATION
    // ============================================================
    
    /// Required SOL to raise (in lamports)
    pub bond_target: u64,
    
    /// Unix timestamp deadline for bonding
    pub bond_deadline: i64,
    
    /// Duration in seconds (for reference)
    pub bond_duration: i64,
    
    // ============================================================
    // DEPOSIT TRACKING
    // ============================================================
    
    /// Total SOL deposited by investors (excludes creator escrow)
    pub total_deposited: u64,
    
    /// Number of unique investor depositors
    pub depositor_count: u32,
    
    /// Creator's escrowed SOL for market buy (max 1% of bond target)
    pub creator_escrow: u64,
    
    // ============================================================
    // TOKEN SUPPLY TRACKING
    // ============================================================
    
    /// Tokens deposited by creator (100% for TokenLaunch, >=30% for BYO)
    pub token_supply_deposited: u64,
    
    /// Total supply of token (for BYO verification)
    pub token_total_supply: u64,
    
    // ============================================================
    // TOKEN LAUNCHER SETTINGS
    // ============================================================
    
    /// Sell fee in basis points (0-300 = 0-3%)
    pub sell_fee_bps: u16,
    
    /// Fee distribution mode
    pub fee_mode: FeeMode,
    
    /// If true, creator cannot change sell_fee_bps
    pub fee_control_renounced: bool,
    
    // ============================================================
    // CREATION FEE ESCROW
    // ============================================================
    
    /// Amount held in creation fee escrow PDA
    pub creation_fee_escrowed: u64,
    
    // ============================================================
    // POOL INFORMATION
    // ============================================================
    
    /// Trashbin SAMM PoolState address (set on finalization)
    pub pool_state: Pubkey,
    
    /// Position NFT mint (held by PermanentLock)
    pub position_mint: Pubkey,
    
    /// Whether pool restriction is active (LP locked to Genesis only)
    pub pool_restricted: bool,
    
    // ============================================================
    // RECOVERY TRACKING
    // ============================================================
    
    /// Target SOL to recover (equals total_deposited)
    pub recovery_target: u64,
    
    /// Total SOL fees distributed to investors
    pub total_sol_fees_distributed: u64,
    
    /// Total token fees distributed to investors
    pub total_token_fees_distributed: u64,
    
    /// Whether recovery phase is complete
    pub recovery_complete: bool,
    
    // ============================================================
    // GOVERNANCE STATE
    // ============================================================
    
    /// Active proposal ID (0 if none)
    pub active_proposal_id: u64,
    
    /// Total proposals created
    pub proposal_count: u64,
    
    /// Whether there's an active proposal
    pub has_active_proposal: bool,
    
    /// Fee threshold in BPS (creator's share)
    pub fee_threshold_bps: u16,
    
    /// Total fees collected (for tracking)
    pub total_fees_collected: u64,
    
    /// Total recovered during recovery phase
    pub total_recovered: u64,
    
    /// Total supply of tokens (for allocation)
    pub total_supply: u64,
    
    /// Genesis NFT collection mint
    pub genesis_nft_mint: Pubkey,
    
    /// Timestamp when unwound (if applicable)
    pub unwound_at: Option<i64>,
    
    /// Last activity timestamp
    pub last_activity: i64,
    
    // ============================================================
    // ACTIVITY CHECK STATE (Active phase only)
    // ============================================================
    
    /// Whether an activity check is in progress
    pub activity_check_initiated: bool,
    
    /// Timestamp when activity check was initiated (Option for cleaner handling)
    pub activity_check_initiated_at: Option<i64>,
    
    /// Timestamp when activity check was initiated (legacy)
    pub activity_check_timestamp: i64,
    
    /// Snapshot of fee_growth_global_a at initiation
    pub fee_growth_snapshot_a: u128,
    
    /// Snapshot of fee_growth_global_b at initiation
    pub fee_growth_snapshot_b: u128,
    
    /// Timestamp of last cancelled activity check (for cooldown)
    pub activity_check_last_cancelled: i64,
    
    // ============================================================
    // UNWIND STATE
    // ============================================================
    
    /// SOL balance after removing liquidity (for claiming)
    pub unwind_sol_balance: u64,
    
    /// Token balance after removing liquidity (for creator)
    pub unwind_token_balance: u64,
    
    // ============================================================
    // TIMESTAMPS
    // ============================================================
    
    /// Last fee collection or activity timestamp
    pub last_activity_timestamp: i64,
    
    /// Timestamp when sovereign was created
    pub created_at: i64,
    
    /// Timestamp when finalized (LP created)
    pub finalized_at: i64,
    
    // ============================================================
    // PDA
    // ============================================================
    
    /// PDA bump seed
    pub bump: u8,
}

impl SovereignState {
    pub const LEN: usize = 8  // discriminator
        + 8   // sovereign_id
        + 32  // creator
        + 32  // token_mint
        + 1   // sovereign_type
        + 1   // state
        + 8   // bond_target
        + 8   // bond_deadline
        + 8   // bond_duration
        + 8   // total_deposited
        + 4   // depositor_count
        + 8   // creator_escrow
        + 8   // token_supply_deposited
        + 8   // token_total_supply
        + 2   // sell_fee_bps
        + 1   // fee_mode
        + 1   // fee_control_renounced
        + 8   // creation_fee_escrowed
        + 32  // pool_state
        + 32  // position_mint
        + 1   // pool_restricted
        + 8   // recovery_target
        + 8   // total_sol_fees_distributed
        + 8   // total_token_fees_distributed
        + 1   // recovery_complete
        + 8   // active_proposal_id
        + 8   // proposal_count
        + 1   // activity_check_initiated
        + 8   // activity_check_timestamp
        + 16  // fee_growth_snapshot_a
        + 16  // fee_growth_snapshot_b
        + 8   // activity_check_last_cancelled
        + 8   // unwind_sol_balance
        + 8   // unwind_token_balance
        + 8   // last_activity_timestamp
        + 8   // created_at
        + 8   // finalized_at
        + 1   // bump
        + 64; // padding for future expansion
    
    /// Calculate maximum creator buy-in based on bond target
    pub fn max_creator_buy_in(&self) -> u64 {
        self.bond_target / 100  // 1% of bond target
    }
    
    /// Check if bonding deadline has passed
    pub fn is_deadline_passed(&self, current_time: i64) -> bool {
        current_time > self.bond_deadline
    }
    
    /// Check if bond target is met
    pub fn is_bond_target_met(&self) -> bool {
        self.total_deposited >= self.bond_target
    }
    
    /// Check if recovery is complete
    pub fn is_recovery_complete(&self) -> bool {
        self.total_sol_fees_distributed >= self.recovery_target
    }
}
