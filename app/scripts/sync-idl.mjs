import { copyFileSync, existsSync, mkdirSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const appRoot = resolve(here, "..");
const repoRoot = resolve(appRoot, "..");

const source = resolve(repoRoot, "target/idl/token2022_compliance.json");
const destination = resolve(appRoot, "public/token2022_compliance.json");

mkdirSync(dirname(destination), { recursive: true });

if (existsSync(source)) {
  copyFileSync(source, destination);
  console.log(`IDL synced: ${source} -> ${destination}`);
  process.exit(0);
}

if (existsSync(destination)) {
  console.log(`Using committed public IDL: ${destination}`);
  process.exit(0);
}

console.error(`IDL not found at either:`);
console.error(`  ${source}`);
console.error(`  ${destination}`);
console.error(
  "Copy the existing Anchor-generated target/idl/token2022_compliance.json into app/public/."
);
process.exit(1);
