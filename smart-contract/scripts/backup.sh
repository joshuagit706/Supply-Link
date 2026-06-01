#!/usr/bin/env bash
# Backup contract state to JSON file
# Usage: SOURCE=alice NETWORK=testnet CONTRACT_ID=... bash scripts/backup.sh
set -euo pipefail

NETWORK="${NETWORK:-testnet}"
SOURCE="${SOURCE:?Set SOURCE to your Stellar account alias}"
CONTRACT_ID="${CONTRACT_ID:?Set CONTRACT_ID to your deployed contract address}"
BACKUP_DIR="${BACKUP_DIR:-.backups}"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
BACKUP_FILE="$BACKUP_DIR/contract_backup_${TIMESTAMP}.json"

mkdir -p "$BACKUP_DIR"

echo "Backing up contract state from $CONTRACT_ID on $NETWORK..."

# Create backup object
{
  echo "{"
  echo "  \"timestamp\": \"$(date -u +%Y-%m-%dT%H:%M:%SZ)\","
  echo "  \"network\": \"$NETWORK\","
  echo "  \"contractId\": \"$CONTRACT_ID\","
  echo "  \"source\": \"$SOURCE\","
  echo "  \"data\": {"

  # Fetch product count
  PRODUCT_COUNT=$(stellar contract invoke \
    --network "$NETWORK" \
    --source "$SOURCE" \
    --id "$CONTRACT_ID" \
    -- get_product_count 2>/dev/null || echo "0")

  echo "    \"productCount\": $PRODUCT_COUNT,"

  # Fetch all products with pagination
  echo "    \"products\": ["
  
  LIMIT=100
  OFFSET=0
  FIRST=true

  while true; do
    PRODUCTS=$(stellar contract invoke \
      --network "$NETWORK" \
      --source "$SOURCE" \
      --id "$CONTRACT_ID" \
      -- list_products --offset "$OFFSET" --limit "$LIMIT" 2>/dev/null || echo "[]")

    if [ "$PRODUCTS" = "[]" ]; then
      break
    fi

    if [ "$FIRST" = false ]; then
      echo ","
    fi
    echo "$PRODUCTS" | jq -c '.[]' | while read -r product_id; do
      if [ "$FIRST" = false ]; then
        echo ","
      fi
      FIRST=false

      # Fetch product details
      PRODUCT=$(stellar contract invoke \
        --network "$NETWORK" \
        --source "$SOURCE" \
        --id "$CONTRACT_ID" \
        -- get_product --id "$product_id" 2>/dev/null || echo "{}")

      echo "$PRODUCT" | jq '.'

      # Fetch events for this product
      EVENTS=$(stellar contract invoke \
        --network "$NETWORK" \
        --source "$SOURCE" \
        --id "$CONTRACT_ID" \
        -- get_tracking_events --product_id "$product_id" 2>/dev/null || echo "[]")

      # Store events in product object
      echo "$PRODUCT" | jq --argjson events "$EVENTS" '.events = $events'
    done

    OFFSET=$((OFFSET + LIMIT))
  done

  echo "    ]"
  echo "  }"
  echo "}"
} > "$BACKUP_FILE"

echo "✓ Backup saved to $BACKUP_FILE"
echo "  File size: $(du -h "$BACKUP_FILE" | cut -f1)"
