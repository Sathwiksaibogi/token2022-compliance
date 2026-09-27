import { useState } from "react";
import { PublicKey } from "@solana/web3.js";
import { useConnection, useWallet } from "@solana/wallet-adapter-react";

import { TOKEN_DECIMALS, explorerTx } from "../config";
import { explainError } from "../lib/errors";
import { parseUnits } from "../lib/format";
import { transferWithCompliance } from "../lib/token";

export function TransferPanel() {
  const { connection } = useConnection();
  const wallet = useWallet();

  const [recipient, setRecipient] = useState("");
  const [amount, setAmount] = useState("40");
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState("");
  const [signature, setSignature] = useState("");

  async function submit() {
    setMessage("");
    setSignature("");

    if (!wallet.publicKey) {
      setMessage("Connect the sender wallet first.");
      return;
    }

    setBusy(true);

    try {
      const recipientWallet = new PublicKey(recipient.trim());
      const baseAmount = parseUnits(amount, TOKEN_DECIMALS);

      const sig = await transferWithCompliance(
        connection,
        wallet,
        recipientWallet,
        baseAmount
      );

      setSignature(sig);
      setMessage("Transfer accepted by Token-2022 and the compliance hook.");
    } catch (e) {
      setMessage(explainError(e));
    } finally {
      setBusy(false);
    }
  }

  return (
    <section className="page-section">
      <div className="section-heading">
        <div>
          <div className="eyebrow">USER FLOW</div>
          <h2>Compliant transfer</h2>
        </div>
      </div>

      <div className="two-column">
        <div className="panel">
          <label className="field">
            <span>Recipient wallet</span>
            <input
              value={recipient}
              onChange={(e) => setRecipient(e.target.value)}
              placeholder="Recipient wallet public key"
            />
          </label>

          <label className="field">
            <span>Gross transfer amount</span>
            <input
              value={amount}
              onChange={(e) => setAmount(e.target.value)}
              inputMode="decimal"
              placeholder="40"
            />
          </label>

          <button className="primary-btn" onClick={() => void submit()} disabled={busy}>
            {busy ? "Sending…" : "Send Token-2022 transfer"}
          </button>

          {message ? (
            <div className={`alert ${signature ? "success" : "error"}`}>{message}</div>
          ) : null}

          {signature ? (
            <a
              className="tx-link"
              href={explorerTx(signature)}
              target="_blank"
              rel="noreferrer"
            >
              View transaction on Solana Explorer ↗
            </a>
          ) : null}
        </div>

        <div className="panel flow-panel">
          <div className="panel-title">What happens on-chain</div>
          <div className="flow-list">
            <Flow n="1" text="Wallet signs Token-2022 TransferChecked." />
            <Flow n="2" text="Token-2022 resolves the ExtraAccountMetaList." />
            <Flow n="3" text="Transfer Hook validates sender, receiver and policy." />
            <Flow n="4" text="Daily and per-transfer limits are enforced." />
            <Flow n="5" text="TransferStats are updated atomically." />
            <Flow n="6" text="Native Token-2022 fee is withheld from the destination." />
          </div>
        </div>
      </div>
    </section>
  );
}

function Flow({ n, text }: { n: string; text: string }) {
  return (
    <div className="flow-item">
      <span>{n}</span>
      <p>{text}</p>
    </div>
  );
}
