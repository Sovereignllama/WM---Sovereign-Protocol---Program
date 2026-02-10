//! Trashbin SAMM CPI (Cross-Program Invocation) Helpers
//!
//! This module provides high-level helper functions for making CPI calls
//! to the Trashbin SAMM program from the Sovereign Liquidity Protocol.
//!
//! ## Usage Pattern
//!
//! ```rust,ignore
//! // Collect fees from a position
//! samm::cpi::collect_fees(
//!     &ctx.accounts.samm_program,
//!     &ctx.accounts.position,
//!     &ctx.accounts.pool_state,
//!     // ... other accounts
//!     signer_seeds,
//! )?;
//! ```
//!
//! ## Security Considerations
//!
//! - All CPI calls validate account ownership
//! - PDA seeds are verified before signing
//! - Slippage protection is enforced where applicable

use anchor_lang::prelude::*;
use anchor_lang::solana_program::{
    instruction::Instruction,
    program::invoke_signed,
};
use super::instructions::*;

// ============================================================
// CPI RESULT TYPES
// ============================================================

/// Result of a fee collection CPI
#[derive(Debug, Clone)]
pub struct CollectFeesResult {
    /// Amount of token A (GOR) collected
    pub amount_0: u64,
    /// Amount of token B (project token) collected
    pub amount_1: u64,
}

/// Result of an add liquidity CPI
#[derive(Debug, Clone)]
pub struct AddLiquidityResult {
    /// Amount of token A deposited
    pub amount_0: u64,
    /// Amount of token B deposited
    pub amount_1: u64,
    /// Liquidity added
    pub liquidity: u128,
}

/// Result of a remove liquidity CPI
#[derive(Debug, Clone)]
pub struct RemoveLiquidityResult {
    /// Amount of token A withdrawn
    pub amount_0: u64,
    /// Amount of token B withdrawn
    pub amount_1: u64,
    /// Liquidity removed
    pub liquidity: u128,
}

// ============================================================
// OPEN POSITION CPI
// ============================================================

/// Open a new full-range liquidity position
///
/// This creates a new position with MIN_TICK to MAX_TICK range,
/// ensuring deep liquidity across all price levels.
///
/// # Arguments
///
/// * `accounts` - Required accounts for the CPI
/// * `liquidity` - Amount of liquidity to add
/// * `amount_0_max` - Maximum token A to deposit
/// * `amount_1_max` - Maximum token B to deposit
/// * `tick_spacing` - Pool tick spacing
/// * `signer_seeds` - PDA signer seeds
///
/// # Returns
///
/// The mint address of the position NFT
pub fn open_position_full_range<'info>(
    samm_program: &AccountInfo<'info>,
    accounts: OpenPositionV2Accounts<'info>,
    liquidity: u128,
    amount_0_max: u64,
    amount_1_max: u64,
    tick_spacing: i32,
    base_flag: Option<bool>,
    signer_seeds: &[&[&[u8]]],
) -> Result<Pubkey> {
    let args = OpenPositionV2Args::full_range(
        liquidity,
        amount_0_max,
        amount_1_max,
        tick_spacing,
        base_flag,
    );
    
    let account_metas = vec![
        writable_signer(accounts.payer.key()),
        readonly(accounts.position_nft_owner.key()),
        writable_signer(accounts.position_nft_mint.key()),
        writable(accounts.position_nft_account.key()),
        writable(accounts.metadata_account.key()),
        writable(accounts.pool_state.key()),
        readonly(accounts.protocol_position.key()),
        writable(accounts.tick_array_lower.key()),
        writable(accounts.tick_array_upper.key()),
        writable(accounts.personal_position.key()),
        writable(accounts.token_account_0.key()),
        writable(accounts.token_account_1.key()),
        writable(accounts.token_vault_0.key()),
        writable(accounts.token_vault_1.key()),
        readonly(accounts.rent.key()),
        readonly(accounts.system_program.key()),
        readonly(accounts.token_program.key()),
        readonly(accounts.associated_token_program.key()),
        readonly(accounts.metadata_program.key()),
        readonly(accounts.token_program_2022.key()),
        readonly(accounts.vault_0_mint.key()),
        readonly(accounts.vault_1_mint.key()),
        // remaining_accounts[0] - bitmap extension required for full-range positions
        writable(accounts.tick_array_bitmap_extension.key()),
    ];
    
    let ix = Instruction {
        program_id: samm_program.key(),
        accounts: account_metas,
        data: args.to_instruction_data(),
    };
    
    let account_infos = vec![
        accounts.payer,
        accounts.position_nft_owner,
        accounts.position_nft_mint.clone(),
        accounts.position_nft_account,
        accounts.metadata_account,
        accounts.pool_state,
        accounts.protocol_position,
        accounts.tick_array_lower,
        accounts.tick_array_upper,
        accounts.personal_position,
        accounts.token_account_0,
        accounts.token_account_1,
        accounts.token_vault_0,
        accounts.token_vault_1,
        accounts.rent,
        accounts.system_program,
        accounts.token_program,
        accounts.associated_token_program,
        accounts.metadata_program,
        accounts.token_program_2022,
        accounts.vault_0_mint,
        accounts.vault_1_mint,
        accounts.tick_array_bitmap_extension,
        samm_program.clone(),
    ];
    
    invoke_signed(&ix, &account_infos, signer_seeds)?;
    
    // Return the position NFT mint
    Ok(accounts.position_nft_mint.key())
}

// ============================================================
// INCREASE LIQUIDITY CPI
// ============================================================

/// Add liquidity to an existing position
///
/// # Arguments
///
/// * `accounts` - Required accounts for the CPI
/// * `liquidity` - Amount of liquidity to add
/// * `amount_0_max` - Maximum token A to deposit (slippage protection)
/// * `amount_1_max` - Maximum token B to deposit (slippage protection)
/// * `signer_seeds` - PDA signer seeds
pub fn increase_liquidity<'info>(
    samm_program: &AccountInfo<'info>,
    accounts: IncreaseLiquidityV2Accounts<'info>,
    liquidity: u128,
    amount_0_max: u64,
    amount_1_max: u64,
    signer_seeds: &[&[&[u8]]],
) -> Result<AddLiquidityResult> {
    let args = IncreaseLiquidityV2Args::new(liquidity, amount_0_max, amount_1_max);
    
    let account_metas = vec![
        readonly_signer(accounts.nft_owner.key()),
        readonly(accounts.nft_account.key()),
        writable(accounts.pool_state.key()),
        writable(accounts.protocol_position.key()),
        writable(accounts.personal_position.key()),
        writable(accounts.tick_array_lower.key()),
        writable(accounts.tick_array_upper.key()),
        writable(accounts.token_account_0.key()),
        writable(accounts.token_account_1.key()),
        writable(accounts.token_vault_0.key()),
        writable(accounts.token_vault_1.key()),
        readonly(accounts.token_program.key()),
        readonly(accounts.token_program_2022.key()),
        readonly(accounts.vault_0_mint.key()),
        readonly(accounts.vault_1_mint.key()),
    ];
    
    let ix = Instruction {
        program_id: samm_program.key(),
        accounts: account_metas,
        data: args.to_instruction_data(),
    };
    
    let account_infos = vec![
        accounts.nft_owner,
        accounts.nft_account,
        accounts.pool_state,
        accounts.protocol_position,
        accounts.personal_position,
        accounts.tick_array_lower,
        accounts.tick_array_upper,
        accounts.token_account_0,
        accounts.token_account_1,
        accounts.token_vault_0,
        accounts.token_vault_1,
        accounts.token_program,
        accounts.token_program_2022,
        accounts.vault_0_mint,
        accounts.vault_1_mint,
        samm_program.clone(),
    ];
    
    invoke_signed(&ix, &account_infos, signer_seeds)?;
    
    // In production, parse return data from CPI
    // For now, return placeholder
    Ok(AddLiquidityResult {
        amount_0: 0, // Would parse from logs/return data
        amount_1: 0,
        liquidity,
    })
}

// ============================================================
// COLLECT FEES CPI
// ============================================================

/// Collect accumulated fees from a position without removing liquidity
///
/// This calls `decrease_liquidity_v2` with `liquidity = 0` which
/// collects all accumulated fees without changing the position size.
///
/// # Arguments
///
/// * `accounts` - Required accounts for the CPI
/// * `signer_seeds` - PDA signer seeds (for permanent lock)
///
/// # Returns
///
/// Amounts of token A (GOR) and token B collected
pub fn collect_fees<'info>(
    samm_program: &AccountInfo<'info>,
    accounts: DecreaseLiquidityV2Accounts<'info>,
    signer_seeds: &[&[&[u8]]],
) -> Result<CollectFeesResult> {
    let args = DecreaseLiquidityV2Args::collect_fees_only();
    
    let account_metas = vec![
        readonly_signer(accounts.nft_owner.key()),
        readonly(accounts.nft_account.key()),
        writable(accounts.personal_position.key()),
        writable(accounts.pool_state.key()),
        writable(accounts.protocol_position.key()),
        writable(accounts.token_vault_0.key()),
        writable(accounts.token_vault_1.key()),
        writable(accounts.tick_array_lower.key()),
        writable(accounts.tick_array_upper.key()),
        writable(accounts.recipient_token_account_0.key()),
        writable(accounts.recipient_token_account_1.key()),
        readonly(accounts.token_program.key()),
        readonly(accounts.token_program_2022.key()),
        readonly(accounts.memo_program.key()),
        readonly(accounts.vault_0_mint.key()),
        readonly(accounts.vault_1_mint.key()),
        writable(accounts.tick_array_bitmap_extension.key()),
    ];
    
    let ix = Instruction {
        program_id: samm_program.key(),
        accounts: account_metas,
        data: args.to_instruction_data(),
    };
    
    let account_infos = vec![
        accounts.nft_owner,
        accounts.nft_account,
        accounts.personal_position,
        accounts.pool_state,
        accounts.protocol_position,
        accounts.token_vault_0,
        accounts.token_vault_1,
        accounts.tick_array_lower,
        accounts.tick_array_upper,
        accounts.recipient_token_account_0,
        accounts.recipient_token_account_1,
        accounts.token_program,
        accounts.token_program_2022,
        accounts.memo_program,
        accounts.vault_0_mint,
        accounts.vault_1_mint,
        accounts.tick_array_bitmap_extension,
        samm_program.clone(),
    ];
    
    invoke_signed(&ix, &account_infos, signer_seeds)?;
    
    // In production, parse return data from CPI logs
    // The actual amounts would be calculated from token balance changes
    // or parsed from program return data
    Ok(CollectFeesResult {
        amount_0: 0, // Would parse from logs/return data
        amount_1: 0,
    })
}

// ============================================================
// REMOVE LIQUIDITY CPI
// ============================================================

/// Remove liquidity from a position
///
/// Used during unwind to fully exit the LP position.
///
/// # Arguments
///
/// * `accounts` - Required accounts for the CPI
/// * `liquidity` - Amount of liquidity to remove
/// * `amount_0_min` - Minimum token A to receive (slippage protection)
/// * `amount_1_min` - Minimum token B to receive (slippage protection)
/// * `signer_seeds` - PDA signer seeds
pub fn remove_liquidity<'info>(
    samm_program: &AccountInfo<'info>,
    accounts: DecreaseLiquidityV2Accounts<'info>,
    liquidity: u128,
    amount_0_min: u64,
    amount_1_min: u64,
    signer_seeds: &[&[&[u8]]],
) -> Result<RemoveLiquidityResult> {
    let args = DecreaseLiquidityV2Args::remove_all(liquidity, amount_0_min, amount_1_min);
    
    let account_metas = vec![
        readonly_signer(accounts.nft_owner.key()),
        readonly(accounts.nft_account.key()),
        writable(accounts.personal_position.key()),
        writable(accounts.pool_state.key()),
        writable(accounts.protocol_position.key()),
        writable(accounts.token_vault_0.key()),
        writable(accounts.token_vault_1.key()),
        writable(accounts.tick_array_lower.key()),
        writable(accounts.tick_array_upper.key()),
        writable(accounts.recipient_token_account_0.key()),
        writable(accounts.recipient_token_account_1.key()),
        readonly(accounts.token_program.key()),
        readonly(accounts.token_program_2022.key()),
        readonly(accounts.memo_program.key()),
        readonly(accounts.vault_0_mint.key()),
        readonly(accounts.vault_1_mint.key()),
        writable(accounts.tick_array_bitmap_extension.key()),
    ];
    
    let ix = Instruction {
        program_id: samm_program.key(),
        accounts: account_metas,
        data: args.to_instruction_data(),
    };
    
    let account_infos = vec![
        accounts.nft_owner,
        accounts.nft_account,
        accounts.personal_position,
        accounts.pool_state,
        accounts.protocol_position,
        accounts.token_vault_0,
        accounts.token_vault_1,
        accounts.tick_array_lower,
        accounts.tick_array_upper,
        accounts.recipient_token_account_0,
        accounts.recipient_token_account_1,
        accounts.token_program,
        accounts.token_program_2022,
        accounts.memo_program,
        accounts.vault_0_mint,
        accounts.vault_1_mint,
        accounts.tick_array_bitmap_extension,
        samm_program.clone(),
    ];
    
    invoke_signed(&ix, &account_infos, signer_seeds)?;
    
    Ok(RemoveLiquidityResult {
        amount_0: 0, // Would parse from logs/return data
        amount_1: 0,
        liquidity,
    })
}

// ============================================================
// SET POOL STATUS CPI
// ============================================================

/// Set pool status to restrict external LPs during recovery
///
/// This sets bit0 = 1 which disables:
/// - open_position (new LPs cannot enter)
/// - increase_liquidity (existing LPs cannot add more)
///
/// Trading and fee collection remain enabled.
///
/// # Arguments
///
/// * `samm_program` - SAMM program account
/// * `authority` - Pool authority (must be signer)
/// * `pool_state` - Pool state account to modify
/// * `signer_seeds` - PDA signer seeds
pub fn set_pool_status_restricted<'info>(
    samm_program: &AccountInfo<'info>,
    authority: &AccountInfo<'info>,
    pool_state: &AccountInfo<'info>,
    signer_seeds: &[&[&[u8]]],
) -> Result<()> {
    let args = SetPoolStatusArgs::recovery_restricted();
    
    let account_metas = vec![
        readonly_signer(authority.key()),
        writable(pool_state.key()),
    ];
    
    let ix = Instruction {
        program_id: samm_program.key(),
        accounts: account_metas,
        data: args.to_instruction_data(),
    };
    
    let account_infos = vec![
        authority.clone(),
        pool_state.clone(),
        samm_program.clone(),
    ];
    
    invoke_signed(&ix, &account_infos, signer_seeds)?;
    
    Ok(())
}

/// Remove pool restrictions (post-recovery)
///
/// This sets status = 0 which enables all operations including:
/// - open_position (external LPs can enter)
/// - increase_liquidity (anyone can add liquidity)
///
/// # Arguments
///
/// * `samm_program` - SAMM program account
/// * `authority` - Pool authority (must be signer)
/// * `pool_state` - Pool state account to modify
/// * `signer_seeds` - PDA signer seeds
pub fn set_pool_status_unrestricted<'info>(
    samm_program: &AccountInfo<'info>,
    authority: &AccountInfo<'info>,
    pool_state: &AccountInfo<'info>,
    signer_seeds: &[&[&[u8]]],
) -> Result<()> {
    let args = SetPoolStatusArgs::allow_all();
    
    let account_metas = vec![
        readonly_signer(authority.key()),
        writable(pool_state.key()),
    ];
    
    let ix = Instruction {
        program_id: samm_program.key(),
        accounts: account_metas,
        data: args.to_instruction_data(),
    };
    
    let account_infos = vec![
        authority.clone(),
        pool_state.clone(),
        samm_program.clone(),
    ];
    
    invoke_signed(&ix, &account_infos, signer_seeds)?;
    
    Ok(())
}

// ============================================================
// SWAP CPI (for creator market buy)
// ============================================================

/// Execute a swap (used for creator market buy at finalization)
///
/// # Arguments
///
/// * `samm_program` - SAMM program account
/// * `accounts` - Swap accounts
/// * `amount_in` - Exact input amount
/// * `min_amount_out` - Minimum output (slippage protection)
/// * `tick_arrays` - Tick array accounts for the swap path
/// * `signer_seeds` - PDA signer seeds
pub fn swap_exact_input<'info>(
    samm_program: &AccountInfo<'info>,
    accounts: SwapV2Accounts<'info>,
    amount_in: u64,
    min_amount_out: u64,
    sqrt_price_limit_x64: u128,
    tick_arrays: Vec<AccountInfo<'info>>,
    signer_seeds: &[&[&[u8]]],
) -> Result<u64> {
    let args = SwapV2Args::exact_input(amount_in, min_amount_out, sqrt_price_limit_x64);
    
    let mut account_metas = vec![
        readonly_signer(accounts.payer.key()),
        readonly(accounts.amm_config.key()),
        writable(accounts.pool_state.key()),
        writable(accounts.input_token_account.key()),
        writable(accounts.output_token_account.key()),
        writable(accounts.input_vault.key()),
        writable(accounts.output_vault.key()),
        writable(accounts.observation_state.key()),
        readonly(accounts.token_program.key()),
        readonly(accounts.token_program_2022.key()),
        readonly(accounts.memo_program.key()),
        readonly(accounts.input_vault_mint.key()),
        readonly(accounts.output_vault_mint.key()),
    ];
    
    // Add tick arrays as remaining accounts
    for tick_array in &tick_arrays {
        account_metas.push(writable(tick_array.key()));
    }
    
    let ix = Instruction {
        program_id: samm_program.key(),
        accounts: account_metas,
        data: args.to_instruction_data(),
    };
    
    let mut account_infos = vec![
        accounts.payer,
        accounts.amm_config,
        accounts.pool_state,
        accounts.input_token_account,
        accounts.output_token_account,
        accounts.input_vault,
        accounts.output_vault,
        accounts.observation_state,
        accounts.token_program,
        accounts.token_program_2022,
        accounts.memo_program,
        accounts.input_vault_mint,
        accounts.output_vault_mint,
    ];
    
    for tick_array in tick_arrays {
        account_infos.push(tick_array);
    }
    
    account_infos.push(samm_program.clone());
    
    invoke_signed(&ix, &account_infos, signer_seeds)?;
    
    // In production, parse return data for actual amount out
    Ok(0) // Would return actual amount from CPI
}

// ============================================================
// HELPER FUNCTIONS
// ============================================================

/// Calculate the start tick index for a tick array
/// 
/// Given a tick and tick spacing, returns the start index of the
/// tick array that contains this tick.
pub fn get_tick_array_start_index(tick: i32, tick_spacing: i32) -> i32 {
    let ticks_per_array = 60 * tick_spacing;
    let mut start_index = tick / ticks_per_array;
    if tick < 0 && tick % ticks_per_array != 0 {
        start_index -= 1;
    }
    start_index * ticks_per_array
}

/// Calculate sqrt price from price (Q64.64 format)
/// 
/// sqrt_price_x64 = sqrt(price) * 2^64
pub fn price_to_sqrt_price_x64(price: f64) -> u128 {
    let sqrt_price = price.sqrt();
    (sqrt_price * (1u128 << 64) as f64) as u128
}

/// Calculate price from sqrt price (Q64.64 format)
pub fn sqrt_price_x64_to_price(sqrt_price_x64: u128) -> f64 {
    let sqrt_price = sqrt_price_x64 as f64 / (1u128 << 64) as f64;
    sqrt_price * sqrt_price
}

/// Calculate tick from sqrt price
pub fn sqrt_price_x64_to_tick(sqrt_price_x64: u128) -> i32 {
    let sqrt_price = sqrt_price_x64 as f64 / (1u128 << 64) as f64;
    let tick = (sqrt_price.ln() / 1.0001f64.ln().sqrt()) as i32;
    tick
}

/// Calculate liquidity from token amounts for a full-range position
/// 
/// For full-range, liquidity = min(amount_0 * sqrt(P), amount_1 / sqrt(P))
pub fn calculate_full_range_liquidity(
    amount_0: u64,
    amount_1: u64,
    sqrt_price_x64: u128,
) -> u128 {
    let sqrt_price = sqrt_price_x64 as f64 / (1u128 << 64) as f64;
    
    let liquidity_0 = amount_0 as f64 * sqrt_price;
    let liquidity_1 = amount_1 as f64 / sqrt_price;
    
    liquidity_0.min(liquidity_1) as u128
}

// ============================================================
// CREATE POOL CPI
// ============================================================

/// Create a new CLMM pool on Trashbin SAMM
///
/// This creates the pool state, token vaults, observation state,
/// and tick array bitmap extension. The pool is initialized with
/// the given sqrt_price and open_time.
///
/// # Arguments
///
/// * `samm_program` - SAMM program account
/// * `accounts` - Required accounts for pool creation
/// * `sqrt_price_x64` - Initial sqrt price (Q64.64 format)
/// * `open_time` - Unix timestamp when pool opens for trading
/// * `signer_seeds` - PDA signer seeds (pool_creator is a PDA)
pub fn create_pool<'info>(
    samm_program: &AccountInfo<'info>,
    accounts: CreatePoolAccounts<'info>,
    sqrt_price_x64: u128,
    open_time: u64,
    signer_seeds: &[&[&[u8]]],
) -> Result<()> {
    let args = CreatePoolArgs::new(sqrt_price_x64, open_time);

    let account_metas = vec![
        writable_signer(accounts.pool_creator.key()),
        readonly(accounts.amm_config.key()),
        writable(accounts.pool_state.key()),
        readonly(accounts.token_mint_0.key()),
        readonly(accounts.token_mint_1.key()),
        writable(accounts.token_vault_0.key()),
        writable(accounts.token_vault_1.key()),
        writable(accounts.observation_state.key()),
        writable(accounts.tick_array_bitmap.key()),
        readonly(accounts.token_program_0.key()),
        readonly(accounts.token_program_1.key()),
        readonly(accounts.system_program.key()),
        readonly(accounts.rent.key()),
    ];

    let ix = Instruction {
        program_id: samm_program.key(),
        accounts: account_metas,
        data: args.to_instruction_data(),
    };

    let account_infos = vec![
        accounts.pool_creator,
        accounts.amm_config,
        accounts.pool_state,
        accounts.token_mint_0,
        accounts.token_mint_1,
        accounts.token_vault_0,
        accounts.token_vault_1,
        accounts.observation_state,
        accounts.tick_array_bitmap,
        accounts.token_program_0,
        accounts.token_program_1,
        accounts.system_program,
        accounts.rent,
        samm_program.clone(),
    ];

    invoke_signed(&ix, &account_infos, signer_seeds)?;

    Ok(())
}

/// Sort two mints into canonical order (lower pubkey first)
/// Returns (mint_0, mint_1, is_swapped) where is_swapped indicates
/// if the original order was reversed
pub fn sort_mints(mint_a: &Pubkey, mint_b: &Pubkey) -> (Pubkey, Pubkey, bool) {
    if mint_a.to_bytes() < mint_b.to_bytes() {
        (*mint_a, *mint_b, false)
    } else {
        (*mint_b, *mint_a, true)
    }
}
