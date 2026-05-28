export type EventType = "HARVEST" | "PROCESSING" | "SHIPPING" | "RETAIL";

export type ActorRole = "Producer" | "Processor" | "Shipper" | "Retailer" | "Any";

export interface OwnershipRecord {
  owner: string;
  transferredAt: number;
}

export interface ActorRoleAssignment {
  actor: string;
  role: ActorRole;
}

export interface AuthPolicy {
  threshold: number;
  roles: ActorRoleAssignment[];
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
  /** true while an on-chain transaction is in-flight (#49) */
  pending?: boolean;
}

export interface TrackingEvent {
  productId: string;
  location: string;
  actor: string;
  timestamp: number;
  eventType: EventType;
  metadata: string;
  /** Stable deterministic event ID — SHA-256 hex (#386) */
  stableId?: string;
  /** true while an on-chain transaction is in-flight (#49) */
  pending?: boolean;
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
