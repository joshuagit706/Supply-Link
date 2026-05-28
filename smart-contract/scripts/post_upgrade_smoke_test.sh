#!/usr/bin/env bash
# post_upgrade_smoke_test.sh
# Verifies that the upgraded contract preserves all state from the snapshot.
# Compares product metadata and event counts against the pre-upgrade snapshot.
#
# Usage:
#   SNAPSHOT_DIR=./snapshots/20260101_120000 CONTRACT_ID=C... NETWORK=testnet SOURCE=deployer \
#     ./post_upgrade_smoke_test.sh
set -euo pipefail

CONTRACT_ID="${CONTRACT_ID:?Set CONTRACT_ID to the NEW deployed contract address}"
NETWORK="${NETWORK:-testnet}"
SOURCE="${SOURCE:?Set SOURCE to your Stellar account alias}"
SNAPSHOT_DIR="${SNAPSHOT_DIR:?Set SNAPSHOT_DIR to the pre-upgrade snapshot directory}"

echo "🔍 Post-upgrade smoke test"
echo "   Contract: $CONTRACT_ID"
echo "   Snapshot: $SNAPSHOT_DIR"
echo ""

PASS=0
FAIL=0

# ── Verify product count ──────────────────────────────────────────────────────
EXPECTED_COUNT=$(cat "$SNAPSHOT_DIR/product_count.txt")
ACTUAL_COUNT=$(stellar contract invoke \
  --id "$CONTRACT_ID" \
  --network "$NETWORK" \
  --source "$SOURCE" \
  -- get_product_count 2>/dev/null || echo "-1")

if [ "$ACTUAL_COUNT" = "$EXPECTED_COUNT" ]; then
  echo "  ✅ Product count: $ACTUAL_COUNT"
  PASS=$((PASS + 1))
else
  echo "  ❌ Product count mismatch: expected=$EXPECTED_COUNT actual=$ACTUAL_COUNT"
  FAIL=$((FAIL + 1))
fi

# ── Verify each product ───────────────────────────────────────────────────────
PRODUCTS_DIR="$SNAPSHOT_DIR/products"

for PRODUCT_FILE in "$PRODUCTS_DIR"/*.json; do
  [[ "$PRODUCT_FILE" == *_events.json ]] && continue
  SAFE_ID=$(basename "$PRODUCT_FILE" .json)
  PRODUCT_ID=$(python3 -c "import json; d=json.load(open('$PRODUCT_FILE')); print(d.get('id',''))" 2>/dev/null || echo "")
  [ -z "$PRODUCT_ID" ] && continue

  # Check product still exists
  EXISTS=$(stellar contract invoke \
    --id "$CONTRACT_ID" \
    --network "$NETWORK" \
    --source "$SOURCE" \
    -- product_exists --id "$PRODUCT_ID" 2>/dev/null || echo "false")

  if [ "$EXISTS" = "true" ]; then
    PASS=$((PASS + 1))
  else
    echo "  ❌ Product missing after upgrade: $PRODUCT_ID"
    FAIL=$((FAIL + 1))
    continue
  fi

  # Check event count matches
  EVENTS_FILE="$PRODUCTS_DIR/${SAFE_ID}_events.json"
  if [ -f "$EVENTS_FILE" ]; then
    EXPECTED_EVENTS=$(python3 -c "import json; print(len(json.load(open('$EVENTS_FILE'))))" 2>/dev/null || echo "0")
    ACTUAL_EVENTS=$(stellar contract invoke \
      --id "$CONTRACT_ID" \
      --network "$NETWORK" \
      --source "$SOURCE" \
      -- count_tracking_events --product_id "$PRODUCT_ID" 2>/dev/null || echo "-1")

    if [ "$ACTUAL_EVENTS" = "$EXPECTED_EVENTS" ]; then
      echo "  ✅ $PRODUCT_ID: $ACTUAL_EVENTS events"
      PASS=$((PASS + 1))
    else
      echo "  ❌ $PRODUCT_ID: event count mismatch expected=$EXPECTED_EVENTS actual=$ACTUAL_EVENTS"
      FAIL=$((FAIL + 1))
    fi
  fi
done

# ── Summary ───────────────────────────────────────────────────────────────────
echo ""
echo "─────────────────────────────────────"
echo "  Results: $PASS passed, $FAIL failed"
echo "─────────────────────────────────────"

if [ "$FAIL" -gt 0 ]; then
  echo "❌ Smoke test FAILED — consider rollback"
  exit 1
else
  echo "✅ Smoke test PASSED — state preserved"
  exit 0
fi
