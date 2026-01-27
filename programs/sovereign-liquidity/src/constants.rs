use anchor_lang::prelude::*;

// ============================================================
// LAMPORTS
// ============================================================

/// SOL per lamport (1 SOL = 1_000_000_000 lamports)
pub const LAMPORTS_PER_SOL: u64 = 1_000_000_000;

/// 0.05 SOL in lamports
pub const POINT_ZERO_FIVE_SOL: u64 = 50_000_000;

/// 0.1 SOL in lamports
pub const POINT_ONE_SOL: u64 = 100_000_000;

/// 50 SOL in lamports
pub const FIFTY_SOL: u64 = 50_000_000_000;

// ============================================================
// TIME CONSTANTS (in seconds)
// ============================================================

/// 1 day in seconds
pub const ONE_DAY: i64 = 24 * 60 * 60;

/// 7 days in seconds
pub const SEVEN_DAYS: i64 = 7 * ONE_DAY;

/// 30 days in seconds
pub const THIRTY_DAYS: i64 = 30 * ONE_DAY;

/// 90 days in seconds
pub const NINETY_DAYS: i64 = 90 * ONE_DAY;

/// 365 days in seconds
pub const THREE_SIXTY_FIVE_DAYS: i64 = 365 * ONE_DAY;

/// Minimum bonding period (7 days)
pub const MIN_BOND_DURATION: i64 = SEVEN_DAYS;

/// Maximum bonding period (30 days)
pub const MAX_BOND_DURATION: i64 = THIRTY_DAYS;

/// Activity check cooldown (7 days after cancelled check)
pub const ACTIVITY_CHECK_COOLDOWN: i64 = SEVEN_DAYS;

/// Voting period for governance (7 days)
pub const VOTING_PERIOD: i64 = SEVEN_DAYS;

/// Voting period in seconds (alias for compatibility)
pub const VOTING_PERIOD_SECONDS: i64 = SEVEN_DAYS;

/// Activity check period (90 days)
pub const ACTIVITY_CHECK_PERIOD_SECONDS: i64 = NINETY_DAYS;

/// Timelock period after passed vote (2 days)
pub const TIMELOCK_PERIOD: i64 = 2 * ONE_DAY;

// ============================================================
// BASIS POINTS
// ============================================================

/// 100% in basis points (denominator for BPS calculations)
pub const BPS_100_PERCENT: u16 = 10000;

/// BPS denominator (alias for compatibility)
pub const BPS_DENOMINATOR: u16 = 10000;

/// Maximum protocol fee (5% = 500 bps)
pub const MAX_PROTOCOL_FEE_BPS: u16 = 500;

/// Maximum sell fee (3% = 300 bps)
pub const MAX_SELL_FEE_BPS: u16 = 300;

/// Maximum creation fee (10% = 1000 bps)
pub const MAX_CREATION_FEE_BPS: u16 = 1000;

/// Maximum unwind fee (10% = 1000 bps)
pub const MAX_UNWIND_FEE_BPS: u16 = 1000;

/// Default creation fee (0.5% = 50 bps)
pub const DEFAULT_CREATION_FEE_BPS: u16 = 50;

/// Default unwind fee (5% = 500 bps)
pub const DEFAULT_UNWIND_FEE_BPS: u16 = 500;

/// Default BYO minimum supply (30% = 3000 bps)
pub const DEFAULT_BYO_MIN_SUPPLY_BPS: u16 = 3000;

/// Default fee threshold BPS (maximum allowed)
pub const DEFAULT_FEE_THRESHOLD_BPS: u16 = 10000;

/// Quorum requirement (67% = 6700 bps)
pub const QUORUM_BPS: u16 = 6700;

/// Pass threshold (51% = 5100 bps)
pub const PASS_THRESHOLD_BPS: u16 = 5100;

/// Creator max buy-in (1% = 100 bps of bond target)
pub const CREATOR_MAX_BUY_BPS: u16 = 100;

/// LP allocation (80% = 8000 bps goes to LP)
pub const LP_ALLOCATION_BPS: u16 = 8000;

// ============================================================
// WHIRLPOOL CONSTANTS
// ============================================================

/// Minimum tick index for full range
pub const MIN_TICK_INDEX: i32 = -443636;

/// Maximum tick index for full range
pub const MAX_TICK_INDEX: i32 = 443636;

/// Default tick spacing for Orca whirlpools
pub const DEFAULT_TICK_SPACING: u16 = 64;

// ============================================================
// PROTOCOL DEFAULTS
// ============================================================

/// Minimum bond target (50 SOL)
pub const MIN_BOND_TARGET: u64 = FIFTY_SOL;

/// Minimum single deposit (0.1 SOL)
pub const MIN_DEPOSIT: u64 = POINT_ONE_SOL;

/// Minimum fee (0.05 SOL - non-refundable on failed bonding)
pub const MIN_FEE: u64 = POINT_ZERO_FIVE_SOL;

/// Governance unwind fee (0.05 SOL)
pub const GOVERNANCE_UNWIND_FEE: u64 = POINT_ZERO_FIVE_SOL;

/// Minimum fee growth threshold (must be > 0)
pub const MIN_FEE_GROWTH_THRESHOLD: u128 = 1000;

/// Maximum slippage for creator market buy (1% = 100 bps)
pub const MAX_SLIPPAGE_BPS: u16 = 100;

// ============================================================
// PDA SEEDS
// ============================================================

pub const PROTOCOL_STATE_SEED: &[u8] = b"protocol_state";
pub const SOVEREIGN_SEED: &[u8] = b"sovereign";
pub const DEPOSIT_RECORD_SEED: &[u8] = b"deposit_record";
pub const CREATOR_TRACKER_SEED: &[u8] = b"creator_tracker";
pub const CREATOR_FEE_TRACKER_SEED: &[u8] = b"creator_tracker"; // Alias
pub const PERMANENT_LOCK_SEED: &[u8] = b"permanent_lock";
pub const TOKEN_VAULT_SEED: &[u8] = b"token_vault";
pub const SOL_VAULT_SEED: &[u8] = b"sol_vault";
pub const CREATION_FEE_ESCROW_SEED: &[u8] = b"creation_fee_escrow";
pub const PROPOSAL_SEED: &[u8] = b"proposal";
pub const VOTE_RECORD_SEED: &[u8] = b"vote_record";
pub const GENESIS_NFT_SEED: &[u8] = b"genesis_nft";
pub const GENESIS_NFT_MINT_SEED: &[u8] = b"genesis_nft_mint";

// ============================================================
// EXTERNAL PROGRAM IDS
// ============================================================

/// Orca Whirlpool Program ID
pub const WHIRLPOOL_PROGRAM_ID: Pubkey = whirlpool::ID;

/// Metaplex Token Metadata Program ID  
pub const METAPLEX_PROGRAM_ID: Pubkey = token_metadata::ID;

pub mod whirlpool {
    use anchor_lang::prelude::*;
    declare_id!("whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc");
}

/// Metaplex Token Metadata Program ID
pub mod token_metadata {
    use anchor_lang::prelude::*;
    declare_id!("metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s");
}
