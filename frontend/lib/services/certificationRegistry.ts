/**
 * Certification registry service.
 *
 * Manages third-party certification issuers and their registry records.
 * Issuers are registered on-chain; records link products to external
 * certificate IDs with a document hash for cross-checking.
 */

import type {
  CertificationIssuer,
  CertificationRegistryRecord,
  CertificationVerificationResult,
} from '@/lib/types';

// ── Issuer management ─────────────────────────────────────────────────────────

export interface RegisterIssuerRequest {
  issuerAddress: string;
  name: string;
  certTypes: string[];
}

export async function registerCertificationIssuer(
  request: RegisterIssuerRequest,
): Promise<CertificationIssuer> {
  const response = await fetch('/api/v1/certification-registry/issuers', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(request),
  });

  if (!response.ok) {
    const err = await response.json().catch(() => ({}));
    throw new Error(
      (err as { error?: string }).error ?? `Failed to register issuer: ${response.statusText}`,
    );
  }

  return response.json();
}

export async function getCertificationIssuer(
  issuerAddress: string,
): Promise<CertificationIssuer> {
  const response = await fetch(
    `/api/v1/certification-registry/issuers/${encodeURIComponent(issuerAddress)}`,
  );

  if (!response.ok) {
    throw new Error(`Failed to fetch issuer: ${response.statusText}`);
  }

  return response.json();
}

export async function deactivateCertificationIssuer(
  issuerAddress: string,
): Promise<boolean> {
  const response = await fetch(
    `/api/v1/certification-registry/issuers/${encodeURIComponent(issuerAddress)}`,
    { method: 'DELETE' },
  );

  if (!response.ok) {
    throw new Error(`Failed to deactivate issuer: ${response.statusText}`);
  }

  return response.json();
}

// ── Registry records ──────────────────────────────────────────────────────────

export interface IssueRegistryRecordRequest {
  productId: string;
  issuerAddress: string;
  recordId: string;
  externalCertId: string;
  certType: string;
  documentHash: string;
}

export async function issueRegistryRecord(
  request: IssueRegistryRecordRequest,
): Promise<CertificationRegistryRecord> {
  const response = await fetch('/api/v1/certification-registry/records', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(request),
  });

  if (!response.ok) {
    const err = await response.json().catch(() => ({}));
    throw new Error(
      (err as { error?: string }).error ?? `Failed to issue record: ${response.statusText}`,
    );
  }

  return response.json();
}

export async function listRegistryRecords(
  productId: string,
): Promise<CertificationRegistryRecord[]> {
  const response = await fetch(
    `/api/v1/certification-registry/records?productId=${encodeURIComponent(productId)}`,
  );

  if (!response.ok) {
    throw new Error(`Failed to list registry records: ${response.statusText}`);
  }

  return response.json();
}

export async function verifyCertificationRecord(
  productId: string,
  recordId: string,
): Promise<CertificationVerificationResult> {
  const response = await fetch(
    `/api/v1/certification-registry/records/${encodeURIComponent(recordId)}/verify?productId=${encodeURIComponent(productId)}`,
  );

  if (!response.ok) {
    throw new Error(`Failed to verify record: ${response.statusText}`);
  }

  return response.json();
}

export async function revokeRegistryRecord(
  productId: string,
  recordId: string,
  issuerAddress: string,
): Promise<boolean> {
  const response = await fetch(
    `/api/v1/certification-registry/records/${encodeURIComponent(recordId)}`,
    {
      method: 'DELETE',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ productId, issuerAddress }),
    },
  );

  if (!response.ok) {
    throw new Error(`Failed to revoke record: ${response.statusText}`);
  }

  return response.json();
}

// ── Client-side helpers ───────────────────────────────────────────────────────

/** Returns only active (non-revoked) registry records. */
export function filterActiveRecords(
  records: CertificationRegistryRecord[],
): CertificationRegistryRecord[] {
  return records.filter((r) => !r.revoked);
}

/** Returns only revoked registry records. */
export function filterRevokedRecords(
  records: CertificationRegistryRecord[],
): CertificationRegistryRecord[] {
  return records.filter((r) => r.revoked);
}

/** Group records by cert type for display. */
export function groupRecordsByCertType(
  records: CertificationRegistryRecord[],
): Record<string, CertificationRegistryRecord[]> {
  return records.reduce<Record<string, CertificationRegistryRecord[]>>((acc, r) => {
    if (!acc[r.certType]) acc[r.certType] = [];
    acc[r.certType].push(r);
    return acc;
  }, {});
}
