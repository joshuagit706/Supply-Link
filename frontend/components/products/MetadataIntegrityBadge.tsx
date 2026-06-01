'use client';

import { useEffect, useState } from 'react';
import { verifyContentHash } from '@/lib/utils/metadata';

interface MetadataIntegrityBadgeProps {
  /** URL of the off-chain asset to verify. */
  assetUrl: string;
  /** Hex SHA-256 hash stored on-chain (the commitment). */
  expectedHash: string;
}

type VerifyState = 'pending' | 'valid' | 'invalid' | 'error';

/**
 * Downloads `assetUrl`, computes its SHA-256 hash, and compares it to
 * `expectedHash`. Displays a clear pass/fail integrity indicator.
 */
export function MetadataIntegrityBadge({ assetUrl, expectedHash }: MetadataIntegrityBadgeProps) {
  const [state, setState] = useState<VerifyState>('pending');

  useEffect(() => {
    let cancelled = false;
    setState('pending');

    (async () => {
      try {
        const res = await fetch(assetUrl);
        if (!res.ok) throw new Error(`HTTP ${res.status}`);
        const buffer = await res.arrayBuffer();
        if (cancelled) return;
        const ok = await verifyContentHash(buffer, expectedHash);
        setState(ok ? 'valid' : 'invalid');
      } catch {
        if (!cancelled) setState('error');
      }
    })();

    return () => {
      cancelled = true;
    };
  }, [assetUrl, expectedHash]);

  const config: Record<VerifyState, { icon: string; label: string; className: string }> = {
    pending: { icon: '⏳', label: 'Verifying integrity…', className: 'text-[var(--muted)]' },
    valid: { icon: '✅', label: 'Integrity verified', className: 'text-green-600' },
    invalid: {
      icon: '❌',
      label: 'Integrity failure — asset may have been tampered with',
      className: 'text-red-600 font-semibold',
    },
    error: { icon: '⚠️', label: 'Could not verify integrity', className: 'text-yellow-600' },
  };

  const { icon, label, className } = config[state];

  return (
    <span className={`inline-flex items-center gap-1 text-xs ${className}`} role="status">
      <span aria-hidden="true">{icon}</span>
      {label}
    </span>
  );
}
