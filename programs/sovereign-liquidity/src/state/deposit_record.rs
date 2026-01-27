use anchor_lang::prelude::*;

/// Tracks an investor's deposit in a sovereign
/// One DepositRecord per depositor per sovereign
/// NOTE: Creator does NOT have a DepositRecord (they use CreatorFeeTracker)
#[account]
#[derive(Default)]
pub struct DepositRecord {
    /// The sovereign this deposit belongs to
    pub sovereign: Pubkey,
    
    /// The investor's wallet address
    pub depositor: Pubkey,
    
    /// Amount deposited in lamports
    pub amount: u64,
    
    /// Share of the pool in basis points (calculated on finalization)
    pub shares_bps: u16,
    
    /// Genesis NFT mint address (set on finalization)
    pub genesis_nft_mint: Pubkey,
    
    /// Total fees claimed
    pub fees_claimed: u64,
    
    /// NFT mint address (if minted)
    pub nft_mint: Option<Pubkey>,
    
    /// Voting power in BPS (set when NFT is minted)
    pub voting_power_bps: u16,
    
    /// Whether NFT has been minted
    pub nft_minted: bool,
    
    /// Whether investor has claimed unwind distribution
    pub unwind_claimed: bool,
    
    /// Whether investor has claimed failed bonding refund
    pub refund_claimed: bool,
    
    /// Timestamp of initial deposit
    pub deposited_at: i64,
    
    /// PDA bump seed
    pub bump: u8,
}

impl DepositRecord {
    pub const LEN: usize = 8  // discriminator
        + 32  // sovereign
        + 32  // depositor
        + 8   // amount
        + 2   // shares_bps
        + 32  // genesis_nft_mint
        + 8   // fees_claimed
        + 33  // nft_mint (Option<Pubkey>)
        + 2   // voting_power_bps
        + 1   // nft_minted
        + 1   // unwind_claimed
        + 1   // refund_claimed
        + 8   // deposited_at
        + 1   // bump
        + 16; // padding
    
    /// Calculate claimable fees based on deposit share
    pub fn calculate_claimable_fees(&self, total_fees: u64, total_deposited: u64) -> u64 {
        if total_deposited == 0 {
            return 0;
        }
        let share = (total_fees as u128 * self.amount as u128 / total_deposited as u128) as u64;
        share.saturating_sub(self.fees_claimed)
    }
    
    /// Calculate claimable SOL fees based on shares
    pub fn calculate_sol_share(&self, total_sol_fees: u64) -> u64 {
        (total_sol_fees as u128 * self.shares_bps as u128 / 10000) as u64
    }
    
    /// Calculate claimable token fees based on shares
    pub fn calculate_token_share(&self, total_token_fees: u64) -> u64 {
        (total_token_fees as u128 * self.shares_bps as u128 / 10000) as u64
    }
    
    /// Check if deposit record has a valid deposit
    pub fn has_deposit(&self) -> bool {
        self.amount > 0
    }
}

/// Tracks creator's fee revenue and purchased tokens
/// Separate from DepositRecord because creator has different rules:
/// - Creator deposits TOKENS (not SOL to LP)
/// - Creator's SOL is escrowed for market buy
/// - Creator does NOT get Genesis NFT
/// - Creator does NOT get LP fee share
#[account]
#[derive(Default)]
pub struct CreatorFeeTracker {
    /// The sovereign this tracker belongs to
    pub sovereign: Pubkey,
    
    /// The creator's wallet address
    pub creator: Pubkey,
    
    /// Total fees earned by creator
    pub total_earned: u64,
    
    /// Total fees claimed by creator
    pub total_claimed: u64,
    
    /// Pending withdrawal amount
    pub pending_withdrawal: u64,
    
    /// Whether fee threshold has been renounced
    pub threshold_renounced: bool,
    
    /// Tokens purchased via market buy (from escrowed SOL)
    pub purchased_tokens: u64,
    
    /// Whether purchased tokens are locked
    pub tokens_locked: bool,
    
    /// Whether creator has claimed purchased tokens (after recovery)
    pub purchased_tokens_claimed: bool,
    
    /// Whether creator has claimed unwind tokens (LP tokens + purchased)
    pub tokens_claimed: bool,
    
    /// Sell tax revenue accumulated (Token Launcher only)
    pub sell_tax_accumulated: u64,
    
    /// Sell tax revenue claimed by creator
    pub sell_tax_claimed: u64,
    
    /// Whether creator has reclaimed on failed bonding
    pub failed_reclaimed: bool,
    
    /// Timestamp when tokens were purchased
    pub purchased_at: i64,
    
    /// PDA bump seed
    pub bump: u8,
}

impl CreatorFeeTracker {
    pub const LEN: usize = 8  // discriminator
        + 32  // sovereign
        + 32  // creator
        + 8   // total_earned
        + 8   // total_claimed
        + 8   // pending_withdrawal
        + 1   // threshold_renounced
        + 8   // purchased_tokens
        + 1   // tokens_locked
        + 1   // purchased_tokens_claimed
        + 1   // tokens_claimed
        + 8   // sell_tax_accumulated
        + 8   // sell_tax_claimed
        + 1   // failed_reclaimed
        + 8   // purchased_at
        + 1   // bump
        + 16; // padding
}
