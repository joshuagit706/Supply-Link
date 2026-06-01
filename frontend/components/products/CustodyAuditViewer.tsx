'use client';

import { useEffect, useState } from 'react';
import type { CustodyEntry } from '@/app/api/v1/events/signature-ledger/route';

interface CustodyAuditViewerProps {
  productId: string;
}

type AuditState = 'idle' | 'loading' | 'valid' | 'tampered' | 'error';

interface AuditResult {
  state: AuditState;
  entries: CustodyEntry[];
  /** Index of the first broken link, or -1 if chain is intact. */
  brokenAt: number;
  errorMsg?: string;
}

async function sha256hex(data: string): Promise<string> {
  const buf = await crypto.subtle.digest('SHA-256', new TextEncoder().encode(data));
  return Array.from(new Uint8Array(buf))
    .map((b) => b.toString(16).padStart(2, '0'))
    .join('');
}

/** Validate the hash chain client-side without trusting the server's chainAnchor values. */
async function validateChain(entries: CustodyEntry[]): Promise<number> {
  const GENESIS = '0'.repeat(64);
  let prevHash = entries[0]?.prevHash === GENESIS ? GENESIS : (entries[0]?.prevHash ?? GENESIS);

  for (let i = 0; i < entries.length; i++) {
    const e = entries[i];
    const expectedAnchor = await sha256hex(e.eventHash + prevHash);
    if (expectedAnchor !== e.chainAnchor) return i;
    prevHash = e.chainAnchor;
  }
  return -1;
}

export function CustodyAuditViewer({ productId }: CustodyAuditViewerProps) {
  const [result, setResult] = useState<AuditResult>({
    state: 'idle',
    entries: [],
    brokenAt: -1,
  });

  useEffect(() => {
    setResult({ state: 'loading', entries: [], brokenAt: -1 });

    (async () => {
      try {
        const res = await fetch(
          `/api/v1/events/signature-ledger?productId=${encodeURIComponent(productId)}`,
        );
        if (!res.ok) throw new Error(`HTTP ${res.status}`);
        const data = await res.json();
        const entries: CustodyEntry[] = data.entries ?? [];
        const brokenAt = entries.length > 0 ? await validateChain(entries) : -1;
        setResult({
          state: brokenAt === -1 ? 'valid' : 'tampered',
          entries,
          brokenAt,
        });
      } catch (err) {
        setResult({
          state: 'error',
          entries: [],
          brokenAt: -1,
          errorMsg: err instanceof Error ? err.message : 'Unknown error',
        });
      }
    })();
  }, [productId]);

  const { state, entries, brokenAt, errorMsg } = result;

  return (
    <div data-testid="custody-audit-viewer">
      {state === 'loading' && <p className="text-sm text-[var(--muted)]">Auditing chain…</p>}

      {state === 'error' && <p className="text-sm text-red-600">⚠️ Audit failed: {errorMsg}</p>}

      {(state === 'valid' || state === 'tampered') && (
        <>
          <div
            data-testid="chain-status"
            className={`mb-4 px-4 py-2 rounded-lg text-sm font-medium ${
              state === 'valid'
                ? 'bg-green-50 border border-green-300 text-green-800'
                : 'bg-red-50 border border-red-300 text-red-800'
            }`}
          >
            {state === 'valid'
              ? `✅ Chain of custody intact — ${entries.length} event(s) verified`
              : `❌ Chain broken at event #${brokenAt} — custody may have been tampered with`}
          </div>

          <ol className="space-y-2">
            {entries.map((e, i) => (
              <li
                key={e.eventIndex}
                data-testid={`custody-entry-${i}`}
                className={`p-3 rounded-lg border text-xs font-mono ${
                  brokenAt !== -1 && i >= brokenAt
                    ? 'border-red-300 bg-red-50'
                    : 'border-[var(--card-border)] bg-[var(--card)]'
                }`}
              >
                <div className="flex items-center gap-2 mb-1">
                  <span className="font-semibold text-[var(--foreground)]">
                    #{e.eventIndex} {e.eventType}
                  </span>
                  <span className="text-[var(--muted)]">@ {e.location}</span>
                  {brokenAt !== -1 && i === brokenAt && (
                    <span className="text-red-600 font-bold">← broken link</span>
                  )}
                </div>
                <div className="text-[var(--muted)] truncate">signer: {e.signerAddress}</div>
                <div className="text-[var(--muted)] truncate">
                  hash: {e.eventHash.slice(0, 16)}…
                </div>
                <div className="text-[var(--muted)] truncate">
                  anchor: {e.chainAnchor.slice(0, 16)}…
                </div>
              </li>
            ))}
          </ol>
        </>
      )}
    </div>
  );
}
