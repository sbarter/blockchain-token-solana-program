import * as fs from "fs";
import * as path from "path";
import { strict as assert } from "assert";
import { describe, it, before } from "mocha";
import * as anchor from "@coral-xyz/anchor";
import {
  ComputeBudgetProgram,
  Connection,
  Keypair,
  PublicKey,
  SystemProgram,
} from "@solana/web3.js";
import {
  createMint,
  getMint,
  getAssociatedTokenAddress,
  getAccount,
  TOKEN_2022_PROGRAM_ID,
  ASSOCIATED_TOKEN_PROGRAM_ID,
} from "@solana/spl-token";
import { SbarterTokenPrograms } from "../target/types/sbarter_token_programs";

const PROGRAM_ID = new PublicKey("Hvpe662GeFcr5oVsjhvFZ2dyfuVtHCVVmcjBU6ozQYzE");
const SYSTEM_PROGRAM_PID = SystemProgram.programId;

const DEVNET_EXPLORER_TX = (sig: string) =>
  `https://explorer.solana.com/tx/${sig}?cluster=devnet`;
const DEVNET_EXPLORER_ADDR = (addr: PublicKey) =>
  `https://explorer.solana.com/address/${addr.toBase58()}?cluster=devnet`;

// categories we will use
const CATEGORY_NAMES = [
  "preseed",
  "seed",
  "institutional",
  "vgp",
  "marketing",
  "founders",
  "reserve",
  "liquidity",
];

describe("sbarterTokenPrograms (devnet)", function() {
  // devnet network calls can be slow
  this.timeout(1000 * 60 * 10);

  let provider: anchor.AnchorProvider;
  let connection: Connection;
  let program: anchor.Program<SbarterTokenPrograms>
  let master: Keypair;
  let mint: PublicKey;

  // derived maps
  const categoryPdas: Record<string, PublicKey> = {};
  const categoryAtas: Record<string, PublicKey> = {};
  let masterAta: PublicKey;

  before(async () => {
    // 1) load local keypair from ~/.config/solana/id.json (master)
    const home = process.env.HOME || process.env.USERPROFILE || ".";
    const idPath = path.join(home, ".config", "solana", "id.json");
    const raw = fs.readFileSync(idPath, "utf8");
    const arr = JSON.parse(raw) as number[];
    master = Keypair.fromSecretKey(Uint8Array.from(arr));

    // connection = new Connection("https://api.devnet.solana.com", "confirmed");
    connection = new Connection("http://127.0.0.1:8899", "confirmed");
    const wallet = new anchor.Wallet(master);

    provider = new anchor.AnchorProvider(connection, wallet, {
      preflightCommitment: "confirmed",
    });
    anchor.setProvider(provider);

    program = anchor.workspace.sbarterTokenPrograms as anchor.Program<SbarterTokenPrograms>;

    // 2) create a new token-2022 mint with master as mint authority
    // decimals: 6 (adjust if you want)
    const decimals = 6;
    // createMint uses the spl-token library; pass TOKEN_2022_PROGRAM_ID to createToken-2022 mint
    // createMint(connection, payer, mintAuthority, freezeAuthority, decimals, programId?);
    // Note: the exported constant TOKEN_2022_PROGRAM_ID is available from spl-token; we also have our constant above.
    mint = await createMint(
      connection,
      master, // payer
      master.publicKey, // mint authority
      null, // freeze authority
      decimals,
      Keypair.generate(),
      { commitment: 'confirmed' },
      TOKEN_2022_PROGRAM_ID // token-2022 program id
    );
    //
    console.log("Mint created:", mint.toBase58());
    console.log(
      "Mint explorer:",
      DEVNET_EXPLORER_ADDR(mint)
    );

    // derive PDAs for categories: use [utf8(categoryName), mintPubkey] as seeds and program id
    for (const cat of CATEGORY_NAMES) {
      const [pda] = PublicKey.findProgramAddressSync(
        [Buffer.from(cat), mint.toBuffer()],
        PROGRAM_ID
      );
      categoryPdas[cat] = pda;

      // derive associated token account for that PDA (owner = pda)
      const ata = await getAssociatedTokenAddress(
        mint,
        pda,
        true,
        TOKEN_2022_PROGRAM_ID,
        ASSOCIATED_TOKEN_PROGRAM_ID
      );
      categoryAtas[cat] = ata;
    }

    // master ATA (master wallet's ATA for this mint)
    masterAta = await getAssociatedTokenAddress(
      mint,
      master.publicKey,
      false,
      TOKEN_2022_PROGRAM_ID,
      ASSOCIATED_TOKEN_PROGRAM_ID
    );

    // Log ATAs and PDAs explorer links
    console.log("Master:", master.publicKey.toBase58());
    console.log("Master ATA:", masterAta.toBase58(), DEVNET_EXPLORER_ADDR(masterAta));
    for (const cat of CATEGORY_NAMES) {
      console.log(
        `${cat} pda:`,
        categoryPdas[cat].toBase58(),
        DEVNET_EXPLORER_ADDR(categoryPdas[cat])
      );
      console.log(
        `${cat} ata:`,
        categoryAtas[cat].toBase58(),
        DEVNET_EXPLORER_ADDR(categoryAtas[cat])
      );
    }
  });

  it("invoke initialize", async () => {
    // Build accounts object according to IDL instruction `initialize`
    // IDL expects many accounts. Map categories accordingly.
    // Note: some accounts in IDL are named slightly differently (preSeed vs preseed). We'll match the IDL names.
    const accounts: Record<string, PublicKey> = {
      master: master.publicKey,
      masterAta: masterAta,
      preSeedCat: categoryPdas["preseed"],
      preSeedAta: categoryAtas["preseed"],
      seedCat: categoryPdas["seed"],
      seedAta: categoryAtas["seed"],
      institutionalCat: categoryPdas["institutional"],
      institutionalAta: categoryAtas["institutional"],
      vgpCat: categoryPdas["vgp"],
      vgpAta: categoryAtas["vgp"],
      marketingCat: categoryPdas["marketing"],
      marketingAta: categoryAtas["marketing"],
      foundersCat: categoryPdas["founders"],
      foundersAta: categoryAtas["founders"],
      reserveCat: categoryPdas["reserve"],
      reserveAta: categoryAtas["reserve"],
      liquidityCat: categoryPdas["liquidity"],
      liquidityAta: categoryAtas["liquidity"],
      mint,
      tokenProgram: TOKEN_2022_PROGRAM_ID,
      associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
      systemProgram: SYSTEM_PROGRAM_PID,
    };

    const computeIx = ComputeBudgetProgram.setComputeUnitLimit({ units: 400_000 });

    // Call initialize
    try {
      const sig = await program.methods.initialize().accounts(accounts).preInstructions([computeIx]).signers([master]).rpc({ commitment: 'confirmed' });
      console.log("initialize tx:", DEVNET_EXPLORER_TX(sig));
      try {
        const marketingCat = await program.account.categoryData.fetch(categoryPdas["marketing"]);
        console.log("Marketing PDA fetch succeded:", marketingCat);
      } catch (e: any) {
        console.log("Marketing PDA fetch failed:", e);
      }
    } catch (e: any) {
      console.log(e);
      throw e;
    }
  });

  it("invoke tge: mints to master and transfers to marketing & liquidity; check mint authority", async () => {
    // build accounts for tge (see IDL)
    const accounts: Record<string, PublicKey> = {
      master: master.publicKey,
      masterAta, // master ATA PDA (IDl names it masterAta pda)
      marketingCat: categoryPdas["marketing"],
      marketingAta: categoryAtas["marketing"],
      liquidityCat: categoryPdas["liquidity"],
      liquidityAta: categoryAtas["liquidity"],
      reserveCat: categoryPdas["reserve"],
      reserveAta: categoryAtas["reserve"],
      mint,
      tokenProgram: TOKEN_2022_PROGRAM_ID,
      associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
      systemProgram: SYSTEM_PROGRAM_PID,
    };

    const computeIx = ComputeBudgetProgram.setComputeUnitLimit({ units: 400_000 });

    try {
      const sig = await program.methods.tge().preInstructions([]).accounts(accounts).signers([master]).rpc({ commitment: 'confirmed' });
      console.log("tge tx:", DEVNET_EXPLORER_TX(sig));
    } catch (e: any) {
      console.log(e);
    }

    // fetch mint info
    const mintInfo = await getMint(connection, mint, 'confirmed', TOKEN_2022_PROGRAM_ID);
    // mintAuthority may have been set to null after tge (Anchor error code indicates "TGE already happened or wrong mint authority")
    // getMint returns null for mintAuthority if none
    console.log("mintAuthority (post-tge):", String(mintInfo.mintAuthority));
    assert.notDeepEqual(
      mintInfo.mintAuthority?.toBase58?.(),
      master.publicKey.toBase58(),
      "master should no longer be mint authority"
    );

    // fetch balances for master, marketing, liquidity ATAs
    const masterAcc = await getAccount(connection, masterAta, "confirmed", TOKEN_2022_PROGRAM_ID);
    const marketingAcc = await getAccount(connection, categoryAtas["marketing"], "confirmed", TOKEN_2022_PROGRAM_ID);
    const liquidityAcc = await getAccount(connection, categoryAtas["liquidity"], "confirmed", TOKEN_2022_PROGRAM_ID);

    console.log("master ATA balance (raw):", masterAcc.amount.toString());
    console.log("marketing ATA balance (raw):", marketingAcc.amount.toString());
    console.log("liquidity ATA balance (raw):", liquidityAcc.amount.toString());

    // simple sanity: master must have non-zero balance after mint, and marketing & liquidity got some tokens
    assert(masterAcc.amount > BigInt(0), "master ATA should have tokens after tge");
    assert(marketingAcc.amount >= BigInt(0), "marketing ATA exists");
    assert(liquidityAcc.amount >= BigInt(0), "liquidity ATA exists");
  });

  it("invoke transferCategoryVestings 48 times and track balances", async () => {
    const getBalances = async () => {
      const out = {};
      for (const cat of CATEGORY_NAMES) {
        try {
          const acc = await getAccount(connection, categoryAtas[cat], "confirmed", TOKEN_2022_PROGRAM_ID);
          const category = await program.account.categoryData.fetch(categoryPdas[cat]);
          out[cat] = {
            amount: acc.amount, pda: {
              cliffMonthsRemaining: category.cliffMonthsRemaining,
              vestingMonthsRemaining: category.vestingMonthsRemaining
            }
          };
        } catch {
          out[cat] = "error";
        }
      }
      try {
        const acc = await getAccount(connection, masterAta, "confirmed", TOKEN_2022_PROGRAM_ID);
        out["master"] = acc.amount;
      } catch {
        out["master"] = BigInt(0);
      }
      return out;
    };

    const beforeBalances = await getBalances();
    console.log("Balances before transferCategoryVestings:", beforeBalances);

    for (let i = 0; i < 48; i++) {
      const accounts: Record<string, PublicKey> = {
        master: master.publicKey,
        masterAta: masterAta,
        preSeedCat: categoryPdas["preseed"],
        preSeedAta: categoryAtas["preseed"],
        seedCat: categoryPdas["seed"],
        seedAta: categoryAtas["seed"],
        institutionalCat: categoryPdas["institutional"],
        institutionalAta: categoryAtas["institutional"],
        vgpCat: categoryPdas["vgp"],
        vgpAta: categoryAtas["vgp"],
        marketingCat: categoryPdas["marketing"],
        marketingAta: categoryAtas["marketing"],
        foundersCat: categoryPdas["founders"],
        foundersAta: categoryAtas["founders"],
        reserveCat: categoryPdas["reserve"],
        reserveAta: categoryAtas["reserve"],
        liquidityCat: categoryPdas["liquidity"],
        liquidityAta: categoryAtas["liquidity"],
        mint,
        tokenProgram: TOKEN_2022_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        systemProgram: SYSTEM_PROGRAM_PID,
      };
      console.log("\n");

      const computeIx = ComputeBudgetProgram.setComputeUnitLimit({ units: 400_000 });

      const sig = await program.methods.transferCategoryVestings().preInstructions([]).accounts(accounts).signers([master]).rpc({ commitment: 'confirmed' });
      console.log(`transferCategoryVestings #${i + 1} tx:`, DEVNET_EXPLORER_TX(sig));
      console.log(await getBalances());
      console.log("\n");
    }

    // No strict asserts beyond ensuring the test completed — but ensure function ran
    assert.ok(true);
  });
});
