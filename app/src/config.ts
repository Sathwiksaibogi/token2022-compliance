import { clusterApiUrl, PublicKey } from "@solana/web3.js";

export const NETWORK = "devnet" as const;

export const PROGRAM_ID = new PublicKey(
  "Bzf35ZWCmtpzrYxQRd3y6x73NDHASarXkxK3RmFsmsW6"
);

export const MINT = new PublicKey(
  "CqLtUZQBK9phoD33ye6sQQG2EdoMrEvraHgijwpy9ZDF"
);

export const ADMIN = new PublicKey(
  "J2GL63eJnVsmJipKkFiaLvwrdSiGnfj5ESpa1HcJXS2G"
);

export const TOKEN_DECIMALS = 6;

const configuredRpc =
  import.meta.env.VITE_RPC_URL?.trim();

const browserProxy =
  typeof window !== "undefined"
    ? `${window.location.origin}/api/rpc`
    : clusterApiUrl("devnet");

export const RPC_ENDPOINT =
  configuredRpc || browserProxy;

export const EXPLORER_BASE = "https://explorer.solana.com";

export function explorerTx(signature: string) {
  return `${EXPLORER_BASE}/tx/${signature}?cluster=devnet`;
}

export function explorerAddress(address: string) {
  return `${EXPLORER_BASE}/address/${address}?cluster=devnet`;
}
