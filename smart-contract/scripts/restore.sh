#!/usr/bin/env bash
# Restore contract state from backup JSON file
# Usage: SOURCE=alice NETWORK=testnet CONTRACT_ID=... bash scripts/restore.sh backups/contract_backup_*.json
set -euo pipefail

NETWORK="${NETWORK:-testnet}"
SOURCE="${SOURCE:?Set SOURCE to your Stellar account alias}"
CONTRACT_ID="${CONTRACT_ID:?Set CONTRACT_ID to your deployed contract address}"
BACKUP_FILE="${1:?Provide backup file path as argument}"

if [ ! -f "$BACKUP_FILE" ]; then
  echo "Error: Backup file not found: $BACKUP_FILE"
  exit 1
fi

echo "Restoring contract state from $BACKUP_FILE..."

# Validate backup file structure
if ! jq empty "$BACKUP_FILE" 2>/dev/null; then
  echo "Error: Invalid JSON in backup file"
  exit 1
fi

BACKUP_NETWORK=$(jq -r '.network' "$BACKUP_FILE")
BACKUP_CONTRACT=$(jq -r '.contractId' "$BACKUP_FILE")
BACKUP_TIMESTAMP=$(jq -r '.timestamp' "$BACKUP_FILE")

echo "Backup info:"
echo "  Created: $BACKUP_TIMESTAMP"
echo "  Network: $BACKUP_NETWORK"
echo "  Original Contract: $BACKUP_CONTRACT"
echo "  Target Network: $NETWORK"
echo "  Target Contract: $CONTRACT_ID"
echo ""

# Confirm before restoring
read -p "Continue with restore? (y/N) " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
  echo "Restore cancelled."
  exit 0
fi

# Extract products from backup
PRODUCTS=$(jq -r '.data.products[]' "$BACKUP_FILE")

RESTORED_COUNT=0
FAILED_COUNT=0

echo "Restoring products..."

while IFS= read -r product; do
  PRODUCT_ID=$(echo "$product" | jq -r '.id')
  PRODUCT_NAME=$(echo "$product" | jq -r '.name')
  PRODUCT_ORIGIN=$(echo "$product" | jq -r '.origin')
  PRODUCT_OWNER=$(echo "$product" | jq -r '.owner')

  echo -n "  Restoring product $PRODUCT_ID... "

  # Register product
  if stellar contract invoke \
    --network "$NETWORK" \
    --source "$SOURCE" \
    --id "$CONTRACT_ID" \
    -- register_product \
    --id "$PRODUCT_ID" \
    --name "$PRODUCT_NAME" \
    --origin "$PRODUCT_ORIGIN" \
    --owner "$PRODUCT_OWNER" 2>/dev/null; then
    
    echo "✓"
    RESTORED_COUNT=$((RESTORED_COUNT + 1))

    # Restore events for this product
    EVENTS=$(echo "$product" | jq -r '.events[]? // empty')
    
    while IFS= read -r event; do
      EVENT_LOCATION=$(echo "$event" | jq -r '.location')
      EVENT_TYPE=$(echo "$event" | jq -r '.eventType')
      EVENT_METADATA=$(echo "$event" | jq -c '.metadata')

      stellar contract invoke \
        --network "$NETWORK" \
        --source "$SOURCE" \
        --id "$CONTRACT_ID" \
        -- add_tracking_event \
        --product_id "$PRODUCT_ID" \
        --caller "$PRODUCT_OWNER" \
        --location "$EVENT_LOCATION" \
        --event_type "$EVENT_TYPE" \
        --metadata "$EVENT_METADATA" 2>/dev/null || true
    done <<< "$EVENTS"
  else
    echo "✗"
    FAILED_COUNT=$((FAILED_COUNT + 1))
  fi
done <<< "$PRODUCTS"

echo ""
echo "Restore complete:"
echo "  Restored: $RESTORED_COUNT products"
echo "  Failed: $FAILED_COUNT products"

if [ $FAILED_COUNT -eq 0 ]; then
  echo "✓ All products restored successfully"
  exit 0
else
  echo "⚠ Some products failed to restore"
  exit 1
fi
