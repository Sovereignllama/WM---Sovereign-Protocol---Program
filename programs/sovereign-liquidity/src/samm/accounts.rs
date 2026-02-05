//! Trashbin SAMM Account Structures
//!
//! These structures are used to deserialize account data from the Trashbin SAMM
//! program (Raydium CLMM fork). They mirror the on-chain account layouts.
//!
//! ## Account Sizes
//!
//! | Account | Size (bytes) |
//! |---------|--------------|
//! | PoolState | 1544 |
//! | PersonalPositionState | 200 |
//! | ProtocolPositionState | 144 |
//! | TickArrayState | 4483 |
//! | ObservationState | 52 |

use anchor_lang::prelude::*;

/// Discriminator for SAMM accounts (8 bytes)
pub const SAMM_ACCOUNT_DISCRIMINATOR_LEN: usize = 8;

// ============================================================
// POOL STATE
// ============================================================

/// The main pool state account for a CLMM pool
/// Size: 1544 bytes
#[derive(Clone)]
pub struct PoolState {
    /// Bump seed for the pool PDA
    pub bump: [u8; 1],
    /// AMM config account
    pub amm_config: Pubkey,
    /// Pool creator
    pub owner: Pubkey,
    
    /// Token mint for token A (typically the base token)
    pub token_mint_0: Pubkey,
    /// Token mint for token B (typically the quote token)
    pub token_mint_1: Pubkey,
    
    /// Token vault for token A
    pub token_vault_0: Pubkey,
    /// Token vault for token B
    pub token_vault_1: Pubkey,
    
    /// Observation account for oracle
    pub observation_key: Pubkey,
    
    /// Mint decimals for token A
    pub mint_decimals_0: u8,
    /// Mint decimals for token B
    pub mint_decimals_1: u8,
    
    /// Tick spacing for the pool
    pub tick_spacing: u16,
    
    /// Current liquidity in the pool
    pub liquidity: u128,
    
    /// Current sqrt price (Q64.64 format)
    pub sqrt_price_x64: u128,
    
    /// Current tick index
    pub tick_current: i32,
    
    /// Padding for alignment
    pub padding_3: u16,
    pub padding_4: u16,
    
    /// Global fee growth for token A (Q64.64)
    pub fee_growth_global_0_x64: u128,
    /// Global fee growth for token B (Q64.64)
    pub fee_growth_global_1_x64: u128,
    
    /// Protocol fees owed for token A
    pub protocol_fees_token_0: u64,
    /// Protocol fees owed for token B
    pub protocol_fees_token_1: u64,
    
    /// Swap in amount for token A
    pub swap_in_amount_token_0: u128,
    /// Swap out amount for token A
    pub swap_out_amount_token_0: u128,
    /// Swap in amount for token B
    pub swap_in_amount_token_1: u128,
    /// Swap out amount for token B
    pub swap_out_amount_token_1: u128,
    
    /// Pool status flags (bit field)
    /// bit0: OpenPositionOrIncreaseLiquidity (1 = disabled)
    /// bit1: DecreaseLiquidity (1 = disabled)
    /// bit2: CollectFee (1 = disabled)
    /// bit3: CollectReward (1 = disabled)
    /// bit4: Swap (1 = disabled)
    pub status: u8,
    
    /// Padding
    pub padding: [u8; 7],
    
    /// Reward info array (3 reward tokens max)
    pub reward_infos: [RewardInfo; 3],
    
    /// Tick array bitmap for efficient tick lookup
    pub tick_array_bitmap: [u64; 16],
    
    /// Total fees claimed for token A
    pub total_fees_claimed_token_0: u64,
    /// Total fees claimed for token B
    pub total_fees_claimed_token_1: u64,
    
    /// Fund fees for token A
    pub fund_fees_token_0: u64,
    /// Fund fees for token B
    pub fund_fees_token_1: u64,
    
    /// Open time for the pool
    pub open_time: u64,
    
    /// Recent epoch
    pub recent_epoch: u64,
    
    /// Padding for future use
    pub padding_1: [u64; 24],
    pub padding_2: [u64; 32],
}

impl PoolState {
    /// Account size in bytes
    pub const LEN: usize = 1544;
    
    /// Check if pool allows opening positions
    pub fn can_open_position(&self) -> bool {
        self.status & 0b00001 == 0
    }
    
    /// Check if pool allows decreasing liquidity
    pub fn can_decrease_liquidity(&self) -> bool {
        self.status & 0b00010 == 0
    }
    
    /// Check if pool allows fee collection
    pub fn can_collect_fee(&self) -> bool {
        self.status & 0b00100 == 0
    }
    
    /// Check if pool allows swaps
    pub fn can_swap(&self) -> bool {
        self.status & 0b10000 == 0
    }
    
    /// Check if this is a recovery-restricted pool
    /// (only genesis position can LP, but trading is open)
    pub fn is_recovery_restricted(&self) -> bool {
        // bit0 set (no new LPs) but bit4 clear (swaps allowed)
        (self.status & 0b00001 != 0) && (self.status & 0b10000 == 0)
    }
}

// ============================================================
// REWARD INFO
// ============================================================

/// Reward token info within a pool
#[derive(Clone, Copy, Default, AnchorSerialize, AnchorDeserialize)]
pub struct RewardInfo {
    /// Reward state (0 = uninitialized, 1 = initialized, 2 = opening)
    pub reward_state: u8,
    /// Open time
    pub open_time: u64,
    /// End time
    pub end_time: u64,
    /// Last update time
    pub last_update_time: u64,
    /// Emissions per second (Q64.64)
    pub emissions_per_second_x64: u128,
    /// Reward total emitted
    pub reward_total_emitted: u64,
    /// Reward claimed
    pub reward_claimed: u64,
    /// Reward token mint
    pub token_mint: Pubkey,
    /// Reward token vault
    pub token_vault: Pubkey,
    /// Authority (funder)
    pub authority: Pubkey,
    /// Reward growth global (Q64.64)
    pub reward_growth_global_x64: u128,
}

impl RewardInfo {
    pub const LEN: usize = 161;
}

// ============================================================
// PERSONAL POSITION STATE
// ============================================================

/// Individual LP position (PersonalPositionState in Raydium CLMM)
/// This is what each liquidity provider owns
/// Size: ~200 bytes
#[derive(Clone)]
pub struct PersonalPositionState {
    /// Bump seed
    pub bump: u8,
    
    /// NFT mint for this position
    pub nft_mint: Pubkey,
    
    /// Pool this position belongs to
    pub pool_id: Pubkey,
    
    /// Lower tick boundary
    pub tick_lower_index: i32,
    /// Upper tick boundary
    pub tick_upper_index: i32,
    
    /// Liquidity owned by this position
    pub liquidity: u128,
    
    /// Fee growth inside at last update for token A (Q64.64)
    pub fee_growth_inside_0_last_x64: u128,
    /// Fee growth inside at last update for token B (Q64.64)
    pub fee_growth_inside_1_last_x64: u128,
    
    /// Uncollected fees for token A
    pub token_fees_owed_0: u64,
    /// Uncollected fees for token B
    pub token_fees_owed_1: u64,
    
    /// Reward info for each reward token
    pub reward_infos: [PositionRewardInfo; 3],
    
    /// Padding for future use
    pub padding: [u64; 8],
}

impl PersonalPositionState {
    /// Account size in bytes
    pub const LEN: usize = 200;
    
    /// Check if this is a full-range position
    pub fn is_full_range(&self) -> bool {
        self.tick_lower_index == super::tick::MIN_TICK
            && self.tick_upper_index == super::tick::MAX_TICK
    }
}

/// Position reward info
#[derive(Clone, Copy, Default, AnchorSerialize, AnchorDeserialize)]
pub struct PositionRewardInfo {
    /// Growth inside at last update (Q64.64)
    pub growth_inside_last_x64: u128,
    /// Uncollected reward amount
    pub reward_amount_owed: u64,
}

impl PositionRewardInfo {
    pub const LEN: usize = 24;
}

// ============================================================
// PROTOCOL POSITION STATE
// ============================================================

/// Protocol position state (used for tracking protocol-level positions)
/// Size: ~144 bytes
#[derive(Clone)]
pub struct ProtocolPositionState {
    /// Bump seed
    pub bump: u8,
    
    /// Pool this position belongs to
    pub pool_id: Pubkey,
    
    /// Lower tick boundary
    pub tick_lower_index: i32,
    /// Upper tick boundary
    pub tick_upper_index: i32,
    
    /// Liquidity
    pub liquidity: u128,
    
    /// Fee growth inside at last update for token A
    pub fee_growth_inside_0_last_x64: u128,
    /// Fee growth inside at last update for token B
    pub fee_growth_inside_1_last_x64: u128,
    
    /// Uncollected fees for token A
    pub token_fees_owed_0: u64,
    /// Uncollected fees for token B
    pub token_fees_owed_1: u64,
    
    /// Reward growth inside
    pub reward_growth_inside: [u128; 3],
}

impl ProtocolPositionState {
    pub const LEN: usize = 144;
}

// ============================================================
// TICK ARRAY STATE
// ============================================================

/// A tick array containing multiple ticks
/// Each tick array covers a range of ticks
/// Size: ~4483 bytes
#[derive(Clone)]
pub struct TickArrayState {
    /// Pool this tick array belongs to
    pub pool_id: Pubkey,
    
    /// Starting tick index of this array
    pub start_tick_index: i32,
    
    /// Array of ticks (60 ticks per array for tick_spacing=64)
    pub ticks: [TickState; 60],
    
    /// Initialized tick count
    pub initialized_tick_count: u8,
    
    /// Padding
    pub padding: [u8; 115],
}

impl TickArrayState {
    pub const LEN: usize = 4483;
    
    /// Number of ticks per array
    pub const TICK_COUNT: usize = 60;
}

/// Individual tick state
#[derive(Clone, Copy, Default, AnchorSerialize, AnchorDeserialize)]
pub struct TickState {
    /// The tick index
    pub tick: i32,
    
    /// Net liquidity change when crossing this tick
    pub liquidity_net: i128,
    
    /// Gross liquidity referencing this tick
    pub liquidity_gross: u128,
    
    /// Fee growth outside for token A (Q64.64)
    pub fee_growth_outside_0_x64: u128,
    /// Fee growth outside for token B (Q64.64)
    pub fee_growth_outside_1_x64: u128,
    
    /// Reward growth outside for each reward token
    pub reward_growths_outside_x64: [u128; 3],
    
    /// Padding
    pub padding: [u32; 13],
}

impl TickState {
    pub const LEN: usize = 73;
}

// ============================================================
// OBSERVATION STATE
// ============================================================

/// Oracle observation for TWAP
#[derive(Clone, Copy, Default, AnchorSerialize, AnchorDeserialize)]
pub struct ObservationState {
    /// Whether this observation has been initialized
    pub initialized: bool,
    /// Block timestamp of observation
    pub block_timestamp: u32,
    /// Cumulative tick value
    pub tick_cumulative: i64,
    /// Padding
    pub padding: [u64; 4],
}

impl ObservationState {
    pub const LEN: usize = 52;
}

// ============================================================
// AMM CONFIG
// ============================================================

/// AMM configuration account
#[derive(Clone)]
pub struct AmmConfig {
    /// Bump seed
    pub bump: u8,
    /// Disable create pool flag
    pub disable_create_pool: bool,
    /// Index of the config
    pub index: u16,
    /// Trade fee rate in hundredths of a bip (1e-6)
    pub trade_fee_rate: u32,
    /// Protocol fee rate (percentage of trade fee)
    pub protocol_fee_rate: u32,
    /// Fund fee rate
    pub fund_fee_rate: u32,
    /// Create pool fee
    pub create_pool_fee: u64,
    /// Protocol owner
    pub protocol_owner: Pubkey,
    /// Fund owner
    pub fund_owner: Pubkey,
    /// Padding
    pub padding: [u64; 16],
}

impl AmmConfig {
    pub const LEN: usize = 200;
}

// ============================================================
// DESERIALIZATION HELPERS
// ============================================================

/// Trait for deserializing SAMM accounts from raw data
pub trait SammAccountDeserialize: Sized {
    /// Deserialize from account data (skipping discriminator)
    fn try_deserialize(data: &[u8]) -> Result<Self>;
    
    /// Get the expected account discriminator
    fn discriminator() -> [u8; 8];
}

impl SammAccountDeserialize for PoolState {
    fn try_deserialize(data: &[u8]) -> Result<Self> {
        if data.len() < Self::LEN {
            return Err(error!(crate::errors::SovereignError::InvalidAccountData));
        }
        
        // Verify discriminator
        let disc = &data[0..8];
        if disc != Self::discriminator() {
            return Err(error!(crate::errors::SovereignError::InvalidAccountData));
        }
        
        // Parse the account data
        // This is a simplified version - full implementation would use borsh
        let data = &data[8..]; // Skip discriminator
        
        Ok(Self {
            bump: [data[0]],
            amm_config: Pubkey::try_from(&data[1..33]).unwrap(),
            owner: Pubkey::try_from(&data[33..65]).unwrap(),
            token_mint_0: Pubkey::try_from(&data[65..97]).unwrap(),
            token_mint_1: Pubkey::try_from(&data[97..129]).unwrap(),
            token_vault_0: Pubkey::try_from(&data[129..161]).unwrap(),
            token_vault_1: Pubkey::try_from(&data[161..193]).unwrap(),
            observation_key: Pubkey::try_from(&data[193..225]).unwrap(),
            mint_decimals_0: data[225],
            mint_decimals_1: data[226],
            tick_spacing: u16::from_le_bytes([data[227], data[228]]),
            liquidity: u128::from_le_bytes(data[229..245].try_into().unwrap()),
            sqrt_price_x64: u128::from_le_bytes(data[245..261].try_into().unwrap()),
            tick_current: i32::from_le_bytes(data[261..265].try_into().unwrap()),
            padding_3: 0,
            padding_4: 0,
            fee_growth_global_0_x64: u128::from_le_bytes(data[269..285].try_into().unwrap()),
            fee_growth_global_1_x64: u128::from_le_bytes(data[285..301].try_into().unwrap()),
            protocol_fees_token_0: u64::from_le_bytes(data[301..309].try_into().unwrap()),
            protocol_fees_token_1: u64::from_le_bytes(data[309..317].try_into().unwrap()),
            swap_in_amount_token_0: u128::from_le_bytes(data[317..333].try_into().unwrap()),
            swap_out_amount_token_0: u128::from_le_bytes(data[333..349].try_into().unwrap()),
            swap_in_amount_token_1: u128::from_le_bytes(data[349..365].try_into().unwrap()),
            swap_out_amount_token_1: u128::from_le_bytes(data[365..381].try_into().unwrap()),
            status: data[381],
            padding: [0; 7],
            reward_infos: [RewardInfo::default(); 3],
            tick_array_bitmap: [0; 16],
            total_fees_claimed_token_0: 0,
            total_fees_claimed_token_1: 0,
            fund_fees_token_0: 0,
            fund_fees_token_1: 0,
            open_time: 0,
            recent_epoch: 0,
            padding_1: [0; 24],
            padding_2: [0; 32],
        })
    }
    
    fn discriminator() -> [u8; 8] {
        // Raydium CLMM PoolState discriminator
        [247, 237, 227, 245, 215, 195, 222, 70]
    }
}

impl SammAccountDeserialize for PersonalPositionState {
    fn try_deserialize(data: &[u8]) -> Result<Self> {
        if data.len() < Self::LEN {
            return Err(error!(crate::errors::SovereignError::InvalidAccountData));
        }
        
        let disc = &data[0..8];
        if disc != Self::discriminator() {
            return Err(error!(crate::errors::SovereignError::InvalidAccountData));
        }
        
        let data = &data[8..];
        
        Ok(Self {
            bump: data[0],
            nft_mint: Pubkey::try_from(&data[1..33]).unwrap(),
            pool_id: Pubkey::try_from(&data[33..65]).unwrap(),
            tick_lower_index: i32::from_le_bytes(data[65..69].try_into().unwrap()),
            tick_upper_index: i32::from_le_bytes(data[69..73].try_into().unwrap()),
            liquidity: u128::from_le_bytes(data[73..89].try_into().unwrap()),
            fee_growth_inside_0_last_x64: u128::from_le_bytes(data[89..105].try_into().unwrap()),
            fee_growth_inside_1_last_x64: u128::from_le_bytes(data[105..121].try_into().unwrap()),
            token_fees_owed_0: u64::from_le_bytes(data[121..129].try_into().unwrap()),
            token_fees_owed_1: u64::from_le_bytes(data[129..137].try_into().unwrap()),
            reward_infos: [PositionRewardInfo::default(); 3],
            padding: [0; 8],
        })
    }
    
    fn discriminator() -> [u8; 8] {
        // Raydium CLMM PersonalPositionState discriminator
        [65, 160, 103, 121, 128, 171, 106, 95]
    }
}

// ============================================================
// PDA DERIVATION
// ============================================================

/// PDA seeds for SAMM accounts
pub mod pda_seeds {
    pub const POOL_STATE_SEED: &[u8] = b"pool";
    pub const POSITION_SEED: &[u8] = b"position";
    pub const PROTOCOL_POSITION_SEED: &[u8] = b"protocol_position";
    pub const TICK_ARRAY_SEED: &[u8] = b"tick_array";
    pub const OBSERVATION_SEED: &[u8] = b"observation";
}

/// Derive the PoolState PDA
pub fn derive_pool_state_pda(
    amm_config: &Pubkey,
    token_mint_0: &Pubkey,
    token_mint_1: &Pubkey,
    samm_program: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            pda_seeds::POOL_STATE_SEED,
            amm_config.as_ref(),
            token_mint_0.as_ref(),
            token_mint_1.as_ref(),
        ],
        samm_program,
    )
}

/// Derive the PersonalPositionState PDA
pub fn derive_personal_position_pda(
    nft_mint: &Pubkey,
    samm_program: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            pda_seeds::POSITION_SEED,
            nft_mint.as_ref(),
        ],
        samm_program,
    )
}

/// Derive the TickArray PDA
pub fn derive_tick_array_pda(
    pool_state: &Pubkey,
    start_tick_index: i32,
    samm_program: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            pda_seeds::TICK_ARRAY_SEED,
            pool_state.as_ref(),
            &start_tick_index.to_le_bytes(),
        ],
        samm_program,
    )
}
