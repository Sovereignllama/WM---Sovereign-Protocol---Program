/**
 * Sovereign Liquidity Protocol - Deployment Script
 * 
 * This script handles:
 * 1. Protocol initialization (ProtocolState PDA)
 * 2. Optional parameter updates post-deployment
 * 
 * Usage:
 *   npx ts-node scripts/deploy.ts --network devnet
 *   npx ts-node scripts/deploy.ts --network mainnet-beta
 */

import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { PublicKey, Keypair, Connection, LAMPORTS_PER_SOL } from "@solana/web3.js";
import { SovereignLiquidity } from "../target/types/sovereign_liquidity";
import * as fs from "fs";
import * as path from "path";

// ============================================================================
// CONFIGURATION - Update these for your deployment
// ============================================================================

const CONFIG = {
  // Network endpoints
  networks: {
    localnet: "http://localhost:8899",
    devnet: "https://api.devnet.solana.com",
    "mainnet-beta": "https://api.mainnet-beta.solana.com",
  },

  // Protocol parameters (defaults match on-chain defaults, adjust as needed)
  protocolParams: {
    // Creation fee: 1% of bond target (100 bps)
    creationFeeBps: 100,
    
    // Minimum fee: 0.05 SOL (non-refundable on failed bonding)
    minFeeLamports: 0.05 * LAMPORTS_PER_SOL,
    
    // Governance unwind fee: 0.05 SOL
    governanceUnwindFeeLamports: 0.05 * LAMPORTS_PER_SOL,
    
    // Unwind fee: 5% of SOL returned (500 bps)
    unwindFeeBps: 500,
    
    // BYO Token minimum supply: 30% (3000 bps)
    byoMinSupplyBps: 3000,
    
    // Minimum bond target: 50 SOL
    minBondTarget: 50 * LAMPORTS_PER_SOL,
    
    // Minimum deposit: 0.1 SOL
    minDeposit: 0.1 * LAMPORTS_PER_SOL,
    
    // Auto-unwind period: 90 days (in seconds)
    autoUnwindPeriod: 90 * 24 * 60 * 60,
    
    // Activity check threshold (minimum fee growth to count as "active")
    minFeeGrowthThreshold: 1000,
  },
};

// ============================================================================
// DEPLOYMENT FUNCTIONS
// ============================================================================

async function loadKeypair(keypairPath: string): Promise<Keypair> {
  const absolutePath = path.resolve(keypairPath);
  const secretKey = JSON.parse(fs.readFileSync(absolutePath, "utf-8"));
  return Keypair.fromSecretKey(new Uint8Array(secretKey));
}

async function getProtocolStatePDA(programId: PublicKey): Promise<[PublicKey, number]> {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("protocol")],
    programId
  );
}

async function initializeProtocol(
  program: Program<SovereignLiquidity>,
  authority: Keypair,
  treasury: PublicKey
): Promise<string> {
  const [protocolStatePDA] = await getProtocolStatePDA(program.programId);

  console.log("\n📋 Protocol State PDA:", protocolStatePDA.toBase58());
  console.log("👤 Authority:", authority.publicKey.toBase58());
  console.log("💰 Treasury:", treasury.toBase58());

  // Check if already initialized
  try {
    const existingState = await program.account.protocolState.fetch(protocolStatePDA);
    console.log("\n⚠️  Protocol already initialized!");
    console.log("   Current authority:", existingState.authority.toBase58());
    console.log("   Current treasury:", existingState.treasury.toBase58());
    return "already_initialized";
  } catch {
    // Not initialized, proceed
  }

  console.log("\n🚀 Initializing protocol...");

  const tx = await program.methods
    .initializeProtocol()
    .accounts({
      authority: authority.publicKey,
      protocolState: protocolStatePDA,
      treasury: treasury,
      systemProgram: anchor.web3.SystemProgram.programId,
    })
    .signers([authority])
    .rpc();

  console.log("✅ Protocol initialized!");
  console.log("   Transaction:", tx);

  return tx;
}

async function updateProtocolFees(
  program: Program<SovereignLiquidity>,
  authority: Keypair,
  params: typeof CONFIG.protocolParams
): Promise<string> {
  const [protocolStatePDA] = await getProtocolStatePDA(program.programId);

  console.log("\n📝 Updating protocol fees...");

  // The on-chain function uses Option<T> for each parameter
  // Pass the values you want to update
  const tx = await program.methods
    .updateProtocolFees(
      params.creationFeeBps,                        // Option<u16>
      new anchor.BN(params.minFeeLamports),         // Option<u64>
      new anchor.BN(params.minDeposit),             // Option<u64>
      new anchor.BN(params.minBondTarget)           // Option<u64>
    )
    .accounts({
      authority: authority.publicKey,
      protocolState: protocolStatePDA,
    })
    .signers([authority])
    .rpc();

  console.log("✅ Protocol fees updated!");
  console.log("   Transaction:", tx);

  return tx;
}

async function printProtocolState(program: Program<SovereignLiquidity>): Promise<void> {
  const [protocolStatePDA] = await getProtocolStatePDA(program.programId);

  try {
    const state = await program.account.protocolState.fetch(protocolStatePDA);
    
    console.log("\n" + "=".repeat(60));
    console.log("PROTOCOL STATE");
    console.log("=".repeat(60));
    console.log(`PDA:                          ${protocolStatePDA.toBase58()}`);
    console.log(`Authority:                    ${state.authority.toBase58()}`);
    console.log(`Treasury:                     ${state.treasury.toBase58()}`);
    console.log("-".repeat(60));
    console.log("FEES:");
    console.log(`  Creation Fee:               ${state.creationFeeBps / 100}% (${state.creationFeeBps} bps)`);
    console.log(`  Min Fee:                    ${Number(state.minFeeLamports) / LAMPORTS_PER_SOL} SOL`);
    console.log("-".repeat(60));
    console.log("LIMITS:");
    console.log(`  Min Bond Target:            ${Number(state.minBondTarget) / LAMPORTS_PER_SOL} SOL`);
    console.log(`  Min Deposit:                ${Number(state.minDeposit) / LAMPORTS_PER_SOL} SOL`);
    console.log(`  Auto-Unwind Period:         ${Number(state.autoUnwindPeriod) / (24 * 60 * 60)} days`);
    console.log("-".repeat(60));
    console.log("STATS:");
    console.log(`  Total Sovereigns:           ${state.sovereignCount}`);
    console.log(`  Total Fees Collected:       ${Number(state.totalFeesCollected) / LAMPORTS_PER_SOL} SOL`);
    console.log("=".repeat(60));
  } catch (e) {
    console.log("\n❌ Protocol not initialized yet");
  }
}

// ============================================================================
// MAIN
// ============================================================================

async function main() {
  // Parse arguments
  const args = process.argv.slice(2);
  const networkArg = args.find((arg) => arg.startsWith("--network="));
  const network = networkArg?.split("=")[1] || "devnet";
  
  const initOnly = args.includes("--init-only");
  const updateOnly = args.includes("--update-only");
  const statusOnly = args.includes("--status");

  console.log("╔════════════════════════════════════════════════════════════╗");
  console.log("║     SOVEREIGN LIQUIDITY PROTOCOL - DEPLOYMENT SCRIPT       ║");
  console.log("╚════════════════════════════════════════════════════════════╝");
  console.log(`\n🌐 Network: ${network}`);

  // Get connection
  const endpoint = CONFIG.networks[network as keyof typeof CONFIG.networks];
  if (!endpoint) {
    throw new Error(`Unknown network: ${network}`);
  }
  
  const connection = new Connection(endpoint, "confirmed");
  console.log(`📡 Endpoint: ${endpoint}`);

  // Load authority keypair
  const keypairPath = process.env.ANCHOR_WALLET || 
    `${process.env.HOME}/.config/solana/id.json`;
  console.log(`🔑 Keypair: ${keypairPath}`);
  
  const authority = await loadKeypair(keypairPath);
  console.log(`👤 Deployer: ${authority.publicKey.toBase58()}`);

  // Check balance
  const balance = await connection.getBalance(authority.publicKey);
  console.log(`💰 Balance: ${balance / LAMPORTS_PER_SOL} SOL`);

  if (balance < 0.1 * LAMPORTS_PER_SOL) {
    console.log("\n⚠️  Warning: Low balance. Consider funding the wallet.");
    if (network === "devnet") {
      console.log("   Run: solana airdrop 2");
    }
  }

  // Setup Anchor provider and program
  const wallet = new anchor.Wallet(authority);
  const provider = new anchor.AnchorProvider(connection, wallet, {
    commitment: "confirmed",
  });
  anchor.setProvider(provider);

  // Load program
  const idlPath = path.resolve(__dirname, "../target/idl/sovereign_liquidity.json");
  if (!fs.existsSync(idlPath)) {
    throw new Error("IDL not found. Run 'anchor build' first.");
  }
  const idl = JSON.parse(fs.readFileSync(idlPath, "utf-8"));
  
  // Get program ID from IDL or Anchor.toml
  const anchorTomlPath = path.resolve(__dirname, "../Anchor.toml");
  const anchorToml = fs.readFileSync(anchorTomlPath, "utf-8");
  const programIdMatch = anchorToml.match(/sovereign_liquidity = "([^"]+)"/);
  if (!programIdMatch) {
    throw new Error("Program ID not found in Anchor.toml");
  }
  const programId = new PublicKey(programIdMatch[1]);
  console.log(`📦 Program ID: ${programId.toBase58()}`);

  const program = new Program(idl, provider) as Program<SovereignLiquidity>;

  // Status only - just print current state
  if (statusOnly) {
    await printProtocolState(program);
    return;
  }

  // Treasury address (defaults to authority, update for production!)
  const treasury = process.env.TREASURY_ADDRESS 
    ? new PublicKey(process.env.TREASURY_ADDRESS)
    : authority.publicKey;

  if (treasury.equals(authority.publicKey) && network === "mainnet-beta") {
    console.log("\n⚠️  Warning: Treasury is set to deployer address.");
    console.log("   Set TREASURY_ADDRESS env var for production.");
  }

  // Execute deployment steps
  if (!updateOnly) {
    await initializeProtocol(program, authority, treasury);
  }

  if (!initOnly) {
    // Update parameters to match config (the on-chain defaults are usually fine)
    // Only call this if you need to override the defaults
    try {
      await updateProtocolFees(program, authority, CONFIG.protocolParams);
    } catch (e: any) {
      console.log("\n⚠️  Could not update fees (may not be needed):", e.message);
    }
  }

  // Print final state
  await printProtocolState(program);

  console.log("\n🎉 Deployment complete!\n");
}

main().catch((err) => {
  console.error("\n❌ Deployment failed:", err);
  process.exit(1);
});
