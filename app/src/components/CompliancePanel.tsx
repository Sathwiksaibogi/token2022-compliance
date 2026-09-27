import { useCallback, useEffect, useState } from "react";
import { useAnchorWallet, useConnection } from "@solana/wallet-adapter-react";

import { TOKEN_DECIMALS } from "../config";
import { createAnchorProgram, fetchAnchorAccount } from "../lib/anchor";
import { formatUnits, statusName, toBigInt } from "../lib/format";
import { deriveAuthorizationPda, deriveTransferStatsPda } from "../lib/pda";
import type { AuthorizationView, TransferStatsView } from "../types";
import { StatCard } from "./StatCard";

export function CompliancePanel() {
  const { connection } = useConnection();
  const wallet = useAnchorWallet();

  const [authorization, setAuthorization] = useState<AuthorizationView | null>(null);
  const [stats, setStats] = useState<TransferStatsView | null>(null);
  const [error, setError] = useState("");

  const refresh = useCallback(async () => {
    setError("");

    if (!wallet) {
      setAuthorization(null);
      setStats(null);
      return;
    }

    try {
      const program = await createAnchorProgram(connection, wallet);
      const authPda = deriveAuthorizationPda(wallet.publicKey);
      const statsPda = deriveTransferStatsPda(wallet.publicKey);

      const [auth, transferStats] = await Promise.all([
        fetchAnchorAccount<AuthorizationView>(program, "authorization", authPda),
        fetchAnchorAccount<TransferStatsView>(program, "transferStats", statsPda)
      ]);

      setAuthorization(auth);
      setStats(transferStats);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  }, [connection, wallet]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  return (
    <section className="page-section">
      <div className="section-heading">
        <div>
          <div className="eyebrow">WALLET-LEVEL POLICY</div>
          <h2>My compliance state</h2>
        </div>
        <button className="secondary-btn" onClick={() => void refresh()}>
          Refresh
        </button>
      </div>

      {!wallet ? (
        <div className="alert info">Connect a wallet to inspect its compliance state.</div>
      ) : null}

      {error ? <div className="alert error">{error}</div> : null}

      <div className="stats-grid">
        <StatCard
          label="Authorization"
          value={authorization ? statusName(authorization.status) : "Not initialized"}
          sub="Wallet-level PDA"
        />
        <StatCard
          label="Amount today"
          value={
            stats
              ? formatUnits(toBigInt(stats.amountToday), TOKEN_DECIMALS)
              : "—"
          }
          sub={stats ? `UTC day index ${toBigInt(stats.dayIndex)}` : "No stats PDA"}
        />
        <StatCard
          label="Lifetime transferred"
          value={
            stats
              ? formatUnits(toBigInt(stats.totalTransferred), TOKEN_DECIMALS)
              : "—"
          }
          sub="Gross amount before Token-2022 fee"
        />
        <StatCard
          label="Transfer count"
          value={stats ? toBigInt(stats.transferCount).toString() : "—"}
          sub="Successful compliant transfers"
        />
      </div>

      <div className="panel">
        <div className="panel-title">Why wallet-level stats matter</div>
        <p className="muted">
          Limits are keyed by wallet, not by token account. Creating another token
          account does not reset the sender's daily allowance.
        </p>
      </div>
    </section>
  );
}
