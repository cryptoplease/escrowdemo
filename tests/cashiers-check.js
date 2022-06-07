const anchor = require("@project-serum/anchor");
const serumCmn = require("@project-serum/common");
const { assert } = require("chai");
const { TOKEN_PROGRAM_ID } = require("@solana/spl-token");

describe("cashiers-check", () => {
  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.env();
  // hack so we don't have to update serum-common library
  // to the new AnchorProvider class and Provider interface
  provider.send = provider.sendAndConfirm;
  anchor.setProvider(provider);

  const program = anchor.workspace.Escrowdemo;

  let mint = null;
  let god = null;
  let receiver = null;

  it("Sets up initial test state", async () => {
    const [_mint, _god] = await serumCmn.createMintAndVault(
      program.provider,
      new anchor.BN(1000000)
    );
    mint = _mint;
    god = _god;

    receiver = await serumCmn.createTokenAccount(
      program.provider,
      mint,
      program.provider.wallet.publicKey
    );
  });

  const check = anchor.web3.Keypair.generate();
  const vault = anchor.web3.Keypair.generate();

  let escrowSigner = null;

  it("Creates an escrow!", async () => {
    let [_escrowSigner, nonce] = await anchor.web3.PublicKey.findProgramAddress(
      [check.publicKey.toBuffer()],
      program.programId
    );
    escrowSigner = _escrowSigner;

    await program.rpc.createEscrow(new anchor.BN(100), nonce, {
      accounts: {
        escrow: check.publicKey,
        vault: vault.publicKey,
        escrowSigner,
        sellerToken: god,
        buyerToken: receiver,
        seller: program.provider.wallet.publicKey,
        tokenProgram: TOKEN_PROGRAM_ID,
        rent: anchor.web3.SYSVAR_RENT_PUBKEY,
      },
      signers: [check, vault],
      instructions: [
        await program.account.escrow.createInstruction(check, 300),
        ...(await serumCmn.createTokenAccountInstrs(
          program.provider,
          vault.publicKey,
          mint,
          escrowSigner
        )),
      ],
    });

    const checkAccount = await program.account.escrow.fetch(check.publicKey);
    assert.isTrue(checkAccount.sellerToken.equals(god));
    assert.isTrue(checkAccount.buyerToken.equals(receiver));
    assert.isTrue(checkAccount.amount.eq(new anchor.BN(100)));
    assert.isTrue(checkAccount.vault.equals(vault.publicKey));
    assert.strictEqual(checkAccount.nonce, nonce);
    assert.isFalse(checkAccount.burned);

    let vaultAccount = await serumCmn.getTokenAccount(
      program.provider,
      checkAccount.vault
    );
    assert.isTrue(vaultAccount.amount.eq(new anchor.BN(100)));
  });

  it("Cashes a check", async () => {
    await program.rpc.cashCheck({
      accounts: {
        escrow: check.publicKey,
        vault: vault.publicKey,
        escrowSigner: escrowSigner,
        buyerToken: receiver,
        buyer: program.provider.wallet.publicKey,
        tokenProgram: TOKEN_PROGRAM_ID,
      },
    });

    const checkAccount = await program.account.escrow.fetch(check.publicKey);
    assert.isTrue(checkAccount.burned);

    let vaultAccount = await serumCmn.getTokenAccount(
      program.provider,
      checkAccount.vault
    );
    assert.isTrue(vaultAccount.amount.eq(new anchor.BN(0)));

    let receiverAccount = await serumCmn.getTokenAccount(
      program.provider,
      receiver
    );
    assert.isTrue(receiverAccount.amount.eq(new anchor.BN(100)));
  });
});
