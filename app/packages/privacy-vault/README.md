# Privacy Vault

Frontend for the Payy Privacy Vault onboarding flow.

## Live Site

- Production: https://payy-privacy-vault.vercel.app

## Features

- Wallet authentication via EIP-712 typed signature
- Add Payy Testnet to MetaMask
- Request testnet funds from the faucet

## Development

```bash
# From the app/ directory
cd packages/privacy-vault

yarn dev
```

## Build

```bash
# From the app/ directory
cd packages/privacy-vault

yarn build
```

## Deployment

- Hosting: Vercel
- Team: `polybase`
- Project: `payy-privacy-vault`
- Project ID: `prj_n953NkFqyTVQqx8mQWHOvGf2yHkg`

### CI Auto Deploy (Main)

Deploys are automated via:

- `.github/workflows/privacy-vault.release.mainnet.yml`

Trigger:

- Pushes to `main` that change `app/packages/privacy-vault/**`
- Manual `workflow_dispatch`
