import { useCallback, useEffect, useState } from "react";
import { useAnchorWallet, useConnection } from "@solana/wallet-adapter-react";

import { ADMIN, MINT, PROGRAM_ID, TOKEN_DECIMALS, explorerAddress } from "../config";
import { createAnchorProgram, fetchAnchorAccount } from "../lib/anchor";
import { formatUnits, shortKey, toBigInt, unixToText } from "../lib/format";
import { deriveGlobalPolicyPda } from "../lib/pda";
import { readMintFeeState, readWalletTokenState } from "../lib/token";
import type { GlobalPolicyView } from "../types";
import { StatCard } from "./StatCard";

type ViewState = {
  policy: GlobalPolicyView | null;
  epoch: number;
  currentFeeBps: number;
  currentMaxFee: bigint;
  upcomingFeeBps: number | null;
  upcomingMaxFee: bigint | null;
  upcomingEpoch: bigint | null;
  feeAuthority: string;
  supply: bigint;
  walletBalance: bigint | null;
  walletWithheld: bigint | null;
};

export function Dashboard() {
  const { connection } = useConnection();
  const anchorWallet = useAnchorWallet();
  const [state, setState] = useState<ViewState | null>(null);
  const [error, setError] = useState("");
  const [loading, setLoading] = useState(false);

  const refresh = useCallback(async () => {
    if (!anchorWallet) {
      const fee = await readMintFeeState(connection);
      setState({
        policy: null,
        epoch: fee.epoch,
        currentFeeBps: fee.currentFeeBps,
        currentMaxFee: fee.currentMaxFee,
        upcomingFeeBps: fee.upcomingFeeBps,
        upcomingMaxFee: fee.upcomingMaxFee,
        upcomingEpoch: fee.upcomingEpoch,
        feeAuthority: fee.feeAuthority?.toBase58() ?? "None",
        supply: fee.supply,
        walletBalance: null,
        walletWithheld: null
      });
      return;
    }

    setLoading(true);
    setError("");

    try {
      const [program, fee, tokenState] = await Promise.all([
        createAnchorProgram(connection, anchorWallet),
        readMintFeeState(connection),
        readWalletTokenState(connection, anchorWallet.publicKey)
      ]);

      const policy = await fetchAnchorAccount<GlobalPolicyView>(
        program,
        "globalPolicy",
        deriveGlobalPolicyPda()
      );

      setState({
        policy,
        epoch: fee.epoch,
        currentFeeBps: fee.currentFeeBps,
        currentMaxFee: fee.currentMaxFee,
        upcomingFeeBps: fee.upcomingFeeBps,
        upcomingMaxFee: fee.upcomingMaxFee,
        upcomingEpoch: fee.upcomingEpoch,
        feeAuthority: fee.feeAuthority?.toBase58() ?? "None",
        supply: fee.supply,
        walletBalance: tokenState.amount,
        walletWithheld: tokenState.withheld
      });
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setLoading(false);
    }
  }, [anchorWallet, connection]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const policy = state?.policy;

  return (
    <section className="page-section">
      <div className="section-heading">
        <div>
          <div className="eyebrow">LIVE PROTOCOL STATE</div>
          <h2>Dashboard</h2>
        </div>
        <button className="secondary-btn" onClick={() => void refresh()} disabled={loading}>
          {loading ? "Refreshing…" : "Refresh"}
        </button>
      </div>

      {error ? <div className="alert error">{error}</div> : null}

      <div className="stats-grid">
        <StatCard
          label="Current transfer fee"
          value={`${((state?.currentFeeBps ?? 0) / 100).toFixed(2)}%`}
          sub={`Max ${formatUnits(state?.currentMaxFee ?? 0n, TOKEN_DECIMALS)} tokens`}
        />
        <StatCard
          label="Upcoming fee"
          value={
            state?.upcomingFeeBps == null
              ? "None"
              : `${(state.upcomingFeeBps / 100).toFixed(2)}%`
          }
          sub={
            state?.upcomingEpoch == null
              ? "No pending fee update"
              : `Activates at epoch ${state.upcomingEpoch.toString()}`
          }
        />
        <StatCard
          label="Policy"
          value={policy ? (policy.enabled ? "Enabled" : "Disabled") : "Connect wallet"}
          sub={policy ? `Version ${toBigInt(policy.policyVersion).toString()}` : "Read on-chain policy"}
        />
        <StatCard
          label="Current epoch"
          value={state?.epoch ?? "—"}
          sub="Solana Devnet"
        />
        <StatCard
          label="Your spendable balance"
          value={
            state?.walletBalance == null
              ? "—"
              : formatUnits(state.walletBalance, TOKEN_DECIMALS)
          }
          sub={
            state?.walletWithheld == null
              ? "Connect wallet"
              : `Withheld: ${formatUnits(state.walletWithheld, TOKEN_DECIMALS)}`
          }
        />
        <StatCard
          label="Mint supply"
          value={formatUnits(state?.supply ?? 0n, TOKEN_DECIMALS)}
          sub="Token-2022"
        />
      </div>

      {policy ? (
        <div className="panel">
          <div className="panel-title">Global policy</div>
          <div className="detail-grid">
            <Detail label="Admin" value={shortKey(policy.admin, 8)} />
            <Detail label="Mint" value={shortKey(policy.mint, 8)} />
            <Detail
              label="Max transfer"
              value={`${formatUnits(toBigInt(policy.maxTransferAmount), TOKEN_DECIMALS)} tokens`}
            />
            <Detail
              label="Daily limit"
              value={`${formatUnits(toBigInt(policy.dailyTransferLimit), TOKEN_DECIMALS)} tokens`}
            />
            <Detail label="Expiry" value={unixToText(policy.expiresAt)} />
            <Detail
              label="Fee config authority"
              value={shortKey(state?.feeAuthority ?? "", 8)}
            />
          </div>
        </div>
      ) : null}

      <div className="link-row">
        <a href={explorerAddress(PROGRAM_ID.toBase58())} target="_blank" rel="noreferrer">
          Program on Explorer ↗
        </a>
        <a href={explorerAddress(MINT.toBase58())} target="_blank" rel="noreferrer">
          Mint on Explorer ↗
        </a>
        <span>Admin {shortKey(ADMIN, 6)}</span>
      </div>
    </section>
  );
}

function Detail({ label, value }: { label: string; value: string }) {
  return (
    <div className="detail">
      <span>{label}</span>
      <strong>{value}</strong>
    </div>
  );
}
