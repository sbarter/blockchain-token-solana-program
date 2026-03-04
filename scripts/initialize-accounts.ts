import * as anchor from "@coral-xyz/anchor";
import {
  Connection,
  Keypair,
  PublicKey,
  SystemProgram,
  Transaction,
} from "@solana/web3.js";
import {
  ASSOCIATED_TOKEN_PROGRAM_ID,
  getAssociatedTokenAddress,
  TOKEN_2022_PROGRAM_ID,
} from "@solana/spl-token";
import { SbarterTokenPrograms } from "../target/types/sbarter_token_programs";
import fs from "fs";
import * as bs58 from "bs58";
import path from "path";

const MASTER_PUBKEY = new PublicKey(
  "HUp2467gcy1qBXNjFeaY4VpFyTMUgStMJQTmuFbyCnTx"
);
const MARKETING_AUTHORITY = new PublicKey(
  "2GRnFCAkd8Smm8uJ2zFhZQgjCPgi341MzU9FS2U3De2q"
);
const RESERVE_AUTHORITY = new PublicKey(
  "3kGsEXbQxWjNoVTZ7og1CVivPkBxAjtBPJYuSUn69eWi"
);
const LIQUIDITY_AUTHORITY = new PublicKey(
  "6RQboL2DeTM8jUQubgCYLSHZMJSLbUTRQZTEL2jjDa1M"
);
const MINT = new PublicKey("BFQ23MmV5iEZ6cJPRE5q6okXAvKUWvRCCYYfmUzwu2uW");

const SYSTEM_PROGRAM_ID = SystemProgram.programId;

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

const deriveInvestorCategoryAccounts = async (
  categorySeed: string,
  mint: PublicKey
): Promise<[PublicKey, PublicKey]> => {
  const [pda] = PublicKey.findProgramAddressSync(
    [Buffer.from(categorySeed), mint.toBuffer()],
    program.programId
  );
  const ata = await deriveAta(pda, mint);
  return [pda, ata];
};

const serializeTx = async (
  tx: Transaction,
  ...signers: Keypair[]
): Promise<string> => {
  const connection = new Connection(
    "https://api.devnet.solana.com",
    "confirmed"
  );
  const recentBlockhash = await connection.getLatestBlockhash();
  tx.recentBlockhash = recentBlockhash.blockhash;
  tx.feePayer = MASTER_PUBKEY;
  tx.partialSign(...signers);
  const serialized = tx.serialize({
    requireAllSignatures: false,
    verifySignatures: false,
  });
  return bs58.encode(serialized);
};

const displayAccounts = (accounts: Record<string, PublicKey>) => {
  const stringAccounts = {};
  for (const key in accounts) {
    stringAccounts[key] = accounts[key].toBase58();
  }
  console.dir(stringAccounts);
};

const program = anchor.workspace
  .sbarterTokenPrograms as anchor.Program<SbarterTokenPrograms>;

(async () => {
  const home = process.env.HOME || process.env.USERPROFILE || ".";
  const idPath = path.join(home, ".config", "solana", "id.json");
  const raw = fs.readFileSync(idPath, "utf8");
  const arr = JSON.parse(raw) as number[];
  const feePayer = Keypair.fromSecretKey(Uint8Array.from(arr));

  // const mintKeypair = await readOrCreateMint();
  const mint = MINT;

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

  console.log(
    "Copy the following accounts into the Squads instruction builder:"
  );
  displayAccounts({
    master: MASTER_PUBKEY,
    masterPda,
    masterAta,
    preSeedCat: (await deriveInvestorCategoryAccounts("preseed", mint))[0],
    preSeedAta: (await deriveInvestorCategoryAccounts("preseed", mint))[1],
    seedCat: (await deriveInvestorCategoryAccounts("seed", mint))[0],
    seedAta: (await deriveInvestorCategoryAccounts("seed", mint))[1],
    institutionalCat: (
      await deriveInvestorCategoryAccounts("institutional", mint)
    )[0],
    institutionalAta: (
      await deriveInvestorCategoryAccounts("institutional", mint)
    )[1],
    vgpCat: (await deriveInvestorCategoryAccounts("vgp", mint))[0],
    vgpAta: (await deriveInvestorCategoryAccounts("vgp", mint))[1],
    foundersCat: (await deriveInvestorCategoryAccounts("founders", mint))[0],
    foundersAta: (await deriveInvestorCategoryAccounts("founders", mint))[1],
    marketingCat: (await deriveInvestorCategoryAccounts("marketing", mint))[0],
    marketingAuthority: MARKETING_AUTHORITY,
    marketingAta: await deriveAta(MARKETING_AUTHORITY, mint),
    reserveCat: (await deriveInvestorCategoryAccounts("reserve", mint))[0],
    reserveAuthority: RESERVE_AUTHORITY,
    reserveAta: await deriveAta(RESERVE_AUTHORITY, mint),
    liquidityCat: (await deriveInvestorCategoryAccounts("liquidity", mint))[0],
    liquidityAuthority: LIQUIDITY_AUTHORITY,
    liquidityAta: await deriveAta(LIQUIDITY_AUTHORITY, mint),
    mint,
    tokenProgram: TOKEN_2022_PROGRAM_ID,
    associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
    systemProgram: SYSTEM_PROGRAM_ID,
  });

  let invTx = await program.methods
    .initializeInvestorCategories()
    .accountsStrict({
      master: MASTER_PUBKEY,
      masterPda,
      preSeedCat: (await deriveInvestorCategoryAccounts("preseed", mint))[0],
      preSeedAta: (await deriveInvestorCategoryAccounts("preseed", mint))[1],
      seedCat: (await deriveInvestorCategoryAccounts("seed", mint))[0],
      seedAta: (await deriveInvestorCategoryAccounts("seed", mint))[1],
      institutionalCat: (
        await deriveInvestorCategoryAccounts("institutional", mint)
      )[0],
      institutionalAta: (
        await deriveInvestorCategoryAccounts("institutional", mint)
      )[1],
      vgpCat: (await deriveInvestorCategoryAccounts("vgp", mint))[0],
      vgpAta: (await deriveInvestorCategoryAccounts("vgp", mint))[1],
      foundersCat: (await deriveInvestorCategoryAccounts("founders", mint))[0],
      foundersAta: (await deriveInvestorCategoryAccounts("founders", mint))[1],
      mint,
      tokenProgram: TOKEN_2022_PROGRAM_ID,
      associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
      systemProgram: SYSTEM_PROGRAM_ID,
    })
    .transaction();

  let funcTx = await program.methods
    .initializeFunctionalCategories()
    .accountsStrict({
      master: MASTER_PUBKEY,
      masterPda,
      masterAta,
      marketingCat: (
        await deriveInvestorCategoryAccounts("marketing", mint)
      )[0],
      marketingAuthority: MARKETING_AUTHORITY,
      marketingAta: await deriveAta(MARKETING_AUTHORITY, mint),
      reserveCat: (await deriveInvestorCategoryAccounts("reserve", mint))[0],
      reserveAuthority: RESERVE_AUTHORITY,
      reserveAta: await deriveAta(RESERVE_AUTHORITY, mint),
      liquidityCat: (
        await deriveInvestorCategoryAccounts("liquidity", mint)
      )[0],
      liquidityAuthority: LIQUIDITY_AUTHORITY,
      liquidityAta: await deriveAta(LIQUIDITY_AUTHORITY, mint),
      mint,
      tokenProgram: TOKEN_2022_PROGRAM_ID,
      associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
      systemProgram: SYSTEM_PROGRAM_ID,
    })
    .transaction();
})();
