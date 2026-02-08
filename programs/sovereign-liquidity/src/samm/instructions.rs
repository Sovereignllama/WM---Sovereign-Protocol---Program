//! Trashbin SAMM Instruction Data Structures
//!
//! This module defines the instruction data structures required for
//! CPI calls to the Trashbin SAMM program.
//!
//! Each instruction has:
//! - A discriminator (8 bytes) identifying the instruction
//! - Serialized parameters following the discriminator
//!
//! ## Instruction Discriminators
//!
//! Discriminators are computed as the first 8 bytes of:
//! `sha256("global:<instruction_name>")`

use anchor_lang::prelude::*;
use anchor_lang::solana_program::instruction::{AccountMeta, Instruction};

/// SAMM Program ID
pub use super::SAMM_PROGRAM_ID;

// ============================================================
// INSTRUCTION DISCRIMINATORS
// ============================================================

/// Instruction discriminators for SAMM CPI calls
/// These are the first 8 bytes of sha256("global:<instruction_name>")
pub mod discriminators {
    /// open_position_v2
    pub const OPEN_POSITION_V2: [u8; 8] = [77, 184, 74, 214, 112, 86, 241, 199];
    
    /// increase_liquidity_v2
    pub const INCREASE_LIQUIDITY_V2: [u8; 8] = [133, 29, 89, 223, 69, 238, 176, 10];
    
    /// decrease_liquidity_v2
    pub const DECREASE_LIQUIDITY_V2: [u8; 8] = [58, 127, 188, 62, 79, 82, 196, 96];
    
    /// set_pool_status
    pub const SET_POOL_STATUS: [u8; 8] = [66, 32, 232, 139, 34, 85, 101, 44];
    
    /// swap_v2
    pub const SWAP_V2: [u8; 8] = [43, 4, 237, 11, 26, 201, 106, 217];
    
    /// close_position
    pub const CLOSE_POSITION: [u8; 8] = [123, 134, 81, 0, 49, 68, 98, 98];
    
    /// create_pool
    pub const CREATE_POOL: [u8; 8] = [233, 146, 209, 142, 207, 104, 64, 188];
    
    /// initialize_reward
    pub const INITIALIZE_REWARD: [u8; 8] = [95, 135, 192, 196, 242, 129, 230, 68];
    
    /// collect_remaining_rewards
    pub const COLLECT_REMAINING_REWARDS: [u8; 8] = [18, 237, 166, 197, 34, 16, 213, 144];
}

// ============================================================
// OPEN POSITION V2
// ============================================================

/// Instruction data for open_position_v2
/// Creates a new liquidity position in a pool
#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct OpenPositionV2Args {
    /// Lower tick boundary
    pub tick_lower_index: i32,
    /// Upper tick boundary  
    pub tick_upper_index: i32,
    /// Tick array lower start index
    pub tick_array_lower_start_index: i32,
    /// Tick array upper start index
    pub tick_array_upper_start_index: i32,
    /// Amount of liquidity to add
    pub liquidity: u128,
    /// Maximum amount of token A to deposit
    pub amount_0_max: u64,
    /// Maximum amount of token B to deposit
    pub amount_1_max: u64,
    /// Whether to use token extensions
    pub with_metadata: bool,
    /// Optional: base flag (usually false)
    pub base_flag: Option<bool>,
}

impl OpenPositionV2Args {
    /// Create args for a FULL RANGE position
    ///
    /// When `liquidity = 0` and `base_flag` is `Some(true)`, the SAMM calculates
    /// optimal liquidity from `amount_0_max`. When `Some(false)`, from `amount_1_max`.
    /// This avoids precision mismatches between our f64 math and the SAMM's Q64.64.
    pub fn full_range(
        liquidity: u128,
        amount_0_max: u64,
        amount_1_max: u64,
        tick_spacing: i32,
        base_flag: Option<bool>,
    ) -> Self {
        let tick_lower = super::tick::MIN_TICK;
        let tick_upper = super::tick::MAX_TICK;
        
        // Use floor division for correct negative tick array alignment
        let tick_array_lower_start = super::cpi::get_tick_array_start_index(tick_lower, tick_spacing);
        let tick_array_upper_start = super::cpi::get_tick_array_start_index(tick_upper, tick_spacing);
        
        Self {
            tick_lower_index: tick_lower,
            tick_upper_index: tick_upper,
            tick_array_lower_start_index: tick_array_lower_start,
            tick_array_upper_start_index: tick_array_upper_start,
            liquidity,
            amount_0_max,
            amount_1_max,
            with_metadata: true,
            base_flag,
        }
    }
    
    /// Serialize to instruction data (with discriminator)
    pub fn to_instruction_data(&self) -> Vec<u8> {
        let mut data = discriminators::OPEN_POSITION_V2.to_vec();
        data.extend(self.try_to_vec().unwrap());
        data
    }
}

/// Account ordering for open_position_v2 CPI
/// All accounts must be passed in this exact order
pub struct OpenPositionV2Accounts<'info> {
    /// [signer] Payer
    pub payer: AccountInfo<'info>,
    /// [signer] Position NFT owner
    pub position_nft_owner: AccountInfo<'info>,
    /// [writable] Position NFT mint (to be created)
    pub position_nft_mint: AccountInfo<'info>,
    /// [writable] Position NFT account
    pub position_nft_account: AccountInfo<'info>,
    /// [] Metadata account (if with_metadata = true)
    pub metadata_account: AccountInfo<'info>,
    /// [writable] Pool state
    pub pool_state: AccountInfo<'info>,
    /// [writable] Protocol position
    pub protocol_position: AccountInfo<'info>,
    /// [writable] Tick array lower
    pub tick_array_lower: AccountInfo<'info>,
    /// [writable] Tick array upper
    pub tick_array_upper: AccountInfo<'info>,
    /// [writable] Personal position state
    pub personal_position: AccountInfo<'info>,
    /// [writable] Token account 0 (owner's)
    pub token_account_0: AccountInfo<'info>,
    /// [writable] Token account 1 (owner's)
    pub token_account_1: AccountInfo<'info>,
    /// [writable] Token vault 0
    pub token_vault_0: AccountInfo<'info>,
    /// [writable] Token vault 1
    pub token_vault_1: AccountInfo<'info>,
    /// [] Rent sysvar
    pub rent: AccountInfo<'info>,
    /// [] System program
    pub system_program: AccountInfo<'info>,
    /// [] Token program
    pub token_program: AccountInfo<'info>,
    /// [] Associated token program
    pub associated_token_program: AccountInfo<'info>,
    /// [] Metadata program (Metaplex)
    pub metadata_program: AccountInfo<'info>,
    /// [] Token program 2022 (optional)
    pub token_program_2022: AccountInfo<'info>,
    /// [] Vault 0 mint
    pub vault_0_mint: AccountInfo<'info>,
    /// [] Vault 1 mint
    pub vault_1_mint: AccountInfo<'info>,
    /// [writable] Tick array bitmap extension (remaining account for full-range)
    pub tick_array_bitmap_extension: AccountInfo<'info>,
}

// ============================================================
// INCREASE LIQUIDITY V2
// ============================================================

/// Instruction data for increase_liquidity_v2
/// Adds liquidity to an existing position
#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct IncreaseLiquidityV2Args {
    /// Amount of liquidity to add
    pub liquidity: u128,
    /// Maximum amount of token A to deposit
    pub amount_0_max: u64,
    /// Maximum amount of token B to deposit
    pub amount_1_max: u64,
    /// Optional: base flag
    pub base_flag: Option<bool>,
}

impl IncreaseLiquidityV2Args {
    pub fn new(liquidity: u128, amount_0_max: u64, amount_1_max: u64) -> Self {
        Self {
            liquidity,
            amount_0_max,
            amount_1_max,
            base_flag: None,
        }
    }
    
    /// Serialize to instruction data (with discriminator)
    pub fn to_instruction_data(&self) -> Vec<u8> {
        let mut data = discriminators::INCREASE_LIQUIDITY_V2.to_vec();
        data.extend(self.try_to_vec().unwrap());
        data
    }
}

/// Account ordering for increase_liquidity_v2 CPI
pub struct IncreaseLiquidityV2Accounts<'info> {
    /// [signer] NFT owner
    pub nft_owner: AccountInfo<'info>,
    /// [] NFT account
    pub nft_account: AccountInfo<'info>,
    /// [writable] Pool state
    pub pool_state: AccountInfo<'info>,
    /// [writable] Protocol position
    pub protocol_position: AccountInfo<'info>,
    /// [writable] Personal position
    pub personal_position: AccountInfo<'info>,
    /// [writable] Tick array lower
    pub tick_array_lower: AccountInfo<'info>,
    /// [writable] Tick array upper
    pub tick_array_upper: AccountInfo<'info>,
    /// [writable] Token account 0 (owner's)
    pub token_account_0: AccountInfo<'info>,
    /// [writable] Token account 1 (owner's)
    pub token_account_1: AccountInfo<'info>,
    /// [writable] Token vault 0
    pub token_vault_0: AccountInfo<'info>,
    /// [writable] Token vault 1
    pub token_vault_1: AccountInfo<'info>,
    /// [] Token program
    pub token_program: AccountInfo<'info>,
    /// [] Token program 2022 (optional)
    pub token_program_2022: AccountInfo<'info>,
    /// [] Vault 0 mint
    pub vault_0_mint: AccountInfo<'info>,
    /// [] Vault 1 mint
    pub vault_1_mint: AccountInfo<'info>,
}

// ============================================================
// DECREASE LIQUIDITY V2
// ============================================================

/// Instruction data for decrease_liquidity_v2
/// Removes liquidity and/or collects fees from a position
/// 
/// **IMPORTANT**: To collect fees ONLY (without removing liquidity),
/// pass `liquidity = 0`. This will still collect accrued fees.
#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct DecreaseLiquidityV2Args {
    /// Amount of liquidity to remove (0 = collect fees only)
    pub liquidity: u128,
    /// Minimum amount of token A to receive
    pub amount_0_min: u64,
    /// Minimum amount of token B to receive
    pub amount_1_min: u64,
}

impl DecreaseLiquidityV2Args {
    /// Create args to collect fees only (no liquidity removal)
    pub fn collect_fees_only() -> Self {
        Self {
            liquidity: 0,
            amount_0_min: 0,
            amount_1_min: 0,
        }
    }
    
    /// Create args to remove all liquidity
    pub fn remove_all(liquidity: u128, amount_0_min: u64, amount_1_min: u64) -> Self {
        Self {
            liquidity,
            amount_0_min,
            amount_1_min,
        }
    }
    
    /// Serialize to instruction data (with discriminator)
    pub fn to_instruction_data(&self) -> Vec<u8> {
        let mut data = discriminators::DECREASE_LIQUIDITY_V2.to_vec();
        data.extend(self.try_to_vec().unwrap());
        data
    }
}

/// Account ordering for decrease_liquidity_v2 CPI
pub struct DecreaseLiquidityV2Accounts<'info> {
    /// [signer] NFT owner
    pub nft_owner: AccountInfo<'info>,
    /// [] NFT account
    pub nft_account: AccountInfo<'info>,
    /// [writable] Personal position
    pub personal_position: AccountInfo<'info>,
    /// [writable] Pool state
    pub pool_state: AccountInfo<'info>,
    /// [writable] Protocol position
    pub protocol_position: AccountInfo<'info>,
    /// [writable] Token vault 0
    pub token_vault_0: AccountInfo<'info>,
    /// [writable] Token vault 1
    pub token_vault_1: AccountInfo<'info>,
    /// [writable] Tick array lower
    pub tick_array_lower: AccountInfo<'info>,
    /// [writable] Tick array upper
    pub tick_array_upper: AccountInfo<'info>,
    /// [writable] Recipient token account 0
    pub recipient_token_account_0: AccountInfo<'info>,
    /// [writable] Recipient token account 1
    pub recipient_token_account_1: AccountInfo<'info>,
    /// [] Token program
    pub token_program: AccountInfo<'info>,
    /// [] Token program 2022 (optional)
    pub token_program_2022: AccountInfo<'info>,
    /// [] Memo program (optional)
    pub memo_program: AccountInfo<'info>,
    /// [] Vault 0 mint
    pub vault_0_mint: AccountInfo<'info>,
    /// [] Vault 1 mint
    pub vault_1_mint: AccountInfo<'info>,
    /// [writable] Tick array bitmap extension (required for wide tick ranges)
    pub tick_array_bitmap_extension: AccountInfo<'info>,
}

// ============================================================
// SET POOL STATUS
// ============================================================

/// Instruction data for set_pool_status
/// Controls pool permissions (used for LP restriction during recovery)
/// 
/// ## Status Bits
/// 
/// | Bit | Effect when set to 1 |
/// |-----|----------------------|
/// | 0 | Disable open_position & increase_liquidity |
/// | 1 | Disable decrease_liquidity |
/// | 2 | Disable fee collection |
/// | 3 | Disable reward collection |
/// | 4 | Disable swaps |
#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct SetPoolStatusArgs {
    /// New status bits
    pub status: u8,
}

impl SetPoolStatusArgs {
    /// Disable external LPs during recovery (bit0 = 1)
    /// Trading and fee collection still allowed
    pub fn recovery_restricted() -> Self {
        Self {
            status: super::pool_status::DISABLE_OPEN_POSITION,
        }
    }
    
    /// Allow all operations (post-recovery)
    pub fn allow_all() -> Self {
        Self {
            status: super::pool_status::ALLOW_ALL,
        }
    }
    
    /// Serialize to instruction data (with discriminator)
    pub fn to_instruction_data(&self) -> Vec<u8> {
        let mut data = discriminators::SET_POOL_STATUS.to_vec();
        data.extend(self.try_to_vec().unwrap());
        data
    }
}

/// Account ordering for set_pool_status CPI
/// Only pool owner can call this
pub struct SetPoolStatusAccounts<'info> {
    /// [signer] Pool owner/authority
    pub authority: AccountInfo<'info>,
    /// [writable] Pool state
    pub pool_state: AccountInfo<'info>,
}

// ============================================================
// SWAP V2
// ============================================================

/// Instruction data for swap_v2
/// Execute a swap on the pool
#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct SwapV2Args {
    /// Amount to swap (input if exact_input, output if exact_output)
    pub amount: u64,
    /// Minimum/maximum amount (depends on direction)
    pub other_amount_threshold: u64,
    /// Sqrt price limit (Q64.64)
    pub sqrt_price_limit_x64: u128,
    /// True if swapping exact input amount
    pub is_base_input: bool,
}

impl SwapV2Args {
    /// Create a swap with exact input
    pub fn exact_input(amount_in: u64, min_amount_out: u64, sqrt_price_limit_x64: u128) -> Self {
        Self {
            amount: amount_in,
            other_amount_threshold: min_amount_out,
            sqrt_price_limit_x64,
            is_base_input: true,
        }
    }
    
    /// Create a swap with exact output
    pub fn exact_output(amount_out: u64, max_amount_in: u64, sqrt_price_limit_x64: u128) -> Self {
        Self {
            amount: amount_out,
            other_amount_threshold: max_amount_in,
            sqrt_price_limit_x64,
            is_base_input: false,
        }
    }
    
    /// Serialize to instruction data (with discriminator)
    pub fn to_instruction_data(&self) -> Vec<u8> {
        let mut data = discriminators::SWAP_V2.to_vec();
        data.extend(self.try_to_vec().unwrap());
        data
    }
}

/// Account ordering for swap_v2 CPI
pub struct SwapV2Accounts<'info> {
    /// [signer] Payer/swapper
    pub payer: AccountInfo<'info>,
    /// [] AMM config
    pub amm_config: AccountInfo<'info>,
    /// [writable] Pool state
    pub pool_state: AccountInfo<'info>,
    /// [writable] Input token account (user's)
    pub input_token_account: AccountInfo<'info>,
    /// [writable] Output token account (user's)
    pub output_token_account: AccountInfo<'info>,
    /// [writable] Input vault
    pub input_vault: AccountInfo<'info>,
    /// [writable] Output vault
    pub output_vault: AccountInfo<'info>,
    /// [writable] Observation state
    pub observation_state: AccountInfo<'info>,
    /// [] Token program
    pub token_program: AccountInfo<'info>,
    /// [] Token program 2022 (optional)
    pub token_program_2022: AccountInfo<'info>,
    /// [] Memo program (optional)
    pub memo_program: AccountInfo<'info>,
    /// [] Input vault mint
    pub input_vault_mint: AccountInfo<'info>,
    /// [] Output vault mint  
    pub output_vault_mint: AccountInfo<'info>,
    // Remaining accounts: tick arrays (varies based on swap path)
}

// ============================================================
// CREATE POOL
// ============================================================

/// Instruction data for create_pool
/// Creates a new CLMM pool (if pool doesn't exist)
#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct CreatePoolArgs {
    /// Initial sqrt price (Q64.64)
    pub sqrt_price_x64: u128,
    /// Pool open time (Unix timestamp)
    pub open_time: u64,
}

impl CreatePoolArgs {
    pub fn new(sqrt_price_x64: u128, open_time: u64) -> Self {
        Self {
            sqrt_price_x64,
            open_time,
        }
    }
    
    /// Serialize to instruction data (with discriminator)
    pub fn to_instruction_data(&self) -> Vec<u8> {
        let mut data = discriminators::CREATE_POOL.to_vec();
        data.extend(self.try_to_vec().unwrap());
        data
    }
}

/// Account ordering for create_pool CPI
/// Creates a new CLMM pool with token vaults, observation, and bitmap
pub struct CreatePoolAccounts<'info> {
    /// [signer, writable] Pool creator (pays rent)
    pub pool_creator: AccountInfo<'info>,
    /// [] AMM config account
    pub amm_config: AccountInfo<'info>,
    /// [writable] Pool state PDA (seeds: ["pool", amm_config, token_mint_0, token_mint_1])
    pub pool_state: AccountInfo<'info>,
    /// [] Token mint 0 (lower pubkey)
    pub token_mint_0: AccountInfo<'info>,
    /// [] Token mint 1 (higher pubkey)
    pub token_mint_1: AccountInfo<'info>,
    /// [writable] Token vault 0 PDA (seeds: ["pool_vault", pool_state, token_mint_0])
    pub token_vault_0: AccountInfo<'info>,
    /// [writable] Token vault 1 PDA (seeds: ["pool_vault", pool_state, token_mint_1])
    pub token_vault_1: AccountInfo<'info>,
    /// [writable] Observation state PDA (seeds: ["observation", pool_state])
    pub observation_state: AccountInfo<'info>,
    /// [writable] Tick array bitmap extension PDA
    pub tick_array_bitmap: AccountInfo<'info>,
    /// [] Token program for token_0
    pub token_program_0: AccountInfo<'info>,
    /// [] Token program for token_1
    pub token_program_1: AccountInfo<'info>,
    /// [] System program
    pub system_program: AccountInfo<'info>,
    /// [] Rent sysvar
    pub rent: AccountInfo<'info>,
}

// ============================================================
// INSTRUCTION BUILDERS
// ============================================================

/// Build a raw instruction for CPI
pub fn build_instruction(
    program_id: Pubkey,
    accounts: Vec<AccountMeta>,
    data: Vec<u8>,
) -> Instruction {
    Instruction {
        program_id,
        accounts,
        data,
    }
}

/// Helper to create writable account meta
pub fn writable(pubkey: Pubkey) -> AccountMeta {
    AccountMeta::new(pubkey, false)
}

/// Helper to create writable signer account meta
pub fn writable_signer(pubkey: Pubkey) -> AccountMeta {
    AccountMeta::new(pubkey, true)
}

/// Helper to create readonly account meta
pub fn readonly(pubkey: Pubkey) -> AccountMeta {
    AccountMeta::new_readonly(pubkey, false)
}

/// Helper to create readonly signer account meta
pub fn readonly_signer(pubkey: Pubkey) -> AccountMeta {
    AccountMeta::new_readonly(pubkey, true)
}
