use anchor_lang::prelude::*;
use anchor_lang::solana_program::program::invoke_signed;
use anchor_spl::token_2022::{
    Token2022,
    spl_token_2022::{
        self,
        instruction as token_instruction,
        extension::{ExtensionType, transfer_hook, transfer_fee},
    },
};
use anchor_spl::token_interface::TokenAccount as Token2022Account;
use anchor_lang::prelude::InterfaceAccount;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::metadata::{
    create_metadata_accounts_v3,
    CreateMetadataAccountsV3,
    Metadata as MetadataProgram,
    mpl_token_metadata::types::DataV2,
};
use crate::state::*;
use crate::constants::*;
use crate::errors::SovereignError;
use crate::events::TokenCreated;

// Import our own program ID for the transfer hook
use crate::ID as SOVEREIGN_PROGRAM_ID;

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
    
    /// Token vault to hold initial supply (Token-2022 account)
    #[account(
        init,
        payer = creator,
        token::mint = token_mint,
        token::authority = sovereign,
        token::token_program = token_program_2022,
        seeds = [TOKEN_VAULT_SEED, sovereign.key().as_ref()],
        bump
    )]
    pub token_vault: InterfaceAccount<'info, Token2022Account>,
    
    /// Metadata account for the token
    /// CHECK: Created via Metaplex CPI
    #[account(mut)]
    pub metadata_account: UncheckedAccount<'info>,
    
    /// Token-2022 program
    pub token_program_2022: Program<'info, Token2022>,
    
    /// Associated token program
    pub associated_token_program: Program<'info, AssociatedToken>,
    
    /// Metaplex metadata program
    pub metadata_program: Program<'info, MetadataProgram>,
    
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
    // Add TransferFeeConfig (for automatic fee withholding) and TransferHook (for custom logic)
    let extensions = if sovereign.sell_fee_bps > 0 {
        vec![
            ExtensionType::TransferFeeConfig,  // Automatic fee withholding
            ExtensionType::TransferHook,       // Custom sell detection logic
        ]
    } else {
        vec![]
    };
    
    let mint_len = ExtensionType::try_calculate_account_len::<spl_token_2022::state::Mint>(&extensions)?;
    let rent = &ctx.accounts.rent;
    let lamports = rent.minimum_balance(mint_len);
    
    // Create the mint account
    let create_account_ix = anchor_lang::solana_program::system_instruction::create_account(
        &ctx.accounts.creator.key(),
        &token_mint.key(),
        lamports,
        mint_len as u64,
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
        
        // Initialize TransferHook extension for custom sell detection
        let init_hook_ix = transfer_hook::instruction::initialize(
            &spl_token_2022::ID,
            &token_mint.key(),
            Some(sovereign.key()), // Hook authority = sovereign PDA
            Some(SOVEREIGN_PROGRAM_ID), // Hook program = our program
        )?;
        
        invoke_signed(
            &init_hook_ix,
            &[
                token_mint.to_account_info(),
            ],
            mint_signer,
        )?;
    }
    
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
    
    // Create metadata using Metaplex
    create_metadata_accounts_v3(
        CpiContext::new_with_signer(
            ctx.accounts.metadata_program.to_account_info(),
            CreateMetadataAccountsV3 {
                metadata: ctx.accounts.metadata_account.to_account_info(),
                mint: token_mint.to_account_info(),
                mint_authority: sovereign.to_account_info(),
                payer: ctx.accounts.creator.to_account_info(),
                update_authority: sovereign.to_account_info(),
                system_program: ctx.accounts.system_program.to_account_info(),
                rent: ctx.accounts.rent.to_account_info(),
            },
            sovereign_signer,
        ),
        DataV2 {
            name: params.name.clone(),
            symbol: params.symbol.clone(),
            uri: params.uri.clone(),
            seller_fee_basis_points: 0,
            creators: None,
            collection: None,
            uses: None,
        },
        true, // is_mutable
        true, // update_authority_is_signer
        None, // collection_details
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
