# Sovereign Liquidity Protocol - Solana Technical Specification

**Version:** 1.2  
**Chain:** Solana  
**DEX:** Forked Orca Whirlpools (with LP restriction)  
**Framework:** Anchor  
**Last Updated:** January 2026

---

## Table of Contents

1. [Overview](#overview)
2. [Solana vs EVM - Key Differences](#solana-vs-evm---key-differences)
3. [Core Concepts](#core-concepts)
4. [Architecture](#architecture)
5. [Protocol Parameters](#protocol-parameters)
6. [Program Accounts](#program-accounts)
7. [Sovereign Lifecycle](#sovereign-lifecycle)
8. [Creator Vault & Token Unlocks](#creator-vault--token-unlocks)
9. [Fee Distribution](#fee-distribution)
10. [Governance & Unwind](#governance--unwind)
11. [Activity Check (Auto-Unwind)](#activity-check-auto-unwind-for-active-phase)
12. [Protocol Administration](#protocol-administration)
13. [Security Considerations](#security-considerations)
14. [Token Launcher](#token-launcher)
15. [Appendix A: External Dependencies](#appendix-a-external-dependencies)
16. [Appendix B: Events](#appendix-b-events)
17. [Appendix C: Error Codes](#appendix-c-error-codes)
18. [Appendix D: Compute Budget](#appendix-d-compute-budget)
19. [Appendix E: Account Sizes](#appendix-e-account-sizes)
20. [Appendix F: Version History](#appendix-f-version-history)

---

## Overview

The Sovereign Liquidity Protocol (SLP) on Solana enables token creators to launch tokens with built-in sell fees and bootstrap liquidity through community bonding. The protocol prioritizes investor protection through a recovery mechanism where ALL trading fees flow to depositors until they recover their principal.

### Token Launch Options

SLP supports two ways to bootstrap liquidity:

| Feature | Token Launcher | BYO Token (Bring Your Own) |
|---------|---------------|----------------------------|
| Token Creation | Protocol creates Token-2022 | Creator brings existing SPL token |
| Sell Tax | Built-in via transfer hooks (0-3%) | Not available (unless token has hooks) |
| Supply Requirement | 100% of newly minted supply | Min 30% of circulating supply (protocol-adjustable) |
| Use Case | New token launches | Existing tokens bootstrapping liquidity |
| Fee Modes | CreatorRevenue, RecoveryBoost, FairLaunch | N/A (swap fees only) |

### Key Principles

- **Dual Launch Modes**: Token Launcher (new tokens) or BYO Token (existing tokens)
- **Creator Token Launch**: Creators launch tokens with customizable sell fees (0-3%)
- **Fair Launch**: Creator can buy up to 1% of token supply via market buy (not LP participation)
- **Creator Tokens Locked**: Creator's purchased tokens locked until post-recovery or unwind
- **Investors Own LP**: Only investors contribute SOL to LP, only investors get Genesis NFTs
- **Full Range Liquidity**: LP always deployed as full range for deep, fair, equal liquidity
- **Restricted LP During Recovery**: Pool locked to Genesis position only until recovery complete
- **Recovery First**: 100% of LP fees go to investors until principal recovered
- **SOL Recovery**: Recovery calculated on SOL fees only (not token fees)
- **Protected Governance**: Investors can vote to unwind during recovery (creator has no vote)
- **Permanent Liquidity**: After recovery, LP is permanently locked
- **Failed Project Protection**: Low volume triggers unwind - investors get SOL, creator gets tokens

---

## Protocol Flow

### Complete Lifecycle

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         SOVEREIGN LIQUIDITY PROTOCOL                     │
│                                                                          │
│  ╔═══════════════════════════════════════════════════════════════════╗  │
│  ║  1. TOKEN LAUNCH                                                   ║  │
│  ║     Choose launch type:                                            ║  │
│  ║       • Token Launcher: Create new token, 0-3% sell fee           ║  │
│  ║       • BYO Token: Bring existing token, no sell tax              ║  │
│  ║     Sets raise amount (e.g., 100 SOL)                             ║  │
│  ║     Deposits tokens: 100% (new) or ≥30% of supply (BYO)           ║  │
│  ║     Optional: Creator buys in (max 1% at theoretical price)       ║  │
│  ║     ⚠️  Creator's tokens LOCKED until post-recovery or unwind     ║  │
│  ╚═══════════════════════════════════════════════════════════════════╝  │
│                                    │                                     │
│                                    ▼                                     │
│  ╔═══════════════════════════════════════════════════════════════════╗  │
│  ║  2. BONDING PHASE                                                  ║  │
│  ║     Investors deposit SOL                                         ║  │
│  ║     Bond target must be met exactly                               ║  │
│  ║     Duration: 7-30 days                                           ║  │
│  ╚═══════════════════════════════════════════════════════════════════╝  │
│                                    │                                     │
│                     ┌──────────────┴──────────────┐                     │
│                     ▼                              ▼                     │
│  ┌─────────────────────────┐        ┌─────────────────────────┐        │
│  │    TARGET MET           │        │   TARGET NOT MET        │        │
│  │    → Finalize           │        │   → Refund all          │        │
│  └───────────┬─────────────┘        └─────────────────────────┘        │
│              │                                                          │
│              ▼                                                          │
│  ╔═══════════════════════════════════════════════════════════════════╗  │
│  ║  3. FINALIZATION                                                   ║  │
│  ║     • Create Orca Whirlpool with SOL + Tokens                     ║  │
│  ║     • Mint Genesis NFTs to INVESTORS ONLY (not creator)          ║  │
│  ║     • NFT tracks: deposit amount, share %, fees claimed           ║  │
│  ║     • Enter RECOVERY PHASE                                        ║  │
│  ╚═══════════════════════════════════════════════════════════════════╝  │
│                                    │                                     │
│                                    ▼                                     │
│  ╔═══════════════════════════════════════════════════════════════════╗  │
│  ║  4. RECOVERY PHASE                                                 ║  │
│  ║                                                                    ║  │
│  ║     ┌─────────────────────────────────────────────────────────┐   ║  │
│  ║     │  100% FEES → INVESTORS (creator has NO LP fee share)      │   ║  │
│  ║     │  Recovery = SOL fees only (tokens excluded)             │   ║  │
│  ║     │  Target = Total SOL deposited by investors              │   ║  │
│  ║     │  Creator's purchased tokens remain LOCKED               │   ║  │
│  ║     │  Pool RESTRICTED - only Genesis position can LP         │   ║  │
│  ║     └─────────────────────────────────────────────────────────┘   ║  │
│  ║                                                                    ║  │
│  ║     GOVERNANCE ACTIVE:                                            ║  │
│  ║     • Investors can vote to unwind (creator CANNOT vote)          ║  │
│  ║     • 67% quorum, 51% to pass, 2-day timelock                    ║  │
│  ║     • Unwind: Investors get SOL, Creator gets tokens              ║  │
│  ╚═══════════════════════════════════════════════════════════════════╝  │
│                                    │                                     │
│              ┌─────────────────────┼─────────────────────┐              │
│              ▼                     │                     ▼              │
│  ┌─────────────────────┐          │        ┌─────────────────────┐     │
│  │  GOVERNANCE UNWIND  │          │        │  RECOVERY COMPLETE  │     │
│  │  (vote passes)      │          │        │  (SOL fees >= bond) │     │
│  │                     │          │        │                     │     │
│  │  • Investors → SOL  │          │        │  → ACTIVE PHASE     │     │
│  │  • Creator → Tokens │          │        │                     │     │
│  └─────────────────────┘          │        └──────────┬──────────┘     │
│                                   │                   │                 │
│                                   │                   ▼                 │
│  ╔════════════════════════════════════════════════════════════════╗    │
│  ║  5. POST-RECOVERY (ACTIVE PHASE)                                ║    │
│  ║                                                                  ║    │
│  ║     • LP is permanently locked (unless low volume)              ║    │
│  ║     • Pool UNLOCKED - external LPs can now provide liquidity    ║    │
│  ║     • 100% fees STILL go to investors (creator has NO share)    ║    │
│  ║     • Creator's purchased tokens UNLOCK - can now claim         ║    │
│  ║     • No governance voting in Active phase                      ║    │
│  ║     • Trading continues                                         ║    │
│  ║                                                                  ║    │
│  ║     ⚠️  LOW VOLUME AUTO-UNWIND (90-365 days, protocol-set):     ║    │
│  ║     • If no trading activity for extended period                ║    │
│  ║     • LP unlocks and can be unwound                             ║    │
│  ║     • Investors → SOL, Creator → Tokens                         ║    │
│  ╚════════════════════════════════════════════════════════════════╝    │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

### Fee Flow During Recovery

```
                    TRADING ACTIVITY
                          │
                          ▼
              ┌───────────────────────┐
              │    Sell Fee (0-3%)    │
              └───────────┬───────────┘
                          │
                          ▼
              ┌───────────────────────┐
              │  Swap Fees (0.3-2%)   │
              └───────────┬───────────┘
                          │
          ┌───────────────┴───────────────┐
          │                               │
          ▼                               ▼
    ┌───────────┐                   ┌───────────┐
    │ SOL Fees  │                   │Token Fees │
    └─────┬─────┘                   └─────┬─────┘
          │                               │
          ▼                               ▼
    ┌───────────────────┐         ┌───────────────────┐
    │ 100% → INVESTORS  │         │ 100% → INVESTORS  │
    │ (recovery credit) │         │ (NOT credited)    │
    └───────────────────┘         └───────────────────┘
          │
          ▼
    Recovery Progress = SOL distributed / Total SOL raised
```

### Fee Flow After Recovery

```
              ┌───────────────────────┐
              │   All Trading Fees    │
              └───────────┬───────────┘
                          │
          ┌───────────────┴───────────────┐
          │                               │
          ▼                               ▼
    ┌───────────────┐             ┌───────────────┐
    │  SOL Fees     │             │  Token Fees   │
    └───────┬───────┘             └───────┬───────┘
            │                             │
            ▼                             ▼
    ┌─────────────────────────────────────────────┐
    │      100% TO INVESTORS (ALWAYS)           │
    │                                             │
    │   Genesis Position (Protocol-controlled):  │
    │     • Investors: 100% (based on NFT shares) │
    │     • Creator: 0% (no LP fee entitlement)   │
    │                                             │
    │   External LPs (Non-Genesis):              │
    │     • Earn fees on their own positions     │
    └─────────────────────────────────────────────┘
```

**Note:** Only the Genesis LP is locked as NFTs. External LPs earn fees on their own positions.

---

## Solana vs EVM - Key Differences

### Account Model vs Contract Storage

| EVM | Solana |
|-----|--------|
| Contract holds its own storage | Programs are stateless; data in separate accounts |
| Single contract address | Program ID + multiple PDAs (Program Derived Addresses) |
| msg.sender | Signer accounts passed to instructions |
| mapping(address => uint) | PDA accounts with seeds |

### Key Architectural Changes

```
EVM Architecture                          Solana Architecture
┌─────────────────────┐                  ┌─────────────────────┐
│  Sovereign Contract    │                  │  SLP Program        │
│  ├─ storage         │                  │  (stateless)        │
│  ├─ deposits[]      │                  └─────────┬───────────┘
│  └─ functions       │                            │
└─────────────────────┘                  ┌─────────┴───────────┐
                                         │    PDAs (Data)      │
                                         ├─────────────────────┤
                                         │ SovereignState PDA     │
                                         │ DepositRecord PDA   │
                                         │ GenesisNFT PDA      │
                                         │ VaultState PDA      │
                                         └─────────────────────┘
```

### Solana-Specific Advantages

| Feature | Benefit |
|---------|---------|
| **Parallel Execution** | Multiple sovereigns can process deposits simultaneously |
| **Low Fees** | ~$0.00025 per transaction vs $5-50 on EVM |
| **Fast Finality** | ~400ms vs 12+ seconds |
| **Native NFTs** | Metaplex standard, compressed NFTs option |
| **Token Extensions** | Transfer hooks, permanent delegate, metadata |

### Solana-Specific Challenges

| Challenge | Solution |
|-----------|----------|
| Account size limits (10MB) | Split large data across multiple accounts |
| Compute limits (200K CU) | Optimize instructions, split complex operations |
| No built-in hooks | Custom CPI (Cross-Program Invocation) patterns |
| Rent exemption | Minimum SOL balance required for accounts |

---

## Core Concepts

### Sovereign

A fundraising period where depositors bond SOL with creator-supplied tokens to form an Orca Whirlpool fork liquidity position. Each sovereign creates:
- A **SovereignState** PDA holding configuration and totals
- Multiple **DepositRecord** PDAs (one per depositor)
- A **Position** account (Orca Whirlpool Position NFT)
- Genesis NFTs for each depositor

### Sovereign Types

```rust
#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Eq)]
pub enum SovereignType {
    TokenLaunch,  // Protocol creates new Token-2022 with transfer hooks
    BYOToken,     // Creator brings existing SPL token (no sell tax)
}
```

**Token Launcher (TokenLaunch):**
- Protocol mints new Token-2022 with transfer hooks
- Sell tax (0-3%) automatically applied on sells
- Creator deposits 100% of supply
- Full fee mode support (CreatorRevenue, RecoveryBoost, FairLaunch)

**BYO Token (Bring Your Own):**
- Creator deposits existing SPL/Token-2022 token
- Minimum 30% of circulating supply required (protocol-adjustable)
- No sell tax (unless token already has transfer hooks)
- Investors earn only from LP swap fees
- Ideal for existing projects bootstrapping liquidity

### Genesis NFT (Metaplex)

A Metaplex NFT representing a depositor's share of a sovereign's liquidity position:
- Minted upon Sovereign Finalization using Metaplex Token Metadata
- Stores share percentage in on-chain metadata
- Transferable (fee claims follow NFT ownership)
- Burned upon claiming unwind proceeds

**Metadata Structure (example - values vary per investor):**
```json
{
  "name": "Genesis #1 - Sovereign ABC",
  "symbol": "GSLP",
  "uri": "https://arweave.net/...",
  "attributes": [
    { "trait_type": "sovereign", "value": "ABC123..." },
    { "trait_type": "Shares BPS", "value": "1000" },
    { "trait_type": "Deposit Amount", "value": "10.5 SOL" }
  ]
}
```

**Note:** `Shares BPS` and `Deposit Amount` are calculated at finalization based on each investor's actual deposit. The example shows 10% share (1000 bps) for a 10.5 SOL deposit.

### Creator Vault

A PDA tracking the creator's token supply deposit and optional token purchase:

**Token Supply Deposit:**
- **Token Launcher**: Creator deposits 100% of newly minted tokens
- **BYO Token**: Creator deposits ≥30% of existing token supply (protocol-adjustable)
- 100% of deposited tokens go to LP
- Creator receives deposited tokens back ONLY on unwind (governance or low volume)

**Creator Token Purchase (Optional - Max 1% of Total Supply):**

```
┌─────────────────────────────────────────────────────────────────┐
│                    CREATOR BUY-IN FLOW                           │
│                                                                  │
│  BONDING PHASE:                                                  │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │  Creator deposits SOL (max 1% of bond target)            │   │
│  │  → SOL held in ESCROW (not added to LP funds)            │   │
│  │  → Does NOT count toward bond target                     │   │
│  │  → Creator does NOT get Genesis NFT                      │   │
│  └─────────────────────────────────────────────────────────┘   │
│                            │                                     │
│                            ▼                                     │
│  FINALIZATION:                                                   │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │  1. Create LP with: Investor SOL + Creator's Tokens      │   │
│  │  2. Mint Genesis NFTs to INVESTORS ONLY                  │   │
│  │  3. Market buy tokens using creator's escrowed SOL       │   │
│  │  4. Lock creator's purchased tokens                      │   │
│  └─────────────────────────────────────────────────────────┘   │
│                            │                                     │
│                            ▼                                     │
│  RECOVERY PHASE:                                                 │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │  Creator's tokens remain LOCKED                          │   │
│  │  Creator has NO fee entitlement (no Genesis NFT)         │   │
│  │  Creator CANNOT vote (no Genesis NFT)                    │   │
│  └─────────────────────────────────────────────────────────┘   │
│                            │                                     │
│              ┌─────────────┴─────────────┐                      │
│              ▼                           ▼                       │
│  ┌──────────────────┐        ┌──────────────────┐              │
│  │ RECOVERY COMPLETE│        │     UNWIND       │              │
│  │                  │        │                  │              │
│  │ Creator can      │        │ Creator receives │              │
│  │ claim purchased  │        │ purchased tokens │              │
│  │ tokens           │        │ + LP tokens back │              │
│  └──────────────────┘        └──────────────────┘              │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

**Key Points:**
- Creator's SOL is NOT part of the LP - it's used for market buy
- Creator does NOT receive a Genesis NFT
- Creator has NO LP fee share (100% goes to investors)
- Creator's only benefit: purchased tokens + sell fee revenue (if configured)

### Permanent Lock

A PDA that controls the Orca Whirlpool position NFT:
- Holds the position NFT as permanent delegate
- Allows fee collection only
- **During Recovery**: LP locked, can be unwound via governance or low volume
- **After Recovery**: LP is **PERMANENTLY LOCKED** - no unwind possible
- Releases liquidity only during recovery phase unwind events

### Bond Target

The exact amount of SOL required to finalize a sovereign:
- At least 50 SOL (~$10,000 at $200/SOL)
- Met exactly (no partial or excess)
- Raised within the bond duration (7-30 days)

---

## Architecture

### Program Structure (Anchor)

```
sovereign_liquidity/
├── programs/
│   └── sovereign-liquidity/
│       └── src/
│           ├── lib.rs              # Program entry point
│           ├── instructions/       # Instruction handlers
│           │   ├── mod.rs
│           │   ├── initialize.rs
│           │   ├── create_sovereign.rs
│           │   ├── deposit.rs
│           │   ├── finalize.rs
│           │   ├── claim_fees.rs
│           │   ├── propose_unwind.rs
│           │   ├── vote.rs
│           │   └── execute_unwind.rs
│           ├── state/              # Account structures
│           │   ├── mod.rs
│           │   ├── protocol.rs
│           │   ├── sovereign.rs
│           │   ├── deposit.rs
│           │   └── proposal.rs
│           ├── errors.rs           # Custom errors
│           ├── events.rs           # Event definitions
│           └── constants.rs        # Protocol constants
├── tests/
│   └── sovereign-liquidity.ts
└── Anchor.toml
```

### PDA Derivation Seeds

| Account | Seeds | Description |
|---------|-------|-------------|
| ProtocolState | `["protocol"]` | Global protocol config |
| SovereignState | `["sovereign", sovereign_id]` | Per-sovereign configuration |
| DepositRecord | `["deposit", sovereign_pubkey, depositor_pubkey]` | Per-investor record (NO creator record) |
| CreatorFeeTracker | `["creator_fees", sovereign_pubkey]` | Creator's purchased tokens (NO fee share) |
| PermanentLock | `["lock", sovereign_pubkey]` | LP position controller |
| FeeVault | `["fees", sovereign_pubkey]` | Collected fee storage |
| Proposal | `["proposal", sovereign_pubkey, proposal_id]` | Governance proposal |

### Account Relationships

```
┌─────────────────────────────────────────────────────────────────────┐
│                         SLP Program                                  │
│                                                                      │
│  ┌──────────────────┐                                               │
│  │  ProtocolState   │◄──────────────────────────────────┐           │
│  │  - authority     │                                    │           │
│  │  - treasury      │                                    │           │
│  │  - fee_bps       │                                    │           │
│  └──────────────────┘                                    │           │
│           │                                              │           │
│           │ creates                                      │           │
│           ▼                                              │           │
│  ┌──────────────────┐     ┌────────────────────┐        │           │
│  │   SovereignState    │────►│ CreatorFeeTracker  │        │           │
│  │  - creator       │     │ - purchased_tokens │        │           │
│  │  - token_mint    │     │ - tokens_locked    │        │           │
│  │  - bond_target   │     │ - unwind_claimed   │        │           │
│  │  - creator_escrow│     │ (NO fee share!)    │        │           │
│  │  - state (Recovery/   │                    │        │           │
│  │    Active/Unwound)    └────────────────────┘        │           │
│  └────────┬─────────┘                                    │           │
│           │               ┌──────────────────┐          │           │
│           │               │  PermanentLock   │          │           │
│           │               │  - position_nft  │          │           │
│           │               │  - locked after  │          │           │
│           │               │    recovery      │          │           │
│           │               └──────────────────┘          │           │
│           │                                              │           │
│           │ has many (INVESTORS ONLY - creator has none)│           │
│           ▼                                              │           │
│  ┌──────────────────┐                                   │           │
│  │  DepositRecord   │                                   │           │
│  │  - depositor     │──────► Genesis NFT (Metaplex)     │           │
│  │  - amount        │       (Creator does NOT get one)  │           │
│  │  - shares_bps    │                                   │           │
│  │  - fees_claimed  │                                   │           │
│  └──────────────────┘                                   │           │
│                                                          │           │
│  ┌──────────────────┐                                   │           │
│  │    Proposal      │◄──────────────────────────────────┘           │
│  │  - for_votes     │  (investors only, no creator)                 │
│  │  - against_votes │                                               │
│  │  - end_time      │                                               │
│  └──────────────────┘                                               │
└─────────────────────────────────────────────────────────────────────┘
```

### Orca Whirlpools Integration

The protocol integrates with **Orca Whirlpools** (concentrated liquidity AMM):

```
┌─────────────────────────────────────────────────────────────────────┐
│                    Sovereign Finalization                               │
│                                                                      │
│  1. Calculate SOL/Token amounts for LP                              │
│  2. CPI to Whirlpool: open_position()                               │
│  3. CPI to Whirlpool: increase_liquidity()                          │
│  4. Store position NFT in PermanentLock PDA                         │
│  5. Mint Genesis NFTs to depositors                                 │
│                                                                      │
│                              │                                       │
│                              ▼                                       │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │                    Orca Whirlpool                            │   │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐         │   │
│  │  │ Position NFT│  │  Whirlpool  │  │  TickArray  │         │   │
│  │  │ (locked)    │  │   State     │  │             │         │   │
│  │  └─────────────┘  └─────────────┘  └─────────────┘         │   │
│  └─────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────┘
```

### Fee Collection Flow

```
┌─────────────────────────────────────────────────────────────────────┐
│                     Fee Collection (claim_fees)                      │
│                                                                      │
│  1. Verify caller owns Genesis NFT                                  │
│  2. CPI to Whirlpool: collect_fees() on position                    │
│  3. Calculate claimable share based on NFT shares                   │
│  4. Transfer SOL fees to caller                                     │
│  5. Transfer token fees to caller                                   │
│  6. Update claimed_fees in DepositRecord                           │
│  7. Update total_distributed in SovereignState                        │
│  8. CPI to CreatorVault: update unlock tracking                    │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

---

## Protocol Parameters

### Creation Requirements

| Parameter | Value | Description |
|-----------|-------|-------------|
| Minimum Bond Target | 50 SOL (~$10K) | Minimum SOL required for a valid sovereign |
| Minimum Deposit | 0.1 SOL | Minimum single deposit to prevent dust spam |
| Creator Token Deposit (Token Launcher) | 100% of supply | Creator must deposit entire token supply |
| Creator Token Deposit (BYO Token) | Min 30% of supply | Minimum supply required (protocol-adjustable) |
| Token → LP | 100% of deposited | All deposited tokens go to liquidity pool |
| Bond Duration | 7-30 days (creator chooses) | Time window to reach bond target |
| Creator Max Buy-In | 1% of deposited supply | Max tokens creator can purchase at theoretical price |
| Creator Token Lock | Until post-recovery | Purchased tokens locked until recovery complete or unwind |
| Liquidity Range | Full Range | Always MIN_TICK to MAX_TICK for fair deep liquidity |
| Auto-Unwind Period | 90-365 days (protocol) | Protocol sets inactivity threshold for auto-unwind |
| Activity Check Cooldown | 7 days | Wait period after cancelled check before new initiation |
| Tick Spacing | Pool default | Orca Whirlpool tick spacing (determines fee tier) |

### Fee Configuration (Creator Set at Launch)

| Parameter | Range | Default | Applies To | Description |
|-----------|-------|---------|------------|-------------|
| Sell Fee | 0-3% (0-300 bps) | 0% | Token Launcher only | Tax on token sells only (updatable by creator) |
| Swap Fee | 0.3-2% (30-200 bps) | 0.3% | Both types | Trading fee on each swap |

**Sell Fee Control (Token Launcher only):**
- Creator can **update** sell fee at any time (within 0-3% range)
- Creator can **renounce** fee control permanently
- Once renounced, sell fee is locked at current value forever
- Renouncing is irreversible - cannot be undone

**BYO Token Note:** Sell fees are not available for BYO tokens unless the token already has Token-2022 transfer hooks configured. BYO token investors earn only from LP swap fees.

### Protocol Revenue (Protocol-Adjustable)

The protocol generates revenue through four mechanisms:

| Fee Type | Range | Default | Description |
|----------|-------|---------|-------------|
| Creation Fee | 0-10% of bond target | 0.5% | **Escrowed** until bonding completes, refundable minus min fee on failure |
| Minimum Fee | Adjustable | 0.05 SOL | Non-refundable portion of creation fee on failed bonding |
| Governance Unwind Fee | Adjustable | 0.05 SOL | Fee to call governance unwind during recovery |
| Unwind Fee | 0-10% of SOL | 5% | Taken from SOL returned during unwind |

**Creation Fee Logic:**
```
┌─────────────────────────────────────────────────────────────────┐
│                    CREATION FEE FLOW                             │
│                                                                  │
│   Creator calls create_sovereign()                               │
│           │                                                      │
│           ▼                                                      │
│   Creation Fee = bond_target × creation_fee_bps / 10000         │
│   (e.g., 100 SOL × 0.5% = 0.5 SOL)                              │
│           │                                                      │
│           ▼                                                      │
│   ┌───────────────────┐                                         │
│   │ ESCROW (Fee PDA)  │ ◄── Fee held during bonding phase       │
│   └─────────┬─────────┘                                         │
│             │                                                    │
│    ┌────────┴────────┐                                          │
│    │                 │                                          │
│    ▼                 ▼                                          │
│ BONDING           BONDING                                       │
│ SUCCEEDS          FAILS                                         │
│    │                 │                                          │
│    ▼                 ▼                                          │
│ Full fee →       Refund fee                                     │
│ Treasury         MINUS min fee                                  │
│                  (min fee → Treasury)                           │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

**Fee Rules:**
- **Creation**: Creator pays `bond_target * creation_fee_bps / 10000` into **escrow**
  - This is separate from the optional token buy-in escrow
  - Fee held in escrow until bonding outcome is determined
- **Successful Bonding**: 
  - Full creation fee transferred from escrow to protocol treasury
  - Investor SOL goes 100% to LP (no deductions)
- **Failed Bonding**: 
  - Creation fee **refunded** to creator minus minimum fee
  - Investors get full SOL refund (via `withdraw()`)
  - Creator gets escrowed SOL back (the token buy-in escrow, if any)
  - Creator gets 100% of tokens back (never went to LP)
- **Governance Unwind**: Proposer pays `governance_unwind_fee` (default 0.05 SOL) to call `propose_unwind()`
- **Unwind Execution**: Protocol takes `unwind_sol * unwind_fee_bps / 10000` before distributing to investors

**All fees are protocol-adjustable** via `ProtocolState` updates by the authority.

### Recovery Phase Rules

| Rule | Description |
|------|-------------|
| Recovery Target | Total SOL deposited by investors (excludes creator) |
| Fee Flow | 100% of ALL fees (SOL + token) go to investors |
| SOL Fees | Count toward recovery progress |
| Token Fees | Claimable by investors but DON'T count toward recovery |
| Pool Restriction | Only Genesis position can LP (external LPs blocked) |
| Creator Locked | Creator cannot claim any fees during recovery |
| Creator No Vote | Creator excluded from governance votes |

### Governance Parameters

| Parameter | Value | Description |
|-----------|-------|-------------|
| Inactivity Period | 90 days | No volume required to enable proposals (recovery only) |
| Auto-Unwind Period | 90-365 days (protocol sets) | Minimum wait for activity check (active phase only) |
| Voting Period | 7 days (604,800 slots) | Duration for vote casting |
| Quorum | 67% | Minimum investor participation required |
| Pass Threshold | 51% | Votes for vs total votes cast |
| Timelock | 2 days (172,800 slots) | Delay after successful vote |
| Who Can Vote | Investors only | Creator CANNOT vote |
| Governance Unwind Fee | 0.05 SOL (protocol-adjustable) | Fee to create unwind proposal during recovery |

**Unwind Rules by Phase:**
- **Recovery Phase**: Governance vote can unwind (investors vote, creator excluded), costs 0.05 SOL
- **Active Phase**: Only auto-unwind via two-call activity check (no voting)

### Activity Check Parameters (Active Phase Auto-Unwind)

| Parameter | Range | Default | Description |
|-----------|-------|---------|-------------|
| Auto-Unwind Period | 90-365 days | 90 days | Minimum time between initiate and execute |
| Fee Growth Threshold | > 0, Protocol-adjustable | 1000 (min) | Minimum fee_growth to count as "active" (must be > 0) |
| Threshold Renounced | true/false | false | If true, threshold locked forever |

**Two-Call Design:**
1. `initiate_activity_check()` - Snapshots current cumulative fee growth
2. Wait 90+ days (enforced on-chain)
3. `execute_activity_check()` - Compares growth to threshold
   - If growth < threshold → **UNWIND** (investors get SOL, creator gets tokens)
   - If growth ≥ threshold → **CANCELLED** (state resets, can try again)

**Note:** Threshold must always be > 0 to prevent edge case where zero activity equals threshold.

**Both calls are permissionless** - anyone can trigger them.

### Unwind Distribution

| Recipient | Asset | Claim Function | Calculation |
|-----------|-------|----------------|-------------|
| Investors | SOL only | `claim_investor_unwind()` (requires Genesis NFT) | Proportional to shares (minus unwind fee) |
| Creator | Tokens only | `claim_creator_unwind()` (requires creator signature) | 100% of tokens from LP + purchased tokens |

---

## Program Accounts

### ProtocolState

```rust
#[account]
pub struct ProtocolState {
    pub authority: Pubkey,           // Protocol admin (can update fees)
    pub treasury: Pubkey,            // Fee recipient wallet
    
    // Creation Fee (escrowed during bonding, released on success, refunded minus min fee on failure)
    pub creation_fee_bps: u16,       // 0-1000 (0-10% of bond target), default 50 (0.5%)
    
    // Minimum Fee (non-refundable portion of creation fee on failed bonding)
    pub min_fee_lamports: u64,       // Default 0.05 SOL (50_000_000 lamports)
    
    // Governance Unwind Fee (fee to create proposal during recovery)
    pub governance_unwind_fee_lamports: u64,  // Default 0.05 SOL (50_000_000 lamports)
    
    // Unwind Fee (taken from SOL during unwind)
    pub unwind_fee_bps: u16,         // 0-1000 (0-10% of unwind SOL), default 500 (5%)
    
    // BYO Token Settings
    pub byo_min_supply_bps: u16,     // Minimum % of supply required for BYO (default 3000 = 30%)
    
    // Protocol limits
    pub min_bond_target: u64,        // Minimum bond target (50 SOL)
    pub auto_unwind_period: i64,     // 90-365 days (protocol-controlled)
    
    // Activity Check Threshold (for auto-unwind in Active phase)
    pub min_fee_growth_threshold: u128,  // Minimum fee growth to count as "active" (MUST be > 0)
    pub fee_threshold_renounced: bool,   // If true, threshold locked forever
    
    // Stats
    pub sovereign_count: u64,        // Total sovereigns created
    pub total_fees_collected: u64,   // Lifetime protocol revenue
    
    pub bump: u8,                    // PDA bump
}
```

### SovereignState

```rust
#[account]
pub struct SovereignState {
    pub sovereign_id: u64,              // Unique identifier
    pub creator: Pubkey,             // Sovereign creator
    pub token_mint: Pubkey,          // SPL token mint
    pub sovereign_type: SovereignType,  // TokenLaunch or BYOToken
    pub bond_target: u64,            // Required SOL (lamports)
    pub bond_deadline: i64,          // Unix timestamp
    pub total_deposited: u64,        // Investor deposits only (not creator)
    pub depositor_count: u32,        // Number of investors
    
    // Token supply tracking
    pub token_supply_deposited: u64, // Tokens deposited by creator (100% for TokenLaunch, >=30% for BYO)
    pub token_total_supply: u64,     // Total supply of token (for BYO verification)
    
    // Creation fee escrow (held during bonding, released/refunded on outcome)
    pub creation_fee_escrowed: u64,  // Amount in creation fee escrow PDA
    
    // Creator buy-in (escrowed for market buy, NOT LP)
    pub creator_escrow: u64,         // Creator's SOL held for market buy
    pub creator_max_buy_bps: u16,    // Max 100 (1% of total supply)
    
    // Sell Fee Configuration
    pub sell_fee_bps: u16,           // 0-300 (0-3%) creator-set sell fee
    pub swap_fee_bps: u16,           // LP swap fee rate
    
    // Liquidity is ALWAYS full range (MIN_TICK to MAX_TICK)
    // No tick configuration needed - ensures fair deep liquidity
    
    // State
    pub state: SovereignStatus,         // Bonding/Recovery/Active/Failed/Unwound
    
    // Recovery tracking (100% of LP fees go to investors)
    pub total_sol_fees_collected: u64,   // Total SOL fees from trading
    pub total_sol_fees_distributed: u64, // SOL distributed to investors
    pub total_token_fees_collected: u64, // Token fees (distributed to investors)
    pub recovery_target: u64,            // = total_deposited (investor SOL)
    pub recovery_complete: bool,         // true when sol_distributed >= recovery_target
    
    // Governance settings (from protocol config)
    pub auto_unwind_period: i64,         // 90-365 days (set by protocol, not creator)
    
    // Activity Check State (for auto-unwind in Active phase)
    pub activity_check_initiated: bool,       // Is a check in progress?
    pub activity_check_timestamp: i64,        // When check was initiated
    pub fee_growth_snapshot_a: u128,          // SOL fee_growth_global at snapshot
    pub fee_growth_snapshot_b: u128,          // Token fee_growth_global at snapshot
    pub activity_check_last_cancelled: i64,   // Timestamp of last cancelled check (for cooldown)
    
    // Derived accounts
    pub permanent_lock: Pubkey,
    pub whirlpool: Pubkey,           // Orca Whirlpool
    pub position_mint: Pubkey,       // Orca Position NFT mint
    
    // Unwind state
    pub unwind_sol_balance: u64,
    pub unwind_token_balance: u64,
    
    pub name: String,                // Sovereign name (max 32 chars)
    pub bump: u8,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Eq)]
pub enum SovereignStatus {
    Bonding,    // Accepting deposits
    Recovery,   // LP active, 100% fees to investors
    Active,     // Recovery complete, LP locked forever, fees still to investors
    Failed,     // Bond target not met, refunds available
    Unwound,    // Governance/low-volume unwind
}
```

### DepositRecord

```rust
#[account]
pub struct DepositRecord {
    pub sovereign: Pubkey,           // Sovereign this belongs to
    pub depositor: Pubkey,           // Investor wallet
    pub amount: u64,                 // SOL deposited (lamports)
    pub shares_bps: u16,             // Share of LP fees (basis points)
    pub genesis_nft: Pubkey,         // Minted NFT address
    
    // Claim tracking (investors only - creator has no fee share)
    pub sol_fees_claimed: u64,       // SOL fees claimed
    pub token_fees_claimed: u64,     // Token fees claimed
    pub unwind_claimed: bool,        // True if unwind proceeds claimed
    
    pub bump: u8,
}
```

Note: Only investors have DepositRecords. Creator does not get one.

### CreatorFeeTracker

```rust
#[account]
pub struct CreatorFeeTracker {
    pub sovereign: Pubkey,           // Associated sovereign
    pub creator: Pubkey,             // Creator wallet
    
    // Creator's purchased tokens (market bought after LP creation)
    pub purchased_tokens: u64,       // Tokens bought via market buy
    pub tokens_locked: bool,         // True during recovery phase
    pub purchased_tokens_claimed: bool, // True if purchased tokens claimed post-recovery
    
    // Creator has NO LP fee entitlement - 100% goes to investors
    // Only benefits: sell fee revenue (direct) + purchased tokens + LP tokens on unwind
    
    // Unwind tracking (creator gets LP tokens + purchased tokens back)
    pub unwind_claimed: bool,        // True if unwind tokens claimed
    
    pub bump: u8,
}
```

### PermanentLock

```rust
#[account]
pub struct PermanentLock {
    pub sovereign: Pubkey,              // Associated sovereign
    pub whirlpool: Pubkey,           // Orca Whirlpool
    pub position: Pubkey,            // Orca Position account
    pub tick_lower: i32,             // Always MIN_TICK (full range)
    pub tick_upper: i32,             // Always MAX_TICK (full range)
    pub liquidity: u128,             // Position liquidity
    pub unlocked: bool,              // True after unwind
    pub bump: u8,
}
```

**Note:** Liquidity is ALWAYS deployed as full range (MIN_TICK to MAX_TICK) to ensure:
- Deep liquidity at all price levels
- Fair and equal trading experience
- No concentration risk or manipulation

### Proposal

```rust
#[account]
pub struct Proposal {
    pub sovereign: Pubkey,
    pub proposal_id: u64,
    pub proposer: Pubkey,
    pub for_votes: u64,              // Votes in favor (weighted by shares)
    pub against_votes: u64,          // Votes against
    pub end_time: i64,               // Voting deadline
    pub timelock_end: i64,           // Execution timelock (0 if not set)
    pub executed: bool,
    pub bump: u8,
}

// Separate account per vote to track who voted
#[account]
pub struct VoteRecord {
    pub proposal: Pubkey,
    pub voter: Pubkey,               // Genesis NFT holder
    pub genesis_nft: Pubkey,         // NFT used to vote
    pub support: bool,
    pub weight: u64,                 // Share weight
    pub bump: u8,
}
```

---

## Sovereign Lifecycle

### State Machine

```
                    ┌─────────────────┐
                    │     Created     │
                    └────────┬────────┘
                             │ deposit()
                             ▼
┌─────────────┐     ┌─────────────────┐
│   Failed    │◄────│    Bonding      │
└─────────────┘     └────────┬────────┘
      ▲              deadline │ target met
      │              passed   │
      │              +not met ▼
      │             ┌─────────────────┐
      │             │   Finalizing    │
      │             └────────┬────────┘
      │                      │ finalize()
      │                      ▼
      │             ┌─────────────────┐
      │             │    Recovery     │◄────┐
      │             └────────┬────────┘     │
      │                      │              │ claim_fees()
      │           ┌──────────┼──────────┐   │
      │           │          │          │   │
      │  governance│   recovery│         │   │
      │     vote   │   complete│         │   │
      │           ▼          ▼          │   │
      │  ┌─────────────┐ ┌─────────────┐│   │
      │  │  Unwinding  │ │   Active    ││   │
      │  └──────┬──────┘ └──────┬──────┘│   │
      │         │               │       │   │
      │         │    low volume │       │   │
      │         │      unwind   │       │   │
      │         │◄──────────────┘       │   │
      │         │ claim_unwind()        │   │
      │         ▼                       │   │
      │  ┌─────────────┐                │   │
      └──│   Unwound   │────────────────┘   │
         └─────────────┘                    │
                                            │
```

### 1. Sovereign Creation

Supports both **Token Launcher** (new tokens) and **BYO Token** (existing tokens):

```rust
#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct CreateSovereignParams {
    pub sovereign_type: SovereignType,   // TokenLaunch or BYOToken
    pub bond_target: u64,                // SOL to raise
    pub bond_duration: i64,              // 7-30 days
    pub name: String,                    // Sovereign name
    
    // Token Launcher only
    pub token_name: Option<String>,      // Token name (for new token)
    pub token_symbol: Option<String>,    // Token symbol
    pub token_supply: Option<u64>,       // Total supply to mint
    pub sell_fee_bps: Option<u16>,       // Sell tax (0-300)
    pub fee_mode: Option<FeeMode>,       // Fee distribution mode
    
    // BYO Token only
    pub existing_mint: Option<Pubkey>,   // Existing token mint
    pub deposit_amount: Option<u64>,     // Tokens to deposit (>= 30% of supply)
}

pub fn create_sovereign(
    ctx: Context<CreateSovereign>,
    params: CreateSovereignParams,
) -> Result<()> {
    let protocol = &ctx.accounts.protocol_state;
    
    // Validate common parameters
    require!(params.bond_target >= protocol.min_bond_target, InvalidBondTarget);
    require!(params.bond_duration >= 7 days && params.bond_duration <= 30 days, InvalidDuration);
    
    // ESCROW CREATION FEE (0-10% of bond target)
    let creation_fee = params.bond_target * protocol.creation_fee_bps as u64 / 10000;
    require!(
        ctx.accounts.creator_payment.lamports() >= creation_fee,
        InsufficientCreationFee
    );
    transfer_sol_to_escrow(creation_fee, &ctx.accounts.creation_fee_escrow)?;
    
    // Initialize SovereignState PDA
    let sovereign = &mut ctx.accounts.sovereign;
    sovereign.sovereign_type = params.sovereign_type.clone();
    sovereign.creation_fee_escrowed = creation_fee;
    
    match params.sovereign_type {
        SovereignType::TokenLaunch => {
            // TOKEN LAUNCHER: Create new Token-2022 with transfer hooks
            require!(params.token_name.is_some(), MissingTokenName);
            require!(params.token_symbol.is_some(), MissingTokenSymbol);
            require!(params.token_supply.is_some(), MissingTokenSupply);
            require!(params.sell_fee_bps.unwrap_or(0) <= 300, SellFeeTooHigh);
            
            // CPI: Create Token-2022 mint with transfer hook
            let token_supply = params.token_supply.unwrap();
            // CPI: Mint 100% supply to sovereign token vault
            
            sovereign.token_mint = /* new mint */;
            sovereign.token_supply_deposited = token_supply;
            sovereign.token_total_supply = token_supply;
            sovereign.sell_fee_bps = params.sell_fee_bps.unwrap_or(0);
        }
        
        SovereignType::BYOToken => {
            // BYO TOKEN: Verify and transfer existing token
            require!(params.existing_mint.is_some(), MissingExistingMint);
            require!(params.deposit_amount.is_some(), MissingDepositAmount);
            
            let mint = params.existing_mint.unwrap();
            let deposit_amount = params.deposit_amount.unwrap();
            
            // Verify token supply and calculate percentage
            let mint_account = ctx.accounts.existing_mint.to_account_info();
            let total_supply = get_token_supply(&mint_account)?;
            let deposit_bps = (deposit_amount * 10000 / total_supply) as u16;
            
            // Verify minimum supply requirement (default 30%)
            require!(
                deposit_bps >= protocol.byo_min_supply_bps,
                InsufficientTokenDeposit
            );
            
            // Transfer tokens from creator to sovereign vault
            transfer_tokens_from_creator(deposit_amount, &ctx)?;
            
            sovereign.token_mint = mint;
            sovereign.token_supply_deposited = deposit_amount;
            sovereign.token_total_supply = total_supply;
            sovereign.sell_fee_bps = 0; // No sell tax for BYO (unless token has hooks)
        }
    }
    
    emit!(SovereignCreated { 
        sovereign_id: sovereign.sovereign_id,
        creator: ctx.accounts.creator.key(),
        token_mint: sovereign.token_mint,
        sovereign_type: params.sovereign_type,
        bond_target: params.bond_target,
        token_supply_deposited: sovereign.token_supply_deposited,
        creation_fee_escrowed: creation_fee,
    });
    
    Ok(())
}
```

**BYO Token Verification:**
```
┌─────────────────────────────────────────────────────────────────┐
│                    BYO TOKEN VERIFICATION                        │
│                                                                  │
│   1. Read token mint account                                    │
│   2. Get total supply from mint                                 │
│   3. Calculate deposit_amount / total_supply                    │
│   4. Verify >= protocol.byo_min_supply_bps (default 30%)       │
│   5. Transfer tokens from creator to sovereign vault            │
│                                                                  │
│   Example:                                                       │
│   - Total Supply: 1,000,000 tokens                              │
│   - Deposit Amount: 350,000 tokens                              │
│   - Percentage: 35% (3500 bps) ✓ >= 30%                        │
│                                                                  │
│   NOTE: Creator retains remaining tokens (65% in example)       │
│   Only deposited tokens go to LP                                │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### 2. Bonding Phase

```rust
pub fn deposit(ctx: Context<Deposit>, amount: u64) -> Result<()> {
    let sovereign = &mut ctx.accounts.sovereign;
    
    require!(sovereign.state == SovereignStatus::Bonding, InvalidState);
    require!(Clock::get()?.unix_timestamp <= sovereign.bond_deadline, DeadlinePassed);
    
    let is_creator = ctx.accounts.depositor.key() == sovereign.creator;
    
    if is_creator {
        // CREATOR DEPOSIT: Goes to escrow for market buy (NOT LP)
        let max_creator_buy = sovereign.bond_target * sovereign.creator_max_buy_bps as u64 / 10000;
        require!(
            sovereign.creator_escrow + amount <= max_creator_buy,
            CreatorDepositExceedsMax
        );
        sovereign.creator_escrow += amount;
        // Transfer to escrow vault, NOT LP funds
        // Creator does NOT get a deposit record or Genesis NFT
        
        emit!(CreatorEscrowed { 
            sovereign: sovereign.key(),
            creator: ctx.accounts.depositor.key(),
            amount: amount
        });
    } else {
        // INVESTOR DEPOSIT: Goes to LP
        require!(sovereign.total_deposited < sovereign.bond_target, BondingComplete);
        require!(amount >= MIN_DEPOSIT_LAMPORTS, DepositTooSmall);
        
        // Cap deposit if would exceed target
        let actual_amount = std::cmp::min(
            amount,
            sovereign.bond_target - sovereign.total_deposited
        );
        
        // Refund excess
        if amount > actual_amount {
            // Transfer back excess SOL
        }
        
        // Create or update DepositRecord PDA (investors only)
        let deposit_record = &mut ctx.accounts.deposit_record;
        deposit_record.amount += actual_amount;
        
        // Update sovereign totals
        sovereign.total_deposited += actual_amount;
        sovereign.recovery_target += actual_amount;  // Same as total_deposited (investors only)
        sovereign.depositor_count += 1;
        
        emit!(Deposited { 
            depositor: ctx.accounts.depositor.key(),
            amount: actual_amount, 
            total: sovereign.total_deposited
        });
    }
}
```

### 3. Finalization

```rust
pub fn finalize(ctx: Context<Finalize>) -> Result<()> {
    let sovereign = &mut ctx.accounts.sovereign;
    let protocol = &ctx.accounts.protocol_state;
    
    require!(sovereign.state == SovereignStatus::Bonding, InvalidState);
    require!(sovereign.total_deposited >= sovereign.bond_target, BondingNotComplete);
    
    // TRANSFER ESCROWED CREATION FEE TO TREASURY (bonding succeeded)
    if sovereign.creation_fee_escrowed > 0 {
        transfer_sol_from_escrow_to_treasury(
            sovereign.creation_fee_escrowed,
            &ctx.accounts.creation_fee_escrow,
            &ctx.accounts.treasury
        )?;
        sovereign.creation_fee_escrowed = 0;
    }
    
    // STEP 1: Create LP with INVESTOR SOL + CREATOR TOKENS
    // All investor SOL goes to LP (no deductions)
    let sol_for_lp = sovereign.total_deposited;  // 100% of investor SOL
    let tokens_for_lp = ctx.accounts.token_vault.amount;  // 100% of creator's tokens
    
    // CPI: Create Orca Whirlpool FULL RANGE position
    // Full range = tick_lower: MIN_TICK, tick_upper: MAX_TICK
    // This ensures deep, fair, equal liquidity across all prices
    let tick_lower = MIN_TICK; // -443636 for most pools
    let tick_upper = MAX_TICK; // +443636 for most pools
    
    // CPI: open_position with full range ticks
    // CPI: increase_liquidity with SOL + Tokens
    // Store position NFT in PermanentLock
    // Pool is created with restricted = true (only Genesis position can LP)
    
    // STEP 2: Mint Genesis NFTs to INVESTORS ONLY (not creator)
    for deposit_record in deposit_records {
        // Calculate shares: deposit_amount / total_deposited (investors only)
        let shares_bps = (deposit_record.amount * 10000 / sovereign.total_deposited) as u16;
        deposit_record.shares_bps = shares_bps;
        
        // CPI: Metaplex create NFT for investor
    }
    
    // STEP 3: ATOMIC market buy for creator using escrowed SOL
    // This happens in the SAME TRANSACTION as LP creation to prevent:
    // - Slippage attacks (LP has full liquidity when buy executes)
    // - MEV sandwich attacks (atomic = no opportunity)
    if sovereign.creator_escrow > 0 {
        // Calculate minimum tokens out based on theoretical price + slippage tolerance
        let theoretical_price = tokens_for_lp / sol_for_lp;
        let expected_tokens = sovereign.creator_escrow * theoretical_price;
        let min_tokens_out = expected_tokens * 99 / 100; // 1% slippage tolerance
        
        // CPI: Swap creator's escrowed SOL for tokens via Whirlpool
        let tokens_bought = swap_sol_for_tokens_with_slippage(
            sovereign.creator_escrow,
            min_tokens_out,
            &ctx.accounts.whirlpool
        )?;
        
        require!(tokens_bought >= min_tokens_out, SlippageExceeded);
        
        // Lock purchased tokens in CreatorFeeTracker
        let tracker = &mut ctx.accounts.creator_fee_tracker;
        tracker.purchased_tokens = tokens_bought;
        tracker.tokens_locked = true;
        
        emit!(CreatorTokensPurchased {
            creator: sovereign.creator,
            sol_spent: sovereign.creator_escrow,
            tokens_received: tokens_bought,
        });
    }
    
    // Enter RECOVERY phase (not Active)
    sovereign.state = SovereignStatus::Recovery;
    sovereign.recovery_complete = false;
    sovereign.last_activity_timestamp = Clock::get()?.unix_timestamp;
    
    emit!(SovereignFinalized { 
        pool_id: sovereign.whirlpool, 
        liquidity,
        recovery_target: sovereign.recovery_target  // Investor deposits only
    });
}
```

### 4. Recovery Phase Operations

During recovery, investors can claim fees and vote on unwind:

```rust
pub fn claim_fees(ctx: Context<ClaimFees>) -> Result<()> {
    // See Fee Distribution section for full implementation
    // During recovery: Only investors claim, only SOL counts
    // After recovery: All depositors claim proportionally
}
```

### 5. Failed Sovereign Withdrawal

```rust
pub fn mark_failed(ctx: Context<MarkFailed>) -> Result<()> {
    let sovereign = &mut ctx.accounts.sovereign;
    let protocol = &ctx.accounts.protocol_state;
    
    require!(sovereign.state == SovereignStatus::Bonding, InvalidState);
    require!(Clock::get()?.unix_timestamp > sovereign.bond_deadline, DeadlineNotPassed);
    require!(sovereign.total_deposited < sovereign.bond_target, BondingComplete);
    
    // REFUND CREATION FEE (minus minimum fee)
    // Creation fee was escrowed during create_sovereign - now refund most of it
    let creation_fee_refund = if sovereign.creation_fee_escrowed > protocol.min_fee_lamports {
        // Refund creation fee minus min fee
        let refund = sovereign.creation_fee_escrowed - protocol.min_fee_lamports;
        // Min fee goes to treasury
        transfer_sol_to_treasury(protocol.min_fee_lamports, &ctx.accounts.treasury)?;
        // Refund remainder to creator
        transfer_sol_to_creator(refund, &ctx.accounts.creator)?;
        refund
    } else {
        // Creation fee is less than or equal to min fee - all goes to treasury
        transfer_sol_to_treasury(sovereign.creation_fee_escrowed, &ctx.accounts.treasury)?;
        0
    };
    sovereign.creation_fee_escrowed = 0;
    
    sovereign.state = SovereignStatus::Failed;
    emit!(SovereignFailed { 
        sovereign_id: sovereign.sovereign_id,
        min_fee_retained: protocol.min_fee_lamports,
        creation_fee_refunded: creation_fee_refund
    });
}

// INVESTOR WITHDRAWAL - uses DepositRecord (only investors have these)
pub fn withdraw(ctx: Context<Withdraw>) -> Result<()> {
    let sovereign = &ctx.accounts.sovereign;
    let deposit = &mut ctx.accounts.deposit_record;
    
    require!(sovereign.state == SovereignStatus::Failed, InvalidState);
    require!(deposit.amount > 0, NoDeposit);
    
    let amount = deposit.amount;
    deposit.amount = 0;
    
    // Transfer SOL back to investor (full refund)
    // Note: Only investors have DepositRecords - creator uses different function
    
    emit!(Withdrawn { depositor: ctx.accounts.depositor.key(), amount });
}

// CREATOR WITHDRAWAL - gets tokens and escrowed buy-in SOL back
// Note: Creation fee refund already happened in mark_failed()
pub fn withdraw_creator_failed(ctx: Context<WithdrawCreatorFailed>) -> Result<()> {
    let sovereign = &ctx.accounts.sovereign;
    
    require!(sovereign.state == SovereignStatus::Failed, InvalidState);
    require!(ctx.accounts.creator.key() == sovereign.creator, NotCreator);
    
    // Return creator's buy-in escrow (for market buy that never happened)
    // Note: Creation fee already refunded minus min fee in mark_failed()
    let escrowed_sol = sovereign.creator_escrow;
    
    // Return creator's 100% tokens (never went to LP since bonding failed)
    let token_amount = /* full token supply */;
    
    sovereign.creator_escrow = 0;
    
    emit!(CreatorFailedWithdrawal { 
        creator: sovereign.creator,
        sol_returned: escrowed_sol,
        tokens_returned: token_amount
    });
}
```

---

## Creator Token Unlocks

### Recovery-Gated Token Access

The Creator has **NO LP fee share** - 100% of fees go to investors forever. However, the creator's **purchased tokens** (from optional SOL deposit → market buy) are locked during recovery:

```
┌─────────────────────────────────────────────────────────────────┐
│                  Creator Token Access Logic                      │
│                                                                  │
│  DURING RECOVERY PHASE:                                         │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │  ALL LP fees (SOL + Token) → Investors (100%)            │   │
│  │  Creator has NO LP fee share at any time                 │   │
│  │  Creator's PURCHASED tokens remain LOCKED                │   │
│  │  Creator benefits only from sell fee revenue (direct)    │   │
│  └─────────────────────────────────────────────────────────┘   │
│                                                                  │
│  AFTER RECOVERY (SOL fees >= investor deposits):                │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │  LP is PERMANENTLY LOCKED                               │   │
│  │  Governance/unwind no longer possible                   │   │
│  │  Creator can claim PURCHASED TOKENS (finally unlocked!) │   │
│  │  LP fees STILL 100% to investors                        │   │
│  └─────────────────────────────────────────────────────────┘   │
│                                                                  │
│  UNWIND (Governance vote or low volume):                        │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │  LP removed from pool                                   │   │
│  │  Investors receive: SOL from LP                         │   │
│  │  Creator receives: Tokens from LP + purchased tokens    │   │
│  │  Clean separation - each party gets their asset back    │   │
│  └─────────────────────────────────────────────────────────┘   │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### Implementation

```rust
pub fn claim_creator_purchased_tokens(ctx: Context<ClaimCreatorPurchasedTokens>) -> Result<()> {
    let tracker = &mut ctx.accounts.creator_fee_tracker;
    let sovereign = &ctx.accounts.sovereign;
    
    // Creator can claim purchased tokens AFTER recovery OR on unwind
    require!(
        sovereign.recovery_complete || sovereign.state == SovereignStatus::Unwound,
        TokensStillLocked
    );
    require!(ctx.accounts.creator.key() == tracker.creator, OnlyCreator);
    require!(!tracker.purchased_tokens_claimed, AlreadyClaimed);
    require!(tracker.purchased_tokens > 0, NothingToClaim);
    
    let tokens_to_transfer = tracker.purchased_tokens;
    tracker.purchased_tokens_claimed = true;
    tracker.tokens_locked = false;
    
    // Transfer purchased tokens to creator
    transfer_tokens(tokens_to_transfer, &ctx.accounts.creator)?;
    
    emit!(CreatorPurchasedTokensClaimed { 
        creator: tracker.creator, 
        amount: tokens_to_transfer
    });
    
    Ok(())
}

// Note: claim_creator_unwind (for LP tokens + purchased tokens) is in Governance section
```

---

## Fee Distribution

### Recovery Phase (ALL Fees → Investors)

During recovery, **ALL** trading fees go to investors to help them recover their principal:

```
┌─────────────────────────────────────────────────────────────────┐
│                   RECOVERY PHASE FEE FLOW                        │
│                                                                  │
│     Trading Activity                                             │
│           │                                                      │
│           ▼                                                      │
│   ┌───────────────┐     ┌───────────────┐                       │
│   │  Sell Fee     │     │  Swap Fee     │                       │
│   │   (0-3%)      │     │  (0.25%)      │                       │
│   └───────┬───────┘     └───────┬───────┘                       │
│           │                     │                                │
│           └──────────┬──────────┘                                │
│                      │                                           │
│          ┌───────────┴───────────┐                               │
│          ▼                       ▼                               │
│   ┌─────────────┐         ┌─────────────┐                       │
│   │  SOL Fees   │         │ Token Fees  │                       │
│   └──────┬──────┘         └──────┬──────┘                       │
│          │                       │                               │
│          ▼                       ▼                               │
│   ┌──────────────────┐    ┌──────────────────┐                  │
│   │ 100% → INVESTORS │    │ 100% → INVESTORS │                  │
│   │ (counts toward   │    │ (claimable but   │                  │
│   │  recovery)       │    │  NOT counted)    │                  │
│   └──────────────────┘    └──────────────────┘                  │
│                                                                  │
│   Recovery Progress = SOL fees distributed / Total investor SOL  │
│   Recovery Complete when SOL distributed >= investor deposits    │
│                                                                  │
│   NOTE: Creator has NO LP share - 100% of fees go to investors  │
│   NOTE: Creator's purchased tokens are LOCKED during recovery   │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### Post-Recovery Phase (Permanent Lock - Fees Still to Investors)

After recovery is complete, LP is **permanently locked** and fees continue to go to investors:

```
┌─────────────────────────────────────────────────────────────────┐
│                  POST-RECOVERY FEE FLOW                          │
│                                                                  │
│     Trading Activity                                             │
│           │                                                      │
│           ▼                                                      │
│   ┌───────────────────────────────────────┐                     │
│   │      All LP Trading Fees              │                     │
│   └───────────────────┬───────────────────┘                     │
│                       │                                          │
│       ┌───────────────┴───────────────┐                          │
│       ▼                               ▼                          │
│ ┌───────────┐                   ┌───────────┐                   │
│ │ SOL Fees  │                   │Token Fees │                   │
│ └─────┬─────┘                   └─────┬─────┘                   │
│       │                               │                          │
│       └───────────────┬───────────────┘                          │
│                       │                                          │
│                       ▼                                          │
│   ┌─────────────────────────────────────────┐                   │
│   │      100% TO INVESTORS (ALWAYS)         │                   │
│   │                                         │                   │
│   │  Creator: 0% (NO LP fee entitlement)    │                   │
│   │  Investors: 100% (based on NFT shares)  │                   │
│   │                                         │                   │
│   │  Creator's ONLY benefit post-recovery:  │                   │
│   │  • Sell tax revenue (Token Launcher only)│                   │
│   │  • Purchased tokens (now unlocked)      │                   │
│   │                                         │                   │
│   │  Pool is now UNLOCKED for external LPs  │                   │
│   └─────────────────────────────────────────┘                   │
│                                                                  │
│   LP is PERMANENTLY LOCKED - no unwind possible after recovery  │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### Fee Claim Implementation

```rust
pub fn claim_fees(ctx: Context<ClaimFees>) -> Result<()> {
    let sovereign = &mut ctx.accounts.sovereign;
    let deposit = &mut ctx.accounts.deposit_record;
    
    // Only investors have DepositRecords (creator does not)
    // Verify Genesis NFT ownership
    require!(
        verify_nft_owner(&ctx.accounts.genesis_nft, &ctx.accounts.claimer)?,
        NotNFTOwner
    );
    
    // CPI: Collect fees from Orca position
    let (sol_collected, tokens_collected) = collect_whirlpool_fees(&ctx)?;
    
    // Investors get 100% of fees in ALL phases (Recovery AND Active)
    let sol_share = sol_collected * deposit.shares_bps as u64 / 10000;
    let token_share = tokens_collected * deposit.shares_bps as u64 / 10000;
    
    // Transfer both SOL and token fees
    transfer_sol(sol_share, &ctx.accounts.claimer)?;
    transfer_tokens(token_share, &ctx.accounts.claimer)?;
    
    deposit.sol_fees_claimed += sol_share;
    deposit.token_fees_claimed += token_share;
    
    // Track recovery progress (SOL only)
    if !sovereign.recovery_complete {
        sovereign.total_sol_fees_distributed += sol_share;
        
        // Check if recovery is complete
        if sovereign.total_sol_fees_distributed >= sovereign.recovery_target {
            sovereign.recovery_complete = true;
            sovereign.state = SovereignStatus::Active;
            
            // UNLOCK POOL for external LPs
            // CPI: Call unlock_pool() on forked Whirlpool
            unlock_whirlpool(&ctx.accounts.whirlpool, &ctx.accounts.permanent_lock)?;
            
            emit!(RecoveryComplete { 
                sovereign_id: sovereign.sovereign_id,
                total_recovered: sovereign.total_sol_fees_distributed 
            });
            
            emit!(PoolUnlocked {
                sovereign: sovereign.key(),
                whirlpool: sovereign.whirlpool,
                timestamp: Clock::get()?.unix_timestamp,
            });
        }
    }
    
    sovereign.last_activity_timestamp = Clock::get()?.unix_timestamp;
    
    emit!(FeesClaimed { 
        depositor: ctx.accounts.claimer.key(),
        sol_amount: sol_share,
        token_amount: token_share
    });
    
    Ok(())
}
```
```

---

## Governance & Unwind

### Overview

Governance allows investors to vote to unwind the LP and recover their investment during the recovery phase. **Key restrictions:**

1. **Recovery Phase Only** - Unwind is ONLY possible during recovery phase
2. **Creator Cannot Vote** - Only investors can participate in governance
3. **Permanent After Recovery** - Once recovery completes, LP is locked forever

### Auto-Unwind (Low Volume / Failed Project)

If the project fails to generate sufficient trading volume **after recovery**, an automatic unwind is triggered. The inactivity threshold is **set by the protocol** (90-365 days):

```rust
pub fn trigger_auto_unwind(ctx: Context<TriggerAutoUnwind>) -> Result<()> {
    let sovereign = &mut ctx.accounts.sovereign;
    
    // Only in ACTIVE phase (post-recovery)
    // During recovery, use governance vote instead
    require!(sovereign.state == SovereignStatus::Active, InvalidState);
    
    let now = Clock::get()?.unix_timestamp;
    let inactive_duration = now - sovereign.last_activity_timestamp;
    
    // Use protocol-configured auto_unwind_period (90-365 days)
    require!(inactive_duration >= sovereign.auto_unwind_period, NotInactiveEnough);
    
    // Execute unwind: Investors get SOL, Creator gets tokens
    execute_unwind_internal(ctx)?;
    
    emit!(AutoUnwindTriggered { 
        sovereign_id: sovereign.sovereign_id,
        inactive_days: inactive_duration / 86400 
    });
}
```

### Proposal Creation

```rust
pub fn propose_unwind(ctx: Context<ProposeUnwind>) -> Result<()> {
    let sovereign = &ctx.accounts.sovereign;
    let deposit = &ctx.accounts.proposer_deposit;
    let protocol = &ctx.accounts.protocol_state;
    
    // Only during recovery phase
    require!(sovereign.state == SovereignStatus::Recovery, CannotUnwindAfterRecovery);
    
    // Only investors have DepositRecords (creator does not, no Genesis NFT either)
    // Creator cannot propose since they have no DepositRecord
    
    require!(can_propose(sovereign)?, CannotPropose);
    
    // CHARGE GOVERNANCE UNWIND FEE (0.05 SOL, protocol-adjustable)
    require!(
        ctx.accounts.proposer_payment.lamports() >= protocol.governance_unwind_fee_lamports,
        InsufficientProposalFee
    );
    transfer_sol_to_treasury(protocol.governance_unwind_fee_lamports, &ctx.accounts.treasury)?;
    
    // Initialize Proposal PDA
    let proposal = &mut ctx.accounts.proposal;
    proposal.sovereign = sovereign.key();
    proposal.proposal_id = /* next id */;
    proposal.proposer = ctx.accounts.proposer.key();
    proposal.end_time = Clock::get()?.unix_timestamp + VOTING_PERIOD;
    
    emit!(ProposalCreated { 
        proposal_id: proposal.proposal_id,
        governance_fee_paid: protocol.governance_unwind_fee_lamports
    });
}

fn can_propose(sovereign: &SovereignState) -> Result<bool> {
    let now = Clock::get()?.unix_timestamp;
    // 90 days of low activity allows proposal
    Ok(now > sovereign.last_activity_timestamp + INACTIVITY_PERIOD)
}
```

### Voting

```rust
pub fn vote(
    ctx: Context<Vote>,
    support: bool,
) -> Result<()> {
    let proposal = &mut ctx.accounts.proposal;
    let deposit = &ctx.accounts.deposit_record;
    let sovereign = &ctx.accounts.sovereign;
    
    require!(Clock::get()?.unix_timestamp <= proposal.end_time, VotingEnded);
    
    // Only investors can vote (creator has no DepositRecord or Genesis NFT)
    // Verify NFT ownership
    // Create VoteRecord to prevent double voting
    
    let weight = deposit.shares_bps as u64;
    
    if support {
        proposal.for_votes += weight;
    } else {
        proposal.against_votes += weight;
    }
    
    emit!(Voted { proposal_id: proposal.proposal_id, voter, support, weight });
}
```

### Execution

```rust
pub fn execute_unwind(ctx: Context<ExecuteUnwind>) -> Result<()> {
    let proposal = &mut ctx.accounts.proposal;
    let sovereign = &mut ctx.accounts.sovereign;
    
    // Can only unwind via governance during RECOVERY phase
    // Post-recovery uses auto-unwind for low volume instead
    require!(sovereign.state == SovereignStatus::Recovery, CannotUnwindAfterRecovery);
    
    require!(!proposal.executed, AlreadyExecuted);
    require!(Clock::get()?.unix_timestamp > proposal.end_time, VotingNotEnded);
    
    // Check quorum (67%)
    let total_votes = proposal.for_votes + proposal.against_votes;
    require!(total_votes >= QUORUM_BPS * 100, NotEnoughVotes);
    
    // Check pass threshold (51%)
    require!(proposal.for_votes * 10000 / total_votes >= PASS_THRESHOLD_BPS, ProposalNotPassed);
    
    // Set timelock on first call
    if proposal.timelock_end == 0 {
        proposal.timelock_end = Clock::get()?.unix_timestamp + TIMELOCK_PERIOD;
        return Ok(());
    }
    
    require!(Clock::get()?.unix_timestamp >= proposal.timelock_end, TimelockNotPassed);
    
    // Execute unwind: Investors get SOL, Creator gets tokens
    proposal.executed = true;
    sovereign.state = SovereignStatus::Unwound;
    
    // CPI: Decrease liquidity from Whirlpool (decrease_liquidity)
    // CPI: Close position (close_position)
    
    // Store assets for claiming
    // SOL goes to investors proportionally
    // Tokens go back to creator
    sovereign.unwind_sol_balance = /* SOL from LP */;
    sovereign.unwind_token_balance = /* Tokens from LP */;
    
    // Deduct unwind fee from SOL (protocol fee)
    let fee = sovereign.unwind_sol_balance * UNWIND_FEE_BPS / 10000;
    sovereign.unwind_sol_balance -= fee;
    
    emit!(UnwindExecuted { proposal_id: proposal.proposal_id });
}

pub fn claim_investor_unwind(ctx: Context<ClaimInvestorUnwind>) -> Result<()> {
    let sovereign = &ctx.accounts.sovereign;
    let deposit = &mut ctx.accounts.deposit_record;
    
    require!(sovereign.state == SovereignStatus::Unwound, InvalidState);
    require!(!deposit.unwind_claimed, AlreadyClaimed);
    
    // Only investors have DepositRecords
    // Investors receive SOL ONLY (proportional to their shares)
    // Tokens go to creator via claim_creator_unwind()
    let sol_share = sovereign.unwind_sol_balance * deposit.shares_bps as u64 / 10000;
    
    deposit.unwind_claimed = true;
    
    // Transfer SOL to investor
    // Burn Genesis NFT
    
    emit!(InvestorUnwindClaimed { 
        investor: ctx.accounts.claimer.key(),
        sol_amount: sol_share 
    });
}
}

pub fn claim_creator_unwind(ctx: Context<ClaimCreatorUnwind>) -> Result<()> {
    let sovereign = &ctx.accounts.sovereign;
    let tracker = &mut ctx.accounts.creator_fee_tracker;
    
    require!(sovereign.state == SovereignStatus::Unwound, InvalidState);
    require!(!tracker.tokens_claimed, AlreadyClaimed);
    require!(sovereign.creator == ctx.accounts.claimer.key(), NotCreator);
    
    // Creator receives:
    // 1. ALL tokens from LP (their 100% contribution)
    // 2. Their purchased tokens (from market buy) - ONLY IF NOT ALREADY CLAIMED
    let lp_tokens = sovereign.unwind_token_balance;
    
    // DOUBLE CLAIM PROTECTION: Only include purchased tokens if not already claimed
    // via claim_creator_purchased_tokens()
    let purchased_tokens = if tracker.purchased_tokens_claimed {
        0 // Already claimed separately, don't double-count
    } else {
        tracker.purchased_tokens
    };
    let total_tokens = lp_tokens + purchased_tokens;
    
    tracker.tokens_claimed = true;
    tracker.purchased_tokens_claimed = true; // Mark as claimed either way
    tracker.tokens_locked = false;
    
    // Transfer all tokens to creator
    
    emit!(CreatorUnwindClaimed { 
        creator: tracker.creator,
        lp_tokens: lp_tokens,
        purchased_tokens: purchased_tokens,
        total_tokens: total_tokens
    });
}
```

### Activity Check (Auto-Unwind for Active Phase)

During the **Active phase**, governance cannot trigger unwind. Instead, a permissionless two-call
activity check mechanism detects prolonged inactivity and triggers auto-unwind.

**Design: Cumulative Fee Growth Snapshots**

Orca Whirlpools track `fee_growth_global_a` and `fee_growth_global_b` at the pool level - these
are cumulative values that never decrease. This allows us to measure actual trading activity
without requiring frequent checks.

```
┌─────────────────────────────────────────────────────────────────┐
│                    ACTIVITY CHECK LIFECYCLE                      │
│                                                                  │
│   STATE: No active check                                         │
│   ┌─────────────────────────────────────────────────────────┐   │
│   │  Anyone can call: initiate_activity_check()              │   │
│   │  • Permissionless                                        │   │
│   │  • Only valid in Active phase                            │   │
│   └─────────────────────────────────────────────────────────┘   │
│                            │                                     │
│                            ▼                                     │
│   STATE: Check in progress (activity_check_initiated = true)     │
│   ┌─────────────────────────────────────────────────────────┐   │
│   │  • Snapshot of fee_growth_global locked in               │   │
│   │  • Timestamp recorded                                    │   │
│   │  • CANNOT call initiate again                            │   │
│   │  • CANNOT call execute yet (< 90 days)                   │   │
│   └─────────────────────────────────────────────────────────┘   │
│                            │                                     │
│                     WAIT 90+ DAYS                                │
│                    (enforced on-chain)                           │
│                            │                                     │
│                            ▼                                     │
│   STATE: Check executable                                        │
│   ┌─────────────────────────────────────────────────────────┐   │
│   │  Anyone can call: execute_activity_check()               │   │
│   └─────────────────────────────────────────────────────────┘   │
│                            │                                     │
│              ┌─────────────┴─────────────┐                      │
│              ▼                           ▼                       │
│   ┌──────────────────┐        ┌──────────────────┐              │
│   │ Growth < threshold│        │ Growth ≥ threshold│             │
│   │                  │        │                  │              │
│   │ → UNWIND ✅      │        │ → CANCELLED ❌   │              │
│   │ → Investors claim│        │ → Pool is active │              │
│   └──────────────────┘        └────────┬─────────┘              │
│                                        │                         │
│                                        ▼                         │
│                               ┌──────────────────┐              │
│                               │ RESET STATE      │              │
│                               │                  │              │
│                               │ • initiated=false│              │
│                               │ • snapshot=0     │              │
│                               │ • timestamp=0    │              │
│                               │                  │              │
│                               │ Ready for new    │              │
│                               │ initiate call    │              │
│                               └──────────────────┘              │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

```rust
/// STEP 1: Initiate activity check (permissionless)
/// Takes a snapshot of current cumulative fee growth
pub fn initiate_activity_check(ctx: Context<InitiateActivityCheck>) -> Result<()> {
    let sovereign = &mut ctx.accounts.sovereign;
    let whirlpool = &ctx.accounts.whirlpool;
    let protocol = &ctx.accounts.protocol_state;
    
    // Only valid in Active phase
    require!(sovereign.state == SovereignStatus::Active, OnlyActivePhase);
    
    // Cannot initiate if check already in progress
    require!(!sovereign.activity_check_initiated, ActivityCheckAlreadyInProgress);
    
    // Enforce cooldown after cancelled check (7 days)
    let now = Clock::get()?.unix_timestamp;
    if sovereign.activity_check_last_cancelled > 0 {
        let cooldown_end = sovereign.activity_check_last_cancelled + ACTIVITY_CHECK_COOLDOWN;
        require!(now >= cooldown_end, ActivityCheckCooldownNotExpired);
    }
    
    // Take snapshot of cumulative fee growth (never decreases)
    sovereign.fee_growth_snapshot_a = whirlpool.fee_growth_global_a;
    sovereign.fee_growth_snapshot_b = whirlpool.fee_growth_global_b;
    sovereign.activity_check_timestamp = now;
    sovereign.activity_check_initiated = true;
    
    emit!(ActivityCheckInitiated {
        sovereign: sovereign.key(),
        snapshot_a: sovereign.fee_growth_snapshot_a,
        snapshot_b: sovereign.fee_growth_snapshot_b,
        timestamp: sovereign.activity_check_timestamp,
    });
    
    Ok(())
}

/// STEP 2: Execute activity check after 90+ days (permissionless)
/// Compares fee growth to threshold and triggers unwind if inactive
pub fn execute_activity_check(ctx: Context<ExecuteActivityCheck>) -> Result<()> {
    let sovereign = &mut ctx.accounts.sovereign;
    let whirlpool = &ctx.accounts.whirlpool;
    let protocol = &ctx.accounts.protocol_state;
    
    // Must have an active check in progress
    require!(sovereign.activity_check_initiated, NoActivityCheckInProgress);
    require!(sovereign.state == SovereignStatus::Active, OnlyActivePhase);
    
    // ENFORCE MINIMUM WAIT PERIOD (90+ days)
    let now = Clock::get()?.unix_timestamp;
    let elapsed = now - sovereign.activity_check_timestamp;
    require!(elapsed >= protocol.auto_unwind_period, ActivityCheckTooEarly);
    
    // Calculate fee growth since snapshot
    let growth_a = whirlpool.fee_growth_global_a
        .checked_sub(sovereign.fee_growth_snapshot_a)
        .unwrap_or(0);
    let growth_b = whirlpool.fee_growth_global_b
        .checked_sub(sovereign.fee_growth_snapshot_b)
        .unwrap_or(0);
    
    // Check if activity meets threshold
    let threshold = protocol.min_fee_growth_threshold;
    let is_active = growth_a >= threshold || growth_b >= threshold;
    
    if !is_active {
        // INSUFFICIENT ACTIVITY → TRIGGER AUTO-UNWIND
        sovereign.state = SovereignStatus::Unwound;
        
        // CPI: Remove liquidity from Whirlpool
        // Store SOL and tokens for claiming
        // Deduct unwind fee
        
        emit!(AutoUnwindTriggered {
            sovereign: sovereign.key(),
            fee_growth_a: growth_a,
            fee_growth_b: growth_b,
            threshold: threshold,
            elapsed_seconds: elapsed as u64,
        });
    } else {
        // SUFFICIENT ACTIVITY → CANCEL CHECK
        // Record cancellation time for cooldown
        sovereign.activity_check_last_cancelled = now;
        
        emit!(ActivityCheckCancelled {
            sovereign: sovereign.key(),
            fee_growth_a: growth_a,
            fee_growth_b: growth_b,
            threshold: threshold,
        });
    }
    
    // ALWAYS RESET after execute (regardless of outcome)
    sovereign.activity_check_initiated = false;
    sovereign.fee_growth_snapshot_a = 0;
    sovereign.fee_growth_snapshot_b = 0;
    sovereign.activity_check_timestamp = 0;
    
    Ok(())
}
```

**Key Properties:**

| Property | Behavior |
|----------|----------|
| **Minimum wait** | Hard 90-day minimum enforced on-chain |
| **No early execute** | Transaction reverts if < 90 days |
| **Always resets** | Whether unwind or cancel, state clears for new check |
| **Cooldown after cancel** | 7-day wait before new initiation after cancelled check |
| **Permissionless** | Anyone can initiate, anyone can execute |
| **One at a time** | Cannot have multiple overlapping checks |
| **Fresh measurement** | Each check measures a clean 90-day window |
| **Cumulative growth** | Uses pool's fee_growth_global (never decreases) |

**Threshold Control:**

The protocol can adjust the minimum fee growth threshold, with the option to renounce:

```rust
pub fn update_fee_threshold(
    ctx: Context<UpdateProtocol>,
    new_threshold: u128,
) -> Result<()> {
    let protocol = &mut ctx.accounts.protocol_state;
    
    require!(!protocol.fee_threshold_renounced, FeeThresholdRenounced);
    require!(ctx.accounts.authority.key() == protocol.authority, NotProtocolAuthority);
    
    emit!(FeeThresholdUpdated {
        old_threshold: protocol.min_fee_growth_threshold,
        new_threshold: new_threshold,
    });
    
    protocol.min_fee_growth_threshold = new_threshold;
    Ok(())
}

pub fn renounce_fee_threshold(ctx: Context<UpdateProtocol>) -> Result<()> {
    let protocol = &mut ctx.accounts.protocol_state;
    
    require!(!protocol.fee_threshold_renounced, AlreadyRenounced);
    require!(ctx.accounts.authority.key() == protocol.authority, NotProtocolAuthority);
    
    protocol.fee_threshold_renounced = true;
    
    emit!(FeeThresholdRenounced {
        locked_threshold: protocol.min_fee_growth_threshold,
    });
    
    Ok(())
}
```

---

## Protocol Administration

### Update Protocol Fees

The protocol authority can adjust fee parameters at any time:

```rust
pub fn update_protocol_fees(
    ctx: Context<UpdateProtocolFees>,
    params: UpdateFeesParams,
) -> Result<()> {
    let protocol = &mut ctx.accounts.protocol_state;
    
    // Only protocol authority can update
    require!(ctx.accounts.authority.key() == protocol.authority, Unauthorized);
    
    // Validate fee ranges (0-10% = 0-1000 bps)
    if let Some(creation_fee_bps) = params.creation_fee_bps {
        require!(creation_fee_bps <= 1000, FeeTooHigh); // Max 10%
        protocol.creation_fee_bps = creation_fee_bps;
    }
    
    if let Some(unwind_fee_bps) = params.unwind_fee_bps {
        require!(unwind_fee_bps <= 1000, FeeTooHigh); // Max 10%
        protocol.unwind_fee_bps = unwind_fee_bps;
    }
    
    if let Some(min_fee_lamports) = params.min_fee_lamports {
        protocol.min_fee_lamports = min_fee_lamports;
    }
    
    if let Some(auto_unwind_period) = params.auto_unwind_period {
        require!(auto_unwind_period >= 90 days && auto_unwind_period <= 365 days, InvalidPeriod);
        protocol.auto_unwind_period = auto_unwind_period;
    }
    
    emit!(ProtocolFeesUpdated {
        creation_fee_bps: protocol.creation_fee_bps,
        unwind_fee_bps: protocol.unwind_fee_bps,
        min_fee_lamports: protocol.min_fee_lamports,
    });
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct UpdateFeesParams {
    pub creation_fee_bps: Option<u16>,    // 0-1000 (0-10%)
    pub unwind_fee_bps: Option<u16>,      // 0-1000 (0-10%)
    pub min_fee_lamports: Option<u64>,    // Minimum fee for failed bonding
    pub auto_unwind_period: Option<i64>,  // 90-365 days
}
```

---

## Security Considerations

### Solana-Specific Security

| Risk | Mitigation |
|------|------------|
| **Account Validation** | All accounts validated via Anchor constraints |
| **Signer Checks** | Explicit signer requirements on all mutations |
| **PDA Ownership** | PDAs owned by program, cannot be transferred |
| **Reentrancy** | Solana's atomic transactions prevent reentrancy |
| **Integer Overflow** | Rust's checked math + explicit overflow checks |

### Protocol-Specific Security

| Risk | Severity | Mitigation |
|------|----------|------------|
| **Activity Check Front-Running** | Medium | Set meaningful fee growth threshold; dust swaps shouldn't satisfy threshold |
| **Unwind Fee Rounding** | Low | Use running balance tracking; reserve dust for protocol |
| **Genesis NFT Transfer** | Low | Document that NFT = economic rights; theft = fund loss |
| **Dust Deposit Spam** | Medium | Enforce MIN_DEPOSIT (e.g., 0.1 SOL) |
| **Protocol Authority Abuse** | Medium | Use multi-sig; add timelock on parameter changes |
| **Unclaimed Fees on Unwind** | Medium | Always collect pending fees BEFORE removing liquidity |

### Required Validations

All instructions MUST validate:

```rust
// 1. Correct Whirlpool for sovereign
constraint = sovereign.whirlpool == whirlpool.key() @ InvalidPool

// 2. Correct program IDs for CPI
constraint = whirlpool_program.key() == WHIRLPOOL_PROGRAM_ID @ InvalidProgram

// 3. State checks
constraint = sovereign.state == expected_state @ InvalidState

// 4. NFT ownership for fee claims
constraint = nft_token_account.owner == claimer.key() @ NotNFTOwner
constraint = nft_token_account.mint == deposit_record.genesis_nft @ WrongNFT
constraint = nft_token_account.amount == 1 @ NotNFTOwner
```

### Activity Check Security

**Front-Running Mitigation:**
- Protocol sets `min_fee_growth_threshold` to a meaningful value (not dust)
- Threshold should represent significant trading activity (e.g., equivalent to 10 SOL in fees)
- Consider: threshold based on percentage of recovery_target, not absolute value

**Cooldown Period:**
After an activity check is cancelled (pool was active), enforce a cooldown before new initiation:

```rust
pub const ACTIVITY_CHECK_COOLDOWN: i64 = 7 * 24 * 60 * 60; // 7 days

// In initiate_activity_check:
if sovereign.activity_check_last_cancelled > 0 {
    let cooldown_end = sovereign.activity_check_last_cancelled + ACTIVITY_CHECK_COOLDOWN;
    require!(now >= cooldown_end, CooldownNotExpired);
}
```

### Unwind Fee Collection

Before removing liquidity during unwind, ALWAYS collect pending fees:

```rust
pub fn execute_unwind_internal(ctx: Context<...>) -> Result<()> {
    // STEP 1: Collect any pending fees from Whirlpool
    let (pending_sol, pending_tokens) = collect_whirlpool_fees(&ctx)?;
    
    // Add to existing collected fees for distribution
    sovereign.total_sol_fees_collected += pending_sol;
    sovereign.total_token_fees_collected += pending_tokens;
    
    // STEP 2: Remove liquidity
    let (sol_from_lp, tokens_from_lp) = remove_whirlpool_liquidity(&ctx)?;
    
    // STEP 3: Calculate distributions
    // ...
}
```

### Minimum Deposit Requirement

Prevent dust deposit spam:

```rust
pub const MIN_DEPOSIT_LAMPORTS: u64 = 100_000_000; // 0.1 SOL

// In deposit instruction:
require!(amount >= MIN_DEPOSIT_LAMPORTS, DepositTooSmall);
```

### CPI Security

```rust
// Always validate CPI target programs
#[account(
    constraint = whirlpool_program.key() == WHIRLPOOL_PROGRAM_ID
)]
pub whirlpool_program: Program<'info, Whirlpool>,

// Use seeds for PDA signing
let seeds = &[
    b"lock",
    sovereign.key().as_ref(),
    &[permanent_lock.bump],
];
let signer = &[&seeds[..]];
```

### Account Validation Example

```rust
#[derive(Accounts)]
pub struct Deposit<'info> {
    #[account(
        mut,
        seeds = [b"sovereign", sovereign.sovereign_id.to_le_bytes().as_ref()],
        bump = sovereign.bump,
        constraint = sovereign.state == SovereignStatus::Bonding @ InvalidState,
    )]
    pub sovereign: Account<'info, SovereignState>,
    
    #[account(
        init_if_needed,
        payer = depositor,
        space = 8 + DepositRecord::INIT_SPACE,
        seeds = [b"deposit", sovereign.key().as_ref(), depositor.key().as_ref()],
        bump,
    )]
    pub deposit_record: Account<'info, DepositRecord>,
    
    #[account(mut)]
    pub depositor: Signer<'info>,
    
    pub system_program: Program<'info, System>,
}
```

---

## Token Launcher

### Overview

The Token Launcher is an **optional** feature for creators who want to deploy new SPL tokens with **transfer hooks** for sell-only taxation. Alternatively, creators can use **BYO Token** to bootstrap liquidity for existing tokens.

```
┌─────────────────────────────────────────────────────────────────┐
│                    SOVEREIGN TYPE COMPARISON                     │
│                                                                  │
│   TOKEN LAUNCHER (SovereignType::TokenLaunch)                   │
│   ┌─────────────────────────────────────────────────────────┐   │
│   │  ✓ Protocol creates new Token-2022                       │   │
│   │  ✓ Transfer hooks for sell-only tax (0-3%)              │   │
│   │  ✓ 100% of supply deposited                             │   │
│   │  ✓ Full fee mode support (CreatorRevenue, etc.)         │   │
│   │  ✓ Ideal for: New token launches                        │   │
│   └─────────────────────────────────────────────────────────┘   │
│                                                                  │
│   BYO TOKEN (SovereignType::BYOToken)                           │
│   ┌─────────────────────────────────────────────────────────┐   │
│   │  ✓ Creator brings existing SPL/Token-2022               │   │
│   │  ✗ No sell tax (unless token already has hooks)         │   │
│   │  ✓ Min 30% of supply deposited (protocol-adjustable)    │   │
│   │  ✓ Investors earn from LP swap fees only                │   │
│   │  ✓ Ideal for: Existing projects, migrations             │   │
│   └─────────────────────────────────────────────────────────┘   │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### Token Metadata (Pinata IPFS)

Token metadata is stored on **Pinata IPFS** for decentralized, permanent storage:

```json
{
  "name": "My Token",
  "symbol": "MTK",
  "description": "A fair launch token on Sovereign Liquidity Protocol",
  "image": "ipfs://QmXxx.../logo.png",
  "external_url": "https://mytoken.com",
  "attributes": [
    { "trait_type": "Launch Platform", "value": "Sovereign Liquidity Protocol" },
    { "trait_type": "Sell Fee", "value": "2%" },
    { "trait_type": "Fee Mode", "value": "FairLaunch" }
  ],
  "properties": {
    "files": [
      { "uri": "ipfs://QmXxx.../logo.png", "type": "image/png" }
    ],
    "category": "token"
  }
}
```

**Pinata Integration:**
- Creator uploads token logo via frontend
- Frontend uploads to Pinata IPFS, receives CID
- Metadata JSON created and uploaded to Pinata
- Final metadata URI: `https://gateway.pinata.cloud/ipfs/{CID}`
- URI stored in Metaplex Token Metadata on-chain

### SPL Token Extensions

Using **Token-2022** with Transfer Hooks:

```rust
// Token with transfer hook for sell detection
pub struct SovereignTokenConfig {
    pub mint: Pubkey,
    pub creator: Pubkey,
    pub sovereign: Option<Pubkey>,
    pub pool: Option<Pubkey>,          // AMM pool address
    pub sell_tax_bps: u16,             // 0-300 (0-3%)
    pub fee_mode: FeeMode,
    pub fee_control_renounced: bool,   // If true, creator cannot change sell_tax_bps
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub enum FeeMode {
    CreatorRevenue,   // Fees always to creator
    RecoveryBoost,    // Fees to NFT during recovery, then creator
    FairLaunch,       // Fees to NFT during recovery, then 0%
}
```

**IMPORTANT: FairLaunch Mode Requirements**

The `FairLaunch` mode does NOT automatically set fees to 0%. To achieve a true fair launch:
1. Creator MUST call `update_sell_fee(0)` after recovery completes
2. Creator MUST call `renounce_fee_control()` to make it permanent

This two-step process ensures intentional commitment. A "FairLaunch" sovereign where the creator hasn't renounced control should be considered **unverified**. Frontend applications should display a warning badge for FairLaunch sovereigns that have:
- `sell_tax_bps > 0` after `recovery_complete = true`, OR
- `fee_control_renounced = false`

### Fee Control Instructions

```rust
// Creator can update sell fee (if not renounced)
pub fn update_sell_fee(ctx: Context<UpdateSellFee>, new_fee_bps: u16) -> Result<()> {
    let config = &mut ctx.accounts.token_config;
    
    require!(ctx.accounts.creator.key() == config.creator, OnlyCreator);
    require!(!config.fee_control_renounced, FeeControlRenounced);
    require!(new_fee_bps <= 300, FeeTooHigh); // Max 3%
    
    config.sell_tax_bps = new_fee_bps;
    
    emit!(SellFeeUpdated { mint: config.mint, new_fee_bps });
}

// Creator can permanently renounce fee control
pub fn renounce_fee_control(ctx: Context<RenounceFeeControl>) -> Result<()> {
    let config = &mut ctx.accounts.token_config;
    
    require!(ctx.accounts.creator.key() == config.creator, OnlyCreator);
    require!(!config.fee_control_renounced, AlreadyRenounced);
    
    config.fee_control_renounced = true;
    
    emit!(FeeControlRenounced { 
        mint: config.mint, 
        final_fee_bps: config.sell_tax_bps 
    });
}
```

### Transfer Hook Logic

```rust
// Called on every token transfer via Token-2022 hook
pub fn transfer_hook(ctx: Context<TransferHook>, amount: u64) -> Result<()> {
    let config = &ctx.accounts.token_config;
    
    // Only tax sells (transfers TO the pool)
    if ctx.accounts.destination.key() != config.pool.unwrap() {
        return Ok(()); // No tax on buys or transfers
    }
    
    if config.sell_tax_bps == 0 {
        return Ok(()); // No tax
    }
    
    let tax = amount * config.sell_tax_bps as u64 / 10000;
    let recipient = determine_fee_recipient(config)?;
    
    // Transfer tax to recipient
    // ...
}

fn determine_fee_recipient(config: &SovereignTokenConfig) -> Result<Pubkey> {
    match config.fee_mode {
        FeeMode::CreatorRevenue => Ok(config.creator),
        FeeMode::RecoveryBoost | FeeMode::FairLaunch => {
            if let Some(sovereign) = config.sovereign {
                let sovereign_state = /* load sovereign */;
                if sovereign_state.recovery_complete() {
                    // Post-recovery: fees go to creator (or 0 if FairLaunch)
                    // Note: FairLaunch should set sell_tax_bps = 0 after recovery
                    Ok(config.creator)
                } else {
                    // During recovery, fees to Genesis NFT vault
                    Ok(sovereign_state.genesis_fee_vault)
                }
            } else {
                Ok(config.creator)
            }
        }
    }
}
```

---

## Appendix A: External Dependencies

### Forked Orca Whirlpools

We deploy a **forked version** of Orca Whirlpools with LP restriction support:

| Program | Address |
|---------|---------|
| SLP Whirlpool Program | `TBD (our deployed fork)` |
| Original Whirlpool | `whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc` |

**Fork Modifications:**
```rust
// Added to WhirlpoolState
pub restricted: bool,        // If true, only owner can add liquidity
pub restriction_owner: Pubkey, // PDA that controls restriction

// Modified open_position / increase_liquidity
if whirlpool.restricted {
    require!(
        signer == whirlpool.restriction_owner,
        ErrorCode::PoolRestricted
    );
}

// New instruction: unlock_pool
pub fn unlock_pool(ctx: Context<UnlockPool>) -> Result<()> {
    require!(ctx.accounts.signer.key() == ctx.accounts.whirlpool.restriction_owner);
    ctx.accounts.whirlpool.restricted = false;
    Ok(())
}
```

**Pool Lifecycle:**
1. Pool created with `restricted = true`
2. Only Genesis position (via protocol PDA) can provide liquidity
3. On recovery complete: `unlock_pool()` called
4. Pool becomes open - anyone can LP

### Metaplex

| Program | Address |
|---------|---------|
| Token Metadata | `metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s` |
| Token Auth Rules | `auth9SigNpDKz4sJJ1DfCTuZrZNSAgh9sFD3rboVmgg` |

### SPL Token

| Program | Address |
|---------|---------|
| Token Program | `TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA` |
| Token-2022 | `TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb` |
| Associated Token | `ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL` |

### Pinata IPFS (Token Metadata)

| Service | Endpoint |
|---------|----------|
| Pinata API | `https://api.pinata.cloud` |
| Pinata Gateway | `https://gateway.pinata.cloud/ipfs/{CID}` |
| Dedicated Gateway | `https://{gateway}.mypinata.cloud/ipfs/{CID}` |

**Usage:**
- Token logos uploaded via Pinata SDK/API
- Metadata JSON stored on IPFS
- CID referenced in Metaplex Token Metadata URI field
- Ensures decentralized, permanent metadata storage

---

## Appendix B: Events

### Sovereign Events

```rust
#[event]
pub struct SovereignCreated {
    pub sovereign_id: Pubkey,
    pub creator: Pubkey,
    pub token_mint: Pubkey,
    pub bond_target: u64,
    pub sell_fee_bps: u16,
    pub swap_fee_bps: u16,
}

#[event]
pub struct Deposited {
    pub sovereign: Pubkey,
    pub depositor: Pubkey,
    pub amount: u64,
    pub total: u64,
    // Note: is_creator field removed - only investors deposit to LP
}

#[event]
pub struct CreatorEscrowed {
    pub sovereign: Pubkey,
    pub creator: Pubkey,
    pub amount: u64,
    // Creator's SOL goes to escrow for market buy, NOT to LP
}

#[event]
pub struct CreatorTokensPurchased {
    pub sovereign: Pubkey,
    pub creator: Pubkey,
    pub sol_amount: u64,
    pub tokens_received: u64,
    // Emitted during finalize() after LP creation
}

#[event]
pub struct Withdrawn {
    pub sovereign: Pubkey,
    pub depositor: Pubkey,
    pub amount: u64,
    // Investor SOL refund on failed bonding
}

#[event]
pub struct CreatorFailedWithdrawal {
    pub sovereign: Pubkey,
    pub creator: Pubkey,
    pub sol_returned: u64,      // Escrowed SOL minus min fee
    pub tokens_returned: u64,   // 100% of tokens (never went to LP)
}

#[event]
pub struct SovereignFinalized {
    pub sovereign: Pubkey,
    pub total_deposited: u64,
    pub token_amount: u64,
    pub position_mint: Pubkey,
    pub creation_fee: u64,
}

#[event]
pub struct SovereignFailed {
    pub sovereign: Pubkey,
    pub total_deposited: u64,
    pub bond_target: u64,
    pub minimum_fee: u64,
}

#[event]
pub struct FeesCollected {
    pub sovereign: Pubkey,
    pub sol_amount: u64,
    pub token_amount: u64,
}

#[event]
pub struct FeesClaimed {
    pub sovereign: Pubkey,
    pub nft_mint: Pubkey,
    pub claimer: Pubkey,
    pub sol_amount: u64,
    pub token_amount: u64,
}

#[event]
pub struct RecoveryComplete {
    pub sovereign: Pubkey,
    pub total_recovered: u64,
    pub timestamp: i64,
}

#[event]
pub struct PoolUnlocked {
    pub sovereign: Pubkey,
    pub whirlpool: Pubkey,
    pub timestamp: i64,
}
```

### Governance Events

```rust
#[event]
pub struct ProposalCreated {
    pub sovereign: Pubkey,
    pub proposal_id: Pubkey,
    pub proposer: Pubkey,
    pub voting_ends: i64,
}

#[event]
pub struct Voted {
    pub proposal: Pubkey,
    pub nft_mint: Pubkey,
    pub support: bool,
    pub shares_bps: u16,
}

#[event]
pub struct UnwindExecuted {
    pub sovereign: Pubkey,
    pub sol_balance: u64,
    pub token_balance: u64,
    pub unwind_fee: u64,
}

#[event]
pub struct UnwindClaimed {
    pub sovereign: Pubkey,
    pub nft_mint: Pubkey,
    pub claimer: Pubkey,
    pub sol_amount: u64,
    // For investors only - they receive SOL
}

#[event]
pub struct CreatorUnwindClaimed {
    pub sovereign: Pubkey,
    pub creator: Pubkey,
    pub lp_tokens: u64,        // Tokens returned from LP (creator's 100% contribution)
    pub purchased_tokens: u64, // Market-bought tokens from escrow
    pub total_tokens: u64,     // Total tokens received
}

#[event]
pub struct AutoUnwindTriggered {
    pub sovereign: Pubkey,
    pub fee_growth_a: u128,
    pub fee_growth_b: u128,
    pub threshold: u128,
    pub elapsed_seconds: u64,
}

#[event]
pub struct ActivityCheckInitiated {
    pub sovereign: Pubkey,
    pub snapshot_a: u128,
    pub snapshot_b: u128,
    pub timestamp: i64,
}

#[event]
pub struct ActivityCheckCancelled {
    pub sovereign: Pubkey,
    pub fee_growth_a: u128,
    pub fee_growth_b: u128,
    pub threshold: u128,
}
```

### Creator Events

```rust
#[event]
pub struct CreatorTokensUnlocked {
    pub sovereign: Pubkey,
    pub creator: Pubkey,
    pub token_amount: u64,
}

#[event]
pub struct SellFeeUpdated {
    pub sovereign: Pubkey,
    pub old_fee_bps: u16,
    pub new_fee_bps: u16,
}

#[event]
pub struct SellFeeRenounced {
    pub sovereign: Pubkey,
    pub locked_fee_bps: u16,
}
```

### Protocol Admin Events

```rust
#[event]
pub struct ProtocolFeesUpdated {
    pub old_creation_fee_bps: u16,
    pub new_creation_fee_bps: u16,
    pub old_minimum_fee: u64,
    pub new_minimum_fee: u64,
    pub old_unwind_fee_bps: u16,
    pub new_unwind_fee_bps: u16,
}

#[event]
pub struct AutoUnwindDaysUpdated {
    pub old_days: u16,
    pub new_days: u16,
}

#[event]
pub struct FeeThresholdUpdated {
    pub old_threshold: u128,
    pub new_threshold: u128,
}

#[event]
pub struct FeeThresholdRenounced {
    pub locked_threshold: u128,
}
```

---

## Appendix C: Error Codes

```rust
#[error_code]
pub enum SovereignError {
    // State Errors
    #[msg("Sovereign is not in the expected state")]
    InvalidState,
    
    #[msg("Bonding deadline has passed")]
    DeadlinePassed,
    
    #[msg("Bonding target not yet met")]
    BondingNotComplete,
    
    #[msg("Bonding target already met")]
    BondingComplete,
    
    // Deposit Errors
    #[msg("Creator deposit exceeds maximum allowed (1% of total supply)")]
    CreatorDepositExceedsMax,
    
    #[msg("Deposit amount is zero")]
    ZeroDeposit,
    
    #[msg("Deposit amount below minimum (0.1 SOL)")]
    DepositTooSmall,
    
    #[msg("No deposit record found")]
    NoDepositRecord,
    
    // NFT Errors
    #[msg("Caller is not the NFT owner")]
    NotNFTOwner,
    
    #[msg("NFT has already been used for this action")]
    NFTAlreadyUsed,
    
    #[msg("Wrong NFT for this deposit record")]
    WrongNFT,
    
    // Recovery Phase Errors
    #[msg("Creator cannot claim fees during recovery phase")]
    CreatorCannotClaimDuringRecovery,
    
    #[msg("Creator cannot vote during recovery phase")]
    CreatorCannotVote,
    
    #[msg("Creator tokens are locked until recovery complete or unwind")]
    CreatorTokensLocked,
    
    // Governance Errors
    #[msg("Not enough inactivity to propose unwind")]
    InsufficientInactivity,
    
    #[msg("Voting period has not ended")]
    VotingNotEnded,
    
    #[msg("Voting period has ended")]
    VotingEnded,
    
    #[msg("Proposal did not reach quorum (67%)")]
    QuorumNotReached,
    
    #[msg("Proposal did not pass (need 51%)")]
    ProposalNotPassed,
    
    #[msg("Already voted on this proposal")]
    AlreadyVoted,
    
    #[msg("Governance is only active during recovery phase")]
    GovernanceNotActive,
    
    // Active Phase Errors
    #[msg("Cannot unwind in active phase via governance")]
    CannotGovernanceUnwindInActivePhase,
    
    #[msg("Auto-unwind conditions not met")]
    AutoUnwindConditionsNotMet,
    
    #[msg("Activity check only valid in Active phase")]
    OnlyActivePhase,
    
    #[msg("Activity check already in progress")]
    ActivityCheckAlreadyInProgress,
    
    #[msg("No activity check in progress")]
    NoActivityCheckInProgress,
    
    #[msg("Must wait 90+ days before executing activity check")]
    ActivityCheckTooEarly,
    
    #[msg("Must wait 7 days after cancelled check before initiating new one")]
    ActivityCheckCooldownNotExpired,
    
    #[msg("Fee threshold has been renounced and cannot be changed")]
    FeeThresholdRenounced,
    
    #[msg("Fee threshold already renounced")]
    AlreadyRenounced,
    
    // Validation Errors
    #[msg("Invalid pool - does not match sovereign's whirlpool")]
    InvalidPool,
    
    #[msg("Invalid program ID for CPI")]
    InvalidProgram,
    
    // Pool Errors
    #[msg("Pool is restricted - only Genesis position can LP")]
    PoolRestricted,
    
    #[msg("Pool is not restricted")]
    PoolNotRestricted,
    
    // Fee Errors
    #[msg("Sell fee exceeds maximum (3%)")]
    SellFeeExceedsMax,
    
    #[msg("Swap fee outside valid range (0.3-2%)")]
    InvalidSwapFee,
    
    #[msg("Fee control has been renounced")]
    FeeControlRenounced,
    
    // Protocol Admin Errors
    #[msg("Caller is not the protocol authority")]
    NotProtocolAuthority,
    
    #[msg("Creation fee exceeds maximum (10%)")]
    CreationFeeExceedsMax,
    
    #[msg("Unwind fee exceeds maximum (10%)")]
    UnwindFeeExceedsMax,
    
    #[msg("Auto-unwind days outside valid range (90-365)")]
    InvalidAutoUnwindDays,
    
    // Token Launcher Errors
    #[msg("Token metadata URI is too long")]
    MetadataURITooLong,
    
    #[msg("Token name is too long")]
    TokenNameTooLong,
    
    #[msg("Token symbol is too long")]
    TokenSymbolTooLong,
    
    // BYO Token Errors
    #[msg("BYO Token: Missing existing mint address")]
    MissingExistingMint,
    
    #[msg("BYO Token: Missing deposit amount")]
    MissingDepositAmount,
    
    #[msg("BYO Token: Insufficient token deposit (below minimum % required)")]
    InsufficientTokenDeposit,
    
    #[msg("BYO Token: Failed to read token supply")]
    FailedToReadTokenSupply,
    
    #[msg("Token Launcher: Missing token name")]
    MissingTokenName,
    
    #[msg("Token Launcher: Missing token symbol")]
    MissingTokenSymbol,
    
    #[msg("Token Launcher: Missing token supply")]
    MissingTokenSupply,
    
    // Arithmetic Errors
    #[msg("Arithmetic overflow")]
    Overflow,
    
    #[msg("Arithmetic underflow")]
    Underflow,
}
```

---

## Appendix D: Compute Budget

| Operation | Estimated CU | Notes |
|-----------|-------------|-------|
| create_sovereign | ~100,000 | Multiple account inits |
| deposit | ~30,000 | Simple transfer + PDA update |
| finalize | ~250,000 | CPI to Whirlpool + NFT mints |
| claim_fees | ~100,000 | CPI to Whirlpool + transfers |
| vote | ~20,000 | Simple PDA updates |
| execute_unwind | ~200,000 | CPI to close position |
| update_protocol_fees | ~15,000 | Simple PDA update |
| initiate_activity_check | ~25,000 | Read Whirlpool + update PDA |
| execute_activity_check | ~200,000 | Read Whirlpool + potential CPI |
| update_fee_threshold | ~15,000 | Simple PDA update |
| renounce_fee_threshold | ~10,000 | Simple flag update |

---

## Appendix E: Account Sizes

| Account | Size (bytes) | Rent Exempt (SOL) |
|---------|-------------|-------------------|
| ProtocolState | ~250 | 0.00234 |
| SovereignState | ~700 | 0.00600 |
| DepositRecord | ~200 | 0.00203 |
| CreatorFeeTracker | ~150 | 0.00168 |
| PermanentLock | ~150 | 0.00168 |
| Proposal | ~200 | 0.00203 |
| VoteRecord | ~100 | 0.00128 |

---

## Appendix F: Version History

### Version 1.2 (January 2026)
**Major Creator Flow Correction:**
- Creator's optional SOL now goes to **escrow**, NOT to LP
- After LP is created with investor SOL + creator tokens, creator's escrowed SOL performs market buy
- Creator purchased tokens are locked during recovery, unlocked after
- Creator does **NOT** receive Genesis NFT (investors only)
- Creator has **NO** LP fee share - 100% of LP fees go to investors forever
- Removed `is_creator` field from DepositRecord (only investors have deposit records)
- Renamed `creator_deposit` to `creator_escrow` in SovereignState
- Added `CreatorEscrowed` and `CreatorTokensPurchased` events
- Updated `CreatorUnwindClaimed` event with lp_tokens and purchased_tokens

**Creation Fee Escrow System:**
- Creation fee (0-10% of bond target) is now **escrowed** during `create_sovereign`
- Fee is held in escrow until bonding outcome is determined:
  - **Success**: Full creation fee transferred to protocol treasury
  - **Failure**: Creation fee refunded to creator minus minimum fee
- Added `creation_fee_escrowed` field to SovereignState
- Minimum fee is the non-refundable portion (default 0.05 SOL)
- Investor SOL goes 100% to LP (no deductions)

**Governance Unwind Fee:**
- Added governance unwind fee (default 0.05 SOL) to `propose_unwind()`
- Fee is protocol-adjustable via `governance_unwind_fee_lamports`
- Only applies during recovery phase (active phase uses auto-unwind)

**Unwind/Failed Bonding Clarification:**
- Investors claim SOL via `claim_investor_unwind()` using Genesis NFT
- Creator claims tokens via `claim_creator_unwind()` (separate function)
- Failed bonding: Investors get SOL refund, Creator gets tokens + buy-in escrow + creation fee refund (minus min fee)
- Added `withdraw_creator_failed()` function for creator to reclaim on failed bonding
- Added `CreatorFailedWithdrawal` event

**Security Improvements:**
- Added minimum deposit requirement (0.1 SOL)
- Added activity check cooldown (7 days between cancellation and re-initiation)
- Expanded security considerations documentation
- Creator market buy is now **atomic** with LP creation (prevents MEV/slippage)
- Added 1% slippage protection on creator market buy
- Pool unlock (`unlock_pool()`) called automatically when recovery completes
- Activity threshold must be > 0 (minimum 1000) to prevent edge cases
- Added double-claim protection in `claim_creator_unwind()` for purchased tokens

**FairLaunch Mode Clarification:**
- FairLaunch does NOT auto-set fees to 0%
- Requires creator to explicitly call `update_sell_fee(0)` + `renounce_fee_control()`
- Frontend should display warning for unverified FairLaunch sovereigns

**BYO Token (Bring Your Own) Support:**
- Added `SovereignType` enum: `TokenLaunch` vs `BYOToken`
- BYO Token allows existing tokens to bootstrap liquidity
- Minimum 30% of supply required (protocol-adjustable via `byo_min_supply_bps`)
- No sell tax for BYO (unless token already has transfer hooks)
- Added `token_supply_deposited` and `token_total_supply` to SovereignState
- Updated `create_sovereign` to handle both token types
- BYO investors earn from LP swap fees only (no sell tax revenue)

### Version 1.1 (January 2026)
- Two-call activity check for Active phase auto-unwind
- Cumulative fee growth snapshots from Orca Whirlpools
- Protocol-adjustable fee growth threshold with renounce option
- Permissionless initiate and execute activity check

### Version 1.0 (January 2026)
- Initial Solana specification
- Anchor framework with PDAs
- Orca Whirlpools integration (forked)
- Metaplex Genesis NFTs
- Token-2022 transfer hooks for Token Launcher
- Governance with timelock
- Protocol revenue structure (creation fee, minimum fee, unwind fee)
