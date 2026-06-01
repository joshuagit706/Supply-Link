# Contract State Backup and Restore

This document describes how to backup and restore Supply-Link contract state for disaster recovery and data migration.

## Overview

The backup/restore system provides:

- **State snapshots**: Export all products and events to JSON
- **Validation**: Verify backup integrity before restore
- **Safety**: Confirmation prompts prevent accidental overwrites
- **Auditability**: Timestamped backups with metadata

## Backup

### Create a Backup

```bash
SOURCE=alice NETWORK=testnet CONTRACT_ID=CBUW... bash smart-contract/scripts/backup.sh
```

**Environment Variables:**

- `SOURCE` (required): Stellar account alias with contract permissions
- `NETWORK` (optional): Stellar network (`testnet` or `mainnet`, default: `testnet`)
- `CONTRACT_ID` (required): Deployed contract address
- `BACKUP_DIR` (optional): Directory to store backups (default: `.backups`)

**Output:**

```
Backing up contract state from CBUW... on testnet...
✓ Backup saved to .backups/contract_backup_20260601_025518.json
  File size: 2.3M
```

### Backup File Structure

```json
{
  "timestamp": "2026-06-01T02:55:18Z",
  "network": "testnet",
  "contractId": "CBUW...",
  "source": "alice",
  "data": {
    "productCount": 42,
    "products": [
      {
        "id": "prod-001",
        "name": "Ethiopian Coffee",
        "origin": "Addis Ababa",
        "owner": "GXXX...",
        "timestamp": 1000,
        "events": [
          {
            "productId": "prod-001",
            "location": "Farm",
            "actor": "GYYY...",
            "timestamp": 1000,
            "eventType": "HARVEST",
            "metadata": "{...}"
          }
        ]
      }
    ]
  }
}
```

## Restore

### Restore from Backup

```bash
SOURCE=alice NETWORK=testnet CONTRACT_ID=CBUW... bash smart-contract/scripts/restore.sh .backups/contract_backup_20260601_025518.json
```

**Environment Variables:**

- `SOURCE` (required): Stellar account alias with contract permissions
- `NETWORK` (optional): Target network (default: `testnet`)
- `CONTRACT_ID` (required): Target contract address

**Interactive Confirmation:**

```
Restoring contract state from .backups/contract_backup_20260601_025518.json...
Backup info:
  Created: 2026-06-01T02:55:18Z
  Network: testnet
  Original Contract: CBUW...
  Target Network: testnet
  Target Contract: CBUW...

Continue with restore? (y/N) y
Restoring products...
  Restoring product prod-001... ✓
  Restoring product prod-002... ✓
  ...

Restore complete:
  Restored: 42 products
  Failed: 0 products
✓ All products restored successfully
```

## Use Cases

### 1. Disaster Recovery

If the contract is corrupted or data is lost:

```bash
# Create backup before any risky operations
SOURCE=alice NETWORK=testnet CONTRACT_ID=CBUW... bash smart-contract/scripts/backup.sh

# If something goes wrong, restore from backup
SOURCE=alice NETWORK=testnet CONTRACT_ID=CBUW... bash smart-contract/scripts/restore.sh .backups/contract_backup_*.json
```

### 2. Contract Upgrade

When upgrading to a new contract version:

```bash
# Backup current state
SOURCE=alice NETWORK=testnet CONTRACT_ID=OLD_CONTRACT bash smart-contract/scripts/backup.sh

# Deploy new contract
SOURCE=alice bash smart-contract/scripts/deploy.sh

# Restore state to new contract
SOURCE=alice NETWORK=testnet CONTRACT_ID=NEW_CONTRACT bash smart-contract/scripts/restore.sh .backups/contract_backup_*.json
```

### 3. Network Migration

Moving from testnet to mainnet:

```bash
# Backup from testnet
SOURCE=alice NETWORK=testnet CONTRACT_ID=TESTNET_CONTRACT bash smart-contract/scripts/backup.sh

# Deploy to mainnet
SOURCE=alice NETWORK=mainnet bash smart-contract/scripts/deploy.sh

# Restore to mainnet
SOURCE=alice NETWORK=mainnet CONTRACT_ID=MAINNET_CONTRACT bash smart-contract/scripts/restore.sh .backups/contract_backup_*.json
```

## Best Practices

### Regular Backups

Schedule automated backups:

```bash
# Backup daily at 2 AM UTC
0 2 * * * cd /path/to/supply-link && SOURCE=alice NETWORK=testnet CONTRACT_ID=CBUW... bash smart-contract/scripts/backup.sh
```

### Backup Retention

Keep multiple backups:

```bash
# Keep backups for 30 days
find .backups -name "contract_backup_*.json" -mtime +30 -delete
```

### Verify Backups

Always test restore in a safe environment:

```bash
# Create test contract
SOURCE=alice NETWORK=testnet bash smart-contract/scripts/deploy.sh

# Test restore
SOURCE=alice NETWORK=testnet CONTRACT_ID=TEST_CONTRACT bash smart-contract/scripts/restore.sh .backups/contract_backup_*.json

# Verify data integrity
stellar contract invoke --network testnet --source alice --id TEST_CONTRACT -- get_product_count
```

### Backup Storage

Store backups securely:

- **Local**: `.backups/` directory (version controlled separately)
- **Remote**: S3, GCS, or other cloud storage
- **Encrypted**: Use GPG or similar for sensitive backups

```bash
# Encrypt backup
gpg --symmetric .backups/contract_backup_*.json

# Decrypt backup
gpg --decrypt .backups/contract_backup_*.json.gpg > backup.json
```

## Troubleshooting

### Backup Fails

**Issue**: `Error: Contract not found`

**Solution**: Verify `CONTRACT_ID` is correct and deployed on the specified network.

```bash
stellar contract info --network testnet --id CBUW...
```

### Restore Fails

**Issue**: `Error: Invalid JSON in backup file`

**Solution**: Verify backup file is valid JSON.

```bash
jq empty .backups/contract_backup_*.json
```

**Issue**: `Error: Product already exists`

**Solution**: Restore to a fresh contract or clear existing data first.

### Partial Restore

If some products fail to restore, check the logs:

```bash
# Re-run restore with verbose output
bash -x smart-contract/scripts/restore.sh .backups/contract_backup_*.json 2>&1 | tee restore.log
```

## Validation

After restore, validate data integrity:

```bash
# Check product count
stellar contract invoke --network testnet --source alice --id CBUW... -- get_product_count

# Check specific product
stellar contract invoke --network testnet --source alice --id CBUW... -- get_product --id prod-001

# Check events for product
stellar contract invoke --network testnet --source alice --id CBUW... -- get_tracking_events --product_id prod-001
```

## Security Considerations

- **Access Control**: Only authorized accounts can backup/restore
- **Audit Trail**: All backups are timestamped and logged
- **Data Sensitivity**: Backups contain all product data; store securely
- **Encryption**: Consider encrypting backups at rest
- **Validation**: Always verify backup integrity before restore

## Related Documentation

- [Contract Upgrade Runbook](./upgrade/CONTRACT_UPGRADE_RUNBOOK.md)
- [Resilience Guide](./RESILIENCE.md)
- [Release Runbook](./RELEASE_RUNBOOK.md)
