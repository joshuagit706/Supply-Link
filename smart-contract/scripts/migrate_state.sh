#!/usr/bin/env bash
# migrate_state.sh
# Orchestrates a full contract upgrade with pre-snapshot, deploy, and post-validation.
# On failure, prints rollback instructions.
#
# Usage:
#   CONTRACT_ID=C... NETWORK=testnet SOURCE=deployer \
#     ./migrate_state.sh
set -euo pipefail

NETWORK="${NETWORK:-testnet}"
SOURCE="${SOURCE:?Set SOURCE to your Stellar account alias}"
OLD_CONTRACT_ID="${CONTRACT_ID:?Set CONTRACT_ID to the current contract address}"
WASM="target/wasm32-unknown-unknown/release/supply_link.wasm"
SNAPSHOT_BASE="./snapshots"
SNAPSHOT_DIR="$SNAPSHOT_BASE/$(date +%Y%m%d_%H%M%S)"

echo "🚀 Supply-Link Contract Migration"
echo "   Network:      $NETWORK"
echo "   Old contract: $OLD_CONTRACT_ID"
echo ""

# ── Step 1: Build ─────────────────────────────────────────────────────────────
echo "Step 1/4: Building WASM..."
cargo build --target wasm32-unknown-unknown --release
echo "  ✅ Build complete"

# ── Step 2: Pre-upgrade snapshot ─────────────────────────────────────────────
echo "Step 2/4: Taking pre-upgrade snapshot..."
CONTRACT_ID="$OLD_CONTRACT_ID" SNAPSHOT_DIR="$SNAPSHOT_DIR" \
  NETWORK="$NETWORK" SOURCE="$SOURCE" \
  "$(dirname "$0")/pre_upgrade_snapshot.sh"

# ── Step 3: Deploy new contract ───────────────────────────────────────────────
echo "Step 3/4: Deploying new contract..."
NEW_CONTRACT_ID=$(stellar contract deploy \
  --wasm "$WASM" \
  --network "$NETWORK" \
  --source "$SOURCE" \
  --ignore-checks)

echo "  ✅ New contract deployed: $NEW_CONTRACT_ID"
echo "$NEW_CONTRACT_ID" > "$SNAPSHOT_DIR/new_contract_id.txt"

# ── Step 4: Post-upgrade smoke test ──────────────────────────────────────────
echo "Step 4/4: Running post-upgrade smoke test..."
if CONTRACT_ID="$NEW_CONTRACT_ID" SNAPSHOT_DIR="$SNAPSHOT_DIR" \
   NETWORK="$NETWORK" SOURCE="$SOURCE" \
   "$(dirname "$0")/post_upgrade_smoke_test.sh"; then
  echo ""
  echo "✅ Migration complete!"
  echo "   New contract ID: $NEW_CONTRACT_ID"
  echo "   Update NEXT_PUBLIC_CONTRACT_ID in your environment:"
  echo "   export NEXT_PUBLIC_CONTRACT_ID=$NEW_CONTRACT_ID"
else
  echo ""
  echo "❌ Migration validation FAILED"
  echo ""
  echo "── Rollback instructions ──────────────────────────────────────────────"
  echo "  The old contract is still live at: $OLD_CONTRACT_ID"
  echo "  No state was migrated — the old contract is unchanged."
  echo "  Ensure NEXT_PUBLIC_CONTRACT_ID remains: $OLD_CONTRACT_ID"
  echo "  Snapshot saved at: $SNAPSHOT_DIR"
  echo "───────────────────────────────────────────────────────────────────────"
  exit 1
fi
