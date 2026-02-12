import * as anchor from "@coral-xyz/anchor";
import {
  Connection,
  Keypair,
  PublicKey,
  Signer,
  SystemProgram,
  SYSVAR_INSTRUCTIONS_PUBKEY,
  Transaction,
} from "@solana/web3.js";
import { TOKEN_2022_PROGRAM_ID } from "@solana/spl-token";
import { SbarterTokenPrograms } from "../target/types/sbarter_token_programs";
import fs from "fs";
import * as path from "path";

const MPL_TOKEN_METADATA_PROGRAM_ID = new PublicKey(
  "metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s"
);
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

const program = anchor.workspace
  .sbarterTokenPrograms as anchor.Program<SbarterTokenPrograms>;

(async () => {
  const home = process.env.HOME || process.env.USERPROFILE || ".";
  const idPath = path.join(home, ".config", "solana", "id.json");
  const raw = fs.readFileSync(idPath, "utf8");
  const arr = JSON.parse(raw) as number[];
  const feePayer = Keypair.fromSecretKey(Uint8Array.from(arr));

  const connection = new Connection(
    "https://api.mainnet.solana.com",
    "confirmed"
  );
  const wallet = new anchor.Wallet(feePayer);

  const provider = new anchor.AnchorProvider(connection, wallet, {
    preflightCommitment: "confirmed",
  });
  anchor.setProvider(provider);

  const [masterPda] = PublicKey.findProgramAddressSync(
    [Buffer.from("master")],
    program.programId
  );
  const mintKeypair = await readOrCreateMint();
  const mint = mintKeypair.publicKey;
  const [metadata] = PublicKey.findProgramAddressSync(
    [
      Buffer.from("metadata"),
      MPL_TOKEN_METADATA_PROGRAM_ID.toBuffer(),
      mint.toBuffer(),
    ],
    MPL_TOKEN_METADATA_PROGRAM_ID
  );

  let tx = await program.methods
    .initializeMint()
    .accountsStrict({
      authority: feePayer.publicKey,
      masterPda,
      metadata,
      mint,
      mplMetadataProgram: MPL_TOKEN_METADATA_PROGRAM_ID,
      tokenProgram: TOKEN_2022_PROGRAM_ID,
      sysvarInstructions: SYSVAR_INSTRUCTIONS_PUBKEY,
      systemProgram: SYSTEM_PROGRAM_ID,
    })
    .signers([feePayer, mintKeypair])
    .transaction();
  const sig = await sendAndConfirmTx(tx, connection, wallet, [mintKeypair]);
  console.log("Created SBT token mint and initialized metadata:", {
    sig,
    mint: mint.toBase58(),
  });
})();
