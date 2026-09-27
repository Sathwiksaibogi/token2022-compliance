import { useMemo, useState } from "react";
import BN from "bn.js";
import { PublicKey, SystemProgram } from "@solana/web3.js";
import { TOKEN_2022_PROGRAM_ID } from "@solana/spl-token";
import { useAnchorWallet, useConnection, useWallet } from "@solana/wallet-adapter-react";

import { ADMIN, MINT, TOKEN_DECIMALS, explorerTx } from "../config";
import {
  createAnchorProgram,
  fetchAnchorAccount,
  sendAnchorInstruction
} from "../lib/anchor";
import { explainError } from "../lib/errors";
import { parseUnits } from "../lib/format";
import { mintDemoTokens } from "../lib/token";
import {
  deriveAuthorizationPda,
  deriveGlobalPolicyPda,
  deriveTransferStatsPda
} from "../lib/pda";
import type { AuthorizationView, TransferStatsView } from "../types";

type AuthStatus = "Unauthorized" | "Authorized" | "Blocked";

function statusArg(status: AuthStatus) {
  if (status === "Authorized") return { authorized: {} };
  if (status === "Blocked") return { blocked: {} };
  return { unauthorized: {} };
}

export function AdminPanel() {
  const { connection } = useConnection();
  const wallet = useAnchorWallet();
  const browserWallet = useWallet();

  const isAdmin = useMemo(
    () => !!wallet?.publicKey.equals(ADMIN),
    [wallet]
  );

  const [managedWallet, setManagedWallet] = useState("");
  const [status, setStatus] = useState<AuthStatus>("Authorized");

  const [enabled, setEnabled] = useState("");
  const [maxTransfer, setMaxTransfer] = useState("");
  const [dailyLimit, setDailyLimit] = useState("");
  const [expiresAt, setExpiresAt] = useState("");

  const [feeBps, setFeeBps] = useState("200");
  const [maxFee, setMaxFee] = useState("5");
  const [mintRecipient, setMintRecipient] = useState("");
  const [mintAmount, setMintAmount] = useState("100");

  const [busy, setBusy] = useState("");
  const [message, setMessage] = useState("");
  const [lastSignature, setLastSignature] = useState("");

  function commonCandidates(admin: PublicKey) {
    return {
      admin,
      mint: MINT,
      globalPolicy: deriveGlobalPolicyPda(),
      global_policy: deriveGlobalPolicyPda(),
      tokenProgram: TOKEN_2022_PROGRAM_ID,
      token_program: TOKEN_2022_PROGRAM_ID,
      systemProgram: SystemProgram.programId,
      system_program: SystemProgram.programId
    };
  }

  async function requireProgram() {
    if (!wallet) throw new Error("Connect the admin wallet first.");
    if (!wallet.publicKey.equals(ADMIN)) {
      throw new Error("Connected wallet is not the configured policy admin.");
    }
    return createAnchorProgram(connection, wallet);
  }

  async function manageAuthorization() {
    setBusy("auth");
    setMessage("");
    setLastSignature("");

    try {
      const program = await requireProgram();
      const target = new PublicKey(managedWallet.trim());
      const authorization = deriveAuthorizationPda(target);
      const stats = deriveTransferStatsPda(target);

      let authAccount = await fetchAnchorAccount<AuthorizationView>(
        program,
        "authorization",
        authorization
      );

      let signature = "";

      if (!authAccount) {
        signature = await sendAnchorInstruction(
          program,
          "initializeAuthorization",
          [target],
          {
            ...commonCandidates(wallet!.publicKey),
            authorization
          }
        );

        authAccount = await fetchAnchorAccount<AuthorizationView>(
          program,
          "authorization",
          authorization
        );
      }

      signature = await sendAnchorInstruction(
        program,
        "setAuthorizationStatus",
        [statusArg(status)],
        {
          ...commonCandidates(wallet!.publicKey),
          authorization
        }
      );

      const statsAccount = await fetchAnchorAccount<TransferStatsView>(
        program,
        "transferStats",
        stats
      );

      if (!statsAccount && status === "Authorized") {
        signature = await sendAnchorInstruction(
          program,
          "initializeTransferStats",
          [target],
          {
            ...commonCandidates(wallet!.publicKey),
            payer: wallet!.publicKey,
            authorization,
            transferStats: stats,
            transfer_stats: stats
          }
        );
      }

      setLastSignature(signature);
      setMessage(
        `${managedWallet.slice(0, 6)}… is now ${status}. ${
          status === "Authorized" ? "TransferStats are ready." : ""
        }`
      );
    } catch (e) {
      setMessage(explainError(e));
    } finally {
      setBusy("");
    }
  }

  async function updatePolicy() {
    setBusy("policy");
    setMessage("");
    setLastSignature("");

    try {
      const program = await requireProgram();

      const args = {
        enabled:
          enabled === "" ? null : enabled === "true",
        maxTransferAmount:
          maxTransfer.trim() === ""
            ? null
            : new BN(parseUnits(maxTransfer, TOKEN_DECIMALS).toString()),
        dailyTransferLimit:
          dailyLimit.trim() === ""
            ? null
            : new BN(parseUnits(dailyLimit, TOKEN_DECIMALS).toString()),
        expiresAt:
          expiresAt.trim() === ""
            ? null
            : new BN(expiresAt.trim())
      };

      const signature = await sendAnchorInstruction(
        program,
        "updatePolicy",
        [args],
        commonCandidates(wallet!.publicKey)
      );

      setLastSignature(signature);
      setMessage("Global policy updated successfully.");
    } catch (e) {
      setMessage(explainError(e));
    } finally {
      setBusy("");
    }
  }

  async function updateFee() {
    setBusy("fee");
    setMessage("");
    setLastSignature("");

    try {
      const program = await requireProgram();

      const bps = Number(feeBps);
      if (!Number.isInteger(bps) || bps < 0 || bps > 10_000) {
        throw new Error("Fee basis points must be an integer from 0 to 10,000.");
      }

      const maxFeeBase = parseUnits(maxFee, TOKEN_DECIMALS);

      const signature = await sendAnchorInstruction(
        program,
        "updateTransferFee",
        [bps, new BN(maxFeeBase.toString())],
        commonCandidates(wallet!.publicKey)
      );

      setLastSignature(signature);
      setMessage(
        "Dynamic transfer fee update submitted. Token-2022 activates a new fee two epochs later."
      );
    } catch (e) {
      setMessage(explainError(e));
    } finally {
      setBusy("");
    }
  }


  async function mintForDemo() {
    setBusy("mint");
    setMessage("");
    setLastSignature("");

    try {
      await requireProgram();

      if (!browserWallet.publicKey) {
        throw new Error("Connect the mint-authority wallet first.");
      }

      const recipient = new PublicKey(mintRecipient.trim());
      const amount = parseUnits(mintAmount, TOKEN_DECIMALS);

      const signature = await mintDemoTokens(
        connection,
        browserWallet,
        recipient,
        amount
      );

      setLastSignature(signature);
      setMessage("Demo tokens minted successfully.");
    } catch (e) {
      setMessage(explainError(e));
    } finally {
      setBusy("");
    }
  }

  return (
    <section className="page-section">
      <div className="section-heading">
        <div>
          <div className="eyebrow">ADMIN CONTROL PLANE</div>
          <h2>Policy administration</h2>
        </div>
        <div className={`admin-chip ${isAdmin ? "ok" : ""}`}>
          {isAdmin ? "Admin wallet connected" : "Admin wallet required"}
        </div>
      </div>

      <div className="admin-grid">
        <div className="panel">
          <div className="panel-title">Wallet authorization</div>

          <label className="field">
            <span>Wallet</span>
            <input
              value={managedWallet}
              onChange={(e) => setManagedWallet(e.target.value)}
              placeholder="Wallet public key"
            />
          </label>

          <label className="field">
            <span>Status</span>
            <select
              value={status}
              onChange={(e) => setStatus(e.target.value as AuthStatus)}
            >
              <option>Authorized</option>
              <option>Blocked</option>
              <option>Unauthorized</option>
            </select>
          </label>

          <button
            className="primary-btn"
            disabled={!isAdmin || busy === "auth"}
            onClick={() => void manageAuthorization()}
          >
            {busy === "auth" ? "Updating…" : "Apply authorization"}
          </button>
        </div>

        <div className="panel">
          <div className="panel-title">Global policy</div>

          <label className="field">
            <span>Enabled</span>
            <select value={enabled} onChange={(e) => setEnabled(e.target.value)}>
              <option value="">No change</option>
              <option value="true">Enabled</option>
              <option value="false">Disabled</option>
            </select>
          </label>

          <label className="field">
            <span>Max transfer (tokens)</span>
            <input
              value={maxTransfer}
              onChange={(e) => setMaxTransfer(e.target.value)}
              placeholder="Blank = no change"
            />
          </label>

          <label className="field">
            <span>Daily limit (tokens)</span>
            <input
              value={dailyLimit}
              onChange={(e) => setDailyLimit(e.target.value)}
              placeholder="Blank = no change"
            />
          </label>

          <label className="field">
            <span>Expiry Unix timestamp</span>
            <input
              value={expiresAt}
              onChange={(e) => setExpiresAt(e.target.value)}
              placeholder="Blank = no change, 0 = no expiry"
            />
          </label>

          <button
            className="primary-btn"
            disabled={!isAdmin || busy === "policy"}
            onClick={() => void updatePolicy()}
          >
            {busy === "policy" ? "Updating…" : "Update policy"}
          </button>
        </div>

        <div className="panel">
          <div className="panel-title">Dynamic Token-2022 fee</div>

          <label className="field">
            <span>Fee in basis points</span>
            <input value={feeBps} onChange={(e) => setFeeBps(e.target.value)} />
          </label>

          <label className="field">
            <span>Maximum fee (tokens)</span>
            <input value={maxFee} onChange={(e) => setMaxFee(e.target.value)} />
          </label>

          <div className="notice">
            100 bps = 1%. Token-2022 schedules fee changes two epochs ahead.
          </div>

          <button
            className="primary-btn"
            disabled={!isAdmin || busy === "fee"}
            onClick={() => void updateFee()}
          >
            {busy === "fee" ? "Scheduling…" : "Schedule fee update"}
          </button>
        </div>

        <div className="panel">
          <div className="panel-title">Devnet demo mint</div>

          <label className="field">
            <span>Recipient wallet</span>
            <input
              value={mintRecipient}
              onChange={(e) => setMintRecipient(e.target.value)}
              placeholder="Authorized Devnet wallet"
            />
          </label>

          <label className="field">
            <span>Amount (tokens)</span>
            <input
              value={mintAmount}
              onChange={(e) => setMintAmount(e.target.value)}
              placeholder="100"
            />
          </label>

          <div className="notice">
            Demo-only helper. The connected admin is also the current mint authority.
          </div>

          <button
            className="primary-btn"
            disabled={!isAdmin || busy === "mint"}
            onClick={() => void mintForDemo()}
          >
            {busy === "mint" ? "Minting…" : "Mint demo tokens"}
          </button>
        </div>
      </div>

      {message ? (
        <div className={`alert ${lastSignature ? "success" : "error"}`}>
          {message}
          {lastSignature ? (
            <>
              {" "}
              <a href={explorerTx(lastSignature)} target="_blank" rel="noreferrer">
                Explorer ↗
              </a>
            </>
          ) : null}
        </div>
      ) : null}
    </section>
  );
}
