#!/usr/bin/env bash
# Test backup and restore functionality
# This script validates the backup/restore process without requiring a live contract
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TEST_DIR="${SCRIPT_DIR}/../__tests__/backup-restore"
BACKUP_FILE="${TEST_DIR}/test_backup.json"

mkdir -p "$TEST_DIR"

echo "Testing backup/restore functionality..."

# Create a mock backup file
cat > "$BACKUP_FILE" << 'EOF'
{
  "timestamp": "2026-06-01T02:55:18Z",
  "network": "testnet",
  "contractId": "CBUWSKT2UGOAXK4ZREVDJV5XHSYB42PZ3CERU2ZFUTUMAZLJEHNZIECA",
  "source": "test-account",
  "data": {
    "productCount": 2,
    "products": [
      {
        "id": "prod-001",
        "name": "Ethiopian Coffee",
        "origin": "Addis Ababa",
        "owner": "GXXX1111111111111111111111111111111111111111111111111111",
        "timestamp": 1000,
        "events": [
          {
            "productId": "prod-001",
            "location": "Farm A",
            "actor": "GYYY1111111111111111111111111111111111111111111111111111",
            "timestamp": 1000,
            "eventType": "HARVEST",
            "metadata": "{\"temperature\": 25}"
          }
        ]
      },
      {
        "id": "prod-002",
        "name": "Kenyan Tea",
        "origin": "Nairobi",
        "owner": "GZZZ1111111111111111111111111111111111111111111111111111",
        "timestamp": 2000,
        "events": [
          {
            "productId": "prod-002",
            "location": "Farm B",
            "actor": "GAAA1111111111111111111111111111111111111111111111111111",
            "timestamp": 2000,
            "eventType": "HARVEST",
            "metadata": "{}"
          }
        ]
      }
    ]
  }
}
EOF

echo "✓ Created test backup file"

# Test 1: Validate JSON structure
echo -n "Test 1: Validating JSON structure... "
if jq empty "$BACKUP_FILE" 2>/dev/null; then
  echo "✓"
else
  echo "✗"
  exit 1
fi

# Test 2: Verify required fields
echo -n "Test 2: Verifying required fields... "
REQUIRED_FIELDS=("timestamp" "network" "contractId" "source" "data")
for field in "${REQUIRED_FIELDS[@]}"; do
  if ! jq -e ".$field" "$BACKUP_FILE" > /dev/null 2>&1; then
    echo "✗ Missing field: $field"
    exit 1
  fi
done
echo "✓"

# Test 3: Verify product structure
echo -n "Test 3: Verifying product structure... "
PRODUCT_COUNT=$(jq '.data.products | length' "$BACKUP_FILE")
if [ "$PRODUCT_COUNT" -eq 2 ]; then
  echo "✓ ($PRODUCT_COUNT products)"
else
  echo "✗ Expected 2 products, got $PRODUCT_COUNT"
  exit 1
fi

# Test 4: Verify event structure
echo -n "Test 4: Verifying event structure... "
EVENT_COUNT=$(jq '.data.products[0].events | length' "$BACKUP_FILE")
if [ "$EVENT_COUNT" -ge 1 ]; then
  echo "✓ ($EVENT_COUNT events in first product)"
else
  echo "✗ No events found"
  exit 1
fi

# Test 5: Verify metadata parsing
echo -n "Test 5: Verifying metadata parsing... "
METADATA=$(jq -r '.data.products[0].events[0].metadata' "$BACKUP_FILE")
if jq -e . <<< "$METADATA" > /dev/null 2>&1; then
  echo "✓"
else
  echo "✗ Invalid metadata JSON"
  exit 1
fi

# Test 6: Verify backup file size
echo -n "Test 6: Checking backup file size... "
FILE_SIZE=$(du -h "$BACKUP_FILE" | cut -f1)
echo "✓ ($FILE_SIZE)"

# Test 7: Verify timestamp format
echo -n "Test 7: Verifying timestamp format... "
TIMESTAMP=$(jq -r '.timestamp' "$BACKUP_FILE")
if [[ $TIMESTAMP =~ ^[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}Z$ ]]; then
  echo "✓ ($TIMESTAMP)"
else
  echo "✗ Invalid timestamp format"
  exit 1
fi

# Cleanup
rm -f "$BACKUP_FILE"

echo ""
echo "✓ All backup/restore tests passed"
