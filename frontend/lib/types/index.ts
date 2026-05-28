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
export type EventType = "HARVEST" | "PROCESSING" | "SHIPPING" | "RETAIL";
export type ProductStatus = "active" | "inactive";

export interface TemplateStage {
  label: string;
  eventType: EventType;
}

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
  status?: ProductStatus;
  authorizedActors: string[];
  ownershipHistory?: OwnershipRecord[];
  /** Current lifecycle stage (#404) */
  lifecycleStage?: LifecycleStage;
  pending?: boolean;
  /** Number of signatures required for events (0 or 1 = immediate, >1 = multi-sig) */
  requiredSignatures?: number;
  /** true while an on-chain transaction is in-flight (#49) */
  pending?: boolean;
  /** Off-chain image URL stored in product metadata (#112) */
  imageUrl?: string;
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
  /** Stable deterministic event ID — SHA-256 hex (#386) */
  stableId?: string;
  /** true while an on-chain transaction is in-flight (#49) */
  pending?: boolean;
}

export interface EventPage {
  events: TrackingEvent[];
export interface PendingEvent {
  productId: string;
  event: TrackingEvent;
  approvals: string[];
  requiredSignatures: number;
  createdAt: number;
}

export interface Notification {
  id: string; // `${productId}-${timestamp}`
  productId: string;
  productName: string;
  eventType: EventType;
  location: string;
  actor: string;
  timestamp: number;
  read: boolean;
}

export interface TransactionResult {
  hash: string;
  status: "success" | "failed" | "pending";
  fee: string;
  timestamp: number;
}

export interface ContractError {
  code: number;
  message: string;
}

export interface PaginatedResponse<T> {
  items: T[];
  total: number;
  offset: number;
  limit: number;
}

export interface EventFilter {
  eventType?: EventType | null;
  actor?: string | null;
  fromTimestamp?: number | null;
  toTimestamp?: number | null;
export interface Rating {
  id: string;
  productId: string;
  walletAddress: string;
  stars: number;
  comment: string | null;
  timestamp: number;
}
