import { describe, it, expect } from 'vitest';
import { encodeQrProof, verifyQrProof, type QrProofPayload } from '@/lib/services/offlineVerify';

const sample: QrProofPayload = {
  id: 'prod-1',
  name: 'Coffee Beans',
  origin: 'Ethiopia',
  owner: 'GTEST',
  ts: 1700000000000,
};

describe('#391 Offline QR proof validation', () => {
  it('encodes and verifies a valid proof', async () => {
    const token = await encodeQrProof(sample);
    const result = await verifyQrProof(token);
    expect(result.valid).toBe(true);
    expect(result.payload?.id).toBe('prod-1');
    expect(result.payload?.name).toBe('Coffee Beans');
  });

  it('rejects a tampered payload', async () => {
    const token = await encodeQrProof(sample);
    const dot = token.lastIndexOf('.');
    // corrupt the signature portion
    const tampered = token.slice(0, dot + 1) + 'AAAAAAAAAA' + token.slice(dot + 11);
    const result = await verifyQrProof(tampered);
    expect(result.valid).toBe(false);
  });

  it('rejects a malformed token (no dot)', async () => {
    const result = await verifyQrProof('notavalidtoken');
    expect(result.valid).toBe(false);
    expect(result.error).toBeDefined();
  });

  it('rejects an empty string', async () => {
    const result = await verifyQrProof('');
    expect(result.valid).toBe(false);
  });
});
