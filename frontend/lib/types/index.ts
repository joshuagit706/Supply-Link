export type EventType = 'HARVEST' | 'PROCESSING' | 'SHIPPING' | 'RETAIL';
export type ProductStatus = 'active' | 'inactive';

export interface TemplateStage {
  label: string;
  eventType: EventType;
}

export type ActorRole = 'Producer' | 'Processor' | 'Shipper' | 'Retailer' | 'Any';

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

// ── Lifecycle (#404) ──────────────────────────────────────────────────────────

export type LifecycleStage =
  | 'Registered'
  | 'Harvested'
  | 'Processing'
  | 'Shipping'
  | 'Retail'
  | 'Sold'
  | 'Recalled'
  | 'Deactivated';

// ── Certifications (#428) ─────────────────────────────────────────────────────

export interface Certification {
  id: string;
  productId: string;
  certType: string;
  issuer: string;
  issuedAt: number;
  revoked: boolean;
  revokedAt?: number;
}

// ── Assembly relationships ────────────────────────────────────────────────────

/** Parent-child product assembly relationship. */
export interface ProductAssembly {
  /** ID of the parent (assembled) product. */
  parentId: string;
  /** Ordered list of component product IDs. */
  componentIds: string[];
  /** Address of the actor who registered this assembly. */
  registeredBy: string;
  /** Unix ms timestamp when the assembly was registered. */
  registeredAt: number;
  /** Optional description of the assembly process. */
  description: string;
}

// ── Warranty ──────────────────────────────────────────────────────────────────

export type ClaimStatus = 'Pending' | 'Approved' | 'Rejected' | 'Resolved';

/** Warranty metadata stored on-chain for a product. */
export interface WarrantyInfo {
  /** ID of the product this warranty covers. */
  productId: string;
  /** Warranty duration in seconds from product registration. 0 = lifetime. */
  durationSeconds: number;
  /** Address of the actor who registered the warranty. */
  issuer: string;
  /** Unix ms timestamp when the warranty was registered. */
  issuedAt: number;
  /** Short human-readable warranty terms. */
  terms: string;
  /** Off-chain reference to the full warranty document (IPFS CID, URL, etc.). */
  termsRef: string;
  /** Whether this warranty has been voided. */
  voided: boolean;
  /** Unix ms timestamp when the warranty was voided (0 if not voided). */
  voidedAt: number;
}

/** A warranty claim filed against a product. */
export interface WarrantyClaim {
  /** Stable unique identifier for this claim. */
  claimId: string;
  /** ID of the product the claim is filed against. */
  productId: string;
  /** Address of the claimant. */
  claimant: string;
  /** Unix ms timestamp when the claim was filed. */
  filedAt: number;
  /** Description of the issue. */
  description: string;
  /** Off-chain proof reference (IPFS CID, URL, etc.). */
  proofRef: string;
  /** Current status of the claim. */
  status: ClaimStatus;
  /** Unix ms timestamp when the claim status was last updated. */
  updatedAt: number;
}

// ── Product ───────────────────────────────────────────────────────────────────

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
  /** Number of signatures required for events (0 or 1 = immediate, >1 = multi-sig) */
  requiredSignatures?: number;
  /** Current lifecycle stage (#404) */
  lifecycleStage?: LifecycleStage;
  /** Assembly relationship — present if this product is assembled from components. */
  assembly?: ProductAssembly;
  /** Warranty metadata — present if a warranty has been registered. */
  warranty?: WarrantyInfo;
  /** Warranty claims filed against this product. */
  warrantyClaims?: WarrantyClaim[];
}

// ── Batch (#405) ──────────────────────────────────────────────────────────────

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
  lifecycleStage?: LifecycleStage;
  /** true while an on-chain transaction is in-flight (#49) */
  pending?: boolean;
  requiredSignatures?: number;
}

// ── Tracking events ───────────────────────────────────────────────────────────

export interface TrackingEvent {
  productId: string;
  location: string;
  actor: string;
  timestamp: number;
  eventType: EventType;
  metadata: string;
  stableId?: string;
  pending?: boolean;
  schemaVersion?: number;
}

// ── Pending events (#394) ─────────────────────────────────────────────────────

/** Pending event awaiting multi-party approval (#394) */
export interface PendingEvent {
  pendingEventId: number;
  productId: string;
  event: TrackingEvent;
  approvals: string[];
  requiredSignatures: number;
  createdAt: number;
  expiration?: number;
}

// ── Transfer escrow (#396) ────────────────────────────────────────────────────

/** Pending ownership transfer escrow (#396) */
export interface TransferEscrow {
  productId: string;
  currentOwner: string;
  proposedOwner: string;
  requestedAt: number;
  disputed: boolean;
}

// ── Pagination ────────────────────────────────────────────────────────────────

export interface EventPage {
  events: TrackingEvent[];
  total: number;
  offset: number;
  limit: number;
}

export interface PaginatedResponse<T> {
  items: T[];
  total: number;
  offset: number;
  limit: number;
}

// ── Notifications ─────────────────────────────────────────────────────────────

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

// ── Misc ──────────────────────────────────────────────────────────────────────

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

export interface EventFilter {
  eventType?: EventType | null;
  actor?: string | null;
  fromTimestamp?: number | null;
  toTimestamp?: number | null;
}

export interface Rating {
  id: string;
  productId: string;
  walletAddress: string;
  stars: number;
  comment: string | null;
  timestamp: number;
}

export interface Delegation {
  id: number;
  productId: string;
  delegatee: string;
  expiresAt: number;
  active: boolean;
}
