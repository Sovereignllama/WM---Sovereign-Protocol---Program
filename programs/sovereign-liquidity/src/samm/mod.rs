//! Trashbin SAMM (Raydium CLMM Fork) Integration Module
//!
//! This module provides CPI (Cross-Program Invocation) support for interacting
//! with Trashbin SAMM, a Raydium CLMM fork on the Gorbagana chain.
//!
//! ## Key Components
//!
//! - **accounts**: Account structures for deserializing SAMM program accounts
//! - **instructions**: Instruction builders for SAMM CPI calls
//! - **cpi**: High-level CPI helper functions
//!
//! ## Supported Operations
//!
//! - `open_position_v2` - Create a new LP position
//! - `increase_liquidity_v2` - Add liquidity to an existing position
//! - `decrease_liquidity_v2` - Remove liquidity or collect fees (liquidity=0)
//! - `set_pool_status` - Control pool permissions (LP restrictions)
//!
//! ## Pool Status Bits
//!
//! | Bit | Name | Effect when set to 1 |
//! |-----|------|----------------------|
//! | 0 | OpenPositionOrIncreaseLiquidity | Disables new positions & adding liquidity |
//! | 1 | DecreaseLiquidity | Disables removing liquidity |
//! | 2 | CollectFee | Disables fee collection |
//! | 3 | CollectReward | Disables reward collection |
//! | 4 | Swap | Disables swaps |

pub mod accounts;
pub mod instructions;
pub mod cpi;

pub use accounts::*;
pub use instructions::*;
pub use cpi::*;

use anchor_lang::prelude::*;

/// Trashbin SAMM Program ID (Raydium CLMM fork on Gorbagana)
pub const SAMM_PROGRAM_ID: Pubkey = crate::constants::samm::ID;

/// Q64 fixed-point number representation (used for sqrt_price)
pub type Q64 = u128;

/// Pool status flags
pub mod pool_status {
    /// Disable open position and increase liquidity
    pub const DISABLE_OPEN_POSITION: u8 = 0b00001;
    /// Disable decrease liquidity
    pub const DISABLE_DECREASE_LIQUIDITY: u8 = 0b00010;
    /// Disable fee collection
    pub const DISABLE_COLLECT_FEE: u8 = 0b00100;
    /// Disable reward collection
    pub const DISABLE_COLLECT_REWARD: u8 = 0b01000;
    /// Disable swaps
    pub const DISABLE_SWAP: u8 = 0b10000;
    /// All operations allowed
    pub const ALLOW_ALL: u8 = 0b00000;
}

/// Tick math constants
pub mod tick {
    /// Minimum tick index for full range positions (aligned to tick_spacing=10)
    pub const MIN_TICK: i32 = -443630;
    /// Maximum tick index for full range positions (aligned to tick_spacing=10)
    pub const MAX_TICK: i32 = 443630;
    /// Default tick spacing for standard pools (matches AMM configs on Trashbin)
    pub const DEFAULT_TICK_SPACING: i32 = 10;
}
