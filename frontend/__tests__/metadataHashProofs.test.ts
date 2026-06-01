import { describe, it, expect } from 'vitest';
import { computeContentHash, verifyContentHash } from '@/lib/utils/metadata';

const enc = new TextEncoder();

describe('#397 Metadata content hash proofs', () => {
  it('produces a 64-char hex SHA-256 digest', async () => {
    const hash = await computeContentHash(enc.encode('hello').buffer);
    expect(hash).toHaveLength(64);
    expect(hash).toMatch(/^[0-9a-f]+$/);
  });

  it('same content produces same hash', async () => {
    const a = await computeContentHash(enc.encode('supply-link').buffer);
    const b = await computeContentHash(enc.encode('supply-link').buffer);
    expect(a).toBe(b);
  });

  it('different content produces different hash', async () => {
    const a = await computeContentHash(enc.encode('foo').buffer);
    const b = await computeContentHash(enc.encode('bar').buffer);
    expect(a).not.toBe(b);
  });

  it('verifyContentHash returns true for matching hash', async () => {
    const data = enc.encode('authentic payload').buffer;
    const hash = await computeContentHash(data);
    expect(await verifyContentHash(data, hash)).toBe(true);
  });

  it('verifyContentHash returns false for tampered content', async () => {
    const original = enc.encode('original').buffer;
    const hash = await computeContentHash(original);
    const tampered = enc.encode('tampered').buffer;
    expect(await verifyContentHash(tampered, hash)).toBe(false);
  });

  it('verifyContentHash returns false for wrong hash', async () => {
    const data = enc.encode('data').buffer;
    expect(await verifyContentHash(data, 'a'.repeat(64))).toBe(false);
  });
});
