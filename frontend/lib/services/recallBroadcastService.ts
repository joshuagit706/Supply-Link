import type { Product } from '@/lib/types';

export interface RecallBroadcast {
  id: string;
  productId: string;
  productName: string;
  reason: string;
  severity: 'low' | 'medium' | 'high' | 'critical';
  initiatedBy: string;
  initiatedAt: number;
  affectedBatches: string[];
  stakeholders: string[];
  status: 'active' | 'resolved' | 'cancelled';
  broadcastLog: BroadcastEvent[];
}

export interface BroadcastEvent {
  id: string;
  timestamp: number;
  stakeholder: string;
  status: 'pending' | 'delivered' | 'failed' | 'acknowledged';
  channel: 'email' | 'webhook' | 'sms' | 'in-app';
  message?: string;
  error?: string;
}

export interface RecallNotification {
  broadcastId: string;
  productId: string;
  productName: string;
  reason: string;
  severity: 'low' | 'medium' | 'high' | 'critical';
  receivedAt: number;
  acknowledged: boolean;
  acknowledgedAt?: number;
}

// In-memory storage (would be database in production)
const broadcasts = new Map<string, RecallBroadcast>();
const notifications = new Map<string, RecallNotification[]>();

export function initiateBroadcast(
  product: Product,
  reason: string,
  severity: 'low' | 'medium' | 'high' | 'critical',
  initiatedBy: string,
  stakeholders: string[],
  affectedBatches: string[] = [],
): RecallBroadcast {
  const id = `broadcast-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`;

  const broadcast: RecallBroadcast = {
    id,
    productId: product.id,
    productName: product.name,
    reason,
    severity,
    initiatedBy,
    initiatedAt: Date.now(),
    affectedBatches,
    stakeholders,
    status: 'active',
    broadcastLog: [],
  };

  broadcasts.set(id, broadcast);

  // Create notifications for each stakeholder
  stakeholders.forEach((stakeholder) => {
    const notification: RecallNotification = {
      broadcastId: id,
      productId: product.id,
      productName: product.name,
      reason,
      severity,
      receivedAt: Date.now(),
      acknowledged: false,
    };

    if (!notifications.has(stakeholder)) {
      notifications.set(stakeholder, []);
    }
    notifications.get(stakeholder)!.push(notification);

    // Log broadcast event
    broadcast.broadcastLog.push({
      id: `event-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`,
      timestamp: Date.now(),
      stakeholder,
      status: 'delivered',
      channel: 'webhook',
    });
  });

  return broadcast;
}

export function getBroadcast(id: string): RecallBroadcast | undefined {
  return broadcasts.get(id);
}

export function getAllBroadcasts(): RecallBroadcast[] {
  return Array.from(broadcasts.values());
}

export function getActiveBroadcasts(): RecallBroadcast[] {
  return Array.from(broadcasts.values()).filter((b) => b.status === 'active');
}

export function getStakeholderNotifications(stakeholder: string): RecallNotification[] {
  return notifications.get(stakeholder) || [];
}

export function acknowledgeNotification(
  stakeholder: string,
  broadcastId: string,
): RecallNotification | undefined {
  const stakeholderNotifications = notifications.get(stakeholder);
  if (!stakeholderNotifications) return undefined;

  const notification = stakeholderNotifications.find((n) => n.broadcastId === broadcastId);
  if (notification) {
    notification.acknowledged = true;
    notification.acknowledgedAt = Date.now();

    // Update broadcast log
    const broadcast = broadcasts.get(broadcastId);
    if (broadcast) {
      const logEntry = broadcast.broadcastLog.find((e) => e.stakeholder === stakeholder);
      if (logEntry) {
        logEntry.status = 'acknowledged';
      }
    }
  }

  return notification;
}

export function resolveBroadcast(id: string): RecallBroadcast | undefined {
  const broadcast = broadcasts.get(id);
  if (broadcast) {
    broadcast.status = 'resolved';
  }
  return broadcast;
}

export function cancelBroadcast(id: string): RecallBroadcast | undefined {
  const broadcast = broadcasts.get(id);
  if (broadcast) {
    broadcast.status = 'cancelled';
  }
  return broadcast;
}

export function getBroadcastStats(): {
  totalBroadcasts: number;
  activeBroadcasts: number;
  resolvedBroadcasts: number;
  totalNotifications: number;
  acknowledgedNotifications: number;
} {
  const allBroadcasts = Array.from(broadcasts.values());
  const allNotifications = Array.from(notifications.values()).flat();

  return {
    totalBroadcasts: allBroadcasts.length,
    activeBroadcasts: allBroadcasts.filter((b) => b.status === 'active').length,
    resolvedBroadcasts: allBroadcasts.filter((b) => b.status === 'resolved').length,
    totalNotifications: allNotifications.length,
    acknowledgedNotifications: allNotifications.filter((n) => n.acknowledged).length,
  };
}
