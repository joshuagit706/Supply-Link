/**
 * frontend/lib/utils/export.ts
 *
 * Audit trail export utilities (#395).
 * Generates CSV/JSON reports with a deterministic fingerprint for compliance.
 */

import type { Product, TrackingEvent } from "@/lib/types";

export interface AuditExport {
  exportedAt: string;
  product: Product;
  events: TrackingEvent[];
  /** SHA-256 hex fingerprint of the canonical export payload */
  fingerprint: string;
}

/** Build a canonical string representation of the export for hashing. */
function canonicalize(product: Product, events: TrackingEvent[]): string {
  return JSON.stringify({
    productId: product.id,
    owner: product.owner,
    timestamp: product.timestamp,
    events: events.map((e) => ({
      productId: e.productId,
      actor: e.actor,
      eventType: e.eventType,
      timestamp: e.timestamp,
      location: e.location,
      metadata: e.metadata,
      stableId: e.stableId ?? null,
    })),
  });
}

/** Compute a SHA-256 fingerprint of the canonical export string. */
async function computeFingerprint(data: string): Promise<string> {
  const buf = new TextEncoder().encode(data);
  const hashBuf = await crypto.subtle.digest("SHA-256", buf);
  return Array.from(new Uint8Array(hashBuf))
    .map((b) => b.toString(16).padStart(2, "0"))
    .join("");
}

/** Build a full audit export object for a product. */
export async function buildAuditExport(
  product: Product,
  events: TrackingEvent[]
): Promise<AuditExport> {
  const canonical = canonicalize(product, events);
  const fingerprint = await computeFingerprint(canonical);
  return {
    exportedAt: new Date().toISOString(),
    product,
    events,
    fingerprint,
  };
}

/** Trigger a JSON file download in the browser. */
export function downloadJson(data: unknown, filename: string): void {
  const blob = new Blob([JSON.stringify(data, null, 2)], {
    type: "application/json",
  });
import type { TrackingEvent } from "@/lib/types";

const CSV_HEADERS = ["product_id", "event_type", "location", "actor", "timestamp", "metadata"] as const;

function downloadBlob(content: string, filename: string, mime: string) {
  const blob = new Blob([content], { type: mime });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = filename;
  a.click();
  URL.revokeObjectURL(url);
}

/** Convert events to CSV and trigger a download. */
export function downloadCsv(events: TrackingEvent[], filename: string): void {
  const headers = [
    "productId",
    "eventType",
    "location",
    "actor",
    "timestamp",
    "stableId",
    "metadata",
  ];
  const rows = events.map((e) =>
    [
      e.productId,
      e.eventType,
      e.location,
      e.actor,
      new Date(e.timestamp * 1000).toISOString(),
      e.stableId ?? "",
      JSON.stringify(e.metadata).replace(/"/g, '""'),
    ]
      .map((v) => `"${v}"`)
      .join(",")
  );
  const csv = [headers.join(","), ...rows].join("\n");
  const blob = new Blob([csv], { type: "text/csv" });
export function exportToCSV(events: TrackingEvent[], filename = "events.csv") {
  if (events.length === 0) return;
  const rows = events.map((e) =>
    [e.productId, e.eventType, e.location, e.actor, e.timestamp, e.metadata]
      .map((v) => JSON.stringify(String(v ?? "")))
      .join(",")
  );
  downloadBlob([CSV_HEADERS.join(","), ...rows].join("\n"), filename, "text/csv");
}

export function exportToJSON(events: TrackingEvent[], filename = "events.json") {
  downloadBlob(JSON.stringify(events, null, 2), filename, "application/json");
}

export function downloadCSV(csv: string, filename: string): void {
  const blob = new Blob([csv], { type: "text/csv;charset=utf-8;" });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = filename;
  a.click();
  URL.revokeObjectURL(url);
}
