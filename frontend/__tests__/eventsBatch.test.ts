import { describe, it, expect, beforeEach } from 'vitest';
import { NextRequest } from 'next/server';

function makeRequest(
  method: string,
  body?: unknown,
  url = 'http://localhost/api/v1/products/prod-001/events',
) {
  return new NextRequest(url, {
    method,
    headers: { 'Content-Type': 'application/json' },
    body: body ? JSON.stringify(body) : undefined,
  });
}

beforeEach(() => {
  // Reset module cache to clear mock stores
  // vitest will isolate modules by default in many setups; ensure clean state
});

describe('POST /api/v1/products/[id]/events — batch ingestion', () => {
  it('accepts mixed valid and invalid events and returns per-event results', async () => {
    const { POST } = await import('@/app/api/v1/products/[id]/events/route');

    const payload = [
      {
        eventType: 'HARVEST',
        location: 'Yard',
        actor: 'GACTOR1ABCDEFGHIJKLMNOPQRSTUVWXYZ1234567',
        metadata: '{}',
      },
      { eventType: 'UNKNOWN', location: '', actor: 'BADACTOR', metadata: '{}' },
    ];

    const res = await POST(makeRequest('POST', payload));
    expect(res.status).toBe(200);
    const body = await res.json();
    expect(Array.isArray(body.results)).toBe(true);
    expect(body.results).toHaveLength(2);
    expect(body.results[0].success).toBe(true);
    expect(body.results[1].success).toBe(false);
    expect(typeof body.results[1].error).toBe('string');
  });
});
