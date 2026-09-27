# Token-2022 Compliance Frontend

React + Vite frontend for the deployed Token-2022 programmable compliance and dynamic fee engine on Solana Devnet.

## Existing on-chain deployment

- Program: `Bzf35ZWCmtpzrYxQRd3y6x73NDHASarXkxK3RmFsmsW6`
- Mint: `CqLtUZQBK9phoD33ye6sQQG2EdoMrEvraHgijwpy9ZDF`
- Admin: `J2GL63eJnVsmJipKkFiaLvwrdSiGnfj5ESpa1HcJXS2G`
- GlobalPolicy PDA: `EmoBiCLorusZGANzxWfhb2brvTMu2pQFGMquxu466Yh8`

## Install inside the Anchor repository

Place this directory at:

```text
token2022-compliance/app/
```

Then:

```bash
cd ~/projects/token2022-compliance/app
npm install
npm run sync-idl
npm run dev
```

`sync-idl` copies:

```text
../target/idl/token2022_compliance.json
```

to:

```text
app/public/token2022_compliance.json
```

Do not run a new `anchor build` merely for the frontend if you want to preserve the previously built sBPF-v2 deployment artifact. Use the existing IDL generated from the current source. If the IDL is stale, regenerate it deliberately and then rebuild the deployable program with the project's required `cargo build-sbf --arch v2` workflow before another Devnet program upgrade.

## RPC

Default:

```text
https://api.devnet.solana.com
```

Optional:

```bash
cp .env.example .env.local
```

Then edit `VITE_RPC_URL`.

Important: every `VITE_*` value is public in the browser bundle. Never place an unrestricted private RPC/API key in it.

## Build

```bash
npm run build
npm run preview
```

## Deploy to Vercel

### CLI

```bash
npx vercel
npx vercel --prod
```

When Vercel asks for the project root, deploy from `app/`.

### GitHub dashboard

Import the `token2022-compliance` repository and set:

```text
Root Directory: app
Framework Preset: Vite
Build Command: npm run build
Output Directory: dist
```

Before the first Git/Vercel deployment, run `npm run sync-idl` locally and commit:

```text
app/public/token2022_compliance.json
```

`target/` should remain ignored. The sync script uses the local target IDL when present and falls back to the committed public IDL during a Vercel build.

The IDL is public metadata; private keypairs must remain ignored.

## Functional areas

- Dashboard: policy, mint fee state, epoch, balances
- Transfer: real Token-2022 transfer with transfer-hook account resolution
- Compliance: authorization and wallet-level transfer statistics
- Admin: authorize/block wallets, update policy, schedule dynamic transfer fees
- Architecture: portfolio/demo explanation

## Security

Never commit:
- Solana wallet keypairs
- mint keypair files
- buffer/deploy keypairs
- RPC URLs containing unrestricted private API keys


## Admin-wallet warning

The currently configured Devnet admin is also the program upgrade authority and mint authority in this demo deployment. Browser-wallet admin actions therefore require the corresponding Devnet wallet to be connected. Do not copy or import any production/mainnet key into a browser extension just for this demo. For a production deployment, separate operational admin, mint authority, and program upgrade authority roles.

The "Devnet demo mint" panel exists only to make the public demo self-contained.
