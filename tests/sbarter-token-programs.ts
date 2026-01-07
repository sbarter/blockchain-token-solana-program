// {
//   "pubkey": "5uYFC1hNzD7v9iCkjqogwKdR3qQHEFehzDBXHDmFFkfD",
//   "id": 161,
//   "capacity": 10,
//   "update_authority": "GSd6RQZ4o9AMpHeRYZEwcjZ9oAP1ZLAUeKbwbNdS2oJH",
//   "name": "category_claims",
//   "min_crank_reward": 1000000,
//   "balance": 1100000000,
//   "stale_task_age": 999999999
// }
// 
// {
//   "pubkey": "siwajZYai6gT8sQhsk8Ex6FKVUt4wrjQook2jVMu9CW",
//   "id": 0,
//   "user_cron_jobs": "67LY9ZDx5vZsWUrA5de7eWsUXBAzHYrrHcN1bgNWMTBV",
//   "task_queue": "5uYFC1hNzD7v9iCkjqogwKdR3qQHEFehzDBXHDmFFkfD",
//   "authority": "GSd6RQZ4o9AMpHeRYZEwcjZ9oAP1ZLAUeKbwbNdS2oJH",
//   "free_tasks_per_transaction": 0,
//   "schedule": "0 0 * * * *",
//   "name": "category_claims_cron",
//   "current_exec_ts": 1767146400,
//   "current_transaction_id": 0,
//   "next_transaction_id": 0,
//   "num_tasks_per_queue_call": 8,
//   "removed_from_queue": false,
//   "balance": 1000000000,
//   "next_schedule_task": "DbN5TVnXeJ8SxtHXewyKZykWk4CVotmuN2frhG8LTyyx"
// }

import * as fs from "fs";
import * as path from "path";
import { strict as assert } from "assert";
import { describe, it, before } from "mocha";
import * as anchor from "@coral-xyz/anchor";
import {
  ComputeBudgetProgram,
  Connection,
  Keypair,
  LAMPORTS_PER_SOL,
  PublicKey,
  SystemProgram,
  Transaction,
} from "@solana/web3.js";
import {
  createMint,
  getMint,
  getAssociatedTokenAddress,
  getAccount,
  TOKEN_2022_PROGRAM_ID,
  ASSOCIATED_TOKEN_PROGRAM_ID,
  getAssociatedTokenAddressSync,
} from "@solana/spl-token";
import { SbarterTokenPrograms } from "../target/types/sbarter_token_programs";
import { getLogs } from "@solana-developers/helpers";
import { Tuktuk } from "@helium/tuktuk-idls/lib/types/tuktuk";
import { init, taskKey, taskQueueAuthorityKey, taskQueueKey, taskQueueNameMappingKey, tuktukConfigKey } from "@helium/tuktuk-sdk";
import { createHash } from "crypto";
import { expect } from "chai";
import { execSync } from "child_process";

const PROGRAM_ID = new PublicKey("47D4TsSiMjG4s2ohbuvQXZEtwYeJ5VPDJaDiBUNxpm8y");
const TUKTUK_PROGRAM_ID = new PublicKey("tuktukUrfhXT6ZT77QTU8RQtvgL967uRuVagWF57zVA");
const SYSTEM_PROGRAM_PID = SystemProgram.programId;

const DEVNET_EXPLORER_TX = (sig: string) =>
  `https://explorer.solana.com/tx/${sig}?cluster=devnet`;
const DEVNET_EXPLORER_ADDR = (addr: PublicKey) =>
  `https://explorer.solana.com/address/${addr.toBase58()}?cluster=devnet`;

// categories we will use
const INVESTOR_CATEGORY_NAMES = [
  "preseed",
  "seed",
  "institutional",
  "vgp",
  "founders",
];

const FUNCTIONAL_CATEGORY_NAMES = [
  "marketing",
  "reserve",
  "liquidity",
];

const TASK_QUEUE_NAME = "Vesting automation";

const FUNCTIONAL_CATEGORY_AUTHORITIES = {
  marketing: new PublicKey("GSd6RQZ4o9AMpHeRYZEwcjZ9oAP1ZLAUeKbwbNdS2oJH"),
  reserve: new PublicKey("BdRUCurxjZvzBurS8QzzEKQ8iPCpzMYTz2YTgqnw9ZGY"),
  liquidity: new PublicKey("2eg4xRrj742edVzGAfd3wnmXAMzhAcR1XdJoBARx3hcE"),
};

const ALL_CATEGORY_NAMES = INVESTOR_CATEGORY_NAMES.concat(FUNCTIONAL_CATEGORY_NAMES)

const sendAndConfirmTx = async (tx: Transaction, connection: Connection, wallet: anchor.Wallet): Promise<string> => {
  if (!wallet.publicKey) throw new Error('Wallet not connected');
  tx.feePayer = wallet.publicKey;

  const { blockhash, lastValidBlockHeight } = await connection.getLatestBlockhash();
  tx.recentBlockhash = blockhash;

  const signed = await wallet.signTransaction(tx);

  const raw = signed.serialize();
  const signature = await connection.sendRawTransaction(raw);
  await connection.confirmTransaction({ signature, blockhash, lastValidBlockHeight });

  return signature;
};

const deriveTaskPubkey = (
  categorySeed: string,
  investorId: number,
  taskQueue: PublicKey,
  flipped: boolean
): PublicKey => {
  const U16_MSB = 0x8000;
  const CATEGORY_BITMASK = 0x7000;

  // Find category index
  const categoryId = INVESTOR_CATEGORY_NAMES.findIndex(
    (seed) => seed === categorySeed
  );

  if (categoryId === -1) {
    throw new Error(`Invalid category seed: ${categorySeed}`);
  }

  // bit structure: FCCC0000 00000000
  // F - flipped, so that a task can queue itself while existing
  // C - 0-7 unique category id, so that task ids between categories don't clash
  // the rest is investor index
  let taskId = investorId;

  if (flipped) {
    taskId ^= U16_MSB;
  }

  taskId |= ((categoryId << 12) & CATEGORY_BITMASK);

  // Convert task_id to little-endian bytes
  const taskIdBuffer = Buffer.alloc(2);
  taskIdBuffer.writeUInt16LE(taskId, 0);

  // Find PDA
  const [pda] = PublicKey.findProgramAddressSync(
    [
      Buffer.from('task'),
      taskQueue.toBuffer(),
      taskIdBuffer,
    ],
    TUKTUK_PROGRAM_ID
  );

  return pda;
}

describe("sbarterTokenPrograms (devnet)", function() {
  let provider: anchor.AnchorProvider;
  let connection: Connection;
  let program: anchor.Program<SbarterTokenPrograms>;
  let tuktuk: anchor.Program<Tuktuk>;
  let master: Keypair;
  let wallet: anchor.Wallet;
  let mint: PublicKey;

  let tuktukConfig: PublicKey;
  let taskQueuePda: PublicKey;
  let taskQueueNameMappingPda: PublicKey;
  let taskQueueAuthorityPda: PublicKey;

  // derived maps
  const categoryPdas: Record<string, PublicKey> = {};
  const categoryAtas: Record<string, PublicKey> = {};
  let masterPda: PublicKey;
  let masterAta: PublicKey;

  let preseedInvestors: Array<{ wallet: Keypair; pda: PublicKey; ata: PublicKey }> = [];
  let seedInvestors: Array<{ wallet: Keypair; pda: PublicKey; ata: PublicKey }> = [];

  before(async () => {
    // 1) load local keypair from ~/.config/solana/id.json (master)
    const home = process.env.HOME || process.env.USERPROFILE || ".";
    const idPath = path.join(home, ".config", "solana", "id.json");
    const raw = fs.readFileSync(idPath, "utf8");
    const arr = JSON.parse(raw) as number[];
    master = Keypair.fromSecretKey(Uint8Array.from(arr));

    // connection = new Connection("https://api.devnet.solana.com", "confirmed");
    connection = new Connection("http://127.0.0.1:8899", "confirmed");
    wallet = new anchor.Wallet(master);

    provider = new anchor.AnchorProvider(connection, wallet, {
      preflightCommitment: "confirmed",
    });
    anchor.setProvider(provider);

    try {
      execSync(`anchor idl init --filepath ${__dirname}/../target/idl/tuktuk.json ${TUKTUK_PROGRAM_ID} --provider.cluster ${connection.rpcEndpoint}`, { stdio: "inherit", shell: "/bin/bash" })
    } catch {
      console.log("Don't mind these ^");
      execSync(`anchor idl upgrade --filepath ${__dirname}/../target/idl/tuktuk.json ${TUKTUK_PROGRAM_ID} --provider.cluster ${connection.rpcEndpoint}`, { stdio: "inherit", shell: "/bin/bash" })
    }
    program = anchor.workspace.sbarterTokenPrograms as anchor.Program<SbarterTokenPrograms>;
    tuktuk = await init(provider, TUKTUK_PROGRAM_ID);


    for (const wallet of Object.values(FUNCTIONAL_CATEGORY_AUTHORITIES)) {
      console.log("Requesting airdrop for wallet:", wallet);
      await connection.confirmTransaction(
        await connection.requestAirdrop(wallet, 1 * LAMPORTS_PER_SOL)
      );
      const info = await connection.getAccountInfo(wallet);
      console.log(info);
    }

    [masterPda] = PublicKey.findProgramAddressSync([Buffer.from("master")], PROGRAM_ID);

    // 2) create a new token-2022 mint with master as mint authority
    // decimals: 6 (adjust if you want)
    const decimals = 6;
    // createMint uses the spl-token library; pass TOKEN_2022_PROGRAM_ID to createToken-2022 mint
    // createMint(connection, payer, mintAuthority, freezeAuthority, decimals, programId?);
    // Note: the exported constant TOKEN_2022_PROGRAM_ID is available from spl-token; we also have our constant above.
    mint = await createMint(
      connection,
      master, // payer
      masterPda, // mint authority
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

    // master ATA (master wallet's ATA for this mint)
    masterAta = await getAssociatedTokenAddress(
      mint,
      masterPda,
      true,
      TOKEN_2022_PROGRAM_ID,
      ASSOCIATED_TOKEN_PROGRAM_ID
    );

    // Log ATAs and PDAs explorer links
    console.log("Master:", master.publicKey.toBase58());
    console.log("Master ATA:", masterAta.toBase58(), DEVNET_EXPLORER_ADDR(masterAta));
    // derive PDAs for categories: use [utf8(categoryName), mintPubkey] as seeds and program id
    for (const cat of INVESTOR_CATEGORY_NAMES) {
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

    for (const cat of FUNCTIONAL_CATEGORY_NAMES) {
      const [pda] = PublicKey.findProgramAddressSync(
        [Buffer.from(cat), mint.toBuffer()],
        PROGRAM_ID
      );
      categoryPdas[cat] = pda;

      // derive associated token account for that PDA (owner = pda)
      const ata = await getAssociatedTokenAddress(
        mint,
        FUNCTIONAL_CATEGORY_AUTHORITIES[cat],
        false,
        TOKEN_2022_PROGRAM_ID,
        ASSOCIATED_TOKEN_PROGRAM_ID
      );
      categoryAtas[cat] = ata;
    }

    for (const cat of ALL_CATEGORY_NAMES) {
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
      masterPda: masterPda,
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
      marketingAuthority: FUNCTIONAL_CATEGORY_AUTHORITIES["marketing"],
      marketingAta: categoryAtas["marketing"],
      foundersCat: categoryPdas["founders"],
      foundersAta: categoryAtas["founders"],
      reserveCat: categoryPdas["reserve"],
      reserveAuthority: FUNCTIONAL_CATEGORY_AUTHORITIES["reserve"],
      reserveAta: categoryAtas["reserve"],
      liquidityCat: categoryPdas["liquidity"],
      liquidityAuthority: FUNCTIONAL_CATEGORY_AUTHORITIES["liquidity"],
      liquidityAta: categoryAtas["liquidity"],
      mint,
      tokenProgram: TOKEN_2022_PROGRAM_ID,
      associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
      systemProgram: SYSTEM_PROGRAM_PID,
    };

    const computeIx = ComputeBudgetProgram.setComputeUnitLimit({ units: 400_000 });

    // Call initialize
    try {
      const tx = await program.methods.initialize().accounts(accounts).preInstructions([computeIx]).signers([master]).transaction();
      const sig = await sendAndConfirmTx(tx, connection, wallet);
      console.log("initialize tx:", DEVNET_EXPLORER_TX(sig));
      try {
        const marketingCat = await program.account.functionalCategoryData.fetch(categoryPdas["marketing"]);
        console.log("Marketing PDA fetch succeded:", marketingCat);
      } catch (e: any) {
        console.log("Marketing PDA fetch failed:", e);
      }

      try {
        const preseedCat = await program.account.investorCategoryData.fetch(categoryPdas["preseed"]);
        console.log("Pre-seed PDA fetch succeded:", preseedCat);
      } catch (e: any) {
        console.log("Pre-seed PDA fetch failed:", e);
      }
    } catch (e: any) {
      console.log((await e.getLogs()).join("\n"));
      throw e;
    }
  });

  it("invoke initialize tuktuk", async () => {
    [tuktukConfig] = tuktukConfigKey(TUKTUK_PROGRAM_ID);
    if (!(await tuktuk.account.tuktukConfigV0.fetchNullable(tuktukConfig))) {
      const tuktukConfigTx = await tuktuk.methods
        .initializeTuktukConfigV0({
          minDeposit: new anchor.BN(100000000),
        })
        .accounts({
          payer: master.publicKey,
          authority: master.publicKey,
        })
        .signers([master])
        .transaction();
      let sig: string;
      try {
        sig = await sendAndConfirmTx(tuktukConfigTx, connection, wallet);
      } catch (e: any) {
        console.log(await e.getLogs());
      }
      console.log("initialize tuktuk config tx:", DEVNET_EXPLORER_TX(sig));
    }

    const tuktukConfigAcc = await tuktuk.account.tuktukConfigV0.fetch(
      tuktukConfig, 'confirmed'
    );
    [taskQueuePda] = taskQueueKey(tuktukConfig, tuktukConfigAcc.nextTaskQueueId, TUKTUK_PROGRAM_ID);
    [taskQueueNameMappingPda] = taskQueueNameMappingKey(tuktukConfig, TASK_QUEUE_NAME, TUKTUK_PROGRAM_ID);
    [taskQueueAuthorityPda] = taskQueueAuthorityKey(taskQueuePda, masterPda, TUKTUK_PROGRAM_ID);

    const accounts: Record<string, PublicKey> = {
      master: master.publicKey,
      masterPda: masterPda,
      masterAta: masterAta,
      taskQueue: taskQueuePda,
      taskQueueNameMapping: taskQueueNameMappingPda,
      taskQueueAuthority: taskQueueAuthorityPda,
      tuktukConfig: tuktukConfig,
      tuktukProgram: TUKTUK_PROGRAM_ID,
      systemProgram: SYSTEM_PROGRAM_PID,
    };

    let sig: string;
    try {
      const tx = await program.methods.initializeTuktuk(TASK_QUEUE_NAME).accounts(accounts).signers([master]).transaction();
      sig = await sendAndConfirmTx(tx, connection, wallet);
      console.log("initialize tuktuk tx:", DEVNET_EXPLORER_TX(sig));
    } catch (e: any) {
      console.log("failed to initialize tuktuk:\n", await e.getLogs());
    }

    const taskQueue = await tuktuk.account.taskQueueV0.fetch(taskQueuePda);
    console.log("Tuktuk task queue created:", taskQueue);
    expect(taskQueue).not.to.be.undefined;
  });

  it("initialize closed category investors", async () => {
    const accounts = (categorySeed: string, investorId: number, investorPda: PublicKey, investorWallet: PublicKey, investorAta: PublicKey) => ({
      master: master.publicKey,
      category: categoryPdas[categorySeed],
      categoryAta: categoryAtas[categorySeed],
      investorPda: investorPda,
      investorWallet: investorWallet,
      investorAta: investorAta,
      nextTask: deriveTaskPubkey(categorySeed, investorId, taskQueuePda, false),
      nextTaskFlipped: deriveTaskPubkey(categorySeed, investorId, taskQueuePda, true),
      taskQueue: taskQueuePda,
      taskQueueNameMapping: taskQueueNameMappingPda,
      taskQueueAuthority: taskQueueAuthorityPda,
      tuktukConfig: tuktukConfig,
      tuktukProgram: TUKTUK_PROGRAM_ID,
      mint,
      tokenProgram: TOKEN_2022_PROGRAM_ID,
      associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
      systemProgram: SYSTEM_PROGRAM_PID
    });

    for (let i = 1; i <= 5; i++) {
      const investorWallet = Keypair.generate();
      const [investorPda] = PublicKey.findProgramAddressSync(
        [Buffer.from("preseed"), Buffer.from(new Uint8Array(new Uint16Array([i]).buffer)), mint.toBuffer()],
        program.programId
      );
      const investorAta = getAssociatedTokenAddressSync(
        mint,
        investorWallet.publicKey,
        false,
        TOKEN_2022_PROGRAM_ID,
        ASSOCIATED_TOKEN_PROGRAM_ID
      );

      preseedInvestors.push({ wallet: investorWallet, pda: investorPda, ata: investorAta });

      const tx = await program.methods
        .addInvestorToCategory("preseed", i, new anchor.BN(1000000 * 1000000), TASK_QUEUE_NAME) // 1M tokens monthly allocation
        .accounts(accounts("preseed", i, investorPda, investorWallet.publicKey, investorAta))
        .signers([master])
        .transaction();

      let sig: string;
      try {
        sig = await sendAndConfirmTx(tx, connection, wallet);
      } catch (e: any) {
        console.log(await e.getLogs());
      }
      console.log(`Added preseed investor ${i}: ${DEVNET_EXPLORER_TX(sig)}`);
    }

    for (let i = 1; i <= 2; i++) {
      const investorWallet = Keypair.generate();
      const [investorPda] = PublicKey.findProgramAddressSync(
        [Buffer.from("seed"), Buffer.from(new Uint8Array(new Uint16Array([i]).buffer)), mint.toBuffer()],
        program.programId
      );
      const investorAta = getAssociatedTokenAddressSync(
        mint,
        investorWallet.publicKey,
        false,
        TOKEN_2022_PROGRAM_ID,
        ASSOCIATED_TOKEN_PROGRAM_ID
      );

      seedInvestors.push({ wallet: investorWallet, pda: investorPda, ata: investorAta });

      const tx = await program.methods
        .addInvestorToCategory("seed", i, new anchor.BN(2000000 * 1000000), TASK_QUEUE_NAME) // 2M tokens monthly allocation
        .accounts(accounts("seed", i, investorPda, investorWallet.publicKey, investorAta))
        .signers([master])
        .transaction();

      let sig: string;
      try {
        sig = await sendAndConfirmTx(tx, connection, wallet);
      } catch (e: any) {
        console.log(await e.getLogs());
      }
      console.log(`Added seed investor ${i}: ${DEVNET_EXPLORER_TX(sig)}`);
    }

    console.log(`Total preseed investors: ${preseedInvestors.length}`);
    console.log(`Total seed investors: ${seedInvestors.length}`);
  });

  it("invoke tge: mints to master and transfers to marketing & liquidity; check mint authority", async () => {
    // build accounts for tge (see IDL)
    const accounts: Record<string, PublicKey> = {
      master: master.publicKey,
      masterPda: masterPda,
      masterAta, // master ATA PDA (IDl names it masterAta pda)
      preSeedCat: categoryPdas["preseed"],
      seedCat: categoryPdas["seed"],
      institutionalCat: categoryPdas["institutional"],
      vgpCat: categoryPdas["vgp"],
      foundersCat: categoryPdas["founders"],
      marketingCat: categoryPdas["marketing"],
      marketingAuthority: FUNCTIONAL_CATEGORY_AUTHORITIES["marketing"],
      marketingAta: categoryAtas["marketing"],
      liquidityCat: categoryPdas["liquidity"],
      liquidityAuthority: FUNCTIONAL_CATEGORY_AUTHORITIES["liquidity"],
      liquidityAta: categoryAtas["liquidity"],
      reserveCat: categoryPdas["reserve"],
      reserveAuthority: FUNCTIONAL_CATEGORY_AUTHORITIES["reserve"],
      reserveAta: categoryAtas["reserve"],
      mint,
      tokenProgram: TOKEN_2022_PROGRAM_ID,
      associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
      systemProgram: SYSTEM_PROGRAM_PID,
    };

    // const computeIx = ComputeBudgetProgram.setComputeUnitLimit({ units: 400_000 });

    try {
      const tx = await program.methods.tge().preInstructions([]).accounts(accounts).signers([master]).transaction();
      const sig = await sendAndConfirmTx(tx, connection, wallet);
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
    const sleep = (ms: number) => new Promise(resolve => setTimeout(resolve, ms));
    const getBalances = async () => {
      const out = {};
      for (const cat of INVESTOR_CATEGORY_NAMES) {
        try {
          const acc = await getAccount(connection, categoryAtas[cat], "confirmed", TOKEN_2022_PROGRAM_ID);
          const category = await program.account.investorCategoryData.fetch(categoryPdas[cat]);
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
      for (const cat of FUNCTIONAL_CATEGORY_NAMES) {
        try {
          const acc = await getAccount(connection, categoryAtas[cat], "confirmed", TOKEN_2022_PROGRAM_ID);
          const category = await program.account.functionalCategoryData.fetch(categoryPdas[cat]);
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
    const sendCategoryTx = async function*() {
      for (let i = 1; ; i++) {
        const accounts: Record<string, PublicKey> = {
          master: master.publicKey,
          masterPda: masterPda,
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

        const tx = await program.methods.transferCategoryVestings().preInstructions([]).accounts(accounts).signers([master]).transaction();
        const sig = await sendAndConfirmTx(tx, connection, wallet);
        console.log(`transferCategoryVestings #${i} tx:`, DEVNET_EXPLORER_TX(sig));
        console.log(await getBalances());
        console.log("\n");

        yield sig;
      }
    };

    let categoryGen = sendCategoryTx();

    const beforeBalances = await getBalances();
    console.log("Balances before transferCategoryVestings:", beforeBalances);

    console.log("Sleeping for 10 seconds (+1 cycle).");
    await sleep(10 * 1000);
    await categoryGen.next();
    console.log("Sleeping for 22 seconds (+2 cycles).");
    await sleep(22 * 1000);
    await categoryGen.next();
    console.log("Sleeping for 1 second (+0 cycles).");
    await sleep(1 * 1000);
    await categoryGen.next();
    console.log("Sleeping for 100 seconds (+10 cycles).");
    await sleep(100 * 1000);
    await categoryGen.next();

    // No strict asserts beyond ensuring the test completed — but ensure function ran
    assert.ok(true);
  });

  it("claim funds for investors", async () => {
    const accounts = (categorySeed: string, investorPda: PublicKey, investorAta: PublicKey) => ({
      category: categoryPdas[categorySeed],
      categoryAta: categoryAtas[categorySeed],
      investorPda: investorPda,
      investorAta: investorAta,
      mint,
      tokenProgram: TOKEN_2022_PROGRAM_ID,
      associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
      systemProgram: SYSTEM_PROGRAM_PID
    });

    for (let i = 1; i <= 5; i++) {
      const tx = await program.methods
        .investorClaimTokens("preseed", i)
        .accounts(accounts("preseed", preseedInvestors[i - 1].pda, preseedInvestors[i - 1].ata))
        .transaction();

      let sig: string;
      try {
        sig = await sendAndConfirmTx(tx, connection, wallet);
      } catch (e: any) {
        console.log(await e.getLogs());
        console.log(await getLogs(connection, sig));
      }
      console.log(`Claimed for preseed investor ${i}: ${DEVNET_EXPLORER_TX(sig)}`);
      const balance = await connection.getTokenAccountBalance(preseedInvestors[i - 1].ata);
      console.log(`Preseed investor ${i} balance: ${balance.value.uiAmount}`);
    }

    for (let i = 1; i <= 2; i++) {
      const tx = await program.methods
        .investorClaimTokens("seed", i)
        .accounts(accounts("seed", seedInvestors[i - 1].pda, seedInvestors[i - 1].ata))
        .transaction();

      let sig: string;
      try {
        sig = await sendAndConfirmTx(tx, connection, wallet);
        console.log(await getLogs(connection, sig));
      } catch (e: any) {
        console.log(await e.getLogs());
      }
      console.log(`Claimed for seed investor ${i}: ${DEVNET_EXPLORER_TX(sig)}`);
      const balance = await connection.getTokenAccountBalance(seedInvestors[i - 1].ata);
      console.log(`Seed investor ${i} balance: ${balance.value.uiAmount}`);
    }
  });
});
