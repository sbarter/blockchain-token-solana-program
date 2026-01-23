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
  Signer,
  SystemProgram,
  SYSVAR_INSTRUCTIONS_PUBKEY,
  Transaction,
} from "@solana/web3.js";
import {
  getMint,
  getAssociatedTokenAddress,
  getAccount,
  TOKEN_2022_PROGRAM_ID,
  ASSOCIATED_TOKEN_PROGRAM_ID,
  getAssociatedTokenAddressSync,
} from "@solana/spl-token";
import { SbarterTokenPrograms } from "../target/types/sbarter_token_programs";

const MPL_TOKEN_METADATA_PROGRAM_ID = new PublicKey("metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s");
const SYSTEM_PROGRAM_ID = SystemProgram.programId;

const DEVNET_EXPLORER_TX = (sig: string) =>
  `https://explorer.solana.com/tx/${sig}?cluster=devnet`;
const DEVNET_EXPLORER_ADDR = (addr: PublicKey) =>
  `https://explorer.solana.com/address/${addr.toBase58()}?cluster=devnet`;

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

const FUNCTIONAL_CATEGORY_AUTHORITIES = {
  marketing: new PublicKey("GSd6RQZ4o9AMpHeRYZEwcjZ9oAP1ZLAUeKbwbNdS2oJH"),
  reserve: new PublicKey("BdRUCurxjZvzBurS8QzzEKQ8iPCpzMYTz2YTgqnw9ZGY"),
  liquidity: new PublicKey("2eg4xRrj742edVzGAfd3wnmXAMzhAcR1XdJoBARx3hcE"),
};

const ALL_CATEGORY_NAMES = INVESTOR_CATEGORY_NAMES.concat(FUNCTIONAL_CATEGORY_NAMES)

const sendAndConfirmTx = async (
  tx: Transaction,
  connection: Connection,
  wallet: anchor.Wallet,
  additionalSigners: Signer[] = []
): Promise<string> => {
  if (!wallet.publicKey) throw new Error('Wallet not connected');
  tx.feePayer = wallet.publicKey;

  const { blockhash, lastValidBlockHeight } = await connection.getLatestBlockhash();
  tx.recentBlockhash = blockhash;

  if (additionalSigners.length > 0) {
    tx.partialSign(...additionalSigners);
  }

  const signed = await wallet.signTransaction(tx);

  const raw = signed.serialize();
  const signature = await connection.sendRawTransaction(raw);
  await connection.confirmTransaction({ signature, blockhash, lastValidBlockHeight });

  return signature;
};

class TestContext {
  provider: anchor.AnchorProvider;
  connection: Connection;
  program: anchor.Program<SbarterTokenPrograms>;
  master: Keypair;
  wallet: anchor.Wallet;
  mintKeypair: Keypair;
  mint: PublicKey;
  categoryPdas: Record<string, PublicKey> = {};
  categoryAtas: Record<string, PublicKey> = {};
  masterPda: PublicKey;
  masterAta: PublicKey;
  preseedInvestors: Array<{ wallet: Keypair; pda: PublicKey; ata: PublicKey }> = [];
  seedInvestors: Array<{ wallet: Keypair; pda: PublicKey; ata: PublicKey }> = [];

  async executeInstruction(
    name: string,
    buildTx: () => Promise<Transaction>,
    additionalSigners: Signer[] = []
  ): Promise<string> {
    try {
      const tx = await buildTx();
      const sig = await sendAndConfirmTx(tx, this.connection, this.wallet, additionalSigners);
      console.log(`${name} tx:`, DEVNET_EXPLORER_TX(sig));
      return sig;
    } catch (e: any) {
      console.error(`${name} failed:`, e.message);
      if (e.getLogs) {
        const logs = await e.getLogs();
        console.error(logs.join("\n"));
      }
      throw e;
    }
  }

  async initializeMint(): Promise<string> {
    const [metadata] = PublicKey.findProgramAddressSync([
      Buffer.from("metadata"),
      MPL_TOKEN_METADATA_PROGRAM_ID.toBuffer(),
      this.mint.toBuffer()],
      MPL_TOKEN_METADATA_PROGRAM_ID);

    return this.executeInstruction(
      "initialize_mint",
      () => this.program.methods.initializeMint().accountsStrict({
        master: this.master.publicKey,
        masterPda: this.masterPda,
        metadata,
        mint: this.mint,
        mplMetadataProgram: MPL_TOKEN_METADATA_PROGRAM_ID,
        tokenProgram: TOKEN_2022_PROGRAM_ID,
        sysvarInstructions: SYSVAR_INSTRUCTIONS_PUBKEY,
        systemProgram: SYSTEM_PROGRAM_ID,
      }).signers([this.master, this.mintKeypair]).transaction(),
      [this.mintKeypair]
    );
  }

  async initialize(): Promise<string> {
    const accounts: Record<string, PublicKey> = {
      master: this.master.publicKey,
      masterPda: this.masterPda,
      masterAta: this.masterAta,
      preSeedCat: this.categoryPdas["preseed"],
      preSeedAta: this.categoryAtas["preseed"],
      seedCat: this.categoryPdas["seed"],
      seedAta: this.categoryAtas["seed"],
      institutionalCat: this.categoryPdas["institutional"],
      institutionalAta: this.categoryAtas["institutional"],
      vgpCat: this.categoryPdas["vgp"],
      vgpAta: this.categoryAtas["vgp"],
      marketingCat: this.categoryPdas["marketing"],
      marketingAuthority: FUNCTIONAL_CATEGORY_AUTHORITIES["marketing"],
      marketingAta: this.categoryAtas["marketing"],
      foundersCat: this.categoryPdas["founders"],
      foundersAta: this.categoryAtas["founders"],
      reserveCat: this.categoryPdas["reserve"],
      reserveAuthority: FUNCTIONAL_CATEGORY_AUTHORITIES["reserve"],
      reserveAta: this.categoryAtas["reserve"],
      liquidityCat: this.categoryPdas["liquidity"],
      liquidityAuthority: FUNCTIONAL_CATEGORY_AUTHORITIES["liquidity"],
      liquidityAta: this.categoryAtas["liquidity"],
      mint: this.mint,
      tokenProgram: TOKEN_2022_PROGRAM_ID,
      associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
      systemProgram: SYSTEM_PROGRAM_ID,
    };

    const computeIx = ComputeBudgetProgram.setComputeUnitLimit({ units: 400_000 });

    return this.executeInstruction(
      "initialize",
      () => this.program.methods.initialize().accounts(accounts).preInstructions([computeIx]).signers([this.master]).transaction()
    );
  }

  async categoryAddInvestor(
    categorySeed: string,
    investorIndex: number,
    amount: anchor.BN,
    investorWallet: PublicKey,
    investorPda: PublicKey,
    investorAta: PublicKey
  ): Promise<string> {
    const accounts = {
      master: this.master.publicKey,
      masterPda: this.masterPda,
      category: this.categoryPdas[categorySeed],
      categoryAta: this.categoryAtas[categorySeed],
      investorPda: investorPda,
      investorWallet: investorWallet,
      investorAta: investorAta,
      mint: this.mint,
      tokenProgram: TOKEN_2022_PROGRAM_ID,
      associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
      systemProgram: SYSTEM_PROGRAM_ID
    };

    const computeIx = ComputeBudgetProgram.setComputeUnitLimit({ units: 400_000 });

    return this.executeInstruction(
      `categoryAddInvestor_${categorySeed}_${investorIndex}`,
      () => this.program.methods
        .categoryAddInvestor(categorySeed, investorIndex, amount)
        .accounts(accounts)
        .preInstructions([computeIx])
        .signers([this.master])
        .transaction()
    );
  }

  async tge(): Promise<string> {
    const accounts: Record<string, PublicKey> = {
      master: this.master.publicKey,
      masterPda: this.masterPda,
      masterAta: this.masterAta,
      preSeedCat: this.categoryPdas["preseed"],
      seedCat: this.categoryPdas["seed"],
      institutionalCat: this.categoryPdas["institutional"],
      vgpCat: this.categoryPdas["vgp"],
      foundersCat: this.categoryPdas["founders"],
      marketingCat: this.categoryPdas["marketing"],
      marketingAuthority: FUNCTIONAL_CATEGORY_AUTHORITIES["marketing"],
      marketingAta: this.categoryAtas["marketing"],
      liquidityCat: this.categoryPdas["liquidity"],
      liquidityAuthority: FUNCTIONAL_CATEGORY_AUTHORITIES["liquidity"],
      liquidityAta: this.categoryAtas["liquidity"],
      reserveCat: this.categoryPdas["reserve"],
      reserveAuthority: FUNCTIONAL_CATEGORY_AUTHORITIES["reserve"],
      reserveAta: this.categoryAtas["reserve"],
      mint: this.mint,
      tokenProgram: TOKEN_2022_PROGRAM_ID,
      associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
      systemProgram: SYSTEM_PROGRAM_ID,
    };

    return this.executeInstruction(
      "tge",
      () => this.program.methods.tge().preInstructions([]).accounts(accounts).signers([this.master]).transaction()
    );
  }

  async categoryTransferVestings(): Promise<string> {
    const accounts: Record<string, PublicKey> = {
      master: this.master.publicKey,
      masterPda: this.masterPda,
      masterAta: this.masterAta,
      preSeedCat: this.categoryPdas["preseed"],
      preSeedAta: this.categoryAtas["preseed"],
      seedCat: this.categoryPdas["seed"],
      seedAta: this.categoryAtas["seed"],
      institutionalCat: this.categoryPdas["institutional"],
      institutionalAta: this.categoryAtas["institutional"],
      vgpCat: this.categoryPdas["vgp"],
      vgpAta: this.categoryAtas["vgp"],
      marketingCat: this.categoryPdas["marketing"],
      marketingAta: this.categoryAtas["marketing"],
      foundersCat: this.categoryPdas["founders"],
      foundersAta: this.categoryAtas["founders"],
      reserveCat: this.categoryPdas["reserve"],
      reserveAta: this.categoryAtas["reserve"],
      liquidityCat: this.categoryPdas["liquidity"],
      liquidityAta: this.categoryAtas["liquidity"],
      mint: this.mint,
      tokenProgram: TOKEN_2022_PROGRAM_ID,
      associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
      systemProgram: SYSTEM_PROGRAM_ID,
    };

    return this.executeInstruction(
      "categoryTransferVestings",
      () => this.program.methods.categoryTransferVestings().preInstructions([]).accounts(accounts).transaction()
    );
  }

  async investorClaimTokens(categorySeed: string, investorIndex: number, investorPda: PublicKey, investorAta: PublicKey): Promise<string> {
    const accounts = {
      category: this.categoryPdas[categorySeed],
      categoryAta: this.categoryAtas[categorySeed],
      investorPda: investorPda,
      investorAta: investorAta,
      mint: this.mint,
      tokenProgram: TOKEN_2022_PROGRAM_ID,
      associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
      systemProgram: SYSTEM_PROGRAM_ID
    };

    return this.executeInstruction(
      `investorClaimTokens_${categorySeed}_${investorIndex}`,
      () => this.program.methods
        .investorClaimTokens(categorySeed, investorIndex)
        .accounts(accounts)
        .transaction()
    );
  }

  async getBalances() {
    const out = {};
    for (const cat of INVESTOR_CATEGORY_NAMES) {
      try {
        const acc = await getAccount(this.connection, this.categoryAtas[cat], "confirmed", TOKEN_2022_PROGRAM_ID);
        const category = await this.program.account.investorCategoryData.fetch(this.categoryPdas[cat]);
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
        const acc = await getAccount(this.connection, this.categoryAtas[cat], "confirmed", TOKEN_2022_PROGRAM_ID);
        const category = await this.program.account.functionalCategoryData.fetch(this.categoryPdas[cat]);
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
      const acc = await getAccount(this.connection, this.masterAta, "confirmed", TOKEN_2022_PROGRAM_ID);
      out["master"] = acc.amount;
    } catch {
      out["master"] = BigInt(0);
    }
    return out;
  }
}

describe("sbarterTokenPrograms", function() {
  let ctx: TestContext;

  before(async () => {
    ctx = new TestContext();

    const home = process.env.HOME || process.env.USERPROFILE || ".";
    const idPath = path.join(home, ".config", "solana", "id.json");
    const raw = fs.readFileSync(idPath, "utf8");
    const arr = JSON.parse(raw) as number[];
    ctx.master = Keypair.fromSecretKey(Uint8Array.from(arr));

    // ctx.connection = new Connection("https://devnet.helius-rpc.com/?api-key=YOUR-KEY", "confirmed");
    ctx.connection = new Connection("http://127.0.0.1:8899", "confirmed");
    ctx.wallet = new anchor.Wallet(ctx.master);

    ctx.provider = new anchor.AnchorProvider(ctx.connection, ctx.wallet, {
      preflightCommitment: "confirmed",
    });
    anchor.setProvider(ctx.provider);
    ctx.program = anchor.workspace.sbarterTokenPrograms as anchor.Program<SbarterTokenPrograms>;

    [ctx.masterPda] = PublicKey.findProgramAddressSync([Buffer.from("master")], ctx.program.programId);
    ctx.mintKeypair = Keypair.generate();
    ctx.mint = ctx.mintKeypair.publicKey;

    ctx.masterAta = await getAssociatedTokenAddress(
      ctx.mint,
      ctx.masterPda,
      true,
      TOKEN_2022_PROGRAM_ID,
      ASSOCIATED_TOKEN_PROGRAM_ID
    );

    console.log("master:", ctx.master.publicKey.toBase58());
    console.log("master PDA:", ctx.masterPda.toBase58());
    console.log("master ATA:", ctx.masterAta.toBase58(), DEVNET_EXPLORER_ADDR(ctx.masterAta));

    for (const cat of INVESTOR_CATEGORY_NAMES) {
      const [pda] = PublicKey.findProgramAddressSync(
        [Buffer.from(cat), ctx.mint.toBuffer()],
        ctx.program.programId,
      );
      ctx.categoryPdas[cat] = pda;

      const ata = await getAssociatedTokenAddress(
        ctx.mint,
        pda,
        true,
        TOKEN_2022_PROGRAM_ID,
        ASSOCIATED_TOKEN_PROGRAM_ID
      );
      ctx.categoryAtas[cat] = ata;
    }

    for (const cat of FUNCTIONAL_CATEGORY_NAMES) {
      const [pda] = PublicKey.findProgramAddressSync(
        [Buffer.from(cat), ctx.mint.toBuffer()],
        ctx.program.programId
      );
      ctx.categoryPdas[cat] = pda;

      const ata = await getAssociatedTokenAddress(
        ctx.mint,
        FUNCTIONAL_CATEGORY_AUTHORITIES[cat],
        false,
        TOKEN_2022_PROGRAM_ID,
        ASSOCIATED_TOKEN_PROGRAM_ID
      );
      ctx.categoryAtas[cat] = ata;
    }

    for (const cat of ALL_CATEGORY_NAMES) {
      console.log(
        `${cat} pda:`,
        ctx.categoryPdas[cat].toBase58(),
        DEVNET_EXPLORER_ADDR(ctx.categoryPdas[cat])
      );
      console.log(
        `${cat} ata:`,
        ctx.categoryAtas[cat].toBase58(),
        DEVNET_EXPLORER_ADDR(ctx.categoryAtas[cat])
      );
    }
  });

  it("invoke initialize_mint", async () => {
    await ctx.initializeMint();
    console.log("mint created:", ctx.mint.toBase58());
  });

  it("invoke initialize", async () => {
    await ctx.initialize();

    try {
      const marketingCat = await ctx.program.account.functionalCategoryData.fetch(ctx.categoryPdas["marketing"]);
      console.log("Marketing PDA fetch succeded:", marketingCat);
    } catch (e: any) {
      console.log("Marketing PDA fetch failed:", e);
    }

    try {
      const preseedCat = await ctx.program.account.investorCategoryData.fetch(ctx.categoryPdas["preseed"]);
      console.log("Pre-seed PDA fetch succeded:", preseedCat);
    } catch (e: any) {
      console.log("Pre-seed PDA fetch failed:", e);
    }
  });

  it("initialize closed category investors", async () => {
    for (let i = 1; i <= 5; i++) {
      const investorWallet = Keypair.generate();
      const [investorPda] = PublicKey.findProgramAddressSync(
        [Buffer.from("preseed"), Buffer.from(new Uint8Array(new Uint16Array([i]).buffer)), ctx.mint.toBuffer()],
        ctx.program.programId
      );
      const investorAta = getAssociatedTokenAddressSync(
        ctx.mint,
        investorWallet.publicKey,
        false,
        TOKEN_2022_PROGRAM_ID,
        ASSOCIATED_TOKEN_PROGRAM_ID
      );

      ctx.preseedInvestors.push({ wallet: investorWallet, pda: investorPda, ata: investorAta });

      const sig = await ctx.categoryAddInvestor("preseed", i, new anchor.BN(1000000 * 1000000), investorWallet.publicKey, investorPda, investorAta);
      console.log(`Added preseed investor ${i}: ${DEVNET_EXPLORER_TX(sig)}`);
    }

    for (let i = 1; i <= 2; i++) {
      const investorWallet = Keypair.generate();
      const [investorPda] = PublicKey.findProgramAddressSync(
        [Buffer.from("seed"), Buffer.from(new Uint8Array(new Uint16Array([i]).buffer)), ctx.mint.toBuffer()],
        ctx.program.programId
      );
      const investorAta = getAssociatedTokenAddressSync(
        ctx.mint,
        investorWallet.publicKey,
        false,
        TOKEN_2022_PROGRAM_ID,
        ASSOCIATED_TOKEN_PROGRAM_ID
      );

      ctx.seedInvestors.push({ wallet: investorWallet, pda: investorPda, ata: investorAta });

      const sig = await ctx.categoryAddInvestor("seed", i, new anchor.BN(2000000 * 1000000), investorWallet.publicKey, investorPda, investorAta);
      console.log(`Added seed investor ${i}: ${DEVNET_EXPLORER_TX(sig)}`);
    }

    console.log(`Total preseed investors: ${ctx.preseedInvestors.length}`);
    console.log(`Total seed investors: ${ctx.seedInvestors.length}`);
  });

  it("invoke tge: mints to master and transfers to marketing & liquidity; check mint authority", async () => {
    await ctx.tge();

    const mintInfo = await getMint(ctx.connection, ctx.mint, 'confirmed', TOKEN_2022_PROGRAM_ID);
    console.log("mintAuthority (post-tge):", String(mintInfo.mintAuthority));
    assert.notDeepEqual(
      mintInfo.mintAuthority?.toBase58?.(),
      ctx.master.publicKey.toBase58(),
      "master should no longer be mint authority"
    );

    const masterAcc = await getAccount(ctx.connection, ctx.masterAta, "confirmed", TOKEN_2022_PROGRAM_ID);
    const marketingAcc = await getAccount(ctx.connection, ctx.categoryAtas["marketing"], "confirmed", TOKEN_2022_PROGRAM_ID);
    const liquidityAcc = await getAccount(ctx.connection, ctx.categoryAtas["liquidity"], "confirmed", TOKEN_2022_PROGRAM_ID);

    console.log("master ATA balance (raw):", masterAcc.amount.toString());
    console.log("marketing ATA balance (raw):", marketingAcc.amount.toString());
    console.log("liquidity ATA balance (raw):", liquidityAcc.amount.toString());

    assert(masterAcc.amount > BigInt(0), "master ATA should have tokens after tge");
    assert(marketingAcc.amount >= BigInt(0), "marketing ATA exists");
    assert(liquidityAcc.amount >= BigInt(0), "liquidity ATA exists");
  });

  it("invoke categoryTransferVestings a bunch of times and track balances", async () => {
    const sleep = (ms: number) => new Promise(resolve => setTimeout(resolve, ms));

    const beforeBalances = await ctx.getBalances();
    console.log("Balances before transferCategoryVestings:", beforeBalances);

    console.log("Sleeping for 10 seconds (+1 cycle).");
    await sleep(10 * 1000);
    await ctx.categoryTransferVestings();
    console.log(await ctx.getBalances());
    console.log("\n");

    console.log("Sleeping for 22 seconds (+2 cycles).");
    await sleep(22 * 1000);
    await ctx.categoryTransferVestings();
    console.log(await ctx.getBalances());
    console.log("\n");

    console.log("Sleeping for 1 second (+0 cycles).");
    await sleep(1 * 1000);
    await ctx.categoryTransferVestings();
    console.log(await ctx.getBalances());
    console.log("\n");

    console.log("Sleeping for 100 seconds (+10 cycles).");
    await sleep(100 * 1000);
    await ctx.categoryTransferVestings();
    console.log(await ctx.getBalances());
    console.log("\n");

    assert.ok(true);
  });

  it("add a vgp investor mid-vesting", async () => {
    const investorWallet = Keypair.generate();
    const [investorPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("vgp"), Buffer.from(new Uint8Array(new Uint16Array([1]).buffer)), ctx.mint.toBuffer()],
      ctx.program.programId
    );
    const investorAta = getAssociatedTokenAddressSync(
      ctx.mint,
      investorWallet.publicKey,
      false,
      TOKEN_2022_PROGRAM_ID,
      ASSOCIATED_TOKEN_PROGRAM_ID
    );

    const sig = await ctx.categoryAddInvestor("vgp", 1, new anchor.BN(1000000 * 1000000), investorWallet.publicKey, investorPda, investorAta);
    console.log(`Added vgp investor 1: ${DEVNET_EXPLORER_TX(sig)}`);

    const investorData = await ctx.program.account.investor.fetch(investorPda);
    assert(investorData.cliffMonthsRemaining == 1, "investor added during TGE should have an extra cliff month");
  });

  it("claim funds for preseed investors manually", async () => {
    for (let i = 1; i <= 5; i++) {
      const sig = await ctx.investorClaimTokens("preseed", i, ctx.preseedInvestors[i - 1].pda, ctx.preseedInvestors[i - 1].ata);
      console.log(`Claimed for preseed investor ${i}: ${DEVNET_EXPLORER_TX(sig)}`);

      const balance = await ctx.connection.getTokenAccountBalance(ctx.preseedInvestors[i - 1].ata);
      console.log(`Preseed investor ${i} balance: ${balance.value.uiAmount}`);
    }
  });
});
