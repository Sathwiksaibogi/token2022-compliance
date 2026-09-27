import React from "react";
import ReactDOM from "react-dom/client";


import "@solana/wallet-adapter-react-ui/styles.css";
import "./styles.css";

import { SolanaProvider } from "./providers/SolanaProvider";
import App from "./App";



ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <SolanaProvider>
      <App />
    </SolanaProvider>
  </React.StrictMode>
);
