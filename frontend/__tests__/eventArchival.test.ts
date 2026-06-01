import { describe, it, expect } from 'vitest';
import {
  filterActiveEvents,
  groupArchivedByType,
  formatArchivalReason,
} from '@/lib/services/eventArchival';
import type { TrackingEvent, ArchivedEvent } from '@/lib/types';

// ── Helpers ───────────────────────────────────────────────────────────────────

function makeEvent(overrides: Partial<TrackingEvent> = {}): TrackingEvent {
  return {
    productId: 'prod-001',
    location: 'Warehouse A',
    actor: 'GACTOR1',
    timestamp: 1_700_000_000_000,
    eventType: 'HARVEST',
    metadata: '{}',
    stableId: 'stable-abc',
    ...overrides,
  };
}

function makeArchived(event: TrackingEvent, reason = 'retention'): ArchivedEvent {
  return {
    event,
    archivedBy: 'GOWNER1',
    archivedAt: Date.now(),
    reason,
  };
}

// ── filterActiveEvents ────────────────────────────────────────────────────────

describe('filterActiveEvents', () => {
  it('returns all events when none are archived', () => {
    const events = [makeEvent(), makeEvent({ stableId: 'stable-2', eventType: 'SHIPPING' })];
    expect(filterActiveEvents(events)).toHaveLength(2);
  });

  it('excludes events flagged as archived', () => {
    const events = [
      makeEvent({ stableId: 'active-1' }),
      makeEvent({ stableId: 'archived-1', archived: true }),
      makeEvent({ stableId: 'active-2', eventType: 'RETAIL' }),
    ];
    const result = filterActiveEvents(events);
    expect(result).toHaveLength(2);
    expect(result.every((e) => !e.archived)).toBe(true);
  });

  it('returns empty array when all events are archived', () => {
    const events = [
      makeEvent({ archived: true }),
      makeEvent({ archived: true, stableId: 'stable-2' }),
    ];
    expect(filterActiveEvents(events)).toHaveLength(0);
  });

  it('returns empty array for empty input', () => {
    expect(filterActiveEvents([])).toHaveLength(0);
  });
});

// ── groupArchivedByType ───────────────────────────────────────────────────────

describe('groupArchivedByType', () => {
  it('groups archived events by event type', () => {
    const archived = [
      makeArchived(makeEvent({ eventType: 'HARVEST', stableId: 's1' })),
      makeArchived(makeEvent({ eventType: 'HARVEST', stableId: 's2' })),
      makeArchived(makeEvent({ eventType: 'SHIPPING', stableId: 's3' })),
    ];
    const groups = groupArchivedByType(archived);
    expect(groups['HARVEST']).toHaveLength(2);
    expect(groups['SHIPPING']).toHaveLength(1);
    expect(groups['RETAIL']).toBeUndefined();
  });

  it('returns empty object for empty input', () => {
    expect(groupArchivedByType([])).toEqual({});
  });
});

// ── formatArchivalReason ──────────────────────────────────────────────────────

describe('formatArchivalReason', () => {
  it('returns the reason unchanged when within max length', () => {
    expect(formatArchivalReason('annual retention policy')).toBe('annual retention policy');
  });

  it('truncates long reasons with ellipsis', () => {
    const long = 'a'.repeat(100);
    const result = formatArchivalReason(long, 80);
    expect(result.endsWith('…')).toBe(true);
    expect(result.length).toBe(81); // 80 chars + ellipsis
  });

  it('returns fallback for empty reason', () => {
    expect(formatArchivalReason('')).toBe('No reason provided');
  });

  it('respects custom maxLen', () => {
    const result = formatArchivalReason('hello world', 5);
    expect(result).toBe('hello…');
  });
});
