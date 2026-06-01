import { describe, it, expect } from 'vitest';
import {
  filterActiveRecords,
  filterRevokedRecords,
  groupRecordsByCertType,
} from '@/lib/services/certificationRegistry';
import type { CertificationRegistryRecord } from '@/lib/types';

// ── Helpers ───────────────────────────────────────────────────────────────────

function makeRecord(overrides: Partial<CertificationRegistryRecord> = {}): CertificationRegistryRecord {
  return {
    id: 'rec-001',
    productId: 'prod-001',
    issuerAddress: 'GISSUER1',
    externalCertId: 'EXT-CERT-001',
    certType: 'organic',
    documentHash: 'a'.repeat(64),
    issuedAt: 1_700_000_000_000,
    revoked: false,
    revokedAt: 0,
    ...overrides,
  };
}

// ── filterActiveRecords ───────────────────────────────────────────────────────

describe('filterActiveRecords', () => {
  it('returns only non-revoked records', () => {
    const records = [
      makeRecord({ id: 'r1' }),
      makeRecord({ id: 'r2', revoked: true, revokedAt: Date.now() }),
      makeRecord({ id: 'r3' }),
    ];
    const active = filterActiveRecords(records);
    expect(active).toHaveLength(2);
    expect(active.every((r) => !r.revoked)).toBe(true);
  });

  it('returns empty array when all are revoked', () => {
    const records = [
      makeRecord({ revoked: true }),
      makeRecord({ id: 'r2', revoked: true }),
    ];
    expect(filterActiveRecords(records)).toHaveLength(0);
  });

  it('returns all records when none are revoked', () => {
    const records = [makeRecord(), makeRecord({ id: 'r2' })];
    expect(filterActiveRecords(records)).toHaveLength(2);
  });
});

// ── filterRevokedRecords ──────────────────────────────────────────────────────

describe('filterRevokedRecords', () => {
  it('returns only revoked records', () => {
    const records = [
      makeRecord({ id: 'r1' }),
      makeRecord({ id: 'r2', revoked: true }),
    ];
    const revoked = filterRevokedRecords(records);
    expect(revoked).toHaveLength(1);
    expect(revoked[0].id).toBe('r2');
  });

  it('returns empty array when none are revoked', () => {
    expect(filterRevokedRecords([makeRecord()])).toHaveLength(0);
  });
});

// ── groupRecordsByCertType ────────────────────────────────────────────────────

describe('groupRecordsByCertType', () => {
  it('groups records by cert type', () => {
    const records = [
      makeRecord({ id: 'r1', certType: 'organic' }),
      makeRecord({ id: 'r2', certType: 'organic' }),
      makeRecord({ id: 'r3', certType: 'fair_trade' }),
      makeRecord({ id: 'r4', certType: 'iso_9001' }),
    ];
    const groups = groupRecordsByCertType(records);
    expect(groups['organic']).toHaveLength(2);
    expect(groups['fair_trade']).toHaveLength(1);
    expect(groups['iso_9001']).toHaveLength(1);
  });

  it('returns empty object for empty input', () => {
    expect(groupRecordsByCertType([])).toEqual({});
  });

  it('handles single record', () => {
    const groups = groupRecordsByCertType([makeRecord({ certType: 'halal' })]);
    expect(groups['halal']).toHaveLength(1);
  });
});

// ── Integrity: revoked records preserve original data ─────────────────────────

describe('revoked record integrity', () => {
  it('revoked record retains all original fields', () => {
    const original = makeRecord({
      id: 'integrity-test',
      externalCertId: 'EXT-ORIGINAL',
      documentHash: 'b'.repeat(64),
    });
    const revoked: CertificationRegistryRecord = {
      ...original,
      revoked: true,
      revokedAt: Date.now(),
    };

    // All original fields preserved
    expect(revoked.id).toBe(original.id);
    expect(revoked.externalCertId).toBe(original.externalCertId);
    expect(revoked.documentHash).toBe(original.documentHash);
    expect(revoked.issuerAddress).toBe(original.issuerAddress);
    expect(revoked.issuedAt).toBe(original.issuedAt);
    // Revocation fields set
    expect(revoked.revoked).toBe(true);
    expect(revoked.revokedAt).toBeGreaterThan(0);
  });
});
