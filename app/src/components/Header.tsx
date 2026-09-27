import { WalletMultiButton } from "@solana/wallet-adapter-react-ui";
import { MINT, PROGRAM_ID } from "../config";
import { shortKey } from "../lib/format";

export function Header() {
  return (
    <header className="topbar">
      <div>
        <div className="eyebrow">SOLANA DEVNET · TOKEN-2022</div>
        <h1>Compliance & Dynamic Fee Engine</h1>
        <div className="address-line">
          Program <span>{shortKey(PROGRAM_ID, 7)}</span>
          <span className="dot">•</span>
          Mint <span>{shortKey(MINT, 7)}</span>
        </div>
      </div>
      <WalletMultiButton />
    </header>
  );
}
