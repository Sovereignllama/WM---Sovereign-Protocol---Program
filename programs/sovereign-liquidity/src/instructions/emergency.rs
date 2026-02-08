use anchor_lang::prelude::*;
use anchor_lang::solana_program::program::invoke_signed;
use anchor_spl::token_2022::spl_token_2022;
use anchor_spl::token_interface::{
    Mint as MintInterface,
    TokenAccount as TokenAccountInterface,
    TokenInterface,
};
use spl_token_2022::extension::transfer_fee;
use crate::state::*;
use crate::constants::*;
use crate::errors::SovereignError;
use crate::events::{EmergencyUnlocked, EmergencyWithdrawal, EmergencyCreatorWithdrawal, SovereignRetired};
use crate::samm::{self, instructions as samm_ix, cpi as samm_cpi, SammAccountDeserialize};

// ============================================================
// EMERGENCY UNLOCK
// ============================================================

/// Emergency unlock - callable by protocol authority OR creator
/// Transitions sovereign to EmergencyUnlocked state from ANY phase
/// This allows all participants to reclaim their funds
#[derive(Accounts)]
pub struct EmergencyUnlock<'info> {
    #[account(mut)]
    pub caller: Signer<'info>,
    
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
}

pub fn emergency_unlock_handler(ctx: Context<EmergencyUnlock>) -> Result<()> {
    let sovereign = &mut ctx.accounts.sovereign;
    let protocol = &ctx.accounts.protocol_state;
    let caller = ctx.accounts.caller.key();
    let clock = Clock::get()?;
    
    // Only protocol authority can call emergency unlock
    let is_authority = caller == protocol.authority;
    
    require!(
        is_authority,
        SovereignError::Unauthorized
    );
    
    // Cannot emergency unlock if already in EmergencyUnlocked or Unwound state
    require!(
        sovereign.state != SovereignStatus::EmergencyUnlocked,
        SovereignError::AlreadyEmergencyUnlocked
    );
    require!(
        sovereign.state != SovereignStatus::Unwound,
        SovereignError::InvalidState
    );
    
    let previous_state = sovereign.state;
    
    // Transition to EmergencyUnlocked
    sovereign.state = SovereignStatus::EmergencyUnlocked;
    
    emit!(EmergencyUnlocked {
        sovereign_id: sovereign.sovereign_id,
        caller,
        previous_state: previous_state as u8,
        unlocked_at: clock.unix_timestamp,
    });
    
    Ok(())
}

// ============================================================
// EMERGENCY WITHDRAW (Investors)
// ============================================================

/// Emergency withdraw - reclaim GOR from sol_vault.
/// Works only when sovereign is in EmergencyUnlocked state.
/// Closes the deposit record and returns GOR.
/// Sovereign tokens remain in token_vault for the creator to reclaim.
///
/// Authorization model (bearer NFT):
/// - Post-finalization (NFT minted): The NFT holder calls this.
///   `original_depositor` is used only for PDA derivation.
///   Returns PROPORTIONAL share of GOR from sol_vault only.
///   remaining_accounts[0] = nft_mint (writable)
///   remaining_accounts[1] = nft_token_account (writable, amount==1)
///   The NFT will be burned before distributing.
/// - Pre-finalization (no NFT): `holder` must be the original depositor.
///   Returns deposit_record.amount from sol_vault.
///   No remaining_accounts needed.
#[derive(Accounts)]
pub struct EmergencyWithdraw<'info> {
    /// Current NFT holder (or original depositor if pre-finalization)
    #[account(mut)]
    pub holder: Signer<'info>,
    
    /// CHECK: Original depositor wallet — used only for deposit_record PDA derivation.
    /// For pre-finalization (no NFT), this must equal `holder`.
    pub original_depositor: UncheckedAccount<'info>,
    
    #[account(
        mut,
        seeds = [SOVEREIGN_SEED, &sovereign.sovereign_id.to_le_bytes()],
        bump = sovereign.bump,
        constraint = sovereign.state == SovereignStatus::EmergencyUnlocked || sovereign.state == SovereignStatus::Retired @ SovereignError::InvalidState
    )]
    pub sovereign: Account<'info, SovereignState>,
    
    #[account(
        mut,
        close = holder,
        seeds = [DEPOSIT_RECORD_SEED, sovereign.key().as_ref(), original_depositor.key().as_ref()],
        bump = deposit_record.bump,
    )]
    pub deposit_record: Account<'info, DepositRecord>,
    
    /// CHECK: SOL vault PDA
    #[account(
        mut,
        seeds = [SOL_VAULT_SEED, sovereign.key().as_ref()],
        bump
    )]
    pub sol_vault: SystemAccount<'info>,
    
    /// SPL Token program — needed if burning Genesis NFT
    pub token_program: Program<'info, anchor_spl::token::Token>,
    pub system_program: Program<'info, System>,
}

pub fn emergency_withdraw_handler<'info>(
    ctx: Context<'_, '_, 'info, 'info, EmergencyWithdraw<'info>>,
) -> Result<()> {
    let sovereign = &mut ctx.accounts.sovereign;
    let deposit_record = &ctx.accounts.deposit_record;
    
    // Cannot be creator (they use emergency_withdraw_creator)
    require!(
        ctx.accounts.original_depositor.key() != sovereign.creator,
        SovereignError::CreatorMustUseCreatorWithdraw
    );
    
    let amount = deposit_record.amount;
    require!(amount > 0, SovereignError::NothingToWithdraw);
    
    // CRITICAL: Prevent division by zero
    require!(
        sovereign.total_deposited > 0,
        SovereignError::NoDeposits
    );
    
    // ---- Authorization & distribution depends on pre/post finalization ----
    if deposit_record.nft_minted {
        // ============================================================
        // POST-FINALIZATION: Bearer NFT mode
        // Proportional share of GOR only — tokens stay for creator
        // ============================================================
        let remaining = ctx.remaining_accounts;
        require!(remaining.len() >= 2, SovereignError::NoGenesisNFT);
        
        let nft_mint_info = &remaining[0];
        let nft_token_info = &remaining[1];
        
        // Verify NFT mint matches deposit record
        let expected_mint = deposit_record.nft_mint.ok_or(SovereignError::NFTNotMinted)?;
        require!(
            nft_mint_info.key() == expected_mint,
            SovereignError::WrongNFT
        );
        
        // Burn the Genesis NFT (authority = holder, the current NFT possessor)
        let burn_ix = anchor_lang::solana_program::instruction::Instruction {
            program_id: ctx.accounts.token_program.key(),
            accounts: vec![
                anchor_lang::solana_program::instruction::AccountMeta::new(
                    nft_token_info.key(), false,
                ),
                anchor_lang::solana_program::instruction::AccountMeta::new(
                    nft_mint_info.key(), false,
                ),
                anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                    ctx.accounts.holder.key(), true,
                ),
            ],
            data: {
                let mut buf = Vec::with_capacity(9);
                buf.push(8u8); // SPL Token Burn instruction index
                buf.extend_from_slice(&1u64.to_le_bytes());
                buf
            },
        };
        anchor_lang::solana_program::program::invoke(
            &burn_ix,
            &[
                nft_token_info.clone(),
                nft_mint_info.clone(),
                ctx.accounts.holder.to_account_info(),
                ctx.accounts.token_program.to_account_info(),
            ],
        )?;
        msg!("Genesis NFT burned for emergency withdrawal");
        
        // ---- Calculate proportional GOR share (capped at original deposit) ----
        let share_bps = deposit_record.amount
            .checked_mul(BPS_DENOMINATOR as u64)
            .ok_or(SovereignError::Overflow)?
            .checked_div(sovereign.total_deposited)
            .ok_or(SovereignError::DivisionByZero)?;
        
        // GOR share from unwind SOL balance — capped at original deposit amount
        // Any surplus beyond total_deposited goes to the token redemption pool
        let proportional_share = sovereign.unwind_sol_balance
            .checked_mul(share_bps)
            .ok_or(SovereignError::Overflow)?
            .checked_div(BPS_DENOMINATOR as u64)
            .ok_or(SovereignError::DivisionByZero)?;
        let sol_share = proportional_share.min(deposit_record.amount);
        
        // Transfer GOR from sol_vault to holder
        if sol_share > 0 {
            let vault_balance = ctx.accounts.sol_vault.lamports();
            require!(
                vault_balance >= sol_share,
                SovereignError::InsufficientVaultBalance
            );
            
            let sovereign_key = sovereign.key();
            let vault_seeds: &[&[u8]] = &[
                SOL_VAULT_SEED,
                sovereign_key.as_ref(),
                &[ctx.bumps.sol_vault],
            ];
            
            anchor_lang::system_program::transfer(
                CpiContext::new_with_signer(
                    ctx.accounts.system_program.to_account_info(),
                    anchor_lang::system_program::Transfer {
                        from: ctx.accounts.sol_vault.to_account_info(),
                        to: ctx.accounts.holder.to_account_info(),
                    },
                    &[vault_seeds],
                ),
                sol_share,
            )?;
        }
        
        emit!(EmergencyWithdrawal {
            sovereign_id: sovereign.sovereign_id,
            depositor: ctx.accounts.holder.key(),
            amount: sol_share,
        });
    } else {
        // ============================================================
        // PRE-FINALIZATION: Wallet ownership check
        // Returns exact deposit amount from sol_vault
        // ============================================================
        require!(
            deposit_record.depositor == ctx.accounts.holder.key(),
            SovereignError::Unauthorized
        );
        
        let vault_balance = ctx.accounts.sol_vault.lamports();
        require!(
            vault_balance >= amount,
            SovereignError::InsufficientVaultBalance
        );
        
        let sovereign_key = sovereign.key();
        let vault_seeds: &[&[u8]] = &[
            SOL_VAULT_SEED,
            sovereign_key.as_ref(),
            &[ctx.bumps.sol_vault],
        ];
        
        anchor_lang::system_program::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.system_program.to_account_info(),
                anchor_lang::system_program::Transfer {
                    from: ctx.accounts.sol_vault.to_account_info(),
                    to: ctx.accounts.holder.to_account_info(),
                },
                &[vault_seeds],
            ),
            amount,
        )?;
        
        emit!(EmergencyWithdrawal {
            sovereign_id: sovereign.sovereign_id,
            depositor: ctx.accounts.holder.key(),
            amount,
        });
    }
    
    // Note: deposit_record is closed via `close = holder` and rent returned
    
    // Retire only when vault is empty AND creator has reclaimed everything (GOR + tokens)
    let remaining = ctx.accounts.sol_vault.lamports();
    if remaining == 0 && sovereign.creator_escrow == 0 && sovereign.creation_fee_escrowed == 0 && sovereign.token_supply_deposited == 0 {
        sovereign.state = SovereignStatus::Retired;
        emit!(SovereignRetired {
            sovereign_id: sovereign.sovereign_id,
            retired_at: Clock::get()?.unix_timestamp,
        });
    }
    
    Ok(())
}

// ============================================================
// EMERGENCY WITHDRAW CREATOR
// ============================================================

/// Emergency withdraw for creator - reclaim escrow GOR, creation fee, AND tokens from vault
/// Works only when sovereign is in EmergencyUnlocked state
#[derive(Accounts)]
pub struct EmergencyWithdrawCreator<'info> {
    #[account(
        mut,
        address = sovereign.creator @ SovereignError::Unauthorized
    )]
    pub creator: Signer<'info>,
    
    #[account(
        mut,
        seeds = [SOVEREIGN_SEED, &sovereign.sovereign_id.to_le_bytes()],
        bump = sovereign.bump,
        constraint = sovereign.state == SovereignStatus::EmergencyUnlocked || sovereign.state == SovereignStatus::Retired @ SovereignError::InvalidState
    )]
    pub sovereign: Account<'info, SovereignState>,
    
    /// CHECK: SOL vault holding escrow
    #[account(
        mut,
        seeds = [SOL_VAULT_SEED, sovereign.key().as_ref()],
        bump
    )]
    pub sol_vault: SystemAccount<'info>,
    
    /// Creation fee escrow - returned to creator on emergency
    #[account(
        mut,
        close = creator,
        seeds = [CREATION_FEE_ESCROW_SEED, sovereign.key().as_ref()],
        bump
    )]
    pub creation_fee_escrow: Account<'info, CreationFeeEscrow>,
    
    /// CHECK: Token vault PDA holding creator's tokens (Token or Token-2022)
    #[account(
        mut,
        seeds = [TOKEN_VAULT_SEED, sovereign.key().as_ref()],
        bump
    )]
    pub token_vault: UncheckedAccount<'info>,
    
    /// CHECK: Creator's token account to receive tokens back
    #[account(mut)]
    pub creator_token_account: UncheckedAccount<'info>,
    
    /// CHECK: Token mint (sovereign.token_mint) - mut because we disable/re-enable transfer hook
    #[account(
        mut,
        address = sovereign.token_mint
    )]
    pub token_mint: UncheckedAccount<'info>,
    
    /// CHECK: Token program (Token or Token-2022, validated by invoke)
    pub token_program: UncheckedAccount<'info>,
    
    pub system_program: Program<'info, System>,
}

pub fn emergency_withdraw_creator_handler(ctx: Context<EmergencyWithdrawCreator>, burn_tokens: bool) -> Result<()> {
    let sovereign = &mut ctx.accounts.sovereign;
    
    let escrow_amount = sovereign.creator_escrow;
    let creation_fee = ctx.accounts.creation_fee_escrow.amount;
    
    // Transfer creator escrow from vault if any
    if escrow_amount > 0 {
        let vault_balance = ctx.accounts.sol_vault.lamports();
        
        require!(
            vault_balance >= escrow_amount,
            SovereignError::InsufficientVaultBalance
        );
        
        // Transfer SOL from vault to creator using System Program CPI with PDA signer
        let sovereign_key = sovereign.key();
        let vault_seeds: &[&[u8]] = &[
            SOL_VAULT_SEED,
            sovereign_key.as_ref(),
            &[ctx.bumps.sol_vault],
        ];
        
        anchor_lang::system_program::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.system_program.to_account_info(),
                anchor_lang::system_program::Transfer {
                    from: ctx.accounts.sol_vault.to_account_info(),
                    to: ctx.accounts.creator.to_account_info(),
                },
                &[vault_seeds],
            ),
            escrow_amount,
        )?;
        
        sovereign.creator_escrow = 0;
    }
    
    // Track token amount for the event
    let mut token_amount_handled: u64 = 0;
    
    // Transfer all tokens from token_vault back to creator
    // Only if token_mint is set (non-zero) and vault has tokens
    if sovereign.token_mint != Pubkey::default() {
        // Read the vault balance by deserializing the SPL token account data
        let vault_data = ctx.accounts.token_vault.try_borrow_data()?;
        // Token account amount is at bytes 64..72 (both Token and Token-2022)
        if vault_data.len() >= 72 {
            let token_amount = u64::from_le_bytes(vault_data[64..72].try_into().unwrap());
            drop(vault_data);
            
            if token_amount > 0 {
                let sovereign_id_bytes = sovereign.sovereign_id.to_le_bytes();
                let signer_seeds: &[&[u8]] = &[
                    SOVEREIGN_SEED,
                    &sovereign_id_bytes,
                    &[sovereign.bump],
                ];

                if burn_tokens && sovereign.sovereign_type == SovereignType::TokenLaunch {
                    // BURN: Only allowed for TokenLaunch tokens (created by our program)
                    // Never burn BYO tokens — those have external value
                    let burn_ix = spl_token_2022::instruction::burn_checked(
                        &ctx.accounts.token_program.key(),
                        &ctx.accounts.token_vault.key(),
                        &sovereign.token_mint,
                        &sovereign.key(), // vault authority = sovereign PDA
                        &[],
                        token_amount,
                        9, // decimals
                    )?;
                    invoke_signed(
                        &burn_ix,
                        &[
                            ctx.accounts.token_vault.to_account_info(),
                            ctx.accounts.token_mint.to_account_info(),
                            sovereign.to_account_info(),
                        ],
                        &[signer_seeds],
                    )?;
                } else if sovereign.sovereign_type == SovereignType::TokenLaunch {
                    // RECOVER TokenLaunch tokens: must disable hook + fee to avoid reentrancy and fee deduction
                    let original_fee_bps = sovereign.sell_fee_bps;

                    // Step 1: Set transfer fee to 0 (so creator gets 100% of tokens)
                    if original_fee_bps > 0 {
                        let disable_fee_ix = transfer_fee::instruction::set_transfer_fee(
                            &spl_token_2022::ID,
                            &sovereign.token_mint,
                            &sovereign.key(), // fee config authority
                            &[],
                            0,    // 0 bps
                            0,    // 0 max fee
                        )?;
                        invoke_signed(
                            &disable_fee_ix,
                            &[
                                ctx.accounts.token_mint.to_account_info(),
                                sovereign.to_account_info(),
                            ],
                            &[signer_seeds],
                        )?;
                    }

                    // Step 2: Transfer tokens (fee may still apply if epoch-based)
                    let transfer_ix = spl_token_2022::instruction::transfer_checked(
                        &ctx.accounts.token_program.key(),
                        &ctx.accounts.token_vault.key(),
                        &sovereign.token_mint,
                        &ctx.accounts.creator_token_account.key(),
                        &sovereign.key(),
                        &[],
                        token_amount,
                        9,
                    )?;
                    invoke_signed(
                        &transfer_ix,
                        &[
                            ctx.accounts.token_vault.to_account_info(),
                            ctx.accounts.token_mint.to_account_info(),
                            ctx.accounts.creator_token_account.to_account_info(),
                            sovereign.to_account_info(),
                        ],
                        &[signer_seeds],
                    )?;

                    // Step 2.5: Harvest any withheld transfer fees from the destination account
                    // back into the mint, then withdraw from mint to creator
                    // This ensures creator gets 100% even if epoch-based fee was still active
                    if original_fee_bps > 0 {
                        // Harvest withheld fees from creator's token account into the mint
                        let harvest_ix = transfer_fee::instruction::harvest_withheld_tokens_to_mint(
                            &spl_token_2022::ID,
                            &sovereign.token_mint,
                            &[&ctx.accounts.creator_token_account.key()],
                        )?;
                        invoke_signed(
                            &harvest_ix,
                            &[
                                ctx.accounts.token_mint.to_account_info(),
                                ctx.accounts.creator_token_account.to_account_info(),
                            ],
                            &[signer_seeds],
                        )?;

                        // Withdraw withheld fees from mint back to creator's token account
                        let withdraw_ix = transfer_fee::instruction::withdraw_withheld_tokens_from_mint(
                            &spl_token_2022::ID,
                            &sovereign.token_mint,
                            &ctx.accounts.creator_token_account.key(),
                            &sovereign.key(), // withdraw authority = sovereign PDA
                            &[],
                        )?;
                        invoke_signed(
                            &withdraw_ix,
                            &[
                                ctx.accounts.token_mint.to_account_info(),
                                ctx.accounts.creator_token_account.to_account_info(),
                                sovereign.to_account_info(),
                            ],
                            &[signer_seeds],
                        )?;
                    }

                    // Step 3: Restore transfer fee
                    if original_fee_bps > 0 {
                        let restore_fee_ix = transfer_fee::instruction::set_transfer_fee(
                            &spl_token_2022::ID,
                            &sovereign.token_mint,
                            &sovereign.key(),
                            &[],
                            original_fee_bps,
                            u64::MAX,
                        )?;
                        invoke_signed(
                            &restore_fee_ix,
                            &[
                                ctx.accounts.token_mint.to_account_info(),
                                sovereign.to_account_info(),
                            ],
                            &[signer_seeds],
                        )?;
                    }
                } else {
                    // BYO tokens: simple transfer — our program is NOT the hook, no reentrancy risk
                    // Works for both Token (SPL) and Token-2022 since we use spl_token_2022 instruction
                    // which is compatible with both programs
                    let transfer_ix = spl_token_2022::instruction::transfer_checked(
                        &ctx.accounts.token_program.key(),
                        &ctx.accounts.token_vault.key(),
                        &sovereign.token_mint,
                        &ctx.accounts.creator_token_account.key(),
                        &sovereign.key(),
                        &[],
                        token_amount,
                        9,
                    )?;
                    invoke_signed(
                        &transfer_ix,
                        &[
                            ctx.accounts.token_vault.to_account_info(),
                            ctx.accounts.token_mint.to_account_info(),
                            ctx.accounts.creator_token_account.to_account_info(),
                            sovereign.to_account_info(),
                        ],
                        &[signer_seeds],
                    )?;
                }
                
                sovereign.token_supply_deposited = 0;
                token_amount_handled = token_amount;
            }
        }
    }
    
    // Mark creation fee as reclaimed
    sovereign.creation_fee_escrowed = 0;
    
    emit!(EmergencyCreatorWithdrawal {
        sovereign_id: sovereign.sovereign_id,
        creator: ctx.accounts.creator.key(),
        escrow_returned: escrow_amount,
        creation_fee_returned: creation_fee,
        tokens_burned: burn_tokens,
        token_amount: token_amount_handled,
    });
    
    // Note: creation_fee_escrow is closed via `close = creator` and rent + fee returned
    
    // Retire when vault is empty AND creator has reclaimed everything (GOR + tokens)
    let remaining = ctx.accounts.sol_vault.lamports();
    if remaining == 0 && sovereign.creator_escrow == 0 && sovereign.creation_fee_escrowed == 0 && sovereign.token_supply_deposited == 0 {
        sovereign.state = SovereignStatus::Retired;
        emit!(SovereignRetired {
            sovereign_id: sovereign.sovereign_id,
            retired_at: Clock::get()?.unix_timestamp,
        });
    }
    
    Ok(())
}

// ============================================================
// EMERGENCY REMOVE LIQUIDITY
// ============================================================

/// Emergency remove liquidity from SAMM pool.
/// Only callable when sovereign is in EmergencyUnlocked state.
/// This extracts all LP back into sol_vault and token_vault so that
/// emergency_withdraw and emergency_withdraw_creator can distribute funds.
/// Only protocol authority can call this.
#[derive(Accounts)]
pub struct EmergencyRemoveLiquidity<'info> {
    #[account(mut)]
    pub caller: Signer<'info>,
    
    #[account(
        seeds = [PROTOCOL_STATE_SEED],
        bump = protocol_state.bump,
        constraint = protocol_state.authority == caller.key() @ SovereignError::Unauthorized
    )]
    pub protocol_state: Account<'info, ProtocolState>,
    
    #[account(
        mut,
        seeds = [SOVEREIGN_SEED, &sovereign.sovereign_id.to_le_bytes()],
        bump = sovereign.bump,
        constraint = sovereign.state == SovereignStatus::EmergencyUnlocked @ SovereignError::InvalidState
    )]
    pub sovereign: Account<'info, SovereignState>,
    
    #[account(
        mut,
        seeds = [PERMANENT_LOCK_SEED, sovereign.key().as_ref()],
        bump = permanent_lock.bump
    )]
    pub permanent_lock: Account<'info, PermanentLock>,
    
    /// Token mint (supports Token-2022)
    #[account(
        mut,
        address = sovereign.token_mint
    )]
    pub token_mint: InterfaceAccount<'info, MintInterface>,
    
    /// CHECK: SAMM position - MUST match permanent_lock.position_mint
    #[account(
        mut,
        constraint = position.key() == permanent_lock.position_mint @ SovereignError::InvalidPosition
    )]
    pub position: UncheckedAccount<'info>,
    
    /// CHECK: Trashbin SAMM program
    #[account(address = SAMM_PROGRAM_ID)]
    pub samm_program: UncheckedAccount<'info>,
    
    /// CHECK: SOL vault PDA
    #[account(
        mut,
        seeds = [SOL_VAULT_SEED, sovereign.key().as_ref()],
        bump
    )]
    pub sol_vault: SystemAccount<'info>,
    
    #[account(
        mut,
        seeds = [TOKEN_VAULT_SEED, sovereign.key().as_ref()],
        bump
    )]
    pub token_vault: InterfaceAccount<'info, TokenAccountInterface>,
    
    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
    
    /// CHECK: Protocol treasury — receives BYO surplus
    #[account(
        mut,
        address = protocol_state.treasury
    )]
    pub treasury: SystemAccount<'info>,
}

pub fn emergency_remove_liquidity_handler<'info>(
    ctx: Context<'_, '_, 'info, 'info, EmergencyRemoveLiquidity<'info>>,
) -> Result<()> {
    let sovereign = &mut ctx.accounts.sovereign;
    let permanent_lock = &mut ctx.accounts.permanent_lock;
    
    // Guard against double-calling: permanent_lock.liquidity is set to 0 at the end
    require!(permanent_lock.liquidity > 0, SovereignError::NothingToWithdraw);
    
    // ============ Trashbin SAMM Liquidity Removal ============
    // remaining_accounts layout:
    // [0]  nft_account
    // [1]  personal_position (writable)
    // [2]  pool_state (writable)
    // [3]  protocol_position (writable)
    // [4]  token_vault_0 (writable)
    // [5]  token_vault_1 (writable)
    // [6]  tick_array_lower (writable)
    // [7]  tick_array_upper (writable)
    // [8]  recipient_token_account_0 / WGOR ATA (writable)
    // [9]  recipient_token_account_1 / Token ATA (writable)
    // [10] token_program_2022
    // [11] memo_program
    // [12] vault_0_mint
    // [13] vault_1_mint
    // [14] tick_array_bitmap_extension
    
    require!(ctx.remaining_accounts.len() >= 15, SovereignError::MissingSAMMAccounts);
    
    // SECURITY: Validate pool_state matches the permanent_lock's stored pool_state
    require!(
        ctx.remaining_accounts[2].key() == permanent_lock.pool_state,
        SovereignError::InvalidPool
    );
    
    // Read actual liquidity from the SAMM personal position account
    // (may differ from permanent_lock.liquidity if LP was already removed by old program)
    let pp_data = ctx.remaining_accounts[1].try_borrow_data()?;
    let personal_pos = samm::PersonalPositionState::try_deserialize(&pp_data)?;
    let actual_liquidity = personal_pos.liquidity;
    drop(pp_data);
    
    msg!("Permanent lock liquidity: {}, SAMM position liquidity: {}", permanent_lock.liquidity, actual_liquidity);
    
    let sovereign_key = sovereign.key();
    let lock_seeds = &[
        PERMANENT_LOCK_SEED,
        sovereign_key.as_ref(),
        &[permanent_lock.bump],
    ];
    let lock_signer_seeds = &[&lock_seeds[..]];
    
    let recipient_0_info = &ctx.remaining_accounts[8];
    let recipient_1_info = &ctx.remaining_accounts[9];
    
    // ============ Step 1: Remove LP from SAMM (if not already removed) ============
    if actual_liquidity > 0 {
        msg!("Removing liquidity from SAMM pool...");
        
        let decrease_accounts = samm_ix::DecreaseLiquidityV2Accounts {
            nft_owner: permanent_lock.to_account_info(),
            nft_account: ctx.remaining_accounts[0].clone(),
            personal_position: ctx.remaining_accounts[1].clone(),
            pool_state: ctx.remaining_accounts[2].clone(),
            protocol_position: ctx.remaining_accounts[3].clone(),
            token_vault_0: ctx.remaining_accounts[4].clone(),
            token_vault_1: ctx.remaining_accounts[5].clone(),
            tick_array_lower: ctx.remaining_accounts[6].clone(),
            tick_array_upper: ctx.remaining_accounts[7].clone(),
            recipient_token_account_0: ctx.remaining_accounts[8].clone(),
            recipient_token_account_1: ctx.remaining_accounts[9].clone(),
            token_program: ctx.accounts.token_program.to_account_info(),
            token_program_2022: ctx.remaining_accounts[10].clone(),
            memo_program: ctx.remaining_accounts[11].clone(),
            vault_0_mint: ctx.remaining_accounts[12].clone(),
            vault_1_mint: ctx.remaining_accounts[13].clone(),
            tick_array_bitmap_extension: ctx.remaining_accounts[14].clone(),
        };
        
        let _result = samm_cpi::remove_liquidity(
            &ctx.accounts.samm_program.to_account_info(),
            decrease_accounts,
            actual_liquidity,
            0, // Accept any amount during emergency
            0,
            lock_signer_seeds,
        )?;
        msg!("LP removed from SAMM pool");
    } else {
        msg!("SAMM position already drained — skipping CPI, sweeping ATAs");
    }
    
    // ============ Step 2: Read current WGOR ATA balance ============
    let wgor_amount = {
        let data = recipient_0_info.try_borrow_data()?;
        u64::from_le_bytes(data[64..72].try_into().unwrap())
    };
    msg!("WGOR ATA balance: {}", wgor_amount);
    
    // ============ Step 3: Close WGOR ATA → all lamports go to sol_vault ============
    // This unwraps WGOR to native SOL. Uses legacy SPL Token (WGOR is Token program).
    // sol_vault PDA receives all lamports (token amount + rent).
    let close_wgor_ix = anchor_lang::solana_program::instruction::Instruction {
        program_id: anchor_spl::token::ID,
        accounts: vec![
            anchor_lang::solana_program::instruction::AccountMeta::new(
                recipient_0_info.key(), false,
            ),
            anchor_lang::solana_program::instruction::AccountMeta::new(
                ctx.accounts.sol_vault.key(), false,
            ),
            anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                permanent_lock.key(), true,
            ),
        ],
        data: vec![9u8], // SPL Token CloseAccount
    };
    invoke_signed(
        &close_wgor_ix,
        &[
            recipient_0_info.clone(),
            ctx.accounts.sol_vault.to_account_info(),
            permanent_lock.to_account_info(),
        ],
        lock_signer_seeds,
    )?;
    msg!("WGOR ATA closed → {} WGOR unwrapped to sol_vault", wgor_amount);
    
    // ============ Step 4: Update state ============
    // Set unwind_sol_balance to the WGOR token amount (= SOL from LP position).
    // The sol_vault actually receives slightly more (token amount + rent) but
    // using the token amount ensures proportional calculations are accurate.
    sovereign.unwind_sol_balance = wgor_amount;
    
    // Read token ATA balance for informational purposes
    let token_amount = {
        let data = recipient_1_info.try_borrow_data()?;
        u64::from_le_bytes(data[64..72].try_into().unwrap())
    };
    sovereign.unwind_token_balance = token_amount;
    
    // ============ Step 5: Calculate surplus & handle by sovereign type ============
    // No protocol fee during emergency. Investors are capped at original deposit.
    let surplus = wgor_amount.saturating_sub(sovereign.total_deposited);
    
    if sovereign.sovereign_type == SovereignType::TokenLaunch && surplus > 0 {
        // TokenLaunch: surplus → token redemption pool for external token holders
        sovereign.token_redemption_pool = surplus;
        
        // Snapshot circulating tokens: total supply minus protocol-held tokens
        let tv_info = ctx.accounts.token_vault.to_account_info();
        let token_vault_amount = {
            let data = tv_info.try_borrow_data()?;
            u64::from_le_bytes(data[64..72].try_into().unwrap())
        };
        let circulating = ctx.accounts.token_mint.supply
            .saturating_sub(token_vault_amount)
            .saturating_sub(token_amount);
        sovereign.circulating_tokens_at_unwind = circulating;
        sovereign.token_redemption_deadline = Clock::get()?.unix_timestamp + TOKEN_REDEMPTION_WINDOW;
        
        msg!("TokenLaunch surplus: {} GOR → token redemption pool ({} circulating tokens, deadline: {})", 
            surplus, circulating, sovereign.token_redemption_deadline);
    } else if sovereign.sovereign_type == SovereignType::BYOToken && surplus > 0 {
        // BYO: surplus → protocol treasury
        let vault_seeds: &[&[u8]] = &[
            SOL_VAULT_SEED,
            sovereign_key.as_ref(),
            &[ctx.bumps.sol_vault],
        ];
        
        anchor_lang::system_program::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.system_program.to_account_info(),
                anchor_lang::system_program::Transfer {
                    from: ctx.accounts.sol_vault.to_account_info(),
                    to: ctx.accounts.treasury.to_account_info(),
                },
                &[vault_seeds],
            ),
            surplus,
        )?;
        
        // Reduce unwind_sol_balance by surplus sent to treasury
        sovereign.unwind_sol_balance = wgor_amount.saturating_sub(surplus);
        sovereign.token_redemption_pool = 0;
        sovereign.circulating_tokens_at_unwind = 0;
        
        msg!("BYO surplus: {} GOR → treasury", surplus);
    } else {
        sovereign.token_redemption_pool = 0;
        sovereign.circulating_tokens_at_unwind = 0;
    }
    
    // Mark liquidity as fully removed (prevents double-calling)
    permanent_lock.liquidity = 0;
    
    msg!("Emergency sweep complete — unwind_sol_balance: {}, unwind_token_balance: {} (tokens remain in lock ATA)", 
        wgor_amount, token_amount);
    msg!("Token redemption pool: {} GOR surplus, circulating tokens: {}", 
        sovereign.token_redemption_pool, sovereign.circulating_tokens_at_unwind);
    
    Ok(())
}

// ============================================================
// EMERGENCY TOKEN REDEMPTION (External Token Holders)
// ============================================================

/// Emergency token redemption - allows external token holders to burn
/// their sovereign tokens in exchange for a proportional share of the
/// surplus GOR (unwind_sol_balance - total_deposited).
///
/// This protects token holders who bought tokens on the open market
/// and got caught holding them when LP was removed.
///
/// Rate: gor_per_token = token_redemption_pool / circulating_tokens_at_unwind
/// Payout: caller_tokens × token_redemption_pool / circulating_tokens_at_unwind
///
/// Available only after emergency_remove_liquidity has been called and
/// a surplus exists (unwind_sol_balance > total_deposited).
#[derive(Accounts)]
pub struct EmergencyTokenRedemption<'info> {
    #[account(mut)]
    pub caller: Signer<'info>,
    
    #[account(
        mut,
        seeds = [SOVEREIGN_SEED, &sovereign.sovereign_id.to_le_bytes()],
        bump = sovereign.bump,
        constraint = sovereign.state == SovereignStatus::EmergencyUnlocked
            || sovereign.state == SovereignStatus::Retired
            || sovereign.state == SovereignStatus::Unwound @ SovereignError::InvalidState
    )]
    pub sovereign: Account<'info, SovereignState>,
    
    /// CHECK: SOL vault PDA — source of GOR payout
    #[account(
        mut,
        seeds = [SOL_VAULT_SEED, sovereign.key().as_ref()],
        bump
    )]
    pub sol_vault: SystemAccount<'info>,
    
    /// Token mint — needed for burn (mut because supply decreases)
    #[account(
        mut,
        address = sovereign.token_mint
    )]
    pub token_mint: InterfaceAccount<'info, MintInterface>,
    
    /// Caller's token account — tokens will be burned from here
    /// Must be owned by caller and hold the sovereign token
    #[account(
        mut,
        token::mint = token_mint,
        token::authority = caller,
    )]
    pub caller_token_account: InterfaceAccount<'info, TokenAccountInterface>,
    
    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}

pub fn emergency_token_redemption_handler(
    ctx: Context<EmergencyTokenRedemption>,
) -> Result<()> {
    let sovereign = &mut ctx.accounts.sovereign;
    
    // Must have surplus GOR available
    require!(
        sovereign.token_redemption_pool > 0,
        SovereignError::NoRedemptionPool
    );
    require!(
        sovereign.circulating_tokens_at_unwind > 0,
        SovereignError::NoCirculatingTokens
    );
    
    let caller_tokens = ctx.accounts.caller_token_account.amount;
    require!(caller_tokens > 0, SovereignError::NothingToWithdraw);
    
    // Enforce 30-day redemption window
    let clock = Clock::get()?;
    require!(
        clock.unix_timestamp <= sovereign.token_redemption_deadline,
        SovereignError::RedemptionWindowExpired
    );
    
    // Calculate GOR payout: caller_tokens × redemption_pool / circulating_at_unwind
    let gor_payout = (caller_tokens as u128)
        .checked_mul(sovereign.token_redemption_pool as u128)
        .ok_or(SovereignError::Overflow)?
        .checked_div(sovereign.circulating_tokens_at_unwind as u128)
        .ok_or(SovereignError::DivisionByZero)? as u64;
    
    msg!(
        "Token redemption: {} tokens → {} GOR (rate: {} pool / {} circulating)",
        caller_tokens, gor_payout,
        sovereign.token_redemption_pool, sovereign.circulating_tokens_at_unwind
    );
    
    // Burn the caller's sovereign tokens
    let burn_ix = spl_token_2022::instruction::burn_checked(
        &ctx.accounts.token_program.key(),
        &ctx.accounts.caller_token_account.key(),
        &ctx.accounts.token_mint.key(),
        &ctx.accounts.caller.key(),
        &[],
        caller_tokens,
        ctx.accounts.token_mint.decimals,
    )?;
    anchor_lang::solana_program::program::invoke(
        &burn_ix,
        &[
            ctx.accounts.caller_token_account.to_account_info(),
            ctx.accounts.token_mint.to_account_info(),
            ctx.accounts.caller.to_account_info(),
        ],
    )?;
    msg!("Burned {} sovereign tokens", caller_tokens);
    
    // Transfer GOR from sol_vault to caller
    if gor_payout > 0 {
        let vault_balance = ctx.accounts.sol_vault.lamports();
        require!(
            vault_balance >= gor_payout,
            SovereignError::InsufficientVaultBalance
        );
        
        let sovereign_key = sovereign.key();
        let vault_seeds: &[&[u8]] = &[
            SOL_VAULT_SEED,
            sovereign_key.as_ref(),
            &[ctx.bumps.sol_vault],
        ];
        
        anchor_lang::system_program::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.system_program.to_account_info(),
                anchor_lang::system_program::Transfer {
                    from: ctx.accounts.sol_vault.to_account_info(),
                    to: ctx.accounts.caller.to_account_info(),
                },
                &[vault_seeds],
            ),
            gor_payout,
        )?;
        msg!("Transferred {} GOR to token holder", gor_payout);
    }
    
    // Decrement the redemption pool and track consumed circulating tokens
    sovereign.token_redemption_pool = sovereign.token_redemption_pool.saturating_sub(gor_payout);
    sovereign.circulating_tokens_at_unwind = sovereign.circulating_tokens_at_unwind.saturating_sub(caller_tokens);
    
    Ok(())
}

// ============================================================
// SWEEP UNCLAIMED REDEMPTION POOL → TREASURY
// ============================================================

/// After the 30-day token redemption window expires, protocol authority
/// can sweep any unclaimed GOR from the redemption pool to the treasury.
/// This permanently closes the redemption window.
#[derive(Accounts)]
pub struct SweepRedemptionPool<'info> {
    #[account(mut)]
    pub caller: Signer<'info>,
    
    #[account(
        seeds = [PROTOCOL_STATE_SEED],
        bump = protocol_state.bump,
        constraint = protocol_state.authority == caller.key() @ SovereignError::Unauthorized
    )]
    pub protocol_state: Account<'info, ProtocolState>,
    
    #[account(
        mut,
        seeds = [SOVEREIGN_SEED, &sovereign.sovereign_id.to_le_bytes()],
        bump = sovereign.bump,
    )]
    pub sovereign: Account<'info, SovereignState>,
    
    /// CHECK: SOL vault PDA — source of unclaimed GOR
    #[account(
        mut,
        seeds = [SOL_VAULT_SEED, sovereign.key().as_ref()],
        bump
    )]
    pub sol_vault: SystemAccount<'info>,
    
    /// CHECK: Protocol treasury — receives unclaimed GOR
    #[account(
        mut,
        address = protocol_state.treasury
    )]
    pub treasury: SystemAccount<'info>,
    
    pub system_program: Program<'info, System>,
}

pub fn sweep_redemption_pool_handler(ctx: Context<SweepRedemptionPool>) -> Result<()> {
    let sovereign = &mut ctx.accounts.sovereign;
    let clock = Clock::get()?;
    
    // Must have a redemption pool to sweep
    require!(
        sovereign.token_redemption_pool > 0,
        SovereignError::NoRedemptionPool
    );
    
    // Deadline must have passed
    require!(
        clock.unix_timestamp > sovereign.token_redemption_deadline,
        SovereignError::RedemptionWindowNotExpired
    );
    
    let sweep_amount = sovereign.token_redemption_pool;
    
    // Transfer unclaimed GOR from sol_vault → treasury
    let sovereign_key = sovereign.key();
    let vault_seeds: &[&[u8]] = &[
        SOL_VAULT_SEED,
        sovereign_key.as_ref(),
        &[ctx.bumps.sol_vault],
    ];
    
    anchor_lang::system_program::transfer(
        CpiContext::new_with_signer(
            ctx.accounts.system_program.to_account_info(),
            anchor_lang::system_program::Transfer {
                from: ctx.accounts.sol_vault.to_account_info(),
                to: ctx.accounts.treasury.to_account_info(),
            },
            &[vault_seeds],
        ),
        sweep_amount,
    )?;
    
    // Close the redemption window permanently
    sovereign.token_redemption_pool = 0;
    sovereign.circulating_tokens_at_unwind = 0;
    
    msg!("Swept {} unclaimed GOR from redemption pool → treasury", sweep_amount);
    
    Ok(())
}
