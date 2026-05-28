export type EventType =
  | "HARVEST"
  | "PROCESSING"
  | "SHIPPING"
  | "DELIVERY"
  | "RETAIL"
  | "SPOILED"
  | "EXPIRED";

export type LifecycleStage =
  | "Registered"
  | "Harvested"
  | "Processed"
  | "Shipped"
  | "Delivered"
  | "Retail";

export interface OwnershipRecord {
  owner: string;
  transferredAt: number;
}

export interface Product {
  id: string;
  name: string;
  origin: string;
  owner: string;
  timestamp: number;
  active: boolean;
  authorizedActors: string[];
  ownershipHistory?: OwnershipRecord[];
  /** Current lifecycle stage (#404) */
  lifecycleStage?: LifecycleStage;
  pending?: boolean;
}

export interface TrackingEvent {
  productId: string;
  location: string;
  actor: string;
  timestamp: number;
  eventType: EventType;
  metadata: string;
  stableId?: string;
  pending?: boolean;
}

/** Pending ownership transfer escrow (#396) */
export interface TransferEscrow {
  productId: string;
  currentOwner: string;
  proposedOwner: string;
  requestedAt: number;
  disputed: boolean;
}

/** Pending event awaiting multi-party approval (#394) */
export interface PendingEvent {
  productId: string;
  submitter: string;
  location: string;
  eventType: EventType;
  metadata: string;
  submittedAt: number;
  requiredApprovers: string[];
  approvals: string[];
  rejected: boolean;
  expiresAt: number;
}

export interface EventPage {
  events: TrackingEvent[];
  total: number;
  offset: number;
  limit: number;
}

export interface EventFilter {
  eventType?: EventType | null;
  actor?: string | null;
  fromTimestamp?: number | null;
  toTimestamp?: number | null;
}
