use anchor_lang::prelude::*;

#[error_code]
pub enum SovereignError {
    // ============================================================
    // STATE ERRORS (6000-6019)
    // ============================================================
    
    #[msg("Sovereign is not in the expected state")]
    InvalidState,
    
    #[msg("Bonding deadline has passed")]
    DeadlinePassed,
    
    #[msg("Bonding deadline has not passed yet")]
    DeadlineNotPassed,
    
    #[msg("Bonding target not yet met")]
    BondingNotComplete,
    
    #[msg("Bonding target already met")]
    BondingComplete,
    
    #[msg("Recovery phase is not complete")]
    RecoveryNotComplete,
    
    #[msg("Recovery phase is already complete")]
    RecoveryAlreadyComplete,
    
    // ============================================================
    // DEPOSIT ERRORS (6020-6039)
    // ============================================================
    
    #[msg("Creator deposit exceeds maximum allowed (1% of bond target)")]
    CreatorDepositExceedsMax,
    
    #[msg("Deposit amount is zero")]
    ZeroDeposit,
    
    #[msg("Deposit amount below minimum (0.1 SOL)")]
    DepositTooSmall,
    
    #[msg("No deposit record found")]
    NoDepositRecord,
    
    #[msg("Deposit exceeds bond target")]
    DepositExceedsBondTarget,
    
    #[msg("Withdrawal amount exceeds deposit")]
    InsufficientDeposit,
    
    #[msg("Withdrawal amount is zero")]
    ZeroWithdraw,
    
    #[msg("Insufficient deposit balance")]
    InsufficientDepositBalance,
    
    #[msg("Creator cannot withdraw during bonding phase")]
    CreatorCannotWithdrawDuringBonding,
    
    #[msg("Insufficient vault balance")]
    InsufficientVaultBalance,
    
    #[msg("Nothing to withdraw")]
    NothingToWithdraw,
    
    #[msg("Creator must use creator-specific withdraw instruction")]
    CreatorMustUseCreatorWithdraw,
    
    // ============================================================
    // NFT ERRORS (6040-6059)
    // ============================================================
    
    #[msg("Caller is not the NFT owner")]
    NotNFTOwner,
    
    #[msg("NFT has already been used for this action")]
    NFTAlreadyUsed,
    
    #[msg("NFT has already been minted")]
    NFTAlreadyMinted,
    
    #[msg("Wrong NFT for this deposit record")]
    WrongNFT,
    
    #[msg("NFT not yet minted")]
    NFTNotMinted,
    
    #[msg("No Genesis NFT - cannot participate in governance")]
    NoGenesisNFT,
    
    // ============================================================
    // RECOVERY PHASE ERRORS (6060-6079)
    // ============================================================
    
    #[msg("Creator cannot claim fees during recovery phase")]
    CreatorCannotClaimDuringRecovery,
    
    #[msg("Creator cannot vote during recovery phase")]
    CreatorCannotVote,
    
    #[msg("Creator tokens are locked until recovery complete or unwind")]
    CreatorTokensLocked,
    
    // ============================================================
    // GOVERNANCE ERRORS (6080-6099)
    // ============================================================
    
    #[msg("Not enough inactivity to propose unwind")]
    InsufficientInactivity,
    
    #[msg("Voting period has not ended")]
    VotingNotEnded,
    
    #[msg("Voting period has not ended yet")]
    VotingPeriodNotEnded,
    
    #[msg("Voting period has ended")]
    VotingEnded,
    
    #[msg("Voting period has ended")]
    VotingPeriodEnded,
    
    #[msg("Proposal did not reach quorum (67%)")]
    QuorumNotReached,
    
    #[msg("Proposal did not pass (need 51%)")]
    ProposalNotPassed,
    
    #[msg("Proposal is not active")]
    ProposalNotActive,
    
    #[msg("Already voted on this proposal")]
    AlreadyVoted,
    
    #[msg("Governance is only active during recovery phase")]
    GovernanceNotActive,
    
    #[msg("No voting power")]
    NoVotingPower,
    
    #[msg("Timelock period has not expired")]
    TimelockNotExpired,
    
    #[msg("Proposal already executed")]
    ProposalAlreadyExecuted,
    
    #[msg("Active proposal already exists")]
    ActiveProposalExists,
    
    // ============================================================
    // ACTIVE PHASE ERRORS (6100-6119)
    // ============================================================
    
    #[msg("Cannot unwind in active phase via governance")]
    CannotGovernanceUnwindInActivePhase,
    
    #[msg("Auto-unwind conditions not met")]
    AutoUnwindConditionsNotMet,
    
    #[msg("Activity check only valid in Active phase")]
    OnlyActivePhase,
    
    #[msg("Activity check already in progress")]
    ActivityCheckAlreadyInProgress,
    
    #[msg("Activity check already pending")]
    ActivityCheckAlreadyPending,
    
    #[msg("No activity check in progress")]
    NoActivityCheckInProgress,
    
    #[msg("No activity check pending")]
    NoActivityCheckPending,
    
    #[msg("Must wait 90+ days before executing activity check")]
    ActivityCheckTooEarly,
    
    #[msg("Activity check period has not elapsed")]
    ActivityCheckPeriodNotElapsed,
    
    #[msg("Must wait 7 days after cancelled check before initiating new one")]
    ActivityCheckCooldownNotExpired,
    
    #[msg("Fee threshold has been renounced and cannot be changed")]
    FeeThresholdRenounced,
    
    #[msg("Fee threshold already renounced")]
    AlreadyRenounced,
    
    #[msg("Fee threshold already renounced")]
    FeeThresholdAlreadyRenounced,
    
    #[msg("Cannot increase fee threshold")]
    CannotIncreaseFeeThreshold,
    
    #[msg("Invalid fee threshold")]
    InvalidFeeThreshold,
    
    // ============================================================
    // VALIDATION ERRORS (6120-6139)
    // ============================================================
    
    #[msg("Invalid pool - does not match sovereign's whirlpool")]
    InvalidPool,
    
    #[msg("Invalid program ID for CPI")]
    InvalidProgram,
    
    #[msg("Invalid treasury address - cannot be zero")]
    InvalidTreasury,
    
    #[msg("Invalid bond target - must be at least 50 SOL")]
    InvalidBondTarget,
    
    #[msg("Invalid bond duration - must be 7-30 days")]
    InvalidBondDuration,
    
    #[msg("Invalid sell fee - must be 0-3%")]
    InvalidSellFee,
    
    #[msg("Invalid amount")]
    InvalidAmount,
    
    #[msg("Bond target not met")]
    BondTargetNotMet,
    
    #[msg("Bond target already met")]
    BondTargetMet,
    
    #[msg("Unauthorized")]
    Unauthorized,
    
    #[msg("Fee too high")]
    FeeTooHigh,
    
    // ============================================================
    // POOL ERRORS (6140-6159)
    // ============================================================
    
    #[msg("Pool is restricted - only Genesis position can LP")]
    PoolRestricted,
    
    #[msg("Pool is not restricted")]
    PoolNotRestricted,
    
    #[msg("Position already unwound")]
    PositionAlreadyUnwound,
    
    #[msg("Invalid position - does not match permanent lock")]
    InvalidPosition,
    
    // ============================================================
    // FEE ERRORS (6160-6179)
    // ============================================================
    
    #[msg("Sell fee exceeds maximum (3%)")]
    SellFeeExceedsMax,
    
    #[msg("Creation fee exceeds maximum (10%)")]
    CreationFeeExceedsMax,
    
    #[msg("Unwind fee exceeds maximum (10%)")]
    UnwindFeeExceedsMax,
    
    #[msg("Fee control has been renounced")]
    FeeControlRenounced,
    
    #[msg("Insufficient creation fee")]
    InsufficientCreationFee,
    
    // ============================================================
    // PROTOCOL ADMIN ERRORS (6180-6199)
    // ============================================================
    
    #[msg("Caller is not the protocol authority")]
    NotProtocolAuthority,
    
    #[msg("Auto-unwind period outside valid range (90-365 days)")]
    InvalidAutoUnwindPeriod,
    
    // ============================================================
    // TOKEN LAUNCHER ERRORS (6200-6219)
    // ============================================================
    
    #[msg("Token metadata URI is too long")]
    MetadataURITooLong,
    
    #[msg("Token name is too long")]
    TokenNameTooLong,
    
    #[msg("Token symbol is too long")]
    TokenSymbolTooLong,
    
    #[msg("Token Launcher: Missing token name")]
    MissingTokenName,
    
    #[msg("Token Launcher: Missing token symbol")]
    MissingTokenSymbol,
    
    #[msg("Token Launcher: Missing token supply")]
    MissingTokenSupply,
    
    // ============================================================
    // BYO TOKEN ERRORS (6220-6239)
    // ============================================================
    
    #[msg("BYO Token: Missing existing mint address")]
    MissingExistingMint,
    
    #[msg("BYO Token: Missing deposit amount")]
    MissingDepositAmount,
    
    #[msg("BYO Token: Insufficient token deposit (below minimum % required)")]
    InsufficientTokenDeposit,
    
    #[msg("BYO Token: Failed to read token supply")]
    FailedToReadTokenSupply,
    
    // ============================================================
    // CLAIM ERRORS (6240-6259)
    // ============================================================
    
    #[msg("Already claimed")]
    AlreadyClaimed,
    
    #[msg("Nothing to claim")]
    NothingToClaim,
    
    #[msg("Caller is not the creator")]
    NotCreator,
    
    #[msg("Caller is not the depositor")]
    NotDepositor,
    
    // ============================================================
    // ARITHMETIC ERRORS (6260-6279)
    // ============================================================
    
    #[msg("Arithmetic overflow")]
    Overflow,
    
    #[msg("Arithmetic underflow")]
    Underflow,
    
    #[msg("Division by zero")]
    DivisionByZero,
    
    #[msg("No deposits in the sovereign")]
    NoDeposits,
    
    // ============================================================
    // SLIPPAGE ERRORS (6280-6299)
    // ============================================================
    
    #[msg("Slippage tolerance exceeded")]
    SlippageExceeded,
    
    // ============================================================
    // PROTOCOL SAFETY ERRORS (6300-6319)
    // ============================================================
    
    #[msg("Protocol is currently paused")]
    ProtocolPaused,
    
    #[msg("Activity check cooldown has not elapsed (7 days required)")]
    ActivityCheckCooldownNotElapsed,
}
