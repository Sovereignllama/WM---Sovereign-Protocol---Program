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
    /// Pool created on SAMM - ready for liquidity addition
    PoolCreated,
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
    /// Emergency unlocked - all participants can reclaim funds
    EmergencyUnlocked,
    /// All funds reclaimed - sovereign is retired
    Retired,
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
    // METADATA
    // ============================================================
    
    /// Sovereign name (max 32 bytes)
    pub name: String,
    
    /// Token name (max 32 bytes)
    pub token_name: String,
    
    /// Token symbol (max 10 bytes)
    pub token_symbol: String,
    
    /// Metadata URI (max 200 bytes)
    pub metadata_uri: String,
    
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
    // SAMM POOL CONFIGURATION (set at creation)
    // ============================================================
    
    /// Trashbin SAMM AmmConfig address (fee tier chosen by creator)
    pub amm_config: Pubkey,
    
    /// Swap fee in basis points (matching the AMM config tier, e.g. 30 = 0.3%)
    pub swap_fee_bps: u16,
    
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
    // UNWIND OBSERVATION STATE
    // ============================================================
    
    /// Whether an unwind observation period is in progress
    pub activity_check_initiated: bool,
    
    /// Timestamp when unwind observation was initiated (Option for cleaner handling)
    pub activity_check_initiated_at: Option<i64>,
    
    /// Timestamp when unwind observation period ends (90 days after vote passes)
    pub activity_check_timestamp: i64,
    
    /// Snapshot of SAMM fee_growth_global_0_x64 at vote pass time
    pub fee_growth_snapshot_a: u128,
    
    /// Snapshot of SAMM fee_growth_global_1_x64 at vote pass time
    pub fee_growth_snapshot_b: u128,
    
    /// Timestamp of last cancelled unwind observation (for cooldown)
    pub activity_check_last_cancelled: i64,
    
    // ============================================================
    // UNWIND STATE
    // ============================================================
    
    /// SOL balance after removing liquidity (for claiming)
    pub unwind_sol_balance: u64,
    
    /// Token balance after removing liquidity (for creator)
    pub unwind_token_balance: u64,
    
    /// Surplus GOR available for external token holder redemption
    /// = max(0, unwind_sol_balance - total_deposited)
    pub token_redemption_pool: u64,
    
    /// Snapshot of circulating tokens at time of LP removal
    /// = mint.supply - token_vault.amount - lock_ata_tokens
    pub circulating_tokens_at_unwind: u64,
    
    /// Deadline for token holders to redeem surplus GOR (unix timestamp).
    /// After this, unclaimed GOR is swept to treasury.
    pub token_redemption_deadline: i64,
    
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

/// Max length constants for string fields
pub const MAX_NAME_LEN: usize = 32;
pub const MAX_TOKEN_NAME_LEN: usize = 32;
pub const MAX_TOKEN_SYMBOL_LEN: usize = 10;
pub const MAX_METADATA_URI_LEN: usize = 200;

impl SovereignState {
    pub const LEN: usize = 8  // discriminator
        + 8   // sovereign_id
        + 32  // creator
        + 32  // token_mint
        + 1   // sovereign_type
        + 1   // state
        + (4 + MAX_NAME_LEN)      // name (4 bytes length prefix + max chars)
        + (4 + MAX_TOKEN_NAME_LEN) // token_name
        + (4 + MAX_TOKEN_SYMBOL_LEN) // token_symbol
        + (4 + MAX_METADATA_URI_LEN) // metadata_uri
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
        + 32  // amm_config
        + 2   // swap_fee_bps
        + 32  // pool_state
        + 32  // position_mint
        + 1   // pool_restricted
        + 8   // recovery_target
        + 8   // total_sol_fees_distributed
        + 8   // total_token_fees_distributed
        + 1   // recovery_complete
        + 8   // active_proposal_id
        + 8   // proposal_count
        + 1   // has_active_proposal
        + 2   // fee_threshold_bps
        + 8   // total_fees_collected
        + 8   // total_recovered
        + 8   // total_supply
        + 32  // genesis_nft_mint
        + 9   // unwound_at (Option<i64>)
        + 8   // last_activity
        + 1   // activity_check_initiated
        + 9   // activity_check_initiated_at (Option<i64>)
        + 8   // activity_check_timestamp
        + 16  // fee_growth_snapshot_a
        + 16  // fee_growth_snapshot_b
        + 8   // activity_check_last_cancelled
        + 8   // unwind_sol_balance
        + 8   // unwind_token_balance
        + 8   // token_redemption_pool
        + 8   // circulating_tokens_at_unwind
        + 8   // token_redemption_deadline
        + 8   // last_activity_timestamp
        + 8   // created_at
        + 8   // finalized_at
        + 1   // bump
        + 40; // padding for future expansion (was 64, used 24 for redemption fields)
    
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
