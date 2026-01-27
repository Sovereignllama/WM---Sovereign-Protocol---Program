use anchor_lang::prelude::*;

/// Controls the Orca Whirlpool position NFT
/// This PDA is the permanent delegate/owner of the position
/// - Allows fee collection only
/// - During Recovery: LP locked, can be unwound via governance
/// - After Recovery: LP is PERMANENTLY LOCKED - no unwind possible
#[account]
#[derive(Default)]
pub struct PermanentLock {
    /// The sovereign this lock belongs to
    pub sovereign: Pubkey,
    
    /// The Whirlpool address
    pub whirlpool: Pubkey,
    
    /// Position NFT mint address
    pub position_mint: Pubkey,
    
    /// Position account address (PDA derived from position_mint)
    pub position: Pubkey,
    
    /// Token account holding the position NFT
    pub position_token_account: Pubkey,
    
    /// Total liquidity in the position
    pub liquidity: u128,
    
    /// Lower tick index (always MIN_TICK for full range)
    pub tick_lower_index: i32,
    
    /// Upper tick index (always MAX_TICK for full range)
    pub tick_upper_index: i32,
    
    /// Whether the LP has been unwound (only possible during Recovery)
    pub unwound: bool,
    
    /// Timestamp when position was created
    pub created_at: i64,
    
    /// Timestamp when unwound (0 if not unwound)
    pub unwound_at: i64,
    
    /// PDA bump seed
    pub bump: u8,
}

impl PermanentLock {
    pub const LEN: usize = 8  // discriminator
        + 32  // sovereign
        + 32  // whirlpool
        + 32  // position_mint
        + 32  // position
        + 32  // position_token_account
        + 16  // liquidity
        + 4   // tick_lower_index
        + 4   // tick_upper_index
        + 1   // unwound
        + 8   // created_at
        + 8   // unwound_at
        + 1   // bump
        + 16; // padding
    
    /// Check if the position is still active (not unwound)
    pub fn is_active(&self) -> bool {
        !self.unwound
    }
}

/// Escrow account for holding creation fee during bonding
#[account]
#[derive(Default)]
pub struct CreationFeeEscrow {
    /// The sovereign this escrow belongs to
    pub sovereign: Pubkey,
    
    /// Amount escrowed in lamports
    pub amount: u64,
    
    /// Whether fee has been released (to treasury or refunded)
    pub released: bool,
    
    /// PDA bump seed
    pub bump: u8,
}

impl CreationFeeEscrow {
    pub const LEN: usize = 8  // discriminator
        + 32  // sovereign
        + 8   // amount
        + 1   // released
        + 1   // bump
        + 8;  // padding
}

/// Token vault for holding creator's token deposit
#[account]
#[derive(Default)]
pub struct TokenVault {
    /// The sovereign this vault belongs to
    pub sovereign: Pubkey,
    
    /// Token mint address
    pub token_mint: Pubkey,
    
    /// Amount of tokens held
    pub amount: u64,
    
    /// PDA bump seed
    pub bump: u8,
}

impl TokenVault {
    pub const LEN: usize = 8  // discriminator
        + 32  // sovereign
        + 32  // token_mint
        + 8   // amount
        + 1   // bump
        + 8;  // padding
}

/// SOL vault for holding investor deposits during bonding
#[account]
#[derive(Default)]
pub struct SolVault {
    /// The sovereign this vault belongs to
    pub sovereign: Pubkey,
    
    /// Total investor deposits (excluding creator escrow)
    pub investor_deposits: u64,
    
    /// Creator escrow amount
    pub creator_escrow: u64,
    
    /// PDA bump seed
    pub bump: u8,
}

impl SolVault {
    pub const LEN: usize = 8  // discriminator
        + 32  // sovereign
        + 8   // investor_deposits
        + 8   // creator_escrow
        + 1   // bump
        + 8;  // padding
}
