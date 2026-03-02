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

const MPL_TOKEN_METADATA_PROGRAM_ID = new PublicKey(
  "metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s"
);
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

const FUNCTIONAL_CATEGORY_NAMES = ["marketing", "reserve", "liquidity"];

const FUNCTIONAL_CATEGORY_AUTHORITIES = {
  marketing: new PublicKey("H9bPG17JxALFibvXUjNVqWUbLN5mDb9rLpCCn6WPeG3f"),
  reserve: new PublicKey("BJds5FQUDkt11Mowk6pTq7zHDonY7t9Fch2NigvdhJ5e"),
  liquidity: new PublicKey("2b21nqX3ZksapBgoAi6WNRHcRhZKZRCyTAVWMaWpKLZf"),
};

const ALL_CATEGORY_NAMES = INVESTOR_CATEGORY_NAMES.concat(
  FUNCTIONAL_CATEGORY_NAMES
);

const sleep = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));
const sendAndConfirmTx = async (
  tx: Transaction,
  connection: Connection,
  wallet: anchor.Wallet,
  additionalSigners: Signer[] = []
): Promise<string> => {
  if (!wallet.publicKey) throw new Error("Wallet not connected");
  tx.feePayer = wallet.publicKey;

  const { blockhash, lastValidBlockHeight } =
    await connection.getLatestBlockhash();
  tx.recentBlockhash = blockhash;

  if (additionalSigners.length > 0) {
    tx.partialSign(...additionalSigners);
  }

  const signed = await wallet.signTransaction(tx);

  const raw = signed.serialize();
  const signature = await connection.sendRawTransaction(raw);
  await connection.confirmTransaction({
    signature,
    blockhash,
    lastValidBlockHeight,
  });

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
  preseedInvestors: Array<{ wallet: Keypair; pda: PublicKey; ata: PublicKey }> =
    [];
  seedInvestors: Array<{ wallet: Keypair; pda: PublicKey; ata: PublicKey }> =
    [];

  async executeInstruction(
    name: string,
    buildTx: () => Promise<Transaction>,
    additionalSigners: Signer[] = [],
    silentError: boolean = false
  ): Promise<string> {
    try {
      const tx = await buildTx();
      const sig = await sendAndConfirmTx(
        tx,
        this.connection,
        this.wallet,
        additionalSigners
      );
      console.log(`${name} tx:`, DEVNET_EXPLORER_TX(sig));
      return sig;
    } catch (e: any) {
      console.error(`${name} failed:`, e.message);
      if (!silentError) {
        if (e.getLogs) {
          const logs = await e.getLogs();
          console.error(logs.join("\n"));
        }
      }
      throw e;
    }
  }

  async initializeMint(): Promise<string> {
    const [metadata] = PublicKey.findProgramAddressSync(
      [
        Buffer.from("metadata"),
        MPL_TOKEN_METADATA_PROGRAM_ID.toBuffer(),
        this.mint.toBuffer(),
      ],
      MPL_TOKEN_METADATA_PROGRAM_ID
    );

    return this.executeInstruction(
      "initialize_mint",
      () =>
        this.program.methods
          .initializeMint()
          .accountsStrict({
            authority: this.master.publicKey,
            masterPda: this.masterPda,
            metadata,
            mint: this.mint,
            mplMetadataProgram: MPL_TOKEN_METADATA_PROGRAM_ID,
            tokenProgram: TOKEN_2022_PROGRAM_ID,
            sysvarInstructions: SYSVAR_INSTRUCTIONS_PUBKEY,
            systemProgram: SYSTEM_PROGRAM_ID,
          })
          .signers([this.master, this.mintKeypair])
          .transaction(),
      [this.mintKeypair]
    );
  }

  async initializeInvestorCategories(): Promise<string> {
    const computeIx = ComputeBudgetProgram.setComputeUnitLimit({
      units: 400_000,
    });
    return this.executeInstruction("initialize_investor_categories", () =>
      this.program.methods
        .initializeInvestorCategories()
        .preInstructions([computeIx])
        .accountsStrict({
          master: this.master.publicKey,
          masterPda: this.masterPda,
          preSeedCat: this.categoryPdas["preseed"],
          preSeedAta: this.categoryAtas["preseed"],
          seedCat: this.categoryPdas["seed"],
          seedAta: this.categoryAtas["seed"],
          institutionalCat: this.categoryPdas["institutional"],
          institutionalAta: this.categoryAtas["institutional"],
          vgpCat: this.categoryPdas["vgp"],
          vgpAta: this.categoryAtas["vgp"],
          foundersCat: this.categoryPdas["founders"],
          foundersAta: this.categoryAtas["founders"],
          mint: this.mint,
          tokenProgram: TOKEN_2022_PROGRAM_ID,
          associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
          systemProgram: SYSTEM_PROGRAM_ID,
        })
        .signers([this.master])
        .transaction()
    );
  }

  async initializeFunctionalCategories(): Promise<string> {
    const computeIx = ComputeBudgetProgram.setComputeUnitLimit({
      units: 400_000,
    });
    return this.executeInstruction("initialize_functional_categories", () =>
      this.program.methods
        .initializeFunctionalCategories()
        .preInstructions([computeIx])
        .accountsStrict({
          master: this.master.publicKey,
          masterPda: this.masterPda,
          masterAta: this.masterAta,
          marketingCat: this.categoryPdas["marketing"],
          marketingAuthority: FUNCTIONAL_CATEGORY_AUTHORITIES["marketing"],
          marketingAta: this.categoryAtas["marketing"],
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
        })
        .signers([this.master])
        .transaction()
    );
  }

  async categoryAddInvestor(
    categorySeed: string,
    investorIndex: number,
    amountInWholeSbts: anchor.BN,
    investorWallet: PublicKey,
    silentError: boolean = false
  ): Promise<string> {
    const [investorPda] = PublicKey.findProgramAddressSync(
      [
        Buffer.from(categorySeed),
        Buffer.from(new Uint8Array(new Uint16Array([investorIndex]).buffer)),
        this.mint.toBuffer(),
      ],
      this.program.programId
    );
    const investorAta = getAssociatedTokenAddressSync(
      this.mint,
      investorWallet,
      false,
      TOKEN_2022_PROGRAM_ID,
      ASSOCIATED_TOKEN_PROGRAM_ID
    );

    const computeIx = ComputeBudgetProgram.setComputeUnitLimit({
      units: 400_000,
    });

    return this.executeInstruction(
      `categoryAddInvestor_${categorySeed}_${investorIndex}`,
      () =>
        this.program.methods
          .categoryAddInvestor(categorySeed, investorIndex, amountInWholeSbts)
          .accountsStrict({
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
            systemProgram: SYSTEM_PROGRAM_ID,
          })
          .preInstructions([computeIx])
          .signers([this.master])
          .transaction(),
      [],
      silentError
    );
  }

  async tge(silentError: boolean = false): Promise<string> {
    return this.executeInstruction(
      "tge",
      () =>
        this.program.methods
          .tge()
          .preInstructions([])
          .accountsStrict({
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
          })
          .signers([this.master])
          .transaction(),
      [],
      silentError
    );
  }

  async categoryTransferVestings(): Promise<string> {
    return this.executeInstruction("categoryTransferVestings", () =>
      this.program.methods
        .categoryTransferVestings()
        .preInstructions([])
        .accountsStrict({
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
        })
        .transaction()
    );
  }

  async investorClaimTokens(
    categorySeed: string,
    investorIndex: number,
    investorWallet: PublicKey
  ): Promise<string> {
    const [investorPda] = PublicKey.findProgramAddressSync(
      [
        Buffer.from(categorySeed),
        Buffer.from(new Uint8Array(new Uint16Array([investorIndex]).buffer)),
        this.mint.toBuffer(),
      ],
      this.program.programId
    );
    const investorAta = getAssociatedTokenAddressSync(
      this.mint,
      investorWallet,
      false,
      TOKEN_2022_PROGRAM_ID,
      ASSOCIATED_TOKEN_PROGRAM_ID
    );

    return this.executeInstruction(
      `investorClaimTokens_${categorySeed}_${investorIndex}`,
      () =>
        this.program.methods
          .investorClaimTokens(categorySeed, investorIndex)
          .accountsStrict({
            category: this.categoryPdas[categorySeed],
            categoryAta: this.categoryAtas[categorySeed],
            investorPda: investorPda,
            investorAta: investorAta,
            mint: this.mint,
            tokenProgram: TOKEN_2022_PROGRAM_ID,
            associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
            systemProgram: SYSTEM_PROGRAM_ID,
          })
          .transaction()
    );
  }

  async investorChangeWallet(
    categorySeed: string,
    investorIndex: number,
    oldWallet: PublicKey,
    newWallet: PublicKey,
    silentError: boolean = false
  ): Promise<string> {
    const [investorPda] = PublicKey.findProgramAddressSync(
      [
        Buffer.from(categorySeed),
        Buffer.from(new Uint8Array(new Uint16Array([investorIndex]).buffer)),
        this.mint.toBuffer(),
      ],
      this.program.programId
    );
    const newInvestorAta = getAssociatedTokenAddressSync(
      this.mint,
      newWallet,
      false,
      TOKEN_2022_PROGRAM_ID,
      ASSOCIATED_TOKEN_PROGRAM_ID
    );

    return this.executeInstruction(
      `investorChangeWallet_${categorySeed}_${investorIndex}`,
      () =>
        this.program.methods
          .investorChangeWallet(categorySeed, investorIndex)
          .accountsStrict({
            master: this.master.publicKey,
            category: this.categoryPdas[categorySeed],
            investorPda,
            oldInvestorWallet: oldWallet,
            newInvestorWallet: newWallet,
            newInvestorAta,
            mint: this.mint,
            tokenProgram: TOKEN_2022_PROGRAM_ID,
            associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
            systemProgram: SYSTEM_PROGRAM_ID,
          })
          .signers([this.master])
          .transaction(),
      [],
      silentError
    );
  }

  async categoryChangeWallet(
    categorySeed: string,
    oldWallet: PublicKey,
    newWallet: PublicKey,
    silentError: boolean = false
  ): Promise<string> {
    const newManagerAta = getAssociatedTokenAddressSync(
      this.mint,
      newWallet,
      false,
      TOKEN_2022_PROGRAM_ID,
      ASSOCIATED_TOKEN_PROGRAM_ID
    );

    return this.executeInstruction(
      `categoryChangeManagerWallet_${categorySeed}`,
      () =>
        this.program.methods
          .categoryChangeManagerWallet(categorySeed)
          .accountsStrict({
            master: this.master.publicKey,
            category: this.categoryPdas[categorySeed],
            oldManagerWallet: oldWallet,
            newManagerWallet: newWallet,
            newManagerAta,
            mint: this.mint,
            tokenProgram: TOKEN_2022_PROGRAM_ID,
            associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
            systemProgram: SYSTEM_PROGRAM_ID,
          })
          .signers([this.master])
          .transaction(),
      [],
      silentError
    );
  }

  async getBalances() {
    const out = {};
    for (const cat of INVESTOR_CATEGORY_NAMES) {
      try {
        const acc = await getAccount(
          this.connection,
          this.categoryAtas[cat],
          "confirmed",
          TOKEN_2022_PROGRAM_ID
        );
        const category = await this.program.account.investorCategoryData.fetch(
          this.categoryPdas[cat]
        );
        out[cat] = {
          amount: acc.amount,
          pda: {
            cliffMonthsRemaining: category.cliffMonthsRemaining,
            vestingMonthsRemaining: category.vestingMonthsRemaining,
            unallocatedTokensLeft: category.unallocatedTokensLeft.toString(),
            totalAllocatedTokensMonthly:
              category.totalAllocatedTokensMonthly.toString(),
            tokensReservedForClaim: category.tokensReadyForClaim.toString(),
          },
        };
      } catch {
        out[cat] = "error";
      }
    }
    for (const cat of FUNCTIONAL_CATEGORY_NAMES) {
      try {
        const acc = await getAccount(
          this.connection,
          this.categoryAtas[cat],
          "confirmed",
          TOKEN_2022_PROGRAM_ID
        );
        const category =
          await this.program.account.functionalCategoryData.fetch(
            this.categoryPdas[cat]
          );
        out[cat] = {
          amount: acc.amount,
          pda: {
            cliffMonthsRemaining: category.cliffMonthsRemaining,
            vestingMonthsRemaining: category.vestingMonthsRemaining,
          },
        };
      } catch {
        out[cat] = "error";
      }
    }
    try {
      const acc = await getAccount(
        this.connection,
        this.masterAta,
        "confirmed",
        TOKEN_2022_PROGRAM_ID
      );
      out["master"] = acc.amount;
    } catch {
      out["master"] = BigInt(0);
    }
    return out;
  }
}

describe("sbarterTokenPrograms", function () {
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
    ctx.program = anchor.workspace
      .sbarterTokenPrograms as anchor.Program<SbarterTokenPrograms>;

    [ctx.masterPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("master")],
      ctx.program.programId
    );
    ctx.mintKeypair = Keypair.generate();
    ctx.mint = ctx.mintKeypair.publicKey;

    console.log("Generated mint keypair:", ctx.mintKeypair.secretKey);

    ctx.masterAta = await getAssociatedTokenAddress(
      ctx.mint,
      ctx.masterPda,
      true,
      TOKEN_2022_PROGRAM_ID,
      ASSOCIATED_TOKEN_PROGRAM_ID
    );

    console.log("master:", ctx.master.publicKey.toBase58());
    console.log("master PDA:", ctx.masterPda.toBase58());
    console.log(
      "master ATA:",
      ctx.masterAta.toBase58(),
      DEVNET_EXPLORER_ADDR(ctx.masterAta)
    );

    for (const cat of INVESTOR_CATEGORY_NAMES) {
      const [pda] = PublicKey.findProgramAddressSync(
        [Buffer.from(cat), ctx.mint.toBuffer()],
        ctx.program.programId
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
        true,
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

  it("invoke initialize functional/investor categories", async () => {
    await ctx.initializeInvestorCategories();
    await ctx.initializeFunctionalCategories();

    try {
      const marketingCat =
        await ctx.program.account.functionalCategoryData.fetch(
          ctx.categoryPdas["marketing"]
        );
      console.log("Marketing PDA fetch succeded:", marketingCat);
    } catch (e: any) {
      console.log("Marketing PDA fetch failed:", e);
    }

    try {
      const vgpCat = await ctx.program.account.investorCategoryData.fetch(
        ctx.categoryPdas["vgp"]
      );
      console.log("VGPs PDA fetch succeded:", vgpCat);
    } catch (e: any) {
      console.log("VGPs PDA fetch failed:", e);
    }
  });

  it("initialize closed category investors", async () => {
    for (let i = 1; i <= 3; i++) {
      const investorWallet = Keypair.generate();
      const [investorPda] = PublicKey.findProgramAddressSync(
        [
          Buffer.from("preseed"),
          Buffer.from(new Uint8Array(new Uint16Array([i]).buffer)),
          ctx.mint.toBuffer(),
        ],
        ctx.program.programId
      );
      const investorAta = getAssociatedTokenAddressSync(
        ctx.mint,
        investorWallet.publicKey,
        false,
        TOKEN_2022_PROGRAM_ID,
        ASSOCIATED_TOKEN_PROGRAM_ID
      );

      ctx.preseedInvestors.push({
        wallet: investorWallet,
        pda: investorPda,
        ata: investorAta,
      });

      await ctx.categoryAddInvestor(
        "preseed",
        i,
        new anchor.BN(12000000),
        investorWallet.publicKey
      );
    }

    for (let i = 1; i <= 2; i++) {
      const investorWallet = Keypair.generate();
      const [investorPda] = PublicKey.findProgramAddressSync(
        [
          Buffer.from("seed"),
          Buffer.from(new Uint8Array(new Uint16Array([i]).buffer)),
          ctx.mint.toBuffer(),
        ],
        ctx.program.programId
      );
      const investorAta = getAssociatedTokenAddressSync(
        ctx.mint,
        investorWallet.publicKey,
        false,
        TOKEN_2022_PROGRAM_ID,
        ASSOCIATED_TOKEN_PROGRAM_ID
      );

      ctx.seedInvestors.push({
        wallet: investorWallet,
        pda: investorPda,
        ata: investorAta,
      });

      await ctx.categoryAddInvestor(
        "seed",
        i,
        new anchor.BN(24000000),
        investorWallet.publicKey
      );
    }

    console.log(`Total preseed investors: ${ctx.preseedInvestors.length}`);
    console.log(`Total seed investors: ${ctx.seedInvestors.length}`);
  });

  it("fail to add more closed category investors than configured (preseed)", async () => {
    const investorWallet = Keypair.generate();
    try {
      await ctx.categoryAddInvestor(
        "preseed",
        4,
        new anchor.BN(24000000),
        investorWallet.publicKey,
        true
      );
    } catch (e: any) {
      return;
    }
    assert.fail(
      "adding an investor beyond configured amount didn't throw an error"
    );
  });

  it("fail to add more closed category investors than configured (seed)", async () => {
    const investorWallet = Keypair.generate();
    try {
      await ctx.categoryAddInvestor(
        "seed",
        3,
        new anchor.BN(24000000),
        investorWallet.publicKey,
        true
      );
    } catch (e: any) {
      return;
    }
    assert.fail(
      "adding an investor beyond configured amount didn't throw an error"
    );
  });

  it("invoke tge: mints to master and transfers to marketing & liquidity; check mint authority", async () => {
    await ctx.tge();

    const mintInfo = await getMint(
      ctx.connection,
      ctx.mint,
      "confirmed",
      TOKEN_2022_PROGRAM_ID
    );
    console.log("mintAuthority (post-tge):", String(mintInfo.mintAuthority));
    assert.notDeepEqual(
      mintInfo.mintAuthority?.toBase58?.(),
      ctx.master.publicKey.toBase58(),
      "master should no longer be mint authority"
    );

    const masterAcc = await getAccount(
      ctx.connection,
      ctx.masterAta,
      "confirmed",
      TOKEN_2022_PROGRAM_ID
    );
    const marketingAcc = await getAccount(
      ctx.connection,
      ctx.categoryAtas["marketing"],
      "confirmed",
      TOKEN_2022_PROGRAM_ID
    );
    const reserveAcc = await getAccount(
      ctx.connection,
      ctx.categoryAtas["reserve"],
      "confirmed",
      TOKEN_2022_PROGRAM_ID
    );
    const liquidityAcc = await getAccount(
      ctx.connection,
      ctx.categoryAtas["liquidity"],
      "confirmed",
      TOKEN_2022_PROGRAM_ID
    );

    console.log("master ATA balance (raw):", masterAcc.amount.toString());
    console.log("marketing ATA balance (raw):", marketingAcc.amount.toString());
    console.log("liquidity ATA balance (raw):", liquidityAcc.amount.toString());

    assert(
      masterAcc.amount > BigInt(0),
      "master ATA should have tokens after tge"
    );
    assert(
      marketingAcc.amount > BigInt(0),
      "marketing ATA should have tokens after tge"
    );
    assert(
      reserveAcc.amount > BigInt(0),
      "reserve ATA should have tokens after tge"
    );
    assert(
      liquidityAcc.amount > BigInt(0),
      "liquidity ATA should have tokens after tge"
    );
  });

  it("fail to invoke tge a second time", async () => {
    try {
      await ctx.tge(true);
    } catch (e: any) {
      return;
    }
    assert.fail("invoking TGE a second time didn't throw an error");
  });

  it("invoke categoryTransferVestings a bunch of times and track balances", async () => {
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
      [
        Buffer.from("vgp"),
        Buffer.from(new Uint8Array(new Uint16Array([1]).buffer)),
        ctx.mint.toBuffer(),
      ],
      ctx.program.programId
    );
    await ctx.categoryAddInvestor(
      "vgp",
      1,
      new anchor.BN(12000000),
      investorWallet.publicKey
    );

    const investorData = await ctx.program.account.investor.fetch(investorPda);
    assert(
      investorData.cliffMonthsRemaining == 1,
      "investor added during TGE should have an extra cliff month"
    );
  });

  it("claim funds for investors manually", async () => {
    console.log("Waiting for another 10 seconds");
    await sleep(10 * 1000);

    await ctx.categoryTransferVestings();

    for (let i = 1; i <= 3; i++) {
      await ctx.investorClaimTokens(
        "preseed",
        i,
        ctx.preseedInvestors[i - 1].wallet.publicKey
      );

      const balance = await ctx.connection.getTokenAccountBalance(
        ctx.preseedInvestors[i - 1].ata
      );
      console.log(`Preseed investor ${i} balance: ${balance.value.uiAmount}`);
      console.log(await ctx.getBalances());
      assert(
        balance.value.uiAmount > 0,
        "no tokens were claimed for preseed investor"
      );
    }
    for (let i = 1; i <= 2; i++) {
      await ctx.investorClaimTokens(
        "seed",
        i,
        ctx.seedInvestors[i - 1].wallet.publicKey
      );

      const balance = await ctx.connection.getTokenAccountBalance(
        ctx.seedInvestors[i - 1].ata
      );
      console.log(`Seed investor ${i} balance: ${balance.value.uiAmount}`);
      console.log(await ctx.getBalances());
      assert(
        balance.value.uiAmount > 0,
        "no tokens were claimed for preseed investor"
      );
    }
  });

  it("fail to add an investor with the same id", async () => {
    const investorWallet = Keypair.generate();
    try {
      await ctx.categoryAddInvestor(
        "vgp",
        1,
        new anchor.BN(12000000),
        investorWallet.publicKey
      );
    } catch (e: any) {
      return;
    }
    assert.fail("creating a vgp investor with id:1 didn't throw an error");
  });

  it("fail to add an investor with an allocation too large", async () => {
    const investorWallet = Keypair.generate();
    try {
      await ctx.categoryAddInvestor(
        "founders",
        1,
        new anchor.BN(1000000000000),
        investorWallet.publicKey
      );
    } catch (e: any) {
      return;
    }
    assert.fail(
      "creating an investor with an overallocation didn't throw an error"
    );
  });

  it("change investor wallet", async () => {
    let investorAccounts = ctx.seedInvestors[0];
    const newInvestorWallet = Keypair.generate();
    await ctx.investorChangeWallet(
      "seed",
      1,
      investorAccounts.wallet.publicKey,
      newInvestorWallet.publicKey
    );

    await sleep(3 * 1000);
    let investor = await ctx.program.account.investor.fetch(
      investorAccounts.pda,
      "processed"
    );
    assert.equal(
      investor.wallet.toBase58(),
      newInvestorWallet.publicKey.toBase58(),
      "the investor wallet didn't change"
    );
  });

  it("fail to change investor wallet without providing the old one", async () => {
    let investorAccounts = ctx.seedInvestors[1];
    const newInvestorWallet = Keypair.generate();
    try {
      await ctx.investorChangeWallet(
        "seed",
        2,
        SYSTEM_PROGRAM_ID,
        newInvestorWallet.publicKey,
        true
      );
    } catch {}

    await sleep(3 * 1000);
    let investor = await ctx.program.account.investor.fetch(
      investorAccounts.pda,
      "processed"
    );
    assert.equal(
      investor.wallet.toBase58(),
      investorAccounts.wallet.publicKey.toBase58(),
      "the investor wallet changed"
    );
  });

  it("change marketing manager wallet", async () => {
    const newManagerWallet = Keypair.generate();

    await ctx.categoryChangeWallet(
      "marketing",
      FUNCTIONAL_CATEGORY_AUTHORITIES["marketing"],
      newManagerWallet.publicKey
    );

    await sleep(3 * 1000);
    let marketingCatPost =
      await ctx.program.account.functionalCategoryData.fetch(
        ctx.categoryPdas["marketing"],
        "processed"
      );
    assert.equal(
      marketingCatPost.wallet.toBase58(),
      newManagerWallet.publicKey.toBase58(),
      "the manager wallet didn't change"
    );
  });

  it("fail to change reserve manager wallet without providing the old one", async () => {
    const newManagerWallet = Keypair.generate();

    try {
      await ctx.categoryChangeWallet(
        "reserve",
        SYSTEM_PROGRAM_ID,
        newManagerWallet.publicKey,
        true
      );
    } catch {}

    await sleep(3 * 1000);
    let reserveCatPost = await ctx.program.account.functionalCategoryData.fetch(
      ctx.categoryPdas["reserve"],
      "processed"
    );
    assert.equal(
      reserveCatPost.wallet.toBase58(),
      FUNCTIONAL_CATEGORY_AUTHORITIES["reserve"].toBase58(),
      "the manager wallet changed"
    );
  });
});
