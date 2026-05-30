export type EventType = 'HARVEST' | 'PROCESSING' | 'SHIPPING' | 'RETAIL';
export type ProductStatus = 'active' | 'inactive';

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
  active?: boolean;
  authorizedActors: string[];
  ownershipHistory?: OwnershipRecord[];
  /** Unix seconds expiration timestamp. 0 = not set. (#406) */
  expirationTimestamp?: number;
  /** Whether the product has been marked as spoiled. (#406) */
  spoiled?: boolean;
  /** true while an on-chain transaction is in-flight */
  pending?: boolean;
  /** Whether this product has been recalled (#393) */
  recalled?: boolean;
  /** Reason provided when the product was recalled (#393) */
  recallReason?: string;
  /** Ledger timestamp when the product was recalled; 0 if never recalled (#393) */
  recallTimestamp?: number;
  /** Schema version of this record (#392) */
  schemaVersion?: number;
  /** Off-chain image URL stored in product metadata (#112) */
  imageUrl?: string;
  /** Taxonomy category ID (#425) */
  category?: string;
  /** Taxonomy subcategory ID (#425) */
  subcategory?: string;
  /** On-chain certifications attached to this product (#428) */
  certifications?: Certification[];
  /** Number of signatures required for events (0 or 1 = immediate, >1 = multi-sig) */
  requiredSignatures?: number;
}

export interface Batch {
  id: string;
  name: string;
  owner: string;
  productIds: string[];
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
  /** Whether this product has been recalled (#393) */
  recalled?: boolean;
  /** Reason provided when the product was recalled (#393) */
  recallReason?: string;
  /** Ledger timestamp when the product was recalled; 0 if never recalled (#393) */
  recallTimestamp?: number;
  /** Schema version of this record (#392) */
  schemaVersion?: number;
  /** Off-chain image URL stored in product metadata (#112) */
  imageUrl?: string;
  /** Taxonomy category ID (#425) */
  category?: string;
  /** Taxonomy subcategory ID (#425) */
  subcategory?: string;
  /** On-chain certifications attached to this product (#428) */
  certifications?: Certification[];
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
  /** Schema version of this record (#392) */
  schemaVersion?: number;
}

export interface EventPage {
  events: TrackingEvent[];
export interface PendingEvent {
  pendingEventId: number;
  productId: string;
  event: TrackingEvent;
  approvals: string[];
  requiredSignatures: number;
  createdAt: number;
  expiration?: number;
}

export type NotificationType =
  | 'TRACKING_EVENT'
  | 'APPROVAL_PENDING'
  | 'APPROVAL_FINALIZED'
  | 'APPROVAL_REJECTED'
  | 'OWNERSHIP_CHANGED'
  | 'PRODUCT_RECALLED'
  | 'CONTRACT_ERROR';

export interface Notification {
  id: string;
  productId: string;
  productName: string;
  eventType: EventType;
  location: string;
  actor: string;
  timestamp: number;
  read: boolean;
  notificationType: NotificationType;
  message?: string;
}

export interface TransactionResult {
  hash: string;
  status: 'success' | 'failed' | 'pending';
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

// ── Auditor registry types ────────────────────────────────────────────────────

/** A registered auditor who can sign attestations for events and products. */
export interface Auditor {
  /** Stellar address of the auditor. */
  address: string;
  /** Human-readable name of the auditing organisation. */
  name: string;
  /** Whether this auditor registration is currently active. */
  active: boolean;
  /** Unix timestamp (seconds) when the auditor was registered. */
  registeredAt: number;
}

/**
 * Attestation type keys from the controlled vocabulary.
 * Extensible — the contract stores these as free-form strings.
 */
export type AttestationType =
  | 'quality_check'
  | 'compliance_verified'
  | 'safety_approved'
  | 'origin_verified'
  | 'fair_trade_verified'
  | 'organic_certified'
  | string;

/**
 * A signed attestation from a registered auditor for a product or event.
 *
 * The `signature` field carries a hex-encoded Ed25519 signature over the
 * canonical payload: `product_id|target_id|attestation_type|timestamp`
 * (UTF-8, pipe-separated).
 */
export interface Attestation {
  /** Stable unique identifier for this attestation. */
  id: string;
  /** ID of the product this attestation is for. */
  productId: string;
  /**
   * Stable event ID (`TrackingEvent.stableId`) if attesting a specific event,
   * or empty string for a product-level attestation.
   */
  targetId: string;
  /** Stellar address of the auditor who signed this attestation. */
  auditor: string;
  /** Attestation type key. */
  attestationType: AttestationType;
  /** Hex-encoded Ed25519 signature over the canonical payload. */
  signature: string;
  /** Unix timestamp (seconds) when the attestation was submitted. */
  timestamp: number;
  /** Optional human-readable notes from the auditor. */
  notes: string;
}

// ── Batch recall types ────────────────────────────────────────────────────────

/** Batch with recall status fields. */
export interface BatchWithRecall {
  id: string;
  name: string;
  owner: string;
  productIds: string[];
  timestamp: number;
  /** Whether this batch has been recalled. */
  recalled: boolean;
  /** Reason provided when the batch was recalled. */
  recallReason: string;
  /** Unix timestamp (seconds) when the batch was recalled; 0 if never recalled. */
  recallTimestamp: number;
}
