import {
  ASSOCIATED_TOKEN_PROGRAM_ID,
  TOKEN_2022_PROGRAM_ID,
  createAssociatedTokenAccountIdempotentInstruction,
  createMintToCheckedInstruction,
  createTransferCheckedWithTransferHookInstruction,
  getAccount,
  getAssociatedTokenAddressSync,
  getMint,
  getTransferFeeAmount,
  getTransferFeeConfig,
} from "@solana/spl-token";

import {
  Transaction,
  type Connection,
  type PublicKey,
} from "@solana/web3.js";

import type { WalletContextState } from "@solana/wallet-adapter-react";

import {
  MINT,
  TOKEN_DECIMALS,
} from "../config";

export type BrowserWalletSender = Pick<
  WalletContextState,
  "publicKey" | "signTransaction"
>;

// ------------------------------------------------------------
// Generic browser-wallet send + confirmation helper
//
// Important:
//
// We intentionally DO NOT manually set recentBlockhash here.
// Wallet Adapter prepares the transaction immediately before
// requesting the wallet signature.
//
// This avoids a blockhash becoming stale while the user is
// interacting with the wallet popup.
// ------------------------------------------------------------

async function sendAndConfirmBrowserTransaction(
  connection: Connection,
  wallet: BrowserWalletSender,
  transaction: Transaction
): Promise<string> {
  if (!wallet.publicKey) {
    throw new Error("Connect a wallet first.");
  }

  if (!wallet.signTransaction) {
    throw new Error(
      "This wallet does not support signTransaction."
    );
  }

  // Get a fresh blockhash from OUR configured Devnet RPC.
  const latest =
    await connection.getLatestBlockhash(
      "processed"
    );

  transaction.feePayer =
    wallet.publicKey;

  transaction.recentBlockhash =
    latest.blockhash;

  // Wallet ONLY signs.
  // It does not choose where the transaction is submitted.
  const signedTransaction =
    await wallet.signTransaction(
      transaction
    );

  // We submit the signed bytes explicitly through OUR
  // ConnectionProvider RPC: Helius Devnet.
  const signature =
    await connection.sendRawTransaction(
      signedTransaction.serialize(),
      {
        skipPreflight: false,
        preflightCommitment: "confirmed",
        maxRetries: 5,
      }
    );

  // Confirm against the same RPC and the same blockhash lifetime.
  const confirmation =
    await connection.confirmTransaction(
      {
        signature,
        blockhash: latest.blockhash,
        lastValidBlockHeight:
          latest.lastValidBlockHeight,
      },
      "confirmed"
    );

  if (confirmation.value.err) {
    throw new Error(
      `Transaction failed: ${JSON.stringify(
        confirmation.value.err
      )}`
    );
  }

  return signature;
}

// ------------------------------------------------------------
// Canonical Token-2022 ATA
// ------------------------------------------------------------

export function tokenAccountFor(
  wallet: PublicKey
) {
  return getAssociatedTokenAddressSync(
    MINT,
    wallet,
    false,
    TOKEN_2022_PROGRAM_ID,
    ASSOCIATED_TOKEN_PROGRAM_ID
  );
}

// ------------------------------------------------------------
// Read wallet Token-2022 state
// ------------------------------------------------------------

export async function readWalletTokenState(
  connection: Connection,
  wallet: PublicKey
) {
  const tokenAccount =
    tokenAccountFor(wallet);

  const info =
    await connection.getAccountInfo(
      tokenAccount,
      "confirmed"
    );

  if (!info) {
    return {
      tokenAccount,
      exists: false,
      amount: 0n,
      withheld: 0n,
    };
  }

  const account =
    await getAccount(
      connection,
      tokenAccount,
      "confirmed",
      TOKEN_2022_PROGRAM_ID
    );

  return {
    tokenAccount,
    exists: true,
    amount: account.amount,
    withheld:
      getTransferFeeAmount(account)
        ?.withheldAmount ?? 0n,
  };
}

// ------------------------------------------------------------
// Read Token-2022 mint fee state
// ------------------------------------------------------------

export async function readMintFeeState(
  connection: Connection
) {
  const [mintInfo, epochInfo] =
    await Promise.all([
      getMint(
        connection,
        MINT,
        "confirmed",
        TOKEN_2022_PROGRAM_ID
      ),

      connection.getEpochInfo(
        "confirmed"
      ),
    ]);

  const feeConfig =
    getTransferFeeConfig(mintInfo);

  if (!feeConfig) {
    return {
      supply: mintInfo.supply,
      decimals: mintInfo.decimals,
      epoch: epochInfo.epoch,

      currentFeeBps: 0,
      currentMaxFee: 0n,

      upcomingFeeBps:
        null as number | null,

      upcomingMaxFee:
        null as bigint | null,

      upcomingEpoch:
        null as bigint | null,

      feeAuthority:
        null as PublicKey | null,

      withdrawAuthority:
        null as PublicKey | null,
    };
  }

  const currentEpoch =
    BigInt(epochInfo.epoch);

  const newer =
    feeConfig.newerTransferFee;

  const older =
    feeConfig.olderTransferFee;

  const newerIsActive =
    currentEpoch >= newer.epoch;

  const active =
    newerIsActive
      ? newer
      : older;

  const upcoming =
    newerIsActive
      ? null
      : newer;

  return {
    supply: mintInfo.supply,
    decimals: mintInfo.decimals,
    epoch: epochInfo.epoch,

    currentFeeBps:
      active.transferFeeBasisPoints,

    currentMaxFee:
      active.maximumFee,

    upcomingFeeBps:
      upcoming?.transferFeeBasisPoints ??
      null,

    upcomingMaxFee:
      upcoming?.maximumFee ??
      null,

    upcomingEpoch:
      upcoming?.epoch ??
      null,

    feeAuthority:
      feeConfig
        .transferFeeConfigAuthority ??
      null,

    withdrawAuthority:
      feeConfig
        .withdrawWithheldAuthority ??
      null,
  };
}

// ------------------------------------------------------------
// Create destination Token-2022 ATA if missing
// ------------------------------------------------------------

async function ensureDestinationTokenAccount(
  connection: Connection,
  wallet: BrowserWalletSender,
  recipient: PublicKey
) {
  if (!wallet.publicKey) {
    throw new Error(
      "Connect a wallet first."
    );
  }

  const destination =
    tokenAccountFor(recipient);

  const existing =
    await connection.getAccountInfo(
      destination,
      "confirmed"
    );

  if (!existing) {
    const createIx =
      createAssociatedTokenAccountIdempotentInstruction(
        wallet.publicKey,
        destination,
        recipient,
        MINT,
        TOKEN_2022_PROGRAM_ID,
        ASSOCIATED_TOKEN_PROGRAM_ID
      );

    const createTx =
      new Transaction().add(
        createIx
      );

    await sendAndConfirmBrowserTransaction(
      connection,
      wallet,
      createTx
    );
  }

  return destination;
}

// ------------------------------------------------------------
// Real compliant Token-2022 transfer
// ------------------------------------------------------------

export async function transferWithCompliance(
  connection: Connection,
  wallet: BrowserWalletSender,
  recipientWallet: PublicKey,
  amount: bigint
) {
  if (!wallet.publicKey) {
    throw new Error(
      "Connect a wallet first."
    );
  }

  if (amount <= 0n) {
    throw new Error(
      "Transfer amount must be greater than zero."
    );
  }

  const source =
    tokenAccountFor(
      wallet.publicKey
    );

  const sourceInfo =
    await connection.getAccountInfo(
      source,
      "confirmed"
    );

  if (!sourceInfo) {
    throw new Error(
      "Connected wallet does not have a Token-2022 account for this mint."
    );
  }

  const destination =
    await ensureDestinationTokenAccount(
      connection,
      wallet,
      recipientWallet
    );

  // ----------------------------------------------------------
  // This SPL helper:
  //
  // 1. Builds TransferChecked
  // 2. Reads the mint TransferHook extension
  // 3. Loads ExtraAccountMetaList
  // 4. Resolves the extra accounts
  //
  // The resulting instruction triggers our on-chain hook.
  // ----------------------------------------------------------

  const transferIx =
    await createTransferCheckedWithTransferHookInstruction(
      connection,
      source,
      MINT,
      destination,
      wallet.publicKey,
      amount,
      TOKEN_DECIMALS,
      [],
      "confirmed",
      TOKEN_2022_PROGRAM_ID
    );

  const tx =
    new Transaction().add(
      transferIx
    );

  return sendAndConfirmBrowserTransaction(
    connection,
    wallet,
    tx
  );
}

// ------------------------------------------------------------
// Devnet-only demo mint helper
// ------------------------------------------------------------

export async function mintDemoTokens(
  connection: Connection,
  wallet: BrowserWalletSender,
  recipientWallet: PublicKey,
  amount: bigint
) {
  if (!wallet.publicKey) {
    throw new Error(
      "Connect the mint-authority wallet first."
    );
  }

  if (amount <= 0n) {
    throw new Error(
      "Mint amount must be greater than zero."
    );
  }

  const destination =
    await ensureDestinationTokenAccount(
      connection,
      wallet,
      recipientWallet
    );

  const mintIx =
    createMintToCheckedInstruction(
      MINT,
      destination,
      wallet.publicKey,
      amount,
      TOKEN_DECIMALS,
      [],
      TOKEN_2022_PROGRAM_ID
    );

  const tx =
    new Transaction().add(
      mintIx
    );

  return sendAndConfirmBrowserTransaction(
    connection,
    wallet,
    tx
  );
}