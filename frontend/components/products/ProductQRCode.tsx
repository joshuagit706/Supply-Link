'use client';

import { useEffect, useRef, useState } from 'react';
import QRCode from 'qrcode';
import { encodeQrProof, type QrProofPayload } from '@/lib/services/offlineVerify';

interface ProductQRCodeProps {
  productId: string;
  size?: number;
  /** Optional proof payload — when provided the QR embeds a signed offline proof. */
  proof?: QrProofPayload;
}

export default function ProductQRCode({ productId, size = 200, proof }: ProductQRCodeProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const [url, setUrl] = useState('');

  useEffect(() => {
    let cancelled = false;

    (async () => {
      const base = `${window.location.origin}/verify/${productId}`;
      let verifyUrl = base;

      if (proof) {
        try {
          const token = await encodeQrProof(proof);
          verifyUrl = `${base}?proof=${token}`;
        } catch {
          // Fall back to plain URL if signing fails
        }
      }

      if (cancelled) return;
      setUrl(verifyUrl);
      if (canvasRef.current) {
        QRCode.toCanvas(canvasRef.current, verifyUrl, {
          width: size,
          margin: 2,
          color: { dark: '#000000', light: '#ffffff' },
        });
      }
    })();

    return () => {
      cancelled = true;
    };
  }, [productId, size, proof]);

  const handleDownload = () => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const link = document.createElement('a');
    link.download = `qr-product-${productId}.png`;
    link.href = canvas.toDataURL('image/png');
    link.click();
  };

  return (
    <div className="flex flex-col items-center gap-3">
      <canvas ref={canvasRef} aria-label={`QR code for product ${productId}`} />
      <p className="text-xs text-[var(--muted)] break-all text-center max-w-[200px]">{url}</p>
      <button
        onClick={handleDownload}
        className="px-4 py-2 text-sm bg-[var(--primary)] text-[var(--primary-fg)] rounded-md hover:opacity-90 transition-opacity"
      >
        Download QR
      </button>
    </div>
  );
}
