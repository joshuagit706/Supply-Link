'use client';

import { useEffect, useRef, useCallback } from 'react';
import { useStore } from '@/lib/state/store';
import { contractClient } from '@/lib/stellar/contract';
import { withRetry } from '@/lib/resilience';
import type { Notification } from '@/lib/types';

const POLL_INTERVAL_MS = 30_000;

export function useNotifications() {
  const {
    walletAddress,
    products,
    notifications,
    addNotifications,
    markNotificationRead,
    markAllNotificationsRead,
  } = useStore();

  const seenTimestamps = useRef<Record<string, number>>({});

  const poll = useCallback(async () => {
    if (!walletAddress || !products.length) return;

    const incoming: Notification[] = [];

    for (const product of products) {
      try {
        const events = await withRetry(
          () => contractClient.getTrackingEvents(product.id, walletAddress),
          { maxAttempts: 2, baseDelayMs: 1_000 },
        );
        for (const ev of events) {
          const known = seenTimestamps.current[product.id] ?? 0;
          if (ev.timestamp > known) {
            seenTimestamps.current[product.id] = Math.max(known, ev.timestamp);
            incoming.push({
              id: `${product.id}-${ev.timestamp}`,
              productId: product.id,
              productName: product.name,
              eventType: ev.eventType,
              location: ev.location,
              actor: ev.actor,
              timestamp: ev.timestamp,
              read: false,
            });
          }
        }
      } catch {
        // silently skip failed product polls
      }
    }

    if (incoming.length) addNotifications(incoming);
  }, [walletAddress, products, addNotifications]);

  useEffect(() => {
    poll();
    const id = setInterval(poll, POLL_INTERVAL_MS);
    return () => clearInterval(id);
  }, [poll]);

  const unreadCount = notifications.filter((n) => !n.read).length;

  return { notifications, unreadCount, markNotificationRead, markAllNotificationsRead };
}
