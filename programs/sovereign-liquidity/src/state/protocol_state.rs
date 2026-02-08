use anchor_lang::prelude::*;

/// Protocol-level configuration and statistics
/// Single PDA managing global settings for the Sovereign Liquidity Protocol
#[account]
#[derive(Default)]
pub struct ProtocolState {
    /// Protocol admin - can update fees and settings
    pub authority: Pubkey,
    
    /// Treasury wallet receiving protocol fees
    pub treasury: Pubkey,
    
    // ============================================================
    // CREATION FEE (escrowed during bonding)
    // ============================================================
    
    /// Creation fee in basis points (0-1000 = 0-10% of bond target)
    /// Default: 100 (1%)
    pub creation_fee_bps: u16,
    
    /// Minimum fee in lamports (non-refundable on failed bonding)
    /// Default: 0.05 SOL (50_000_000 lamports)
    pub min_fee_lamports: u64,
    
    // ============================================================
    // GOVERNANCE UNWIND FEE
    // ============================================================
    
    /// Fee to create unwind proposal during recovery phase
    /// Default: 0.05 SOL (50_000_000 lamports)
    pub governance_unwind_fee_lamports: u64,
    
    // ============================================================
    // UNWIND FEE
    // ============================================================
    
    /// Fee taken from total WGOR during governance-driven unwind (0-2000 = 0-20%)
    /// NOT applied during emergency unwind. Default: 2000 (20%)
    pub unwind_fee_bps: u16,
    
    // ============================================================
    // BYO TOKEN SETTINGS
    // ============================================================
    
    /// Minimum % of supply required for BYO Token launch
    /// Default: 3000 (30%)
    pub byo_min_supply_bps: u16,
    
    // ============================================================
    // PROTOCOL LIMITS
    // ============================================================
    
    /// Minimum bond target in lamports (50 SOL)
    pub min_bond_target: u64,
    
    /// Minimum single deposit in lamports (0.1 SOL)
    pub min_deposit: u64,
    
    /// Auto-unwind period in seconds (90-365 days)
    /// Protocol-controlled for Active phase activity check
    pub auto_unwind_period: i64,
    
    // ============================================================
    // UNWIND VOLUME THRESHOLD
    // ============================================================
    
    /// Minimum fee growth (Q64.64 delta) during 90-day observation period
    /// to prove the sovereign is still viable and cancel the unwind.
    /// Default: 1.25% of total_deposited annualized (5% APR / 4 quarters).
    /// Adjustable by protocol authority.
    pub min_fee_growth_threshold: u128,
    
    /// If true, fee threshold is locked forever
    pub fee_threshold_renounced: bool,
    
    /// Emergency pause flag
    pub paused: bool,
    
    // ============================================================
    // STATISTICS
    // ============================================================
    
    /// Total sovereigns created
    pub sovereign_count: u64,
    
    /// Lifetime protocol revenue in lamports
    pub total_fees_collected: u64,
    
    /// PDA bump seed
    pub bump: u8,
}

impl ProtocolState {
    pub const LEN: usize = 8  // discriminator
        + 32  // authority
        + 32  // treasury
        + 2   // creation_fee_bps
        + 8   // min_fee_lamports
        + 8   // governance_unwind_fee_lamports
        + 2   // unwind_fee_bps
        + 2   // byo_min_supply_bps
        + 8   // min_bond_target
        + 8   // min_deposit
        + 8   // auto_unwind_period
        + 16  // min_fee_growth_threshold
        + 1   // fee_threshold_renounced
        + 1   // paused
        + 8   // sovereign_count
        + 8   // total_fees_collected
        + 1   // bump
        + 64; // padding for future expansion
    
    /// Default values matching SPEC
    pub fn default_creation_fee_bps() -> u16 { 50 }  // 0.5%
    pub fn default_min_fee_lamports() -> u64 { 50_000_000 }  // 0.05 SOL
    pub fn default_governance_unwind_fee() -> u64 { 50_000_000 }  // 0.05 SOL
    pub fn default_unwind_fee_bps() -> u16 { 2000 }  // 20%
    pub fn default_byo_min_supply_bps() -> u16 { 3000 }  // 30%
    pub fn default_min_bond_target() -> u64 { 50_000_000_000 }  // 50 SOL
    pub fn default_min_deposit() -> u64 { 100_000_000 }  // 0.1 SOL
    pub fn default_auto_unwind_period() -> i64 { 90 * 24 * 60 * 60 }  // 90 days
    pub fn default_min_fee_growth_threshold() -> u128 { 1000 }  // minimum > 0
}
