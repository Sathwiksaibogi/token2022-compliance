import type BN from "bn.js";
import { PublicKey } from "@solana/web3.js";

export function shortKey(value?: PublicKey | string | null, size = 4) {
  if (!value) return "—";
  const text = typeof value === "string" ? value : value.toBase58();
  return `${text.slice(0, size)}…${text.slice(-size)}`;
}

export function toBigInt(value: unknown): bigint {
  if (typeof value === "bigint") return value;
  if (typeof value === "number") return BigInt(value);
  if (typeof value === "string") return BigInt(value);

  const maybeBN = value as BN | undefined;
  if (maybeBN && typeof maybeBN.toString === "function") {
    return BigInt(maybeBN.toString());
  }

  return 0n;
}

export function formatUnits(value: bigint, decimals: number, maxFraction = 6) {
  const negative = value < 0n;
  const n = negative ? -value : value;
  const base = 10n ** BigInt(decimals);
  const whole = n / base;
  const fraction = (n % base).toString().padStart(decimals, "0");
  const trimmed = fraction.slice(0, maxFraction).replace(/0+$/, "");
  return `${negative ? "-" : ""}${whole}${trimmed ? `.${trimmed}` : ""}`;
}

export function parseUnits(value: string, decimals: number): bigint {
  const text = value.trim();
  if (!/^\d+(\.\d+)?$/.test(text)) {
    throw new Error("Enter a valid positive token amount.");
  }

  const [whole, fraction = ""] = text.split(".");
  if (fraction.length > decimals) {
    throw new Error(`This mint supports at most ${decimals} decimal places.`);
  }

  const base = 10n ** BigInt(decimals);
  const wholePart = BigInt(whole) * base;
  const fractionPart = BigInt((fraction + "0".repeat(decimals)).slice(0, decimals));

  return wholePart + fractionPart;
}

export function statusName(status: unknown): string {
  if (!status || typeof status !== "object") return "Unknown";
  const key = Object.keys(status as Record<string, unknown>)[0];
  if (!key) return "Unknown";
  return key.charAt(0).toUpperCase() + key.slice(1);
}

export function unixToText(value: unknown) {
  const seconds = Number(toBigInt(value));
  if (!seconds) return "No expiry";
  return new Date(seconds * 1000).toLocaleString();
}
