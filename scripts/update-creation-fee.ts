import * as anchor from "@coral-xyz/anchor";
import { Program, AnchorProvider, Wallet } from "@coral-xyz/anchor";
import { Connection, Keypair, PublicKey, LAMPORTS_PER_SOL } from "@solana/web3.js";
import * as fs from "fs";
import * as path from "path";

// Gorbagana devnet RPC
const RPC_URL = "https://rpc.trashscan.io";

// Program ID (deployed)
const PROGRAM_ID = new PublicKey("2LPPAG7UhVop1RiRBh8oZtjzMoJ9St9WV4nY7JwmoNbA");

// Seeds
const PROTOCOL_STATE_SEED = Buffer.from("protocol_state");

async function main() {
    console.log("🔧 Updating Creation Fee to 0.01%...\n");

    // Load wallet
    const walletPath = path.join(process.env.HOME || process.env.USERPROFILE || "", ".config/solana/id.json");
    const keypairData = JSON.parse(fs.readFileSync(walletPath, "utf-8"));
    const authority = Keypair.fromSecretKey(Uint8Array.from(keypairData));
    
    console.log("Authority:", authority.publicKey.toBase58());

    // Setup connection
    const connection = new Connection(RPC_URL, "confirmed");
    const wallet = new Wallet(authority);
    const provider = new AnchorProvider(connection, wallet, {
        commitment: "confirmed",
        preflightCommitment: "confirmed",
    });

    // Load IDL
    const idlPath = path.join(__dirname, "../target/idl/sovereign_liquidity.json");
    const idl = JSON.parse(fs.readFileSync(idlPath, "utf-8"));

    // Create program
    const program = new Program(idl, provider) as any;

    // Derive Protocol State PDA
    const [protocolStatePda] = PublicKey.findProgramAddressSync(
        [PROTOCOL_STATE_SEED],
        PROGRAM_ID
    );
    console.log("Protocol State PDA:", protocolStatePda.toBase58());

    // Fetch current protocol state
    console.log("\n📊 Current Protocol State:");
    const stateBefore = await (program.account as any).protocolState.fetch(protocolStatePda);
    console.log("  Creation Fee:", stateBefore.creationFeeBps / 100, "%");
    console.log("  Min Fee:", Number(stateBefore.minFeeLamports) / LAMPORTS_PER_SOL, "GOR");
    console.log("  Min Deposit:", Number(stateBefore.minDeposit) / LAMPORTS_PER_SOL, "GOR");
    console.log("  Min Bond Target:", Number(stateBefore.minBondTarget) / LAMPORTS_PER_SOL, "GOR");

    // New creation fee: 0.01% = 1 bps
    const newCreationFeeBps = 1;

    console.log("\n📝 Updating creation fee to", newCreationFeeBps / 100, "%...");

    try {
        const tx = await program.methods
            .updateProtocolFees(
                newCreationFeeBps,  // new_creation_fee_bps: 1 bps = 0.01%
                null,               // new_min_fee_lamports: unchanged
                null,               // new_min_deposit: unchanged
                null                // new_min_bond_target: unchanged
            )
            .accounts({
                authority: authority.publicKey,
                protocolState: protocolStatePda,
            })
            .signers([authority])
            .rpc();

        console.log("✅ Transaction confirmed:", tx);
        console.log("   Explorer: https://trashscan.io/tx/" + tx);

        // Fetch updated state
        console.log("\n📊 Updated Protocol State:");
        const stateAfter = await (program.account as any).protocolState.fetch(protocolStatePda);
        console.log("  Creation Fee:", stateAfter.creationFeeBps / 100, "%");
        console.log("  Min Fee:", Number(stateAfter.minFeeLamports) / LAMPORTS_PER_SOL, "GOR");
        console.log("  Min Deposit:", Number(stateAfter.minDeposit) / LAMPORTS_PER_SOL, "GOR");
        console.log("  Min Bond Target:", Number(stateAfter.minBondTarget) / LAMPORTS_PER_SOL, "GOR");

        console.log("\n🎉 Creation fee successfully updated to 0.01%!");

    } catch (err: any) {
        console.error("\n❌ Error updating creation fee:", err.message);
        if (err.logs) {
            console.error("\nProgram logs:");
            err.logs.forEach((log: string) => console.error("  ", log));
        }
        process.exit(1);
    }
}

main().catch(console.error);
