import { useState } from "react";

import { Header } from "./components/Header";
import { Dashboard } from "./components/Dashboard";
import { TransferPanel } from "./components/TransferPanel";
import { CompliancePanel } from "./components/CompliancePanel";
import { AdminPanel } from "./components/AdminPanel";
import { ArchitecturePanel } from "./components/ArchitecturePanel";

type Tab = "dashboard" | "transfer" | "compliance" | "admin" | "architecture";

const tabs: Array<[Tab, string]> = [
  ["dashboard", "Dashboard"],
  ["transfer", "Transfer"],
  ["compliance", "Compliance"],
  ["admin", "Admin"],
  ["architecture", "Architecture"]
];

export default function App() {
  const [tab, setTab] = useState<Tab>("dashboard");

  return (
    <div className="app-shell">
      <Header />

      <nav className="tabs">
        {tabs.map(([key, label]) => (
          <button
            key={key}
            className={tab === key ? "active" : ""}
            onClick={() => setTab(key)}
          >
            {label}
          </button>
        ))}
      </nav>

      <main>
        {tab === "dashboard" ? <Dashboard /> : null}
        {tab === "transfer" ? <TransferPanel /> : null}
        {tab === "compliance" ? <CompliancePanel /> : null}
        {tab === "admin" ? <AdminPanel /> : null}
        {tab === "architecture" ? <ArchitecturePanel /> : null}
      </main>

      <footer>
        Token-2022 Compliance & Dynamic Fee Engine · Solana Devnet
      </footer>
    </div>
  );
}
