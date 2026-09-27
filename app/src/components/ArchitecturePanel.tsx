import { MINT, PROGRAM_ID } from "../config";
import { deriveGlobalPolicyPda } from "../lib/pda";
import { shortKey } from "../lib/format";

export function ArchitecturePanel() {
  return (
    <section className="page-section">
      <div className="section-heading">
        <div>
          <div className="eyebrow">SYSTEM DESIGN</div>
          <h2>Protocol architecture</h2>
        </div>
      </div>

      <div className="architecture">
        <Node title="Wallet" body="Signs Token-2022 transfer" />
        <Arrow />
        <Node
          title="Token-2022"
          body="TransferChecked + native TransferFeeConfig"
          accent
        />
        <Arrow />
        <Node title="ExtraAccountMetaList" body="Resolves policy accounts" />
        <Arrow />
        <Node
          title="Transfer Hook"
          body={`Program ${shortKey(PROGRAM_ID, 6)}`}
          accent
        />
      </div>

      <div className="architecture second-row">
        <Node title="GlobalPolicy PDA" body={shortKey(deriveGlobalPolicyPda(), 7)} />
        <Node title="Authorization PDAs" body="Wallet whitelist / block state" />
        <Node title="TransferStats PDA" body="Daily + lifetime gross volume" />
        <Node title="Token-2022 Mint" body={shortKey(MINT, 7)} />
      </div>

      <div className="panel">
        <div className="panel-title">Security boundary</div>
        <p className="muted">
          The GlobalPolicy PDA owns the TransferFeeConfig authority. The admin
          wallet cannot directly change the native Token-2022 fee; it must call
          this program, which signs the CPI with PDA seeds. Transfer-hook account
          inputs are independently validated instead of trusting the
          ExtraAccountMetaList alone.
        </p>
      </div>
    </section>
  );
}

function Node({
  title,
  body,
  accent = false
}: {
  title: string;
  body: string;
  accent?: boolean;
}) {
  return (
    <div className={`architecture-node ${accent ? "accent" : ""}`}>
      <strong>{title}</strong>
      <span>{body}</span>
    </div>
  );
}

function Arrow() {
  return <div className="architecture-arrow">→</div>;
}
