import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { Diary } from "../target/types/diary";
import { Keypair } from "@solana/web3.js";
import { promisify } from "util";
import { readFile } from "fs";
import { homedir } from "os";
import assert from "assert";
import * as borsh from "@coral-xyz/borsh";

const { SystemProgram } = anchor.web3;

async function getPayer() {
  return Keypair.fromSecretKey(
    Buffer.from(
      JSON.parse(
        await promisify(readFile)(homedir() + "/.config/solana/id.json", {
          encoding: "utf-8",
        })
      )
    )
  );
}

describe("diary-test", () => {
  const provider = anchor.AnchorProvider.env();

  anchor.setProvider(provider);

  const program = anchor.workspace.Diary as Program<Diary>;

  const diaryId = 1;
  const name = "My diary 1";

  const recordAcc = new Keypair();

  it("Create diary", async () => {
    const payer = await getPayer();

    const diaryPda = await getDiaryPda(program, payer, diaryId);

    await program.methods
      .createDiary(diaryId, name)
      .accounts({
        authority: payer.publicKey,
        diaryAccount: diaryPda,
        systemProgram: SystemProgram.programId,
      })
      .signers([payer])
      .rpc();

    const diary = await program.account.diary.fetch(diaryPda);

    assert.ok(diary.id == diaryId);
    assert.ok(diary.records.length === 0);
  });
  it("Create record", async () => {
    const payer = await getPayer();

    const diaryPda = await getDiaryPda(program, payer, diaryId);

    const createRecInstr = anchor.web3.SystemProgram.createAccount({
      fromPubkey: payer.publicKey,
      newAccountPubkey: recordAcc.publicKey,
      lamports: 100_000_000_000, // 100 sol,
      space: 10_000_000,
      programId: program.programId,
    });
    let transferSolTrns = new anchor.web3.Transaction({
      feePayer: provider.wallet.publicKey,
      recentBlockhash: (await provider.connection.getLatestBlockhash())
        .blockhash,
    });
    transferSolTrns.add(createRecInstr);
    await provider.sendAndConfirm(transferSolTrns, [payer, recordAcc]);

    await program.methods
      .addRecord(diaryId, "dasdasdasdas")
      .accounts({
        authority: payer.publicKey,
        diaryAccount: diaryPda,
        recordsAccount: recordAcc.publicKey,
      })
      .signers([payer, recordAcc])
      .rpc();

    const rec = await provider.connection.getAccountInfo(recordAcc.publicKey);
    const record = RECORD_LAYOUT.decode(rec.data);
    assert.ok(record.text == "dasdasdasdas");
  });
  it("Remove record", async () => {
    const payer = await getPayer();

    const diaryPda = await getDiaryPda(program, payer, diaryId);

    await program.methods
      .removeRecord(diaryId)
      .accounts({
        authority: payer.publicKey,
        diaryAccount: diaryPda,
        recordsAccount: recordAcc.publicKey,
      })
      .signers([payer, recordAcc])
      .rpc();
    const diary = await program.account.diary.fetch(diaryPda);

    assert.ok(diary.id == diaryId);
  });
});

async function getDiaryPda(program, payer, id) {
  const diary = Buffer.from(anchor.utils.bytes.utf8.encode("diary"));
  const id_ = Buffer.from(anchor.utils.bytes.utf8.encode(id));
  const [diaryPda, _diaryPdaBump] =
    anchor.web3.PublicKey.findProgramAddressSync(
      [payer.publicKey.toBytes(), diary, id_],
      program.programId
    );
  return diaryPda;
}

const RECORD_LAYOUT = borsh.struct([borsh.str("text")]);
