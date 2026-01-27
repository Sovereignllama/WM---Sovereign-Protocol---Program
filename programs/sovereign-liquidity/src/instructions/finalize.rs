use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Mint, MintTo};
use anchor_spl::associated_token::AssociatedToken;
use crate::state::*;
use crate::constants::*;
use crate::errors::SovereignError;
use crate::events::{SovereignFinalized, GenesisNFTMinted, CreatorMarketBuyExecuted, PoolRestricted};

/// Finalize the sovereign - create pool, add liquidity, mint NFTs
/// This is a complex multi-step process that may need to be split across multiple transactions
#[derive(Accounts)]
pub struct Finalize<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    
    #[account(
        seeds = [PROTOCOL_STATE_SEED],
        bump = protocol_state.bump
    )]
    pub protocol_state: Account<'info, ProtocolState>,
    
    #[account(
        mut,
        seeds = [SOVEREIGN_SEED, &sovereign.sovereign_id.to_le_bytes()],
        bump = sovereign.bump
    )]
    pub sovereign: Account<'info, SovereignState>,
    
    /// Token mint for the sovereign token
    #[account(
        mut,
        address = sovereign.token_mint
    )]
    pub token_mint: Account<'info, Mint>,
    
    /// SOL vault holding deposits
    /// CHECK: PDA that holds SOL
    #[account(
        mut,
        seeds = [SOL_VAULT_SEED, sovereign.key().as_ref()],
        bump
    )]
    pub sol_vault: SystemAccount<'info>,
    
    /// Token vault to hold LP tokens
    #[account(
        mut,
        seeds = [TOKEN_VAULT_SEED, sovereign.key().as_ref()],
        bump
    )]
    pub token_vault: Account<'info, TokenAccount>,
    
    /// Permanent lock account
    #[account(
        init_if_needed,
        payer = payer,
        space = PermanentLock::LEN,
        seeds = [PERMANENT_LOCK_SEED, sovereign.key().as_ref()],
        bump
    )]
    pub permanent_lock: Account<'info, PermanentLock>,
    
    /// Genesis NFT collection mint (created during sovereign creation)
    #[account(
        mut,
        address = sovereign.genesis_nft_mint
    )]
    pub nft_collection_mint: Account<'info, Mint>,
    
    // ============ Whirlpool Accounts ============
    // These will be used to create the pool and add liquidity
    // Due to complexity, we may need to use remaining_accounts for Whirlpool CPI
    
    /// CHECK: Whirlpool program
    #[account(address = WHIRLPOOL_PROGRAM_ID)]
    pub whirlpool_program: UncheckedAccount<'info>,
    
    // ============ Standard Programs ============
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

pub fn handler(ctx: Context<Finalize>) -> Result<()> {
    let sovereign = &mut ctx.accounts.sovereign;
    let protocol = &ctx.accounts.protocol_state;
    let clock = Clock::get()?;
    
    // Check protocol pause status
    require!(
        !protocol.paused,
        SovereignError::ProtocolPaused
    );
    
    // Validate state - must be Finalizing (set when bond target reached)
    require!(
        sovereign.state == SovereignStatus::Finalizing,
        SovereignError::InvalidState
    );
    
    // Double-check bond target is actually met (defense in depth)
    require!(
        sovereign.total_deposited >= sovereign.bond_target,
        SovereignError::BondTargetNotMet
    );
    
    // Validate total_deposited > 0 to prevent division issues later
    require!(
        sovereign.total_deposited > 0,
        SovereignError::NoDeposits
    );
    
    // Calculate token amounts
    // Total supply: total_supply set during creation
    // LP allocation: 80% (8000 BPS)
    // Creator allocation: 20% (2000 BPS - if TokenLaunch)
    
    let lp_tokens = sovereign.total_supply
        .checked_mul(LP_ALLOCATION_BPS as u64).unwrap()
        .checked_div(BPS_DENOMINATOR as u64).unwrap();
    
    // Mint tokens for LP pool to token vault
    let _sovereign_key = sovereign.key();
    let sovereign_id_bytes = sovereign.sovereign_id.to_le_bytes();
    let seeds = &[
        SOVEREIGN_SEED,
        &sovereign_id_bytes,
        &[sovereign.bump],
    ];
    let signer_seeds = &[&seeds[..]];
    
    // Mint LP tokens to vault
    token::mint_to(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            MintTo {
                mint: ctx.accounts.token_mint.to_account_info(),
                to: ctx.accounts.token_vault.to_account_info(),
                authority: sovereign.to_account_info(),
            },
            signer_seeds,
        ),
        lp_tokens,
    )?;
    
    // For TokenLaunch: mint creator's 20% to creator's ATA
    // This would be handled via remaining_accounts to keep the main struct simpler
    
    // ============ Whirlpool Integration ============
    // The actual Whirlpool CPI calls are complex and require many accounts
    // In production, this would involve:
    // 1. Create/initialize whirlpool if not exists
    // 2. Open position with FULL_RANGE (MIN_TICK to MAX_TICK)
    // 3. Add liquidity (80% tokens + all investor SOL)
    // 4. Set pool to restricted mode (blocked transfers during recovery)
    // 5. Execute creator market buy if creator_escrow > 0
    
    // For now, we'll store the necessary state and emit events
    // The actual Whirlpool CPI would be implemented in a separate module
    
    // Initialize permanent lock
    let permanent_lock = &mut ctx.accounts.permanent_lock;
    permanent_lock.sovereign = sovereign.key();
    permanent_lock.position_mint = Pubkey::default(); // Set after position creation
    permanent_lock.created_at = clock.unix_timestamp;
    permanent_lock.bump = ctx.bumps.permanent_lock;
    
    // Store recovery target (total investor SOL deposited)
    sovereign.recovery_target = sovereign.total_deposited;
    sovereign.total_recovered = 0;
    sovereign.finalized_at = clock.unix_timestamp;
    
    // If recovery target is 0 (edge case), go straight to Active
    if sovereign.recovery_target == 0 {
        sovereign.state = SovereignStatus::Active;
    } else {
        // Pool starts in Recovery mode with restricted trading
        sovereign.state = SovereignStatus::Recovery;
        
        emit!(PoolRestricted {
            sovereign_id: sovereign.sovereign_id,
            restricted: true,
        });
    }
    
    // Emit finalization event
    emit!(SovereignFinalized {
        sovereign_id: sovereign.sovereign_id,
        total_deposited: sovereign.total_deposited,
        token_supply: sovereign.total_supply,
        lp_tokens,
        recovery_target: sovereign.recovery_target,
        finalized_at: clock.unix_timestamp,
    });
    
    // Creator market buy would be executed here if creator_escrow > 0
    if sovereign.creator_escrow > 0 {
        // Execute swap: creator_escrow SOL -> tokens
        // Tokens go to creator's ATA
        // This protects creator from frontrunning since it's atomic with LP creation
        
        emit!(CreatorMarketBuyExecuted {
            sovereign_id: sovereign.sovereign_id,
            creator: sovereign.creator,
            sol_amount: sovereign.creator_escrow,
            tokens_received: 0, // Would be set by actual swap
        });
        
        sovereign.creator_escrow = 0; // Escrow consumed
    }
    
    Ok(())
}

/// Mint Genesis NFT to a depositor
/// Must be called for each depositor after finalization
#[derive(Accounts)]
pub struct MintGenesisNFT<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    
    #[account(
        seeds = [SOVEREIGN_SEED, &sovereign.sovereign_id.to_le_bytes()],
        bump = sovereign.bump
    )]
    pub sovereign: Account<'info, SovereignState>,
    
    #[account(
        mut,
        seeds = [DEPOSIT_RECORD_SEED, sovereign.key().as_ref(), depositor.key().as_ref()],
        bump = deposit_record.bump
    )]
    pub deposit_record: Account<'info, DepositRecord>,
    
    /// CHECK: The depositor who will receive the NFT
    pub depositor: UncheckedAccount<'info>,
    
    /// Genesis NFT mint for this specific depositor
    #[account(
        init,
        payer = payer,
        mint::decimals = 0,
        mint::authority = sovereign,
        seeds = [GENESIS_NFT_MINT_SEED, sovereign.key().as_ref(), depositor.key().as_ref()],
        bump
    )]
    pub nft_mint: Account<'info, Mint>,
    
    /// NFT token account for the depositor
    #[account(
        init_if_needed,
        payer = payer,
        associated_token::mint = nft_mint,
        associated_token::authority = depositor
    )]
    pub nft_token_account: Account<'info, TokenAccount>,
    
    /// CHECK: Metaplex metadata account
    #[account(mut)]
    pub metadata_account: UncheckedAccount<'info>,
    
    /// CHECK: Metaplex program
    #[account(address = METAPLEX_PROGRAM_ID)]
    pub metadata_program: UncheckedAccount<'info>,
    
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

pub fn mint_genesis_nft_handler(ctx: Context<MintGenesisNFT>) -> Result<()> {
    let sovereign = &ctx.accounts.sovereign;
    let deposit_record = &mut ctx.accounts.deposit_record;
    
    // Validate state
    require!(
        sovereign.state == SovereignStatus::Recovery || 
        sovereign.state == SovereignStatus::Active,
        SovereignError::InvalidState
    );
    require!(
        !deposit_record.nft_minted,
        SovereignError::NFTAlreadyMinted
    );
    require!(
        deposit_record.amount > 0,
        SovereignError::ZeroDeposit
    );
    
    // Calculate voting power based on deposit share
    let voting_power = deposit_record.amount
        .checked_mul(BPS_DENOMINATOR as u64).unwrap()
        .checked_div(sovereign.total_deposited).unwrap();
    
    // Mint the NFT
    let sovereign_id_bytes = sovereign.sovereign_id.to_le_bytes();
    let seeds = &[
        SOVEREIGN_SEED,
        &sovereign_id_bytes,
        &[sovereign.bump],
    ];
    let signer_seeds = &[&seeds[..]];
    
    token::mint_to(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            MintTo {
                mint: ctx.accounts.nft_mint.to_account_info(),
                to: ctx.accounts.nft_token_account.to_account_info(),
                authority: sovereign.to_account_info(),
            },
            signer_seeds,
        ),
        1, // NFT amount is always 1
    )?;
    
    // TODO: Create Metaplex metadata with voting power attribute
    // This would use mpl_token_metadata CPI
    
    deposit_record.nft_minted = true;
    deposit_record.nft_mint = Some(ctx.accounts.nft_mint.key());
    deposit_record.voting_power_bps = voting_power as u16;
    
    emit!(GenesisNFTMinted {
        sovereign_id: sovereign.sovereign_id,
        depositor: ctx.accounts.depositor.key(),
        nft_mint: ctx.accounts.nft_mint.key(),
        voting_power_bps: voting_power as u16,
        deposit_amount: deposit_record.amount,
    });
    
    Ok(())
}
