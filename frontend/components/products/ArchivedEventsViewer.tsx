'use client';

import { useState, useEffect, useCallback } from 'react';
import { Archive, ChevronDown, ChevronUp, RefreshCw, AlertCircle } from 'lucide-react';
import type { ArchivedEvent, EventType } from '@/lib/types';
import { listArchivedEvents } from '@/lib/services/eventArchival';

const EVENT_TYPE_LABELS: Record<EventType, string> = {
  HARVEST: 'Harvest',
  PROCESSING: 'Processing',
  SHIPPING: 'Shipping',
  RETAIL: 'Retail',
};

const EVENT_COLORS: Record<EventType, string> = {
  HARVEST: 'text-green-600 dark:text-green-400',
  PROCESSING: 'text-blue-600 dark:text-blue-400',
  SHIPPING: 'text-yellow-600 dark:text-yellow-400',
  RETAIL: 'text-purple-600 dark:text-purple-400',
};

interface ArchivedEventCardProps {
  item: ArchivedEvent;
  locale?: string;
}

function ArchivedEventCard({ item, locale }: ArchivedEventCardProps) {
  const [expanded, setExpanded] = useState(false);
  const dateFmt = new Intl.DateTimeFormat(locale, { dateStyle: 'medium', timeStyle: 'short' });
  const { event } = item;

  return (
    <div className="rounded-lg border border-[var(--card-border)] bg-[var(--muted-bg)] p-4 opacity-80">
      <div className="flex items-start justify-between gap-3">
        <div className="flex items-start gap-3 min-w-0">
          <Archive size={14} className="shrink-0 mt-0.5 text-[var(--muted)]" />
          <div className="min-w-0">
            <div className="flex items-center gap-2 flex-wrap">
              <span
                className={`text-xs font-semibold uppercase tracking-wide ${EVENT_COLORS[event.eventType]}`}
              >
                {EVENT_TYPE_LABELS[event.eventType]}
              </span>
              <span className="text-xs text-[var(--muted)]">
                {dateFmt.format(new Date(event.timestamp))}
              </span>
            </div>
            <p className="text-sm text-[var(--foreground)] mt-0.5">{event.location}</p>
            <p className="text-xs text-[var(--muted)] font-mono truncate">{event.actor}</p>
          </div>
        </div>

        <button
          type="button"
          onClick={() => setExpanded((v) => !v)}
          className="shrink-0 text-[var(--muted)] hover:text-[var(--foreground)] transition-colors"
          aria-label={expanded ? 'Collapse details' : 'Expand details'}
        >
          {expanded ? <ChevronUp size={14} /> : <ChevronDown size={14} />}
        </button>
      </div>

      {expanded && (
        <div className="mt-3 pt-3 border-t border-[var(--card-border)] space-y-2 text-xs text-[var(--muted)]">
          <div>
            <span className="font-medium text-[var(--foreground)]">Archived by:</span>{' '}
            <span className="font-mono">{item.archivedBy}</span>
          </div>
          <div>
            <span className="font-medium text-[var(--foreground)]">Archived at:</span>{' '}
            {dateFmt.format(new Date(item.archivedAt))}
          </div>
          {item.reason && (
            <div>
              <span className="font-medium text-[var(--foreground)]">Reason:</span>{' '}
              {item.reason}
            </div>
          )}
          {event.stableId && (
            <div>
              <span className="font-medium text-[var(--foreground)]">Stable ID:</span>{' '}
              <span className="font-mono break-all">{event.stableId}</span>
            </div>
          )}
          {event.metadata && event.metadata !== '{}' && (
            <pre className="mt-1 bg-[var(--card)] rounded-md px-3 py-2 overflow-x-auto text-xs">
              {JSON.stringify(JSON.parse(event.metadata), null, 2)}
            </pre>
          )}
        </div>
      )}
    </div>
  );
}

interface ArchivedEventsViewerProps {
  productId: string;
  locale?: string;
}

/**
 * Displays the archive of tracking events for a product.
 * Archived events are excluded from the active timeline but remain
 * fully auditable here with all original fields preserved.
 */
export function ArchivedEventsViewer({ productId, locale }: ArchivedEventsViewerProps) {
  const [items, setItems] = useState<ArchivedEvent[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const data = await listArchivedEvents(productId);
      setItems(data);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load archived events');
    } finally {
      setLoading(false);
    }
  }, [productId]);

  useEffect(() => {
    void load();
  }, [load]);

  return (
    <section aria-label="Archived events">
      <div className="flex items-center justify-between mb-4">
        <div className="flex items-center gap-2">
          <Archive size={16} className="text-[var(--muted)]" />
          <h3 className="text-sm font-semibold text-[var(--foreground)]">
            Event Archive
            {items.length > 0 && (
              <span className="ml-2 text-xs font-normal text-[var(--muted)]">
                ({items.length} event{items.length !== 1 ? 's' : ''})
              </span>
            )}
          </h3>
        </div>
        <button
          type="button"
          onClick={() => void load()}
          disabled={loading}
          className="text-[var(--muted)] hover:text-[var(--foreground)] transition-colors disabled:opacity-40"
          aria-label="Refresh archive"
        >
          <RefreshCw size={14} className={loading ? 'animate-spin' : ''} />
        </button>
      </div>

      {error && (
        <div className="flex items-center gap-2 text-sm text-red-600 dark:text-red-400 mb-4">
          <AlertCircle size={14} />
          {error}
        </div>
      )}

      {!loading && !error && items.length === 0 && (
        <p className="text-sm text-[var(--muted)]">No archived events for this product.</p>
      )}

      {items.length > 0 && (
        <div className="space-y-3">
          <p className="text-xs text-[var(--muted)]">
            Archived events are excluded from the active timeline but their integrity proofs are
            preserved. All original fields remain unchanged.
          </p>
          {items.map((item, i) => (
            <ArchivedEventCard
              key={item.event.stableId ?? i}
              item={item}
              locale={locale}
            />
          ))}
        </div>
      )}
    </section>
  );
}
