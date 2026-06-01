/**
 * Event archival service.
 *
 * Provides helpers for archiving tracking events and querying the archive.
 * Archived events are excluded from the active timeline but remain fully
 * auditable — their stable_id and all original fields are preserved.
 */

import type { ArchivedEvent, TrackingEvent } from '@/lib/types';

export interface ArchiveEventRequest {
  productId: string;
  stableId: string;
  reason: string;
}

export interface ArchiveListOptions {
  offset?: number;
  limit?: number;
}

// ── API calls ─────────────────────────────────────────────────────────────────

/**
 * Archive a tracking event by its stable ID.
 * The event is removed from the active timeline but retained for audit.
 */
export async function archiveTrackingEvent(
  request: ArchiveEventRequest,
): Promise<ArchivedEvent> {
  const response = await fetch('/api/v1/events/archive', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(request),
  });

  if (!response.ok) {
    const err = await response.json().catch(() => ({}));
    throw new Error((err as { error?: string }).error ?? `Archive failed: ${response.statusText}`);
  }

  return response.json();
}

/**
 * List archived events for a product.
 */
export async function listArchivedEvents(
  productId: string,
  options: ArchiveListOptions = {},
): Promise<ArchivedEvent[]> {
  const params = new URLSearchParams({ productId });
  if (options.offset !== undefined) params.set('offset', String(options.offset));
  if (options.limit !== undefined) params.set('limit', String(options.limit));

  const response = await fetch(`/api/v1/events/archive?${params}`);

  if (!response.ok) {
    throw new Error(`Failed to fetch archived events: ${response.statusText}`);
  }

  return response.json();
}

// ── Client-side helpers ───────────────────────────────────────────────────────

/**
 * Filter active events — excludes any event flagged as archived.
 * Used when the contract layer returns a mixed list.
 */
export function filterActiveEvents(events: TrackingEvent[]): TrackingEvent[] {
  return events.filter((e) => !e.archived);
}

/**
 * Group archived events by event type for summary display.
 */
export function groupArchivedByType(
  archived: ArchivedEvent[],
): Record<string, ArchivedEvent[]> {
  return archived.reduce<Record<string, ArchivedEvent[]>>((acc, item) => {
    const key = item.event.eventType;
    if (!acc[key]) acc[key] = [];
    acc[key].push(item);
    return acc;
  }, {});
}

/**
 * Format an archival reason for display — truncates long strings.
 */
export function formatArchivalReason(reason: string, maxLen = 80): string {
  if (!reason) return 'No reason provided';
  return reason.length > maxLen ? `${reason.slice(0, maxLen)}…` : reason;
}
