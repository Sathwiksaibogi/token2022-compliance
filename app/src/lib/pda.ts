import { PublicKey } from "@solana/web3.js";
import { MINT, PROGRAM_ID } from "../config";

const enc = new TextEncoder();

export function deriveGlobalPolicyPda() {
  return PublicKey.findProgramAddressSync(
    [enc.encode("policy"), MINT.toBuffer()],
    PROGRAM_ID
  )[0];
}

export function deriveAuthorizationPda(wallet: PublicKey) {
  return PublicKey.findProgramAddressSync(
    [enc.encode("authorization"), MINT.toBuffer(), wallet.toBuffer()],
    PROGRAM_ID
  )[0];
}

export function deriveTransferStatsPda(wallet: PublicKey) {
  return PublicKey.findProgramAddressSync(
    [enc.encode("stats"), MINT.toBuffer(), wallet.toBuffer()],
    PROGRAM_ID
  )[0];
}
