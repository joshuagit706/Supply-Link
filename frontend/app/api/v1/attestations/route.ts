/**
 * GET  /api/v1/attestations?productId=...  – list attestations for a product
 * POST /api/v1/attestations                – submit a new attestation
 *
 * Authentication: x-api-key (partner)
 * Rate limiting: default tier
 * Idempotency: POST requests via Idempotency-Key header
 */

import { NextRequest, NextResponse } from 'next/server';
import { withCors, handleOptions } from '@/lib/api/cors';
import { apiError, withCorrelationId, ErrorCode } from '@/lib/api/errors';
import { applyRateLimit, RATE_LIMIT_PRESETS } from '@/lib/api/rateLimit';
import { authenticateApiRequest } from '@/lib/api/auth';
import { withIdempotency } from '@/lib/api/idempotency';
import { recordRequest } from '@/lib/api/metrics';
import {
  MOCK_ATTESTATIONS,
  MOCK_AUDITORS,
  getAttestationsByProductId,
} from '@/lib/mock/auditors';
import type { Attestation, PaginatedResponse } from '@/lib/types';

export function OPTIONS(request: NextRequest) {
  return handleOptions(request);
}

async function listAttestations(req: NextRequest): Promise<NextResponse> {
  const productId = req.nextUrl.searchParams.get('productId');
  const targetId = req.nextUrl.searchParams.get('targetId');
  const offset = parseInt(req.nextUrl.searchParams.get('offset') ?? '0', 10);
  const limit = Math.min(parseInt(req.nextUrl.searchParams.get('limit') ?? '50', 10), 100);

  if (!productId) {
    return apiError(req, 400, ErrorCode.MISSING_FIELDS, 'Missing required query param: productId');
  }
  if (offset < 0 || limit < 1) {
    return apiError(req, 400, ErrorCode.VALIDATION_ERROR, 'Invalid offset or limit');
  }

  let all = getAttestationsByProductId(productId);

  // Optionally filter by targetId (event-level vs product-level)
  if (targetId !== null) {
    all = all.filter((a) => a.targetId === targetId);
  }

  const items = all.slice(offset, offset + limit);

  const response: PaginatedResponse<Attestation> = {
    items,
    total: all.length,
    offset,
    limit,
  };

  return withCors(req, withCorrelationId(req, NextResponse.json(response, { status: 200 })));
}

async function submitAttestation(req: NextRequest, rawBody: string): Promise<NextResponse> {
  let payload: unknown;
  try {
    payload = JSON.parse(rawBody);
  } catch {
    return apiError(req, 400, ErrorCode.INVALID_PAYLOAD, 'Invalid JSON');
  }

  const body = payload as Record<string, unknown>;

  // Validate required fields
  const requiredFields = ['productId', 'auditor', 'attestationId', 'attestationType', 'signature'];
  for (const field of requiredFields) {
    if (typeof body[field] !== 'string' || !(body[field] as string).trim()) {
      return apiError(req, 400, ErrorCode.MISSING_FIELDS, `Missing or invalid: ${field}`);
    }
  }

  const auditorAddress = body.auditor as string;
  const auditor = MOCK_AUDITORS.find((a) => a.address === auditorAddress);

  // Only registered auditors may submit attestations
  if (!auditor) {
    return apiError(req, 403, ErrorCode.VALIDATION_ERROR, 'Auditor not registered');
  }
  if (!auditor.active) {
    return apiError(req, 403, ErrorCode.VALIDATION_ERROR, 'Auditor is not active');
  }

  // Check for duplicate attestation ID
  const duplicate = MOCK_ATTESTATIONS.find((a) => a.id === body.attestationId);
  if (duplicate) {
    return apiError(req, 409, ErrorCode.VALIDATION_ERROR, 'Attestation ID already exists');
  }

  const newAttestation: Attestation = {
    id: body.attestationId as string,
    productId: body.productId as string,
    targetId: typeof body.targetId === 'string' ? body.targetId : '',
    auditor: auditorAddress,
    attestationType: body.attestationType as string,
    signature: body.signature as string,
    timestamp: Math.floor(Date.now() / 1000),
    notes: typeof body.notes === 'string' ? body.notes : '',
  };

  // TODO: Persist via Soroban contract call: submit_attestation(...)
  MOCK_ATTESTATIONS.push(newAttestation);

  return withCors(req, withCorrelationId(req, NextResponse.json(newAttestation, { status: 201 })));
}

export async function GET(request: NextRequest): Promise<NextResponse> {
  const start = Date.now();

  const limited = applyRateLimit(
    request,
    'GET /api/v1/attestations',
    RATE_LIMIT_PRESETS.publicRead,
  );
  if (limited) {
    recordRequest('GET /api/v1/attestations', 429, Date.now() - start);
    return limited;
  }

  const auth = await authenticateApiRequest(request, 'partner');
  if (auth.error) {
    recordRequest('GET /api/v1/attestations', 401, Date.now() - start);
    return auth.error;
  }

  const response = await listAttestations(request);
  recordRequest('GET /api/v1/attestations', response.status, Date.now() - start);
  return response;
}

export async function POST(request: NextRequest): Promise<NextResponse> {
  const start = Date.now();

  const limited = applyRateLimit(
    request,
    'POST /api/v1/attestations',
    RATE_LIMIT_PRESETS.default,
  );
  if (limited) {
    recordRequest('POST /api/v1/attestations', 429, Date.now() - start);
    return limited;
  }

  const auth = await authenticateApiRequest(request, 'partner');
  if (auth.error) {
    recordRequest('POST /api/v1/attestations', 401, Date.now() - start);
    return auth.error;
  }

  const response = await withIdempotency(request, (req, rawBody) =>
    submitAttestation(req, rawBody),
  );

  recordRequest('POST /api/v1/attestations', response.status, Date.now() - start);
  return response;
}
