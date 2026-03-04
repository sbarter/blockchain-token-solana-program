/*
 *
 * You can use this script to submit Squads vault transactions for such actions as:
 *    - Initial program setup (init investor and functional categories)
 *    - Adding new investors
 *    - Invoking TGE
 *    - Token claims on category and investor level
 *    - Changing investor and functional category manager wallets in case of a loss
 *    - Withdrawing from an investor category into an arbitrary wallet
 *    - Depositing from a functional category (Squad subaccount) into an investor category.
 *
 * It's generally a mess, but it contains all of the necessary functions that
 * are well-typed and can be called without knowledge about the internal
 * implementation details.
 *
 * That being said, it's recommended to have a 15000ms sleep between submitting
 * Squads transactions to allow the squad state to finalize between them.
 *
 */

import * as anchor from "@coral-xyz/anchor";
import * as multisig from "@sqds/multisig";
import * as path from "path";
import fs from "fs";
import { SbarterTokenPrograms } from "../target/types/sbarter_token_programs";
import {
  Connection,
  Keypair,
  PublicKey,
  TransactionInstruction,
  TransactionMessage,
} from "@solana/web3.js";
import {
  ASSOCIATED_TOKEN_PROGRAM_ID,
  getAssociatedTokenAddress,
  Multisig,
  TOKEN_2022_PROGRAM_ID,
} from "@solana/spl-token";
import { SYSTEM_PROGRAM_ID } from "@coral-xyz/anchor/dist/cjs/native/system";
const { Multisig } = multisig.accounts;

const RPC_URI = "https://mainnet.helius-rpc.com/?api-key=API_KEY";

const MASTER_PUBKEY = new PublicKey(
  "HUp2467gcy1qBXNjFeaY4VpFyTMUgStMJQTmuFbyCnTx"
);
const MULTISIG_PDA = new PublicKey(
  "HuPSmekEL8LSSFnikE1kCCm8qkcBs77oX7KuF4YmeWx3"
);

const MINT = new PublicKey("BFQ23MmV5iEZ6cJPRE5q6okXAvKUWvRCCYYfmUzwu2uW");

const MARKETING_AUTHORITY = new PublicKey(
  "2GRnFCAkd8Smm8uJ2zFhZQgjCPgi341MzU9FS2U3De2q"
);
const RESERVE_AUTHORITY = new PublicKey(
  "3kGsEXbQxWjNoVTZ7og1CVivPkBxAjtBPJYuSUn69eWi"
);
const LIQUIDITY_AUTHORITY = new PublicKey(
  "6RQboL2DeTM8jUQubgCYLSHZMJSLbUTRQZTEL2jjDa1M"
);

const VAULTS = {
  marketing: [1, MARKETING_AUTHORITY],
  reserve: [2, RESERVE_AUTHORITY],
  liquidity: [3, LIQUIDITY_AUTHORITY],
};

type CategorySeed = "preseed" | "seed" | "institutional" | "vgp" | "founders";
type FuncCategorySeed = "marketing" | "reserve" | "liquidity";
type CategorySeedWithFunc = CategorySeed | FuncCategorySeed;

let connection: Connection;
let wallet: anchor.Wallet;
let feePayer: Keypair;
let program: anchor.Program<SbarterTokenPrograms>;
let mint: PublicKey;
let multisigPda: PublicKey;
let vaultPda: PublicKey;

const sleep = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));

const formatBN = (bn: anchor.BN): string =>
  bn.toString().replace(/\B(?=(\d{3})+(?!\d))/g, "_");

const init = async (): Promise<void> => {
  const home = process.env.HOME || process.env.USERPROFILE || ".";
  const idPath = path.join(home, ".config", "solana", "solflare-mw.json");
  const raw = fs.readFileSync(idPath, "utf8");
  const arr = JSON.parse(raw) as number[];
  feePayer = Keypair.fromSecretKey(Uint8Array.from(arr));

  // const mintKeypair = await readOrCreateMint();
  mint = MINT;

  connection = new Connection(RPC_URI, "finalized");
  wallet = new anchor.Wallet(feePayer);
  const provider = new anchor.AnchorProvider(connection, wallet, {
    preflightCommitment: "confirmed",
  });
  anchor.setProvider(provider);
  program = anchor.workspace
    .sbarterTokenPrograms as anchor.Program<SbarterTokenPrograms>;

  multisigPda = MULTISIG_PDA;
  vaultPda = MASTER_PUBKEY;
};

const readOrCreateMint = async (): Promise<Keypair> => {
  try {
    const data = await fs.promises.readFile("sbt_mint.json", "utf8");
    const mint = Keypair.fromSecretKey(new Uint8Array(JSON.parse(data)));
    console.log("Loaded existing mint:", mint.publicKey.toBase58());
    return mint;
  } catch (err) {
    const mint = Keypair.generate();
    console.log("Generated new mint:", mint.publicKey.toBase58());
    await fs.promises.writeFile(
      "sbt_mint.json",
      JSON.stringify(Array.from(mint.secretKey)),
      "utf8"
    );
    return mint;
  }
};

const pretty = (object: any): any => {
  if (Array.isArray(object)) {
    return object.map(pretty);
  }

  if (object !== null && typeof object === "object") {
    if (object instanceof PublicKey) return object.toBase58();
    if (object instanceof anchor.BN) return object.toString();

    return Object.fromEntries(
      Object.entries(object).map(([key, value]) => [key, pretty(value)])
    );
  }

  if (typeof object === "bigint") return object.toString();

  return object;
};

const deriveAta = async (
  key: PublicKey,
  mint: PublicKey
): Promise<PublicKey> => {
  return await getAssociatedTokenAddress(
    mint,
    key,
    true,
    TOKEN_2022_PROGRAM_ID,
    ASSOCIATED_TOKEN_PROGRAM_ID
  );
};

const deriveCategoryPdaAta = async (
  categorySeed: CategorySeedWithFunc,
  mint: PublicKey
): Promise<[PublicKey, PublicKey]> => {
  const [pda] = PublicKey.findProgramAddressSync(
    [Buffer.from(categorySeed), mint.toBuffer()],
    program.programId
  );
  const ata = await deriveAta(pda, mint);
  return [pda, ata];
};

const deriveFullVestingAccounts = async (
  categorySeed: CategorySeedWithFunc = null,
  investorIndex: number = null,
  investorWallet: PublicKey = null
): Promise<{
  masterPda: PublicKey;
  masterAta: PublicKey;
  investorAccount?: {
    wallet: PublicKey;
    monthlyAllocationInBaseUnits: anchor.BN;
    lastOffsetMonths: number;
    cliffMonthsRemaining: number;
    vestingMonthsRemaining: number;
  };
  investorPda?: PublicKey;
  investorAta?: PublicKey;
  categoryAccount?: {
    monthlyAllocation: anchor.BN;
    unallocatedTokensLeft: anchor.BN;
    totalAllocatedTokensMonthly: anchor.BN;
    tokensReadyForClaim: anchor.BN;
    cliffStartedAt: anchor.BN;
    monthsClaimed: number;
    cliffMonthsRemaining: number;
    vestingMonthsRemaining: number;
    investorCount: number;
    isOpen: boolean;
  };
  categoryPda?: PublicKey;
  categoryAta?: PublicKey;
}> => {
  const [masterPda] = PublicKey.findProgramAddressSync(
    [Buffer.from("master")],
    program.programId
  );
  const masterAta = await getAssociatedTokenAddress(
    mint,
    masterPda,
    true,
    TOKEN_2022_PROGRAM_ID,
    ASSOCIATED_TOKEN_PROGRAM_ID
  );

  const [categoryPda] =
    categorySeed != null
      ? PublicKey.findProgramAddressSync(
          [Buffer.from(categorySeed), mint.toBuffer()],
          program.programId
        )
      : [null];
  const categoryAta =
    categoryPda != null
      ? await getAssociatedTokenAddress(
          mint,
          categoryPda,
          true,
          TOKEN_2022_PROGRAM_ID,
          ASSOCIATED_TOKEN_PROGRAM_ID
        )
      : null;
  const categoryAccount =
    categoryPda != null
      ? await program.account.investorCategoryData.fetchNullable(
          categoryPda,
          "confirmed"
        )
      : null;
  const [investorPda] =
    categorySeed && investorIndex
      ? PublicKey.findProgramAddressSync(
          [
            Buffer.from(categorySeed),
            Buffer.from(
              new Uint8Array(new Uint16Array([investorIndex]).buffer)
            ),
            mint.toBuffer(),
          ],
          program.programId
        )
      : [null];
  const investorAccount =
    investorPda != null
      ? await program.account.investor.fetchNullable(investorPda, "confirmed")
      : null;
  const investorAta =
    investorPda != null
      ? await getAssociatedTokenAddress(
          mint,
          investorAccount?.wallet ?? investorWallet,
          false,
          TOKEN_2022_PROGRAM_ID,
          ASSOCIATED_TOKEN_PROGRAM_ID
        )
      : null;

  return {
    masterPda,
    masterAta,
    investorAccount,
    investorPda,
    investorAta,
    categoryAccount,
    categoryPda,
    categoryAta,
  };
};

const nextInvestorIndex = async (
  categorySeed: CategorySeed
): Promise<number> => {
  const [categoryPda] = PublicKey.findProgramAddressSync(
    [Buffer.from(categorySeed), mint.toBuffer()],
    program.programId
  );
  const categoryAccount = await program.account.investorCategoryData.fetch(
    categoryPda,
    "confirmed"
  );
  return categoryAccount.investorCount + 1;
};

const submitSquadsTx = async (
  ixs: TransactionInstruction[],
  memo?: string,
  vaultPda: PublicKey = MASTER_PUBKEY,
  vaultIndex: number = 0
): Promise<string> => {
  const multisigAccount = await Multisig.fromAccountAddress(
    connection,
    multisigPda
  );
  const txMessage = new TransactionMessage({
    instructions: ixs,
    payerKey: vaultPda,
    recentBlockhash: (await connection.getLatestBlockhash()).blockhash,
  });

  const sig = await multisig.rpc.vaultTransactionCreate({
    connection,
    feePayer,
    multisigPda,
    transactionIndex:
      BigInt(multisigAccount.transactionIndex.toString()) + BigInt(1),
    creator: feePayer.publicKey,
    vaultIndex,
    ephemeralSigners: 0,
    transactionMessage: txMessage,
    memo,
  });
  console.log("Submitted Squad transaction:", {
    sig,
    memo,
  });
  return sig;
};

const executeCategoryClaimTx = async (): Promise<string> => {
  const accounts = await deriveFullVestingAccounts();
  const sig = await program.methods
    .categoryTransferVestings()
    .accountsStrict({
      masterPda: accounts.masterPda,
      masterAta: accounts.masterAta,
      preSeedCat: (await deriveCategoryPdaAta("preseed", mint))[0],
      preSeedAta: (await deriveCategoryPdaAta("preseed", mint))[1],
      seedCat: (await deriveCategoryPdaAta("seed", mint))[0],
      seedAta: (await deriveCategoryPdaAta("seed", mint))[1],
      institutionalCat: (await deriveCategoryPdaAta("institutional", mint))[0],
      institutionalAta: (await deriveCategoryPdaAta("institutional", mint))[1],
      vgpCat: (await deriveCategoryPdaAta("vgp", mint))[0],
      vgpAta: (await deriveCategoryPdaAta("vgp", mint))[1],
      foundersCat: (await deriveCategoryPdaAta("founders", mint))[0],
      foundersAta: (await deriveCategoryPdaAta("founders", mint))[1],
      marketingCat: (await deriveCategoryPdaAta("marketing", mint))[0],
      marketingAta: await deriveAta(MARKETING_AUTHORITY, mint),
      reserveCat: (await deriveCategoryPdaAta("reserve", mint))[0],
      reserveAta: await deriveAta(RESERVE_AUTHORITY, mint),
      liquidityCat: (await deriveCategoryPdaAta("liquidity", mint))[0],
      liquidityAta: await deriveAta(LIQUIDITY_AUTHORITY, mint),
      mint,
      tokenProgram: TOKEN_2022_PROGRAM_ID,
      associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
      systemProgram: SYSTEM_PROGRAM_ID,
    })
    .rpc();
  console.log("Executed category claim tx:", sig);
  return sig;
};

const executeInvestorClaimTx = async (
  categorySeed: CategorySeed,
  investorIndex: number
): Promise<string> => {
  const accounts = await deriveFullVestingAccounts(categorySeed, investorIndex);
  const sig = await program.methods
    .investorClaimTokens(categorySeed, investorIndex)
    .accountsStrict({
      investorPda: accounts.investorPda,
      investorAta: accounts.investorAta,
      category: accounts.categoryPda,
      categoryAta: accounts.categoryAta,
      mint,
      tokenProgram: TOKEN_2022_PROGRAM_ID,
      associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
      systemProgram: SYSTEM_PROGRAM_ID,
    })
    .signers([feePayer])
    .rpc();
  console.log(
    `Executed investor claim tx for ${categorySeed}_${investorIndex}: ${sig}`
  );
  return sig;
};

const submitAddInvestorTx = async (
  investorWallet: PublicKey,
  categorySeed: CategorySeed,
  totalAllocation: anchor.BN,
  investorIndex: number
): Promise<string> => {
  const accounts = await deriveFullVestingAccounts(
    categorySeed,
    investorIndex,
    investorWallet
  );
  const multisigAccount = await Multisig.fromAccountAddress(
    connection,
    multisigPda
  );
  const ix = await program.methods
    .categoryAddInvestor(categorySeed, investorIndex, totalAllocation)
    .accountsStrict({
      master: MASTER_PUBKEY,
      masterPda: accounts.masterPda,
      investorPda: accounts.investorPda,
      investorWallet,
      investorAta: accounts.investorAta,
      category: accounts.categoryPda,
      categoryAta: accounts.categoryAta,
      mint,
      tokenProgram: TOKEN_2022_PROGRAM_ID,
      associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
      systemProgram: SYSTEM_PROGRAM_ID,
    })
    .instruction();

  const sig = await submitSquadsTx(
    [ix],
    `Add investor #${investorIndex} to ${categorySeed}. Wallet: ${investorWallet.toBase58()}. Total allocation: ${formatBN(
      totalAllocation
    )} tokens.`
  );
  return sig;
};

const submitInitCategoriesTxs = async (): Promise<[string, string]> => {
  const accounts = await deriveFullVestingAccounts();
  let invIx = await program.methods
    .initializeInvestorCategories()
    .accountsStrict({
      master: MASTER_PUBKEY,
      masterPda: accounts.masterPda,
      preSeedCat: (await deriveCategoryPdaAta("preseed", mint))[0],
      preSeedAta: (await deriveCategoryPdaAta("preseed", mint))[1],
      seedCat: (await deriveCategoryPdaAta("seed", mint))[0],
      seedAta: (await deriveCategoryPdaAta("seed", mint))[1],
      institutionalCat: (await deriveCategoryPdaAta("institutional", mint))[0],
      institutionalAta: (await deriveCategoryPdaAta("institutional", mint))[1],
      vgpCat: (await deriveCategoryPdaAta("vgp", mint))[0],
      vgpAta: (await deriveCategoryPdaAta("vgp", mint))[1],
      foundersCat: (await deriveCategoryPdaAta("founders", mint))[0],
      foundersAta: (await deriveCategoryPdaAta("founders", mint))[1],
      mint,
      tokenProgram: TOKEN_2022_PROGRAM_ID,
      associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
      systemProgram: SYSTEM_PROGRAM_ID,
    })
    .instruction();

  let funcIx = await program.methods
    .initializeFunctionalCategories()
    .accountsStrict({
      master: MASTER_PUBKEY,
      masterPda: accounts.masterPda,
      masterAta: accounts.masterAta,
      marketingCat: (await deriveCategoryPdaAta("marketing", mint))[0],
      marketingAuthority: MARKETING_AUTHORITY,
      marketingAta: await deriveAta(MARKETING_AUTHORITY, mint),
      reserveCat: (await deriveCategoryPdaAta("reserve", mint))[0],
      reserveAuthority: RESERVE_AUTHORITY,
      reserveAta: await deriveAta(RESERVE_AUTHORITY, mint),
      liquidityCat: (await deriveCategoryPdaAta("liquidity", mint))[0],
      liquidityAuthority: LIQUIDITY_AUTHORITY,
      liquidityAta: await deriveAta(LIQUIDITY_AUTHORITY, mint),
      mint,
      tokenProgram: TOKEN_2022_PROGRAM_ID,
      associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
      systemProgram: SYSTEM_PROGRAM_ID,
    })
    .instruction();

  const invSig = await submitSquadsTx(
    [invIx],
    `Initialize investor category PDAs for mint: ${mint.toBase58()}`
  );
  await sleep(15000);
  const funcSig = await submitSquadsTx(
    [funcIx],
    `Initialize functional category PDAs for mint: ${mint.toBase58()}`
  );
  return [invSig, funcSig];
};

const submitTGETx = async (): Promise<string> => {
  const accounts = await deriveFullVestingAccounts();
  let ix = await program.methods
    .tge()
    .accountsStrict({
      master: MASTER_PUBKEY,
      masterPda: accounts.masterPda,
      masterAta: accounts.masterAta,
      preSeedCat: (await deriveCategoryPdaAta("preseed", mint))[0],
      seedCat: (await deriveCategoryPdaAta("seed", mint))[0],
      institutionalCat: (await deriveCategoryPdaAta("institutional", mint))[0],
      vgpCat: (await deriveCategoryPdaAta("vgp", mint))[0],
      foundersCat: (await deriveCategoryPdaAta("founders", mint))[0],
      marketingCat: (await deriveCategoryPdaAta("marketing", mint))[0],
      marketingAuthority: MARKETING_AUTHORITY,
      marketingAta: await deriveAta(MARKETING_AUTHORITY, mint),
      reserveCat: (await deriveCategoryPdaAta("reserve", mint))[0],
      reserveAuthority: RESERVE_AUTHORITY,
      reserveAta: await deriveAta(RESERVE_AUTHORITY, mint),
      liquidityCat: (await deriveCategoryPdaAta("liquidity", mint))[0],
      liquidityAuthority: LIQUIDITY_AUTHORITY,
      liquidityAta: await deriveAta(LIQUIDITY_AUTHORITY, mint),
      mint,
      tokenProgram: TOKEN_2022_PROGRAM_ID,
      associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
      systemProgram: SYSTEM_PROGRAM_ID,
    })
    .instruction();

  const sig = await submitSquadsTx(
    [ix],
    "Invoke TGE (only goes through after categories & closed-category investors initialization)"
  );
  return sig;
};

const submitChangeInvestorWalletTx = async (
  categorySeed: CategorySeed,
  investorIndex: number,
  newWallet: PublicKey
): Promise<string> => {
  const accounts = await deriveFullVestingAccounts(categorySeed, investorIndex);
  let ix = await program.methods
    .investorChangeWallet(categorySeed, investorIndex)
    .accountsStrict({
      master: MASTER_PUBKEY,
      category: accounts.categoryPda,
      investorPda: accounts.investorPda,
      oldInvestorWallet: accounts.investorAccount.wallet,
      newInvestorWallet: newWallet,
      newInvestorAta: await deriveAta(newWallet, mint),
      mint,
      tokenProgram: TOKEN_2022_PROGRAM_ID,
      associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
      systemProgram: SYSTEM_PROGRAM_ID,
    })
    .instruction();

  const sig = await submitSquadsTx(
    [ix],
    `Change ${categorySeed} investor #${investorIndex} wallet from "${accounts.investorAccount.wallet.toBase58()}" to "${newWallet.toBase58()}"`
  );
  return sig;
};

const submitChangeFuncCategoryWalletTx = async (
  categorySeed: FuncCategorySeed,
  newWallet: PublicKey
): Promise<string> => {
  const accounts = await deriveFullVestingAccounts();
  const [categoryPda] = PublicKey.findProgramAddressSync(
    [Buffer.from(categorySeed), mint.toBuffer()],
    program.programId
  );
  const categoryAccount = await program.account.functionalCategoryData.fetch(
    categoryPda
  );
  let ix = await program.methods
    .categoryChangeManagerWallet(categorySeed)
    .accountsStrict({
      master: MASTER_PUBKEY,
      category: categoryPda,
      oldManagerWallet: categoryAccount.wallet,
      newManagerWallet: newWallet,
      newManagerAta: await deriveAta(newWallet, mint),
      mint,
      tokenProgram: TOKEN_2022_PROGRAM_ID,
      associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
      systemProgram: SYSTEM_PROGRAM_ID,
    })
    .instruction();

  const sig = await submitSquadsTx(
    [ix],
    `Change ${categorySeed} manager wallet from "${categoryAccount.wallet.toBase58()}" to "${newWallet.toBase58()}"`
  );
  return sig;
};

const submitWithdrawFromCategoryTx = async (
  categorySeed: CategorySeed,
  amount: anchor.BN,
  recipient: PublicKey
): Promise<string> => {
  const accounts = await deriveFullVestingAccounts(categorySeed);
  let ix = await program.methods
    .categoryWithdraw(categorySeed, amount)
    .accountsStrict({
      master: MASTER_PUBKEY,
      category: accounts.categoryPda,
      categoryAta: accounts.categoryAta,
      recipient: recipient,
      recipientAta: await deriveAta(recipient, mint),
      mint,
      tokenProgram: TOKEN_2022_PROGRAM_ID,
      associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
      systemProgram: SYSTEM_PROGRAM_ID,
    })
    .instruction();

  const sig = await submitSquadsTx(
    [ix],
    `Withdraw ${formatBN(
      amount
    )} tokens from ${categorySeed} category into "${recipient.toBase58()}"`
  );
  return sig;
};

const submitDepositIntoCategoryTx = async (
  recipientCategorySeed: CategorySeed,
  senderCategorySeed: FuncCategorySeed,
  amount: anchor.BN
): Promise<string> => {
  const [vaultIndex, vaultPda] = VAULTS[senderCategorySeed];
  const accounts = await deriveFullVestingAccounts(recipientCategorySeed);
  let ix = await program.methods
    .categoryDeposit(recipientCategorySeed, amount)
    .accountsStrict({
      category: accounts.categoryPda,
      categoryAta: accounts.categoryAta,
      sender: vaultPda as PublicKey,
      senderAta: await deriveAta(vaultPda as PublicKey, mint),
      mint,
      tokenProgram: TOKEN_2022_PROGRAM_ID,
      associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
      systemProgram: SYSTEM_PROGRAM_ID,
    })
    .instruction();

  const sig = await submitSquadsTx(
    [ix],
    `Depositing ${formatBN(
      amount
    )} tokens from ${senderCategorySeed} category into ${recipientCategorySeed} category`,
    vaultPda as PublicKey,
    vaultIndex as number
  );
  return sig;
};

const addAllInvestors = async () => {
  const investorsLines = (
    await fs.promises.readFile("investor-categories-test.csv", "utf8")
  )
    .trim()
    .split("\n")
    .slice(1);

  const investors: Array<{
    index: number;
    pubkey: PublicKey;
    categorySeed: CategorySeed;
    allocation: anchor.BN;
  }> = [];
  for (const line of investorsLines) {
    let [index, pubkey, categorySeed, allocation] = line.split(",");
    investors.push({
      index: Number(index),
      pubkey: new PublicKey(pubkey),
      categorySeed: categorySeed as CategorySeed,
      allocation: new anchor.BN(allocation),
    });
  }

  for (const investor of investors) {
    await submitAddInvestorTx(
      investor.pubkey,
      investor.categorySeed,
      investor.allocation,
      investor.index
    );
    await sleep(15000);
  }
};

(async () => {
  await init();

  // NOTE: Add stuff here.
})().catch((err) => {
  console.log(err);
});
