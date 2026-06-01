/**
 * Provenance storytelling engine for animated timeline narratives.
 * Closes #485
 */

import type { TrackingEvent, EventType } from '@/lib/types';

export interface StorySegment {
  eventType: EventType;
  title: string;
  narrative: string;
  location: string;
  actor: string;
  timestamp: number;
  metadata?: Record<string, unknown>;
}

const EVENT_NARRATIVES: Record<EventType, (location: string, actor: string) => string> = {
  HARVEST: (location, actor) =>
    `Harvested at ${location} by ${actor}. The product begins its journey from origin, carefully selected and prepared for the next stage of processing.`,
  PROCESSING: (location, actor) =>
    `Processed at ${location} by ${actor}. The product undergoes transformation and quality checks to meet standards and prepare for distribution.`,
  SHIPPING: (location, actor) =>
    `Shipped from ${location} by ${actor}. The product is carefully transported through the supply chain, maintaining quality and integrity throughout transit.`,
  RETAIL: (location, actor) =>
    `Delivered to ${location} by ${actor}. The product reaches its final destination, ready for consumer purchase with full provenance transparency.`,
};

const EVENT_TITLES: Record<EventType, string> = {
  HARVEST: 'Origin',
  PROCESSING: 'Processing',
  SHIPPING: 'Distribution',
  RETAIL: 'Retail',
};

/**
 * Generate story segments from tracking events
 */
export function generateProvenanceStory(events: TrackingEvent[]): StorySegment[] {
  const sortedEvents = [...events].sort((a, b) => a.timestamp - b.timestamp);

  return sortedEvents.map((event) => {
    const narrative = EVENT_NARRATIVES[event.eventType](event.location, event.actor);
    const title = EVENT_TITLES[event.eventType];

    let metadata: Record<string, unknown> = {};
    try {
      metadata = typeof event.metadata === 'string' ? JSON.parse(event.metadata) : event.metadata;
    } catch {
      metadata = {};
    }

    return {
      eventType: event.eventType,
      title,
      narrative,
      location: event.location,
      actor: event.actor,
      timestamp: event.timestamp,
      metadata,
    };
  });
}

/**
 * Format timestamp to readable date
 */
export function formatEventDate(timestamp: number): string {
  const date = new Date(timestamp * 1000);
  return date.toLocaleDateString('en-US', {
    year: 'numeric',
    month: 'short',
    day: 'numeric',
  });
}

/**
 * Format timestamp to readable time
 */
export function formatEventTime(timestamp: number): string {
  const date = new Date(timestamp * 1000);
  return date.toLocaleTimeString('en-US', {
    hour: '2-digit',
    minute: '2-digit',
  });
}

/**
 * Get icon name for event type
 */
export function getEventIcon(eventType: EventType): string {
  switch (eventType) {
    case 'HARVEST':
      return 'Sprout';
    case 'PROCESSING':
      return 'Factory';
    case 'SHIPPING':
      return 'Truck';
    case 'RETAIL':
      return 'Store';
    default:
      return 'Package';
  }
}

/**
 * Get color for event type
 */
export function getEventColor(eventType: EventType): string {
  switch (eventType) {
    case 'HARVEST':
      return 'bg-green-100 text-green-700';
    case 'PROCESSING':
      return 'bg-blue-100 text-blue-700';
    case 'SHIPPING':
      return 'bg-purple-100 text-purple-700';
    case 'RETAIL':
      return 'bg-orange-100 text-orange-700';
    default:
      return 'bg-gray-100 text-gray-700';
  }
}

/**
 * Calculate time elapsed between events
 */
export function getTimeElapsed(fromTimestamp: number, toTimestamp: number): string {
  const seconds = toTimestamp - fromTimestamp;
  const days = Math.floor(seconds / (24 * 60 * 60));
  const hours = Math.floor((seconds % (24 * 60 * 60)) / (60 * 60));

  if (days > 0) {
    return `${days} day${days > 1 ? 's' : ''} ${hours}h`;
  }
  return `${hours} hour${hours > 1 ? 's' : ''}`;
}
