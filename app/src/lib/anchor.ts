import {
  AnchorProvider,
  Program,
  type Idl
} from "@anchor-lang/core";
import type { AnchorWallet } from "@solana/wallet-adapter-react";
import type { Connection, PublicKey } from "@solana/web3.js";

import { PROGRAM_ID } from "../config";

let cachedIdl: Idl | null = null;

export async function loadIdl(): Promise<Idl> {
  if (cachedIdl) return cachedIdl;

  const response = await fetch("/token2022_compliance.json", {
    cache: "no-store"
  });

  if (!response.ok) {
    throw new Error(
      "Program IDL was not found. Run `npm run sync-idl` from app/."
    );
  }

  cachedIdl = (await response.json()) as Idl;
  return cachedIdl;
}

export async function createAnchorProgram(
  connection: Connection,
  wallet: AnchorWallet
) {
  const idl = await loadIdl();

  const idlAddress = (idl as any).address;
  if (idlAddress && idlAddress !== PROGRAM_ID.toBase58()) {
    throw new Error(
      `IDL program address ${idlAddress} does not match deployed program ${PROGRAM_ID.toBase58()}.`
    );
  }

  const provider = new AnchorProvider(connection, wallet, {
    commitment: "confirmed",
    preflightCommitment: "confirmed"
  });

  return new Program(idl, provider);
}

function normalize(value: string) {
  return value.replaceAll("_", "").toLowerCase();
}

function flattenInstructionAccounts(
  accounts: any[],
  result: string[] = []
): string[] {
  for (const account of accounts ?? []) {
    if (Array.isArray(account.accounts)) {
      flattenInstructionAccounts(account.accounts, result);
    } else if (account.name) {
      result.push(account.name);
    }
  }
  return result;
}

function getInstruction(program: Program, method: string) {
  return (program.idl.instructions as any[]).find(
    (ix) => normalize(ix.name) === normalize(method)
  );
}

function selectAccounts(
  program: Program,
  method: string,
  candidates: Record<string, PublicKey>
) {
  const instruction = getInstruction(program, method);
  if (!instruction) {
    throw new Error(`Instruction '${method}' is missing from the copied IDL.`);
  }

  const expected = flattenInstructionAccounts(instruction.accounts);
  const selected: Record<string, PublicKey> = {};

  for (const expectedName of expected) {
    const candidateEntry = Object.entries(candidates).find(
      ([candidateName]) => normalize(candidateName) === normalize(expectedName)
    );

    if (candidateEntry) {
      selected[expectedName] = candidateEntry[1];
    }
  }

  return selected;
}

function getMethod(program: Program, method: string) {
  const methods = (program as any).methods;

  const direct = methods[method];
  if (direct) return direct;

  const camel = method.replace(/_([a-z])/g, (_, c) => c.toUpperCase());
  if (methods[camel]) return methods[camel];

  throw new Error(`Anchor client could not find method '${method}'.`);
}

export async function sendAnchorInstruction(
  program: Program,
  method: string,
  args: unknown[],
  candidates: Record<string, PublicKey>
) {
  const fn = getMethod(program, method);
  const builder = fn(...args);
  const accounts = selectAccounts(program, method, candidates);

  return builder.accounts(accounts as any).rpc();
}

export async function fetchAnchorAccount<T>(
  program: Program,
  accountName: string,
  address: PublicKey
): Promise<T | null> {
  const namespace = (program as any).account;
  const direct = namespace[accountName];

  if (direct) {
    return (await direct.fetchNullable(address)) as T | null;
  }

  const normalized = normalize(accountName);
  const actualKey = Object.keys(namespace).find(
    (key) => normalize(key) === normalized
  );

  if (!actualKey) {
    throw new Error(`IDL account '${accountName}' not found.`);
  }

  return (await namespace[actualKey].fetchNullable(address)) as T | null;
}
