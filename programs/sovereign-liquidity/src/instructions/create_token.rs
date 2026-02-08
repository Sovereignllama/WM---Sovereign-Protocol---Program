use anchor_lang::prelude::*;
use anchor_lang::solana_program::program::invoke_signed;
use anchor_spl::token_2022::{
    Token2022,
    spl_token_2022::{
        self,
        instruction as token_instruction,
        extension::{
            ExtensionType, transfer_fee,
            metadata_pointer,
        },
    },
};
use spl_token_metadata_interface::instruction as token_metadata_instruction;
use crate::state::*;
use crate::constants::*;
use crate::errors::SovereignError;
use crate::events::TokenCreated;

/// Parameters for creating a new token for a TokenLaunch sovereign
#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct CreateTokenParams {
    /// Token name for metadata
    pub name: String,
    /// Token symbol for metadata
    pub symbol: String,
    /// Metadata URI (IPFS/Arweave)
    pub uri: String,
}

/// Create a Token-2022 mint for a TokenLaunch sovereign
/// This should be called after create_sovereign for TokenLaunch types
#[derive(Accounts)]
pub struct CreateToken<'info> {
    #[account(mut)]
    pub creator: Signer<'info>,
    
    #[account(
        seeds = [PROTOCOL_STATE_SEED],
        bump = protocol_state.bump
    )]
    pub protocol_state: Box<Account<'info, ProtocolState>>,
    
    #[account(
        mut,
        seeds = [SOVEREIGN_SEED, &sovereign.sovereign_id.to_le_bytes()],
        bump = sovereign.bump,
        constraint = sovereign.creator == creator.key() @ SovereignError::NotCreator,
        constraint = sovereign.sovereign_type == SovereignType::TokenLaunch @ SovereignError::InvalidSovereignType,
        constraint = sovereign.token_mint == Pubkey::default() @ SovereignError::TokenAlreadyCreated,
        constraint = sovereign.state == SovereignStatus::Bonding @ SovereignError::InvalidState
    )]
    pub sovereign: Box<Account<'info, SovereignState>>,
    
    /// Token mint PDA - derived from sovereign
    /// CHECK: Will be initialized as Token-2022 mint via CPI
    #[account(
        mut,
        seeds = [TOKEN_MINT_SEED, sovereign.key().as_ref()],
        bump
    )]
    pub token_mint: UncheckedAccount<'info>,
    
    /// Token vault to hold initial supply - created manually after mint init
    /// CHECK: Will be initialized as Token-2022 token account via CPI after mint is ready
    #[account(
        mut,
        seeds = [TOKEN_VAULT_SEED, sovereign.key().as_ref()],
        bump
    )]
    pub token_vault: UncheckedAccount<'info>,
    
    /// Token-2022 program
    pub token_program_2022: Program<'info, Token2022>,
    
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

pub fn handler(ctx: Context<CreateToken>, params: CreateTokenParams) -> Result<()> {
    let sovereign = &mut ctx.accounts.sovereign;
    let token_mint = &ctx.accounts.token_mint;
    
    // Validate params
    require!(
        !params.name.is_empty() && params.name.len() <= 32,
        SovereignError::InvalidTokenName
    );
    require!(
        !params.symbol.is_empty() && params.symbol.len() <= 10,
        SovereignError::InvalidTokenSymbol
    );
    require!(
        !params.uri.is_empty() && params.uri.len() <= 200,
        SovereignError::InvalidMetadataUri
    );
    
    // Get total supply from sovereign state (set during create_sovereign)
    let total_supply = sovereign.token_total_supply;
    require!(total_supply > 0, SovereignError::InvalidTokenSupply);
    
    // Derive sovereign PDA seeds for signing
    let sovereign_id_bytes = sovereign.sovereign_id.to_le_bytes();
    let sovereign_seeds = &[
        SOVEREIGN_SEED,
        &sovereign_id_bytes,
        &[sovereign.bump],
    ];
    let sovereign_signer = &[&sovereign_seeds[..]];
    
    // Derive token mint PDA seeds
    let sovereign_key = sovereign.key();
    let mint_bump = ctx.bumps.token_mint;
    let mint_seeds = &[
        TOKEN_MINT_SEED,
        sovereign_key.as_ref(),
        &[mint_bump],
    ];
    let mint_signer = &[&mint_seeds[..]];
    
    // Calculate space needed for Token-2022 mint with extensions
    // MetadataPointer is always added (points metadata to the mint itself)
    // TransferFeeConfig and TransferHook are added when sell fee > 0
    let extensions = if sovereign.sell_fee_bps > 0 {
        vec![
            ExtensionType::TransferFeeConfig,  // Automatic fee withholding
            ExtensionType::MetadataPointer,    // Points to self for token metadata
        ]
    } else {
        vec![
            ExtensionType::MetadataPointer,    // Points to self for token metadata
        ]
    };
    
    let mint_len = ExtensionType::try_calculate_account_len::<spl_token_2022::state::Mint>(&extensions)?;
    
    // Estimate total size including Token-2022 native metadata (for lamport calculation)
    // Token-2022 will realloc the account during metadata init, but needs enough lamports
    // TLV header (12) + update_authority (32) + mint (32) + strings (4+len each) + empty vec (4)
    let metadata_space = 12 + 32 + 32
        + 4 + params.name.len()
        + 4 + params.symbol.len()
        + 4 + params.uri.len()
        + 4;
    let total_mint_len = mint_len + metadata_space;
    
    let rent = &ctx.accounts.rent;
    // Fund with enough lamports for the final size (after metadata realloc),
    // but create with mint_len so InitializeMint2 sees the correct extension layout
    let lamports = rent.minimum_balance(total_mint_len);
    
    // Create the mint account with exact extension size
    let create_account_ix = anchor_lang::solana_program::system_instruction::create_account(
        &ctx.accounts.creator.key(),
        &token_mint.key(),
        lamports,
        mint_len as u64,  // exact extension size (NOT total_mint_len)
        &spl_token_2022::ID,
    );
    
    invoke_signed(
        &create_account_ix,
        &[
            ctx.accounts.creator.to_account_info(),
            token_mint.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
        ],
        mint_signer,
    )?;
    
    // Initialize extensions if sell fee is configured
    if sovereign.sell_fee_bps > 0 {
        // Initialize TransferFeeConfig extension
        // Fee authority = sovereign PDA (can update fees)
        // Withdraw authority = sovereign PDA (can withdraw collected fees)
        let init_fee_ix = transfer_fee::instruction::initialize_transfer_fee_config(
            &spl_token_2022::ID,
            &token_mint.key(),
            Some(&sovereign.key()), // Transfer fee config authority
            Some(&sovereign.key()), // Withdraw withheld authority
            sovereign.sell_fee_bps,  // Fee in basis points
            u64::MAX,                // Maximum fee (no cap)
        )?;
        
        invoke_signed(
            &init_fee_ix,
            &[
                token_mint.to_account_info(),
            ],
            mint_signer,
        )?;
    }
    
    // Initialize MetadataPointer extension - points metadata to the mint itself
    let init_metadata_pointer_ix = metadata_pointer::instruction::initialize(
        &spl_token_2022::ID,
        &token_mint.key(),
        None, // No separate authority
        Some(token_mint.key()), // Metadata address = the mint itself
    )?;
    
    invoke_signed(
        &init_metadata_pointer_ix,
        &[
            token_mint.to_account_info(),
        ],
        mint_signer,
    )?;
    
    // Initialize the mint
    let init_mint_ix = token_instruction::initialize_mint2(
        &spl_token_2022::ID,
        &token_mint.key(),
        &sovereign.key(), // Mint authority = sovereign PDA
        Some(&sovereign.key()), // Freeze authority = sovereign PDA (can be removed later)
        TOKEN_DECIMALS,
    )?;
    
    invoke_signed(
        &init_mint_ix,
        &[
            token_mint.to_account_info(),
        ],
        mint_signer,
    )?;
    
    // Now create and initialize the token vault as a Token-2022 token account
    // This must happen AFTER the mint is initialized
    let vault_bump = ctx.bumps.token_vault;
    let vault_seeds = &[
        TOKEN_VAULT_SEED,
        sovereign_key.as_ref(),
        &[vault_bump],
    ];
    let vault_signer = &[&vault_seeds[..]];
    
    // Calculate space for Token-2022 token account
    // Must include extensions matching the mint: TransferFeeAmount (for TransferFeeConfig)
    // and TransferHookAccount (for TransferHook)
    let vault_extensions: Vec<ExtensionType> = if sovereign.sell_fee_bps > 0 {
        vec![
            ExtensionType::TransferFeeAmount,
        ]
    } else {
        vec![]
    };
    let vault_len = ExtensionType::try_calculate_account_len::<spl_token_2022::state::Account>(&vault_extensions)?;
    let vault_lamports = rent.minimum_balance(vault_len);
    
    // Create the vault account
    let create_vault_ix = anchor_lang::solana_program::system_instruction::create_account(
        &ctx.accounts.creator.key(),
        &ctx.accounts.token_vault.key(),
        vault_lamports,
        vault_len as u64,
        &spl_token_2022::ID,
    );
    
    invoke_signed(
        &create_vault_ix,
        &[
            ctx.accounts.creator.to_account_info(),
            ctx.accounts.token_vault.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
        ],
        vault_signer,
    )?;
    
    // Initialize the vault as a token account owned by the sovereign PDA
    let init_vault_ix = token_instruction::initialize_account3(
        &spl_token_2022::ID,
        &ctx.accounts.token_vault.key(),
        &token_mint.key(),
        &sovereign.key(),
    )?;
    
    invoke_signed(
        &init_vault_ix,
        &[
            ctx.accounts.token_vault.to_account_info(),
            token_mint.to_account_info(),
        ],
        vault_signer,
    )?;
    
    // Mint total supply to token vault
    let mint_to_ix = token_instruction::mint_to(
        &spl_token_2022::ID,
        &token_mint.key(),
        &ctx.accounts.token_vault.key(),
        &sovereign.key(),
        &[],
        total_supply,
    )?;
    
    invoke_signed(
        &mint_to_ix,
        &[
            token_mint.to_account_info(),
            ctx.accounts.token_vault.to_account_info(),
            sovereign.to_account_info(),
        ],
        sovereign_signer,
    )?;
    
    // Create token metadata using Token-2022's native TokenMetadata extension
    // This stores metadata directly on the mint account (no separate Metaplex account needed)
    let init_token_metadata_ix = token_metadata_instruction::initialize(
        &spl_token_2022::ID,
        &token_mint.key(),
        &sovereign.key(),  // Update authority
        &token_mint.key(),  // Mint
        &sovereign.key(),  // Mint authority
        params.name.clone(),
        params.symbol.clone(),
        params.uri.clone(),
    );
    
    invoke_signed(
        &init_token_metadata_ix,
        &[
            token_mint.to_account_info(),
            sovereign.to_account_info(),
        ],
        sovereign_signer,
    )?;
    
    // Update sovereign state with token mint
    sovereign.token_mint = token_mint.key();
    
    emit!(TokenCreated {
        sovereign_id: sovereign.sovereign_id,
        token_mint: token_mint.key(),
        total_supply,
        decimals: TOKEN_DECIMALS,
        name: params.name,
        symbol: params.symbol,
        uri: params.uri,
    });
    
    Ok(())
}
