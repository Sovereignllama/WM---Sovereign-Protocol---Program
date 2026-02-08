use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount, Mint, Transfer, transfer};
use crate::state::*;
use crate::constants::*;
use crate::errors::SovereignError;
use crate::events::SovereignCreated;

/// Parameters for creating a new sovereign
#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct CreateSovereignParams {
    /// Type of launch (TokenLaunch or BYOToken)
    pub sovereign_type: SovereignType,
    
    /// SOL to raise (in lamports)
    pub bond_target: u64,
    
    /// Duration in seconds (7-30 days)
    pub bond_duration: i64,
    
    /// Sovereign name (for metadata)
    pub name: String,
    
    // Token Launcher only
    pub token_name: Option<String>,
    pub token_symbol: Option<String>,
    pub token_supply: Option<u64>,
    pub sell_fee_bps: Option<u16>,
    pub fee_mode: Option<FeeMode>,
    pub metadata_uri: Option<String>,
    
    // BYO Token only
    pub deposit_amount: Option<u64>,
    
    // SAMM pool configuration
    /// AMM config account address (determines swap fee tier)
    pub amm_config: Pubkey,
    /// Swap fee in basis points (for display/reference, must match amm_config tier)
    pub swap_fee_bps: u16,
}

#[derive(Accounts)]
#[instruction(params: CreateSovereignParams)]
pub struct CreateSovereign<'info> {
    #[account(mut)]
    pub creator: Signer<'info>,
    
    /// Use Box to reduce stack usage - ProtocolState is a large account
    #[account(
        mut,
        seeds = [PROTOCOL_STATE_SEED],
        bump = protocol_state.bump
    )]
    pub protocol_state: Box<Account<'info, ProtocolState>>,
    
    /// Use Box to reduce stack usage - SovereignState is the largest account
    #[account(
        init,
        payer = creator,
        space = SovereignState::LEN,
        seeds = [SOVEREIGN_SEED, &(protocol_state.sovereign_count + 1).to_le_bytes()],
        bump
    )]
    pub sovereign: Box<Account<'info, SovereignState>>,
    
    /// Use Box to reduce stack usage
    #[account(
        init,
        payer = creator,
        space = CreatorFeeTracker::LEN,
        seeds = [CREATOR_TRACKER_SEED, sovereign.key().as_ref()],
        bump
    )]
    pub creator_tracker: Box<Account<'info, CreatorFeeTracker>>,
    
    /// Use Box to reduce stack usage
    #[account(
        init,
        payer = creator,
        space = CreationFeeEscrow::LEN,
        seeds = [CREATION_FEE_ESCROW_SEED, sovereign.key().as_ref()],
        bump
    )]
    pub creation_fee_escrow: Box<Account<'info, CreationFeeEscrow>>,
    
    /// Token mint - for BYO this is the existing token
    /// For TokenLaunch this will be created in a separate instruction
    pub token_mint: Option<Box<Account<'info, Mint>>>,
    
    /// Creator's token account (for BYO token transfer)
    #[account(mut)]
    pub creator_token_account: Option<Box<Account<'info, TokenAccount>>>,
    
    /// Sovereign's token vault - Use Box to reduce stack usage
    #[account(
        init,
        payer = creator,
        token::mint = token_mint,
        token::authority = sovereign,
        seeds = [TOKEN_VAULT_SEED, sovereign.key().as_ref()],
        bump
    )]
    pub token_vault: Option<Box<Account<'info, TokenAccount>>>,
    
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<CreateSovereign>, params: CreateSovereignParams) -> Result<()> {
    let protocol = &mut ctx.accounts.protocol_state;
    let sovereign = &mut ctx.accounts.sovereign;
    let creator_tracker = &mut ctx.accounts.creator_tracker;
    let creation_fee_escrow = &mut ctx.accounts.creation_fee_escrow;
    let clock = Clock::get()?;
    
    // Check protocol pause status
    require!(
        !protocol.paused,
        SovereignError::ProtocolPaused
    );
    
    // Validate common parameters
    require!(
        params.bond_target >= protocol.min_bond_target,
        SovereignError::InvalidBondTarget
    );
    require!(
        params.bond_duration >= MIN_BOND_DURATION && params.bond_duration <= MAX_BOND_DURATION,
        SovereignError::InvalidBondDuration
    );
    
    // Calculate and escrow creation fee
    let creation_fee = (params.bond_target as u128 * protocol.creation_fee_bps as u128 / BPS_100_PERCENT as u128) as u64;
    
    // Transfer creation fee to escrow
    let escrow_transfer = anchor_lang::system_program::Transfer {
        from: ctx.accounts.creator.to_account_info(),
        to: creation_fee_escrow.to_account_info(),
    };
    anchor_lang::system_program::transfer(
        CpiContext::new(
            ctx.accounts.system_program.to_account_info(),
            escrow_transfer,
        ),
        creation_fee,
    )?;
    
    // Increment sovereign count
    protocol.sovereign_count += 1;
    
    // Initialize sovereign state
    sovereign.sovereign_id = protocol.sovereign_count;
    sovereign.creator = ctx.accounts.creator.key();
    sovereign.sovereign_type = params.sovereign_type;
    sovereign.state = SovereignStatus::Bonding;
    sovereign.name = params.name;
    sovereign.token_name = params.token_name.clone().unwrap_or_default();
    sovereign.token_symbol = params.token_symbol.clone().unwrap_or_default();
    sovereign.metadata_uri = params.metadata_uri.clone().unwrap_or_default();
    sovereign.bond_target = params.bond_target;
    sovereign.bond_duration = params.bond_duration;
    sovereign.bond_deadline = clock.unix_timestamp + params.bond_duration;
    sovereign.creation_fee_escrowed = creation_fee;
    sovereign.amm_config = params.amm_config;
    sovereign.swap_fee_bps = params.swap_fee_bps;
    sovereign.pool_restricted = true;
    sovereign.created_at = clock.unix_timestamp;
    sovereign.bump = ctx.bumps.sovereign;
    
    // Handle type-specific initialization
    match params.sovereign_type {
        SovereignType::TokenLaunch => {
            // Validate Token Launcher params
            require!(params.token_name.is_some(), SovereignError::MissingTokenName);
            require!(params.token_symbol.is_some(), SovereignError::MissingTokenSymbol);
            require!(params.token_supply.is_some(), SovereignError::MissingTokenSupply);
            
            let sell_fee = params.sell_fee_bps.unwrap_or(0);
            require!(sell_fee <= MAX_SELL_FEE_BPS, SovereignError::SellFeeExceedsMax);
            
            sovereign.sell_fee_bps = sell_fee;
            sovereign.fee_mode = params.fee_mode.unwrap_or(FeeMode::CreatorRevenue);
            sovereign.token_supply_deposited = params.token_supply.unwrap();
            sovereign.token_total_supply = params.token_supply.unwrap();
            
            // Token will be created in a separate CPI instruction
            // For now, store the parameters
        }
        
        SovereignType::BYOToken => {
            // Validate BYO Token params
            require!(params.deposit_amount.is_some(), SovereignError::MissingDepositAmount);
            
            let token_mint = ctx.accounts.token_mint.as_ref()
                .ok_or(SovereignError::MissingExistingMint)?;
            let creator_token_account = ctx.accounts.creator_token_account.as_ref()
                .ok_or(SovereignError::MissingExistingMint)?;
            let token_vault = ctx.accounts.token_vault.as_ref()
                .ok_or(SovereignError::MissingExistingMint)?;
            
            let deposit_amount = params.deposit_amount.unwrap();
            let total_supply = token_mint.supply;
            
            // Calculate deposit percentage
            let deposit_bps = (deposit_amount as u128 * BPS_100_PERCENT as u128 / total_supply as u128) as u16;
            
            // Verify minimum supply requirement
            require!(
                deposit_bps >= protocol.byo_min_supply_bps,
                SovereignError::InsufficientTokenDeposit
            );
            
            // Transfer tokens from creator to vault
            let transfer_ctx = CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: creator_token_account.to_account_info(),
                    to: token_vault.to_account_info(),
                    authority: ctx.accounts.creator.to_account_info(),
                },
            );
            transfer(transfer_ctx, deposit_amount)?;
            
            sovereign.token_mint = token_mint.key();
            sovereign.token_supply_deposited = deposit_amount;
            sovereign.token_total_supply = total_supply;
            sovereign.sell_fee_bps = 0; // No sell tax for BYO
        }
    }
    
    // Initialize creator tracker
    creator_tracker.sovereign = sovereign.key();
    creator_tracker.creator = ctx.accounts.creator.key();
    creator_tracker.tokens_locked = false;
    creator_tracker.bump = ctx.bumps.creator_tracker;
    
    // Initialize creation fee escrow
    creation_fee_escrow.sovereign = sovereign.key();
    creation_fee_escrow.amount = creation_fee;
    creation_fee_escrow.released = false;
    creation_fee_escrow.bump = ctx.bumps.creation_fee_escrow;
    
    emit!(SovereignCreated {
        sovereign_id: sovereign.sovereign_id,
        creator: sovereign.creator,
        token_mint: sovereign.token_mint,
        sovereign_type: sovereign.sovereign_type,
        bond_target: sovereign.bond_target,
        bond_deadline: sovereign.bond_deadline,
        token_supply_deposited: sovereign.token_supply_deposited,
        creation_fee_escrowed: creation_fee,
        sell_fee_bps: sovereign.sell_fee_bps,
        fee_mode: sovereign.fee_mode,
        amm_config: sovereign.amm_config,
        swap_fee_bps: sovereign.swap_fee_bps,
    });
    
    Ok(())
}
