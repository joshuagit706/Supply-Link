'use client';

import { useState, useMemo } from 'react';
import type { TrackingEvent } from '@/lib/types';
import {
  ChevronDown,
  ChevronUp,
  ChevronRight,
  Paperclip,
  Layers,
  MapPin,
  User,
} from 'lucide-react';
import { EVENT_TYPE_CONFIG } from '@/lib/eventTypeConfig';

type GroupBy = 'none' | 'eventType' | 'location' | 'actor';

function MetadataViewer({ raw }: { raw: string }) {
  const [open, setOpen] = useState(false);
  let parsed: Record<string, unknown> | null = null;
  try {
    parsed = JSON.parse(raw);
  } catch {
    return null;
  }
  if (!parsed || Object.keys(parsed).length === 0) return null;

  const { attachmentUrl, ...rest } = parsed as Record<string, unknown>;
  const attachmentHref = typeof attachmentUrl === 'string' ? attachmentUrl : null;
  const hasOtherKeys = Object.keys(rest).length > 0;

  return (
    <div className="mt-2 flex flex-col gap-1">
      {attachmentHref && (
        <a
          href={attachmentHref}
          target="_blank"
          rel="noopener noreferrer"
          className="inline-flex items-center gap-1.5 text-xs text-violet-500 hover:underline"
        >
          <Paperclip size={12} />
          View attachment
        </a>
      )}
      {hasOtherKeys && (
        <>
          <button
            onClick={() => setOpen((v: boolean) => !v)}
            className="flex items-center gap-1 text-xs text-[var(--muted)] hover:text-[var(--foreground)] transition-colors w-fit"
          >
            {open ? <ChevronUp size={12} /> : <ChevronDown size={12} />}
            {open ? 'Hide' : 'Show'} metadata
          </button>
          {open && (
            <pre className="text-xs bg-[var(--muted-bg)] text-[var(--muted)] rounded-md px-3 py-2 overflow-x-auto">
              {JSON.stringify(rest, null, 2)}
            </pre>
          )}
        </>
      )}
    </div>
  );
}

function EventCard({
  event,
  highlighted,
  onSelect,
}: {
  event: TrackingEvent;
  highlighted?: boolean;
  onSelect?: (e: TrackingEvent) => void;
}) {
  const cfg = EVENT_TYPE_CONFIG[event.eventType];
  const Icon = cfg.icon;
  return (
    <li
      className={`ml-6 cursor-pointer rounded-lg transition-colors ${highlighted ? 'bg-violet-500/5 -mx-2 px-2' : ''}`}
      onClick={() => onSelect?.(event)}
    >
      <span
        className={`absolute -left-2 mt-1.5 h-4 w-4 rounded-full border-2 border-[var(--background)] ${cfg.dotClass}`}
      />
      <div className="flex flex-wrap items-center gap-2 mb-1">
        <span
          className={`inline-flex items-center gap-1 text-xs font-semibold px-2 py-0.5 rounded-full ${cfg.badgeClass}`}
        >
          <Icon size={11} />
          {cfg.label}
        </span>
        <span className="text-xs text-[var(--muted)]">
          {new Date(event.timestamp).toLocaleString()}
        </span>
      </div>
      <p className="text-sm text-[var(--foreground)]">{event.location}</p>
      <p className="text-xs font-mono text-[var(--muted)] mt-0.5">
        {event.actor.slice(0, 8)}…{event.actor.slice(-6)}
      </p>
      <MetadataViewer raw={event.metadata} />
    </li>
  );
}

function GroupSection({
  label,
  events,
  collapsed,
  onToggle,
  highlightedEvent,
  onSelectEvent,
}: {
  label: string;
  events: TrackingEvent[];
  collapsed: boolean;
  onToggle: () => void;
  highlightedEvent?: TrackingEvent | null;
  onSelectEvent?: (e: TrackingEvent) => void;
}) {
  return (
    <div className="mb-4">
      <button
        onClick={onToggle}
        className="flex items-center gap-2 text-xs font-semibold text-[var(--muted)] uppercase tracking-wider mb-2 hover:text-[var(--foreground)] transition-colors"
      >
        <ChevronRight
          size={13}
          className={`transition-transform ${collapsed ? '' : 'rotate-90'}`}
        />
        {label}
        <span className="font-normal normal-case tracking-normal">({events.length})</span>
      </button>
      {!collapsed && (
        <ol className="relative border-l border-[var(--card-border)] ml-3 space-y-6">
          {events.map((event, i) => (
            <EventCard
              key={i}
              event={event}
              highlighted={highlightedEvent === event}
              onSelect={onSelectEvent}
            />
          ))}
        </ol>
      )}
    </div>
  );
}

interface EventTimelineProps {
  events: TrackingEvent[];
  highlightedEvent?: TrackingEvent | null;
  onSelectEvent?: (event: TrackingEvent) => void;
}

export function EventTimeline({ events, highlightedEvent, onSelectEvent }: EventTimelineProps) {
  const [groupBy, setGroupBy] = useState<GroupBy>('none');
  const [collapsedGroups, setCollapsedGroups] = useState<Set<string>>(new Set());

  const grouped = useMemo(() => {
    if (groupBy === 'none') return null;

    const map = new Map<string, TrackingEvent[]>();
    for (const event of events) {
      const key =
        groupBy === 'eventType'
          ? event.eventType
          : groupBy === 'location'
            ? event.location
            : `${event.actor.slice(0, 8)}…${event.actor.slice(-6)}`;
      if (!map.has(key)) map.set(key, []);
      map.get(key)!.push(event);
    }
    return map;
  }, [events, groupBy]);

  function toggleGroup(label: string) {
    setCollapsedGroups((prev) => {
      const next = new Set(prev);
      if (next.has(label)) next.delete(label);
      else next.add(label);
      return next;
    });
  }

  if (events.length === 0) {
    return (
      <p className="text-sm text-[var(--muted)] py-6 text-center">
        No events recorded for this product yet.
      </p>
    );
  }

  return (
    <div>
      {/* Grouping controls */}
      <div className="flex items-center gap-2 mb-4 flex-wrap">
        <span className="text-xs text-[var(--muted)]">Group by:</span>
        {(
          [
            { value: 'none', label: 'None', icon: null },
            { value: 'eventType', label: 'Stage', icon: <Layers size={11} /> },
            { value: 'location', label: 'Location', icon: <MapPin size={11} /> },
            { value: 'actor', label: 'Actor', icon: <User size={11} /> },
          ] as const
        ).map(({ value, label, icon }) => (
          <button
            key={value}
            onClick={() => setGroupBy(value)}
            className={`inline-flex items-center gap-1 text-xs px-2.5 py-1 rounded-full border transition-colors ${
              groupBy === value
                ? 'bg-violet-600 text-white border-violet-600'
                : 'border-[var(--card-border)] text-[var(--muted)] hover:text-[var(--foreground)]'
            }`}
          >
            {icon}
            {label}
          </button>
        ))}
      </div>

      {grouped ? (
        <div>
          {Array.from(grouped.entries()).map(([label, groupEvents]) => (
            <GroupSection
              key={label}
              label={label}
              events={groupEvents}
              collapsed={collapsedGroups.has(label)}
              onToggle={() => toggleGroup(label)}
              highlightedEvent={highlightedEvent}
              onSelectEvent={onSelectEvent}
            />
          ))}
        </div>
      ) : (
        <ol className="relative border-l border-[var(--card-border)] ml-3 space-y-6">
          {events.map((event, i) => (
            <EventCard
              key={i}
              event={event}
              highlighted={highlightedEvent === event}
              onSelect={onSelectEvent}
            />
          ))}
        </ol>
      )}
    </div>
  );
}
