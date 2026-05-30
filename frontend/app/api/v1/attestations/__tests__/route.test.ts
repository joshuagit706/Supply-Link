/**
 * Tests for GET /api/v1/attestations and POST /api/v1/attestations
 */

import { describe, it, expect, vi } from 'vitest';
import { NextRequest } from 'next/server';

// ── Mock dependencies ─────────────────────────────────────────────────────────

vi.mock('@/lib/api/rateLimit', () => ({
  applyRateLimit: () => null,
  RATE_LIMIT_PRESETS: { default: {}, publicRead: {}, authenticated: {} },
}));

vi.mock('@/lib/api/auth', () => ({
  authenticateApiRequest: () => Promise.resolve({ apiKey: 'test-key', error: null }),
}));

vi.mock('@/lib/api/metrics', () => ({
  recordRequest: () => {},
}));

vi.mock('@/lib/api/cors', () => ({
  withCors: (_req: unknown, res: unknown) => res,
  handleOptions: () => new Response(null, { status: 204 }),
}));

vi.mock('@/lib/api/errors', () => ({
  withCorrelationId: (_req: unknown, res: unknown) => res,
  apiError: (_req: unknown, status: number, _code: unknown, message: string) =>
    new Response(JSON.stringify({ error: { message } }), { status }),
  ErrorCode: {
    VALIDATION_ERROR: 'VALIDATION_ERROR',
    MISSING_FIELDS: 'MISSING_FIELDS',
    INVALID_PAYLOAD: 'INVALID_PAYLOAD',
  },
}));

vi.mock('@/lib/api/idempotency', () => ({
  withIdempotency: (_req: unknown, handler: (req: unknown, body: string) => unknown) =>
    handler(_req, ''),
}));

// ── Import after mocks ────────────────────────────────────────────────────────

import { GET, POST } from '../route';

// ── Helpers ───────────────────────────────────────────────────────────────────

function makeRequest(method: string, url: string, body?: unknown): NextRequest {
  const init: RequestInit = { method };
  if (body) {
    init.body = JSON.stringify(body);
    init.headers = { 'Content-Type': 'application/json' };
  }
  return new NextRequest(url, init);
}

// ── Tests ─────────────────────────────────────────────────────────────────────

describe('GET /api/v1/attestations', () => {
  it('returns attestations for a product', async () => {
    const req = makeRequest(
      'GET',
      'http://localhost/api/v1/attestations?productId=prod-001',
    );
    const res = await GET(req);
    expect(res.status).toBe(200);

    const body = await res.json();
    expect(body).toHaveProperty('items');
    expect(Array.isArray(body.items)).toBe(true);
    // All returned attestations should belong to prod-001
    body.items.forEach((a: { productId: string }) => {
      expect(a.productId).toBe('prod-001');
    });
  });

  it('returns 400 when productId is missing', async () => {
    const req = makeRequest('GET', 'http://localhost/api/v1/attestations');
    const res = await GET(req);
    expect(res.status).toBe(400);
  });

  it('filters by targetId when provided', async () => {
    const req = makeRequest(
      'GET',
      'http://localhost/api/v1/attestations?productId=prod-001&targetId=',
    );
    const res = await GET(req);
    expect(res.status).toBe(200);

    const body = await res.json();
    // targetId='' means product-level attestations only
    body.items.forEach((a: { targetId: string }) => {
      expect(a.targetId).toBe('');
    });
  });

  it('returns empty list for unknown product', async () => {
    const req = makeRequest(
      'GET',
      'http://localhost/api/v1/attestations?productId=nonexistent',
    );
    const res = await GET(req);
    expect(res.status).toBe(200);

    const body = await res.json();
    expect(body.items).toHaveLength(0);
    expect(body.total).toBe(0);
  });
});

describe('POST /api/v1/attestations', () => {
  it('rejects submission from unregistered auditor', async () => {
    const req = makeRequest('POST', 'http://localhost/api/v1/attestations', {
      productId: 'prod-001',
      auditor: 'GUNREGISTERED1234567890ABCDEFGHIJKLMNOPQ',
      attestationId: 'att-test-001',
      attestationType: 'quality_check',
      signature: 'deadbeef',
    });
    const res = await POST(req);
    expect(res.status).toBe(403);
  });

  it('rejects submission from inactive auditor', async () => {
    // GAUDITOR3 is inactive in mock data
    const req = makeRequest('POST', 'http://localhost/api/v1/attestations', {
      productId: 'prod-001',
      auditor: 'GAUDITOR3ABCDEFGHIJKLMNOPQRSTUVWXYZ123456',
      attestationId: 'att-test-002',
      attestationType: 'quality_check',
      signature: 'deadbeef',
    });
    const res = await POST(req);
    expect(res.status).toBe(403);
  });

  it('returns 400 when required fields are missing', async () => {
    const req = makeRequest('POST', 'http://localhost/api/v1/attestations', {
      productId: 'prod-001',
      // missing auditor, attestationId, attestationType, signature
    });
    const res = await POST(req);
    expect(res.status).toBe(400);
  });

  it('returns 400 for invalid JSON', async () => {
    const req = new NextRequest('http://localhost/api/v1/attestations', {
      method: 'POST',
      body: 'not-json',
      headers: { 'Content-Type': 'application/json' },
    });
    const res = await POST(req);
    expect(res.status).toBe(400);
  });
});
