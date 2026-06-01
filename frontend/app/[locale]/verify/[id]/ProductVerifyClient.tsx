'use client';

import { useEffect, useState } from 'react';
import { useSearchParams } from 'next/navigation';
import { recordScan } from '@/lib/services/scanTracking';
import { verifyQrProof, type QrProofResult } from '@/lib/services/offlineVerify';
import type { Product } from '@/lib/types';

interface ProductVerifyClientProps {
  product: Product;
  children: React.ReactNode;
}

type Mode = 'online' | 'offline';

export default function ProductVerifyClient({ product, children }: ProductVerifyClientProps) {
  const searchParams = useSearchParams();
  const [mode, setMode] = useState<Mode>('online');
  const [offlineResult, setOfflineResult] = useState<QrProofResult | null>(null);

  useEffect(() => {
    // Detect connectivity and switch to offline mode when unavailable
    const handleOffline = () => setMode('offline');
    const handleOnline = () => setMode('online');
    window.addEventListener('offline', handleOffline);
    window.addEventListener('online', handleOnline);
    if (!navigator.onLine) setMode('offline');

    return () => {
      window.removeEventListener('offline', handleOffline);
      window.removeEventListener('online', handleOnline);
    };
  }, []);

  useEffect(() => {
    // Validate embedded QR proof if present
    const token = searchParams.get('proof');
    if (!token) return;

    verifyQrProof(token).then(setOfflineResult);
  }, [searchParams]);

  useEffect(() => {
    // Record scan for recall notifications (online only)
    if (mode !== 'online') return;
    (async () => {
      try {
        const res = await fetch('/api/client-ip');
        const { ip } = await res.json();
        if (ip) await recordScan(product.id, ip);
      } catch {
        console.debug('Could not record scan');
      }
    })();
  }, [product.id, mode]);

  return (
    <>
      {/* Mode indicator */}
      {mode === 'offline' && (
        <div
          data-testid="offline-mode-banner"
          className="w-full bg-amber-50 border border-amber-300 text-amber-800 px-6 py-3 mb-4 rounded-lg text-sm flex items-center gap-2"
        >
          <span aria-hidden="true">📴</span>
          <span>
            <strong>Offline mode</strong> — showing locally verified QR proof. Connect to the
            internet to load the full product history.
          </span>
        </div>
      )}

      {/* QR proof validation result */}
      {offlineResult && (
        <div
          data-testid="qr-proof-result"
          className={`w-full px-6 py-3 mb-4 rounded-lg text-sm flex items-center gap-2 ${
            offlineResult.valid
              ? 'bg-green-50 border border-green-300 text-green-800'
              : 'bg-red-50 border border-red-300 text-red-800'
          }`}
        >
          <span aria-hidden="true">{offlineResult.valid ? '✅' : '❌'}</span>
          <span>
            {offlineResult.valid
              ? 'QR proof verified — product provenance is authentic'
              : `QR proof invalid: ${offlineResult.error}`}
          </span>
        </div>
      )}

      {/* RECALLED Banner */}
      {!product.active && (
        <div className="w-full bg-gradient-to-r from-red-600 to-red-700 text-white px-6 py-4 mb-6 rounded-lg border-2 border-red-800">
          <div className="max-w-2xl mx-auto flex items-start gap-3">
            <div className="text-2xl">⚠️</div>
            <div>
              <h2 className="text-lg font-bold mb-1">PRODUCT RECALLED</h2>
              <p className="text-sm text-red-50">
                This product has been recalled and removed from sale. Do not use this product.
                Please return it or dispose of it safely.
              </p>
            </div>
          </div>
        </div>
      )}

      {children}
    </>
  );
}
