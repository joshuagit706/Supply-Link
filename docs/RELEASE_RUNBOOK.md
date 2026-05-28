# Supply-Link Release Runbook

> Covers every production release scenario: smart contract deployment, frontend
> deployment, testnet-to-mainnet promotion, and post-deployment verification.

---

## Table of Contents

1. [Prerequisites](#1-prerequisites)
2. [Release Types](#2-release-types)
3. [Contract Deployment](#3-contract-deployment)
4. [Frontend Deployment](#4-frontend-deployment)
5. [Testnet → Mainnet Promotion](#5-testnet--mainnet-promotion)
6. [Post-Deployment Verification](#6-post-deployment-verification)
7. [Rollback Procedures](#7-rollback-procedures)
8. [Secrets & Environment Configuration](#8-secrets--environment-configuration)
9. [Release Checklist (Quick Reference)](#9-release-checklist-quick-reference)
10. [Release Log](#10-release-log)

---

## 1. Prerequisites

### Tooling

| Tool                      | Version | Check                            |
| ------------------------- | ------- | -------------------------------- |
| `stellar-cli`             | ≥ 21    | `stellar --version`              |
| `cargo` + `wasm32` target | stable  | `rustup target list --installed` |
| `jq`                      | any     | `jq --version`                   |
| Node.js                   | ≥ 20    | `node --version`                 |
| Vercel CLI                | latest  | `vercel --version`               |
| `gh` (GitHub CLI)         | any     | `gh --version`                   |

### Environment variables

Set these before running any release command:

```bash
export NETWORK=testnet          # testnet | mainnet
export SOURCE=alice             # stellar keys alias (funded account)
export OLD_CONTRACT=<current-address>
export VERCEL_TOKEN=<token>     # from Vercel dashboard
```

### Access requirements

- Funded Stellar account with ≥ 10 XLM for deployment gas
- Vercel project write access
- GitHub repository write access (to update `NEXT_PUBLIC_CONTRACT_ID` secret)
- Access to the `#supply-link-ops` channel for incident communication

---

## 2. Release Types

| Type                 | Trigger                             | Contract change | Frontend change                 |
| -------------------- | ----------------------------------- | --------------- | ------------------------------- |
| **Frontend-only**    | UI/UX fix, no contract changes      | No              | Yes                             |
| **Contract upgrade** | On-chain logic or data model change | Yes             | Possibly (new contract address) |
| **Full release**     | Feature touching both layers        | Yes             | Yes                             |
| **Hotfix**           | Critical production bug             | May be either   | May be either                   |

Choose the applicable section(s) below based on the release type.

---

## 3. Contract Deployment

> For a full upgrade procedure including state migration, also see
> [`docs/upgrade/CONTRACT_UPGRADE_RUNBOOK.md`](upgrade/CONTRACT_UPGRADE_RUNBOOK.md).

### 3.1 Pre-deployment checks

```bash
# 1. Snapshot existing on-chain state
bash smart-contract/scripts/pre_upgrade_snapshot.sh
# Writes: upgrade-snapshot-<timestamp>.json

# 2. Build and test
cd smart-contract
cargo build --target wasm32-unknown-unknown --release
cargo test --all
cargo clippy --all-targets --all-features -- -D warnings

# 3. Dry-run simulation on testnet
bash smart-contract/scripts/simulate_upgrade.sh
```

All three steps must exit 0 before proceeding.

### 3.2 Deploy contract

```bash
# Deploy to the target network
SOURCE=$SOURCE bash smart-contract/scripts/deploy.sh

# Copy the printed contract address and export it
export NEW_CONTRACT=<printed-address>
echo "New contract: $NEW_CONTRACT"
```

### 3.3 Migrate state (upgrade scenario only)

Skip this step for fresh deployments (no existing state).

```bash
bash smart-contract/scripts/migrate_state.sh
# Uses upgrade-snapshot-<timestamp>.json
# Idempotent — safe to re-run on partial failure
```

Monitor output for `ERROR` lines. Address any errors before continuing.

### 3.4 Verify WASM hash

```bash
stellar contract info --id "$NEW_CONTRACT" --network "$NETWORK" | jq '.wasm_hash'
# Compare with the hash reported by cargo build
```

### 3.5 Update contract address in client config

```bash
# In frontend/.env.local (local) or Vercel project env (production):
NEXT_PUBLIC_CONTRACT_ID=<NEW_CONTRACT>
```

For CI/CD secrets:

```bash
gh secret set NEXT_PUBLIC_CONTRACT_ID --body "$NEW_CONTRACT"
```

---

## 4. Frontend Deployment

### 4.1 Pre-deployment checks

```bash
cd frontend
npm ci
npm test -- --run         # unit tests must pass
npx tsc --noEmit          # type-check must pass
npm run build             # production build must succeed
```

### 4.2 Deploy to Vercel (production)

```bash
# Deploy and promote to production alias
vercel --prod --token "$VERCEL_TOKEN"
```

Or trigger via Git push to `main` if Vercel GitHub integration is enabled:

```bash
git push origin main
```

### 4.3 Verify deployment URL

```bash
# Confirm the deployment is live and returns 200
DEPLOY_URL="https://supply-link.vercel.app"
curl -sSf "$DEPLOY_URL" -o /dev/null && echo "Frontend OK"
```

### 4.4 Environment variable checklist

| Variable                      | Required                   | Where set                 |
| ----------------------------- | -------------------------- | ------------------------- |
| `NEXT_PUBLIC_CONTRACT_ID`     | Yes                        | Vercel project settings   |
| `NEXT_PUBLIC_STELLAR_NETWORK` | Yes (`testnet`\|`mainnet`) | Vercel project settings   |
| `STELLAR_FEE_BUMP_SECRET`     | Yes (production)           | Vercel environment secret |
| `BLOB_READ_WRITE_TOKEN`       | Optional                   | Vercel environment secret |
| `KV_REST_API_URL`             | Optional                   | Vercel environment secret |
| `KV_REST_API_TOKEN`           | Optional                   | Vercel environment secret |

---

## 5. Testnet → Mainnet Promotion

This procedure promotes a release that has been validated on testnet to mainnet.

### 5.1 Mainnet pre-flight

```bash
# Switch to mainnet
export NETWORK=mainnet
export SOURCE=<mainnet-funded-alias>

# Confirm account balance (needs ≥ 10 XLM)
stellar account show --id "$(stellar keys address $SOURCE)" --network mainnet \
  | jq '.balances[] | select(.asset_type=="native") | .balance'
```

### 5.2 Snapshot mainnet state (if upgrading existing contract)

```bash
OLD_CONTRACT=<current-mainnet-contract> \
bash smart-contract/scripts/pre_upgrade_snapshot.sh
```

### 5.3 Deploy to mainnet

```bash
SOURCE=$SOURCE bash smart-contract/scripts/deploy.sh
export MAINNET_CONTRACT=<printed-address>
```

### 5.4 Migrate state to mainnet contract

```bash
bash smart-contract/scripts/migrate_state.sh
```

### 5.5 Update Vercel production environment

```bash
# Update contract ID for production
vercel env add NEXT_PUBLIC_CONTRACT_ID production --token "$VERCEL_TOKEN"
# Enter the value: $MAINNET_CONTRACT

vercel env add NEXT_PUBLIC_STELLAR_NETWORK production --token "$VERCEL_TOKEN"
# Enter the value: mainnet

# Redeploy frontend to pick up new env vars
vercel --prod --token "$VERCEL_TOKEN"
```

### 5.6 Switchover checklist

```
[ ] Mainnet contract deployed and address recorded
[ ] State migrated (if upgrade)
[ ] NEXT_PUBLIC_CONTRACT_ID updated to mainnet address
[ ] NEXT_PUBLIC_STELLAR_NETWORK set to mainnet
[ ] Frontend redeployed
[ ] Post-deployment smoke tests passed (Section 6)
[ ] Team notified in #supply-link-ops
```

---

## 6. Post-Deployment Verification

Run these checks after every contract or frontend deployment.

### 6.1 Automated smoke test

```bash
bash smart-contract/scripts/post_upgrade_smoke_test.sh
# Exits 0 on success, non-zero on failure
```

The smoke test verifies:

- `get_product_count` matches the pre-deployment snapshot
- Spot-checks 5 random products (fields + event counts)
- `product_exists` behaves correctly for known and unknown IDs
- `add_tracking_event` succeeds on the new contract

### 6.2 Health endpoint

```bash
HEALTH_URL="https://supply-link.vercel.app/api/health"
curl -sSf "$HEALTH_URL" | jq .
```

Expected response:

```json
{
  "status": "ok",
  "contract": "reachable",
  "fee_bump": "configured",
  "blob": "configured | degraded",
  "kv": "configured | degraded"
}
```

`status: "ok"` is required. `blob` and `kv` may be `degraded` without blocking a release.

### 6.3 Frontend functional verification

Perform these manual checks after every release:

```
[ ] Homepage loads and displays product search
[ ] Connect Freighter wallet — wallet connect succeeds
[ ] Register a test product — transaction confirmed on-chain
[ ] Add a tracking event — event appears in history
[ ] Scan / visit /verify/<product-id> — product history visible
[ ] QR code generation works on product detail page
[ ] Dashboard analytics page loads without errors
```

### 6.4 Metrics and SLO check

After a mainnet release, monitor the following for 30 minutes:

| Metric                     | Threshold | Check via        |
| -------------------------- | --------- | ---------------- |
| API p95 latency            | < 500 ms  | Vercel Analytics |
| Frontend error rate        | < 0.1 %   | Vercel Logs      |
| Fee-bump `200` rate        | = 100 %   | `/api/health`    |
| Contract call success rate | > 99 %    | RPC logs         |

---

## 7. Rollback Procedures

### 7.1 Frontend rollback

Revert to the previous Vercel deployment:

```bash
# List recent deployments
vercel list --token "$VERCEL_TOKEN"

# Promote the last known-good deployment to production
vercel promote <deployment-url> --token "$VERCEL_TOKEN"
```

Or via the Vercel dashboard: **Deployments** → find last good deploy → **Promote to Production**.

### 7.2 Contract rollback

Soroban contracts are immutable; the "rollback" is to point clients back at the old contract address.

```bash
# Revert NEXT_PUBLIC_CONTRACT_ID to OLD_CONTRACT
vercel env add NEXT_PUBLIC_CONTRACT_ID production --token "$VERCEL_TOKEN"
# Enter the value: $OLD_CONTRACT

# Redeploy frontend
vercel --prod --token "$VERCEL_TOKEN"
```

The old contract remains live on-chain and retains all its state — no data is lost.

### 7.3 Rollback triggers

Initiate rollback immediately if any of the following are observed within 30 minutes of go-live:

- Smoke test exits non-zero
- Health endpoint returns `status: "error"`
- Any product count or event count mismatch vs. snapshot
- p95 latency exceeds 1 s for more than 5 minutes
- User-reported inability to register products or view history

### 7.4 Post-rollback actions

1. Notify team in `#supply-link-ops` with the rollback reason and timestamp.
2. Open a GitHub issue referencing this runbook and the failed deployment.
3. Schedule a post-mortem within 48 hours.
4. Do not re-deploy until the root cause is identified and resolved.

---

## 8. Secrets & Environment Configuration

All secrets are managed through environment variables. No secret value may appear in source code or be committed to the repository.

For the full secret inventory, rotation schedule, and emergency rotation procedure see:

> [`docs/SECRET_ROTATION_RUNBOOK.md`](SECRET_ROTATION_RUNBOOK.md)

### 8.1 Vercel secret configuration

```bash
# Add or update a secret
vercel env add <VAR_NAME> <environment> --token "$VERCEL_TOKEN"
# environment: production | preview | development

# List current env vars (values redacted)
vercel env ls --token "$VERCEL_TOKEN"

# Remove a secret
vercel env rm <VAR_NAME> <environment> --token "$VERCEL_TOKEN"
```

### 8.2 Local development

Copy `.env.example` files and fill in values:

```bash
cp .env.example .env.local
cp frontend/.env.example frontend/.env.local
# Edit .env.local — never commit this file
```

### 8.3 Secret validation at startup

The frontend validates secrets on startup via `frontend/lib/secrets.ts`. If a critical secret is missing or invalid, the affected endpoint returns `503` with a generic error message (no secret value is ever exposed).

To validate secrets locally:

```bash
cd frontend
node -e "
const { validateSecrets } = require('./lib/secrets');
validateSecrets().forEach(r => console.log(r.key, r.present ? '✓' : '✗', r.reason ?? ''));
"
```

### 8.4 CI secret scanning

Every pull request runs `bash scripts/check-secrets.sh` via the CI workflow, which fails the build if any source file contains a pattern matching a known secret format.

---

## 9. Release Checklist (Quick Reference)

### Frontend-only release

```
[ ] npm ci && npm test -- --run           (tests pass)
[ ] npx tsc --noEmit                      (types pass)
[ ] npm run build                          (build succeeds)
[ ] vercel --prod                          (deployed)
[ ] curl /api/health → status: ok
[ ] Manual smoke test (Section 6.3)
```

### Contract upgrade release

```
[ ] pre_upgrade_snapshot.sh               (snapshot taken)
[ ] cargo test && cargo clippy            (tests/lint pass)
[ ] simulate_upgrade.sh                   (dry-run passes)
[ ] deploy.sh                             (contract deployed)
[ ] migrate_state.sh                      (state migrated)
[ ] Update NEXT_PUBLIC_CONTRACT_ID        (env updated)
[ ] vercel --prod                          (frontend redeployed)
[ ] post_upgrade_smoke_test.sh            (smoke test passes)
[ ] curl /api/health → status: ok
[ ] Manual smoke test (Section 6.3)
[ ] Record in Release Log (Section 10)
```

### Mainnet promotion (additional)

```
[ ] Testnet release validated for ≥ 24 hours
[ ] Mainnet account funded (≥ 10 XLM)
[ ] NEXT_PUBLIC_STELLAR_NETWORK=mainnet set
[ ] All Vercel production env vars updated
[ ] Post-deployment monitoring (30 min, Section 6.4)
[ ] Team notified in #supply-link-ops
```

---

## 10. Release Log

| Date | Network | Type | Contract Address | Frontend URL | Outcome | Author |
| ---- | ------- | ---- | ---------------- | ------------ | ------- | ------ |
| —    | —       | —    | —                | —            | —       | —      |
