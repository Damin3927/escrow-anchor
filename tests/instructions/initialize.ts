import * as anchor from "@project-serum/anchor";
import { Program } from "@project-serum/anchor";
import { PublicKey, SystemProgram, Transaction, Connection, Commitment, LAMPORTS_PER_SOL } from "@solana/web3.js";
import { createAccount, createMint, getAccount, mintTo, TOKEN_PROGRAM_ID } from "@solana/spl-token";
import { EscrowAnchor, IDL } from "../../target/types/escrow_anchor";
import { expect } from "chai";
import { extractConstValue } from "../../app/utils/constant";

describe("initialize", function () {
  const provider = anchor.AnchorProvider.env();
  const connection = provider.connection;
  // provider.opts.commitment = "finalized";
  anchor.setProvider(provider);

  const program = anchor.workspace.EscrowAnchor as Program<EscrowAnchor>;

  let mintA: PublicKey = null;
  let mintB: PublicKey = null;
  let initializerTokenAccountA: PublicKey = null;
  let initializerTokenAccountB: PublicKey = null;
  let takerTokenAccountA: PublicKey = null;
  let takerTokenAccountB: PublicKey = null;

  let vault_account_pda: PublicKey = null;
  let vault_account_bump: number = null;
  let vault_authority_pda: PublicKey = null;

  const takerAmount = 1000;
  const initializerAmount = 500;

  const escrowAccount = anchor.web3.Keypair.generate();
  const payer = anchor.web3.Keypair.generate();
  const mintAuthority = anchor.web3.Keypair.generate(); // account that has the authority of mintA and mintB
  const initializerMainAccount = anchor.web3.Keypair.generate();
  const takerMainAccount = anchor.web3.Keypair.generate();

  it("initializes program state", async function () {
    // Airdrop 1 sol to a payer
    await connection.confirmTransaction(await connection.requestAirdrop(payer.publicKey, LAMPORTS_PER_SOL));

    // Fund main accounts
    await provider.sendAndConfirm(
      new Transaction().add(
        SystemProgram.transfer({
          fromPubkey: payer.publicKey,
          toPubkey: initializerMainAccount.publicKey,
          lamports: LAMPORTS_PER_SOL / 4,
        }),
        SystemProgram.transfer({
          fromPubkey: payer.publicKey,
          toPubkey: takerMainAccount.publicKey,
          lamports: LAMPORTS_PER_SOL / 4,
        })
      ),
      [payer]
    );

    // Creates mintA and mintB
    mintA = await createMint(
      connection,
      payer, // payer of the transaction
      mintAuthority.publicKey, // account that has the authority of this mint
      null, // authority that can freeze the token accounts
      9 // decimal places
    );

    mintB = await createMint(connection, payer, mintAuthority.publicKey, null, 9);

    // Creates token accounts for both initializer and taker
    initializerTokenAccountA = await createAccount(
      connection,
      payer, // payer of the transaction
      mintA, // mint account
      initializerMainAccount.publicKey // new authority of this token account
    );
    initializerTokenAccountB = await createAccount(connection, payer, mintB, initializerMainAccount.publicKey);

    takerTokenAccountA = await createAccount(connection, payer, mintA, takerMainAccount.publicKey);
    takerTokenAccountB = await createAccount(connection, payer, mintB, takerMainAccount.publicKey);

    // mint A tokens to initializer and B tokens to taker
    await mintTo(
      connection,
      payer, // payer of the transaction
      mintA, // mint for the account
      initializerTokenAccountA, // destination
      mintAuthority, // minting authority
      initializerAmount // amount to mint
    );
    await mintTo(connection, payer, mintB, takerTokenAccountB, mintAuthority, takerAmount);

    // get the token account info
    let _initializerTokenAccountA = await getAccount(connection, initializerTokenAccountA);
    let _takerTokenAccountB = await getAccount(connection, takerTokenAccountB);

    expect(_initializerTokenAccountA.amount).to.equal(BigInt(initializerAmount));
    expect(_takerTokenAccountB.amount).to.equal(BigInt(takerAmount));
  });

  it("initializes escrow", async function () {
    const [_vault_account_pda, _vault_account_bump] = PublicKey.findProgramAddressSync(
      [
        Buffer.from(
          anchor.utils.bytes.utf8.encode(
            extractConstValue(IDL.constants.find((constant) => constant.name === "VAULT_ACCOUNT_SEED").value)
          )
        ),
      ], // seeds
      program.programId // program id
    ); // we can get the same value each time

    vault_account_pda = _vault_account_pda;
    vault_account_bump = _vault_account_bump;

    const [_vault_authority_pda, _vault_authority_bump] = PublicKey.findProgramAddressSync(
      [
        Buffer.from(
          anchor.utils.bytes.utf8.encode(
            extractConstValue(IDL.constants.find((constant) => constant.name === "VAULT_AUTHORITY_SEED").value)
          )
        ),
      ],
      program.programId
    ); // we can get the same value each time

    vault_authority_pda = _vault_authority_pda;

    await program.methods
      .initialize(
        vault_account_bump,
        new anchor.BN(initializerAmount), // initializer_amount
        new anchor.BN(takerAmount) // taker_amount
      )
      .accounts({
        initializer: initializerMainAccount.publicKey,
        mint: mintA,
        vaultAccount: vault_account_pda,
        initializerDepositTokenAccount: initializerTokenAccountA,
        initializerReceiveTokenAccount: initializerTokenAccountB,
        escrowAccount: escrowAccount.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
        rent: anchor.web3.SYSVAR_RENT_PUBKEY,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .preInstructions([
        await program.account.escrowAccount.createInstruction(escrowAccount), // create escrow account in advance so that the initialize ix can use it
      ])
      .signers([
        initializerMainAccount, // this is needed by the definition of the Initialize struct
        escrowAccount, // this is needed to create the escrow account
      ])
      .rpc();

    let _vault = await getAccount(connection, vault_account_pda);
    let _escrowAccount = await program.account.escrowAccount.fetch(escrowAccount.publicKey);

    // Check that the new owner is the PDA
    expect(_vault.owner.equals(vault_authority_pda)).to.be.true;

    // Check that the values in the escrow account match what we expect
    expect(_escrowAccount.initializerKey.equals(initializerMainAccount.publicKey)).to.be.true;
    expect(_escrowAccount.initializerAmount.toNumber()).to.equal(initializerAmount);
    expect(_escrowAccount.takerAmount.toNumber()).to.equal(takerAmount);
    expect(_escrowAccount.initializerDepositTokenAccount.equals(initializerTokenAccountA)).to.be.true;
    expect(_escrowAccount.initializerReceiveTokenAccount.equals(initializerTokenAccountB)).to.be.true;
  });

  it("exchanges escrow state", async function () {
    await program.methods
      .exchange()
      .accounts({
        taker: takerMainAccount.publicKey,
        takerDepositTokenAccount: takerTokenAccountB,
        takerReceiveTokenAccount: takerTokenAccountA,
        initializerDepositTokenAccount: initializerTokenAccountA,
        initializerReceiveTokenAccount: initializerTokenAccountB,
        initializer: initializerMainAccount.publicKey,
        escrowAccount: escrowAccount.publicKey,
        vaultAccount: vault_account_pda,
        vaultAuthority: vault_authority_pda,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .signers([takerMainAccount])
      .rpc();

    let _takerTokenAccountA = await getAccount(connection, takerTokenAccountA);
    let _takerTokenAccountB = await getAccount(connection, takerTokenAccountB);
    let _initializerTokenAccountA = await getAccount(connection, initializerTokenAccountA);
    let _initializerTokenAccountB = await getAccount(connection, initializerTokenAccountB);

    expect(_takerTokenAccountA.amount).to.equal(BigInt(initializerAmount));
    expect(_initializerTokenAccountA.amount).to.equal(BigInt(0));
    expect(_initializerTokenAccountB.amount).to.equal(BigInt(takerAmount));
    expect(_takerTokenAccountB.amount).to.equal(BigInt(0));
  });

  it("initializes escrow and cancels escrow", async function () {
    // mint A to initializer's tokenAccountA
    await mintTo(connection, payer, mintA, initializerTokenAccountA, mintAuthority, initializerAmount);

    await program.methods
      .initialize(vault_account_bump, new anchor.BN(initializerAmount), new anchor.BN(takerAmount))
      .accounts({
        initializer: initializerMainAccount.publicKey,
        vaultAccount: vault_account_pda,
        mint: mintA,
        initializerDepositTokenAccount: initializerTokenAccountA,
        initializerReceiveTokenAccount: initializerTokenAccountB,
        escrowAccount: escrowAccount.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
        rent: anchor.web3.SYSVAR_RENT_PUBKEY,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .preInstructions([await program.account.escrowAccount.createInstruction(escrowAccount)])
      .signers([escrowAccount, initializerMainAccount])
      .rpc();

    await program.methods
      .cancel()
      .accounts({
        initializer: initializerMainAccount.publicKey,
        initializerDepositTokenAccount: initializerTokenAccountA,
        vaultAccount: vault_account_pda,
        vaultAuthority: vault_authority_pda,
        escrowAccount: escrowAccount.publicKey,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .signers([initializerMainAccount])
      .rpc();

    const _initializerTokenAccountA = await getAccount(connection, initializerTokenAccountA);
    expect(_initializerTokenAccountA.owner.equals(initializerMainAccount.publicKey)).to.be.true;
    expect(_initializerTokenAccountA.amount).to.equal(BigInt(initializerAmount));
  });
});
