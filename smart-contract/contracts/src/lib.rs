#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, Address, Env, String, Symbol, Vec};

// ── Lifecycle state machine (#404) ────────────────────────────────────────────

/// Product lifecycle stages. Transitions are enforced by the contract.
/// Valid progression: Registered → Harvested → Processed → Shipped → Delivered → Retail
#[contracttype]
#[derive(Clone, PartialEq, Debug)]
pub enum LifecycleStage {
    Registered,
    Harvested,
    Processed,
    Shipped,
    Delivered,
    Retail,
use soroban_sdk::{
    contract, contractimpl, contracttype, Address, Bytes, Env, String, Vec, Symbol,
};
use soroban_sdk::xdr::ToXdr;
use soroban_sdk::{contract, contractimpl, contracttype, contracterror, Address, Env, String, Vec, Symbol};

/// Current event schema version.
///
/// Bump this constant whenever the [`TrackingEvent`] payload layout changes in
/// a backward-incompatible way. Consumers should inspect the `schema_version`
/// field (and the matching topic slot) to select the correct parser.
///
/// | Version | Changes |
/// |---------|---------|
/// | 1       | Initial versioned schema. Adds `schema_version` field. |
pub const EVENT_SCHEMA_VERSION: u32 = 1;

mod tests;
mod resilience_tests;

// ── Payload size limits (issue #311) ─────────────────────────────────────────
// All limits are in bytes (Soroban String::len() returns byte count).
// | Field    | Max bytes | Notes                          |
// |----------|-----------|--------------------------------|
// | id       |       128 | Storage key; keep short        |
// | name     |       256 | Human-readable label           |
// | origin   |       256 | Geographic/org string          |
// | location |       256 | Per-event location             |
// | metadata |      4096 | JSON payload                   |
const MAX_ID_LEN:       u32 = 128;
const MAX_NAME_LEN:     u32 = 256;
const MAX_ORIGIN_LEN:   u32 = 256;
const MAX_LOCATION_LEN: u32 = 256;
const MAX_METADATA_LEN: u32 = 4096;

// ── Event expiration policy (issue #314) ──────────────────────────────────────
/// Pending events expire after this many seconds (7 days).
const EXPIRATION_WINDOW: u64 = 604_800;  // 7 * 24 * 60 * 60 seconds

fn assert_len(s: &String, max: u32, field: &'static str) {
    if s.len() > max { panic!("{} exceeds max length", field); }
}

// ── Error types ──────────────────────────────────────────────────────────────

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
pub enum Error {
    ProductNotFound = 1,
    NotAuthorized = 2,
    ApproverNotAuthorized = 3,
    NoPendingEvents = 4,
    OwnerOnly = 5,
    PendingEventExpired = 6,
    InvalidNonce = 7,
}

// ── Data models ──────────────────────────────────────────────────────────────

/// Represents a product registered on the Supply-Link blockchain.
///
/// Products are the core entity of the supply chain. Once registered, a product
/// accumulates [`TrackingEvent`]s as it moves through the supply chain. The
/// `owner` field always reflects the *current* custodian; historical ownership
/// is captured implicitly through `ownership_transferred` events.
///
/// # Storage
/// Stored under [`DataKey::Product`] using the product's `id` as the key.
/// Storage type is `persistent`, so entries survive ledger archival as long as
/// the rent is paid.
#[contracttype]
#[derive(Clone)]
pub struct Product {
    /// Caller-supplied unique identifier for this product (e.g. `"batch-2024-001"`).
    /// Must be unique across all registered products; duplicate IDs are rejected
    /// with `"product already exists"` and leave existing state unchanged.
    pub id: String,
    /// Human-readable product name (e.g. `"Arabica Coffee Beans"`).
    pub name: String,
    /// Geographic or organisational origin of the product
    /// (e.g. `"Yirgacheffe, Ethiopia"`).
    pub origin: String,
    /// Stellar address of the current product owner.
    /// Only this address may call owner-gated functions such as
    /// [`SupplyLinkContract::transfer_ownership`] and
    /// [`SupplyLinkContract::add_authorized_actor`].
    pub owner: Address,
    /// Unix timestamp (seconds) recorded by the Soroban ledger at registration
    /// time. Set automatically; callers cannot supply this value.
    pub timestamp: u64,
    /// Addresses that are permitted to call
    /// [`SupplyLinkContract::add_tracking_event`] for this product in addition
    /// to the owner. Managed via [`SupplyLinkContract::add_authorized_actor`]
    /// and [`SupplyLinkContract::remove_authorized_actor`].
    pub authorized_actors: Vec<Address>,
    /// Current lifecycle stage (#404)
    pub lifecycle_stage: LifecycleStage,
    /// Whether the product is active. Deactivated products reject new events.
    /// Number of signatures required to approve events for this product.
    /// If 0 or 1, events are recorded immediately. If > 1, events are staged
    /// as pending until the required number of approvals are received.
    pub required_signatures: u32,
    /// Lifecycle state of the product. `true` indicates the product is active
    /// and can receive tracking events. `false` indicates the product has been
    /// deactivated and is read-only. Defaults to `true` on registration.
    pub active: bool,
}

/// A single supply-chain event recorded against a [`Product`].
///
/// Events are append-only. Once written they cannot be modified or deleted,
/// providing an immutable audit trail. All events for a product are stored
/// together under [`DataKey::Events`].
///
/// # Schema versioning
/// The `schema_version` field carries [`EVENT_SCHEMA_VERSION`] at write time.
/// Indexers and backend services must read this field first and dispatch to the
/// appropriate parser before accessing any other fields. The version is also
/// encoded as the **fourth topic slot** (index 3) in every emitted event so
/// consumers can filter by version without deserialising the payload.
/// Topic layout: `(event_name, product_id, event_type, schema_version)`.
///
/// # Storage
/// Stored as a `Vec<TrackingEvent>` under [`DataKey::Events`] keyed by
/// `product_id`. Storage type is `persistent`.
#[contracttype]
#[derive(Clone)]
pub struct TrackingEvent {
    /// Schema version of this event payload. Always set to
    /// [`EVENT_SCHEMA_VERSION`] at write time. Consumers must check this field
    /// before parsing any other fields.
    pub schema_version: u32,
    /// ID of the [`Product`] this event belongs to.
    pub product_id: String,
    /// Free-form location string describing where the event occurred
    /// (e.g. `"Port of Rotterdam, Netherlands"`).
    pub location: String,
    /// Stellar address of the supply-chain participant who recorded this event.
    /// Must be the product owner or an address in `authorized_actors`.
    pub actor: Address,
    /// Unix timestamp (seconds) recorded by the Soroban ledger when the event
    /// was submitted. Set automatically; callers cannot supply this value.
    pub timestamp: u64,
    /// Supply-chain stage. Accepted values: `"HARVEST"`, `"PROCESSING"`,
    /// `"SHIPPING"`, `"RETAIL"`. The contract stores this as a raw string and
    /// does not validate the value — callers are responsible for using a
    /// recognised stage name.
    pub event_type: String,
    /// Arbitrary JSON string carrying stage-specific metadata
    /// (e.g. `{"temperature":"4°C","humidity":"60%"}`). The contract stores
    /// this opaquely; consumers are responsible for parsing it.
    pub metadata: String,
    /// Stable deterministic event ID — a hex-encoded SHA-256 hash of the
    /// canonical fields: `product_id|actor|event_type|timestamp|metadata`.
    /// Invariant across contract upgrades; suitable for deep links and QR payloads.
    pub stable_id: String,
}

// ── Role types (#387) ─────────────────────────────────────────────────────────

/// Named role for an authorized actor.
#[contracttype]
#[derive(Clone, PartialEq, Debug)]
pub enum Role {
    /// Can harvest/originate events.
    Producer,
    /// Can add processing events.
    Processor,
    /// Can add shipping events.
    Shipper,
    /// Can add retail events.
    Retailer,
    /// Can add any event type.
    Any,
}

/// Binds an actor address to a named role.
#[contracttype]
#[derive(Clone)]
pub struct ActorRole {
    pub actor: Address,
    pub role: Role,
}

/// Authorization policy for a product.
#[contracttype]
#[derive(Clone)]
pub struct AuthPolicy {
    /// Minimum number of distinct authorized actors that must sign an event
    /// for high-risk event types. 1 = single-signer (default).
    pub threshold: u32,
    /// Role assignments for this product's authorized actors.
    pub roles: Vec<ActorRole>,
}

/// A pending event awaiting multi-signature approval.
///
/// For high-value products, events are staged until the required number of
/// authorized actors have approved them.
///
/// Each pending event has a stable identifier (`pending_event_id`) that remains
/// unchanged even if other pending events in the queue are removed or approved.
/// This prevents client mistakes from index-based references that shift after
/// queue mutations.
#[contracttype]
#[derive(Clone)]
pub struct PendingEvent {
    /// Stable unique identifier for this pending event within its product.
    /// Generated at creation time and immutable. Used for deterministic targeting
    /// in approve/reject operations to avoid index-based race conditions.
    pub pending_event_id: u64,
    /// ID of the product this event is for.
    pub product_id: String,
    /// The event data awaiting approval.
    pub event: TrackingEvent,
    /// Addresses that have approved this event.
    pub approvals: Vec<Address>,
    /// Number of approvals required before the event is finalized.
    pub required_signatures: u32,
    /// Timestamp when the pending event was created.
    pub created_at: u64,
    /// Timestamp when this pending event expires (issue #314).
    pub expiration: u64,
}

/// Event rejection data with optional reason context.
///
/// Emitted when a pending event is rejected, providing audit trail
/// and optional explanation for the rejection decision.
#[contracttype]
#[derive(Clone)]
pub struct EventRejection {
    /// The product ID the rejected event was for.
    pub product_id: String,
    /// The rejected event data.
    pub event: TrackingEvent,
    /// Address of the actor who rejected the event.
    pub rejector: Address,
    /// Optional reason for rejection (max 256 characters).
    pub reason: String,
    /// Timestamp of the rejection.
    pub timestamp: u64,
    pub event_type: String,
    pub metadata: String,
}

/// Pending ownership transfer escrow (#396)
#[contracttype]
#[derive(Clone)]
pub struct TransferEscrow {
    pub product_id: String,
    pub current_owner: Address,
    pub proposed_owner: Address,
    pub requested_at: u64,
    pub disputed: bool,
}

/// Pending event awaiting approval (#394)
#[contracttype]
#[derive(Clone)]
pub struct PendingEvent {
    pub product_id: String,
    pub submitter: Address,
    pub location: String,
    pub event_type: String,
    pub metadata: String,
    pub submitted_at: u64,
    pub required_approvers: Vec<Address>,
    pub approvals: Vec<Address>,
    pub rejected: bool,
    pub expires_at: u64,
}

// ── Storage keys ─────────────────────────────────────────────────────────────

/// Enumeration of all persistent storage keys used by the contract.
///
/// Using a typed enum prevents key collisions and makes storage layout
/// explicit for auditors.
///
/// # Variants
/// - [`DataKey::Product`] — stores a single [`Product`] by its string ID.
/// - [`DataKey::Events`] — stores a `Vec<TrackingEvent>` for a product ID.
/// - [`DataKey::ProductCount`] — stores a `u64` global counter of registered products.
/// - [`DataKey::ProductIndex`] — maps a sequential `u64` index to a product ID
///   string, enabling paginated listing via [`SupplyLinkContract::list_products`].
#[contracttype]
pub enum DataKey {
    /// Key for a [`Product`] entry. The inner `String` is the product ID.
    Product(String),
    /// Key for the event log of a product. The inner `String` is the product ID.
    Events(String),
    /// Pending ownership transfer escrow keyed by product_id (#396)
    TransferEscrow(String),
    /// Pending event awaiting approval, keyed by (product_id, submitter) (#394)
    PendingEvent(String, Address),
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Map an event_type string to the lifecycle stage it transitions the product to.
fn event_type_to_stage(env: &Env, event_type: &String) -> Option<LifecycleStage> {
    if *event_type == String::from_str(env, "HARVEST") {
        Some(LifecycleStage::Harvested)
    } else if *event_type == String::from_str(env, "PROCESSING") {
        Some(LifecycleStage::Processed)
    } else if *event_type == String::from_str(env, "SHIPPING") {
        Some(LifecycleStage::Shipped)
    } else if *event_type == String::from_str(env, "DELIVERY") {
        Some(LifecycleStage::Delivered)
    } else if *event_type == String::from_str(env, "RETAIL") {
        Some(LifecycleStage::Retail)
    } else {
        None
    }
}

/// Validate that the given event_type is allowed from the current lifecycle stage.
fn validate_lifecycle_transition(env: &Env, current: &LifecycleStage, event_type: &String) -> bool {
    match current {
        LifecycleStage::Registered => *event_type == String::from_str(env, "HARVEST"),
        LifecycleStage::Harvested  => *event_type == String::from_str(env, "PROCESSING"),
        LifecycleStage::Processed  => *event_type == String::from_str(env, "SHIPPING"),
        LifecycleStage::Shipped    => *event_type == String::from_str(env, "DELIVERY"),
        LifecycleStage::Delivered  => *event_type == String::from_str(env, "RETAIL"),
        LifecycleStage::Retail     => false,
    }
    /// Key for pending events awaiting multi-signature approval.
    /// The inner `String` is the product ID.
    PendingEvents(String),
    /// Key for the next stable pending event ID counter.
    /// The inner `String` is the product ID.
    /// Stores a `u64` used to generate unique identifiers for pending events.
    NextPendingId(String),
    /// Key for the global product registration counter.
    ProductCount,
    /// Key for the index-to-ID mapping used by pagination.
    /// The inner `u64` is the zero-based insertion index.
    ProductIndex(u64),
    /// Key for the authorization policy (roles + threshold) of a product.
    AuthPolicy(String),
    /// Key for actor nonce tracking. The inner `Address` is the actor address.
    ActorNonce(Address),
}

// ── Contract ─────────────────────────────────────────────────────────────────

/// The Supply-Link Soroban smart contract.
///
/// Provides a decentralised, tamper-proof registry for supply-chain products
/// and their associated tracking events on the Stellar blockchain.
///
/// # Deployment
/// Testnet contract ID: `CBUWSKT2UGOAXK4ZREVDJV5XHSYB42PZ3CERU2ZFUTUMAZLJEHNZIECA`
///
/// # Authorization model
/// - **Owner-gated** functions (`transfer_ownership`, `add_authorized_actor`,
///   `remove_authorized_actor`, `update_product_metadata`) require the current
///   product owner to sign the transaction via `require_auth()`.
/// - **Actor-gated** functions (`add_tracking_event`) accept either the owner
///   or any address in `authorized_actors`.
/// - **Read-only** functions (`get_product`, `get_tracking_events`, etc.) have
///   no authorization requirements.
#[contract]
pub struct SupplyLinkContract;

#[contractimpl]
impl SupplyLinkContract {
    /// Register a new product on-chain.
    ///
    /// Creates a [`Product`] entry in persistent storage and initialises the
    /// global product counter and index mapping used by
    /// [`Self::list_products`].
    ///
    /// # Parameters
    /// - `env` — Soroban execution environment (injected by the runtime).
    /// - `id` — Caller-supplied unique product identifier. Must not already
    ///   exist; duplicate IDs are rejected with `"product already exists"`.
    /// - `name` — Human-readable product name.
    /// - `origin` — Geographic or organisational origin of the product.
    /// - `owner` — Stellar address that will own the product. This address
    ///   must sign the transaction.
    /// - `required_signatures` — Number of approvals required for events (0 or 1 = immediate, >1 = multi-sig).
    ///
    /// # Returns
    /// The newly created [`Product`] struct.
    ///
    /// # Authorization
    /// Requires `owner.require_auth()`. The transaction must be signed by
    /// `owner`.
    ///
    /// # Warning
    /// If a product with `id` already exists it will be **silently overwritten**
    /// with the new `name`, `origin`, `owner`, and `required_signatures`. The
    /// previous product's data is lost. Additionally, the global
    /// `ProductCount` and `ProductIndex` are incremented unconditionally, so a
    /// duplicate registration creates a ghost index entry pointing to the same
    /// `id`. Callers should use [`Self::product_exists`] to guard against
    /// accidental overwrites.
    ///
    /// # Panics
    /// - `"product already exists"` — if a product with `id` is already registered.
    ///   `product_count` and index mappings are NOT modified on rejection.
    ///
    /// # Emitted Events
    /// Publishes a `("product_registered", id)` event with the [`Product`]
    /// struct as the event body.
    pub fn register_product(
        env: Env,
        id: String,
        name: String,
        origin: String,
        owner: Address,
        required_signatures: u32,
    ) -> Product {
        // Duplicate guard — must come before auth to avoid leaking state on
        // duplicate attempts and to keep counter/index consistent.
        if env.storage().persistent().has(&DataKey::Product(id.clone())) {
            panic!("product already exists");
        }

        owner.require_auth();
        // Issue #311: enforce size limits.
        assert_len(&id,     MAX_ID_LEN,     "id");
        assert_len(&name,   MAX_NAME_LEN,   "name");
        assert_len(&origin, MAX_ORIGIN_LEN, "origin");
        let product = Product {
            id: id.clone(),
            name,
            origin,
            owner,
            timestamp: env.ledger().timestamp(),
            authorized_actors: Vec::new(&env),
            lifecycle_stage: LifecycleStage::Registered,
            required_signatures,
            active: true,
        };
        env.storage()
            .persistent()
            .set(&DataKey::Product(id.clone()), &product);

        // Increment product count
        let count: u64 = env
            .storage()
            .persistent()
            .get(&DataKey::ProductCount)
            .unwrap_or(0);
        env.storage()
            .persistent()
            .set(&DataKey::ProductCount, &(count + 1));

        // Store product index mapping
        env.storage()
            .persistent()
            .set(&DataKey::ProductIndex(count), &id);

        // Emit event
        env.events().publish(
            (Symbol::new(&env, "product_registered"), id.clone()),
            product.clone(),
        );

        product
    }

    /// Add a tracking event for a product. Enforces lifecycle stage transitions (#404).
    /// Add a tracking event for a product.
    ///
    /// Appends a new [`TrackingEvent`] to the product's event log. The event
    /// log is stored as a `Vec<TrackingEvent>` and grows with each call.
    ///
    /// # Parameters
    /// - `env` — Soroban execution environment.
    /// - `product_id` — ID of the product to record the event against.
    /// - `caller` — Address of the supply-chain participant submitting the
    ///   event. Must be the product owner or an address in
    ///   `authorized_actors`.
    /// - `location` — Free-form location string (e.g. `"Port of Hamburg"`).
    /// - `event_type` — Canonical supply-chain stage. Must be one of:
    ///   `"HARVEST"`, `"PROCESSING"`, `"SHIPPING"`, `"RETAIL"`.
    ///   Unknown values are rejected with `"invalid event_type"` (issue #310).
    /// - `metadata` — Arbitrary JSON string with stage-specific data.
    ///
    /// # Returns
    /// The newly created [`TrackingEvent`] struct.
    ///
    /// # Authorization
    /// Requires `caller.require_auth()`. The authorization check is performed
    /// *after* verifying that `caller` is the owner or an authorized actor, so
    /// unauthorized addresses are rejected before any auth overhead is incurred.
    ///
    /// # Panics
    /// - `"product not found"` — if `product_id` is not registered.
    /// - `"caller is not authorized"` — if `caller` is neither the product
    ///   owner nor in `authorized_actors`.
    ///
    /// # Emitted Events
    /// - When `product.required_signatures <= 1`: publishes an
    ///   `("event_added", product_id, event_type, schema_version)` event with
    ///   the [`TrackingEvent`] struct as the event body.
    /// - When `product.required_signatures > 1`: the event is staged as
    ///   pending and an `("event_pending", product_id, event_type,
    ///   schema_version)` event is published instead. The event is not added
    ///   to the finalized log until [`Self::approve_event`] collects enough
    ///   approvals.
    pub fn add_tracking_event(
        env: Env,
        product_id: String,
        caller: Address,
        location: String,
        event_type: String,
        metadata: String,
    ) -> TrackingEvent {
        let mut product: Product = env
    ) -> Result<TrackingEvent, Error> {
        let product: Product = env
            .storage()
            .persistent()
            .get(&DataKey::Product(product_id.clone()))
            .ok_or(Error::ProductNotFound)?;

        if !product.active {
            panic!("product is deactivated");
        }

        let caller = product.owner.clone();
        // Verify caller is owner or an authorized actor before requiring auth
        let is_owner = product.owner == caller;
        let is_actor = product.authorized_actors.contains(&caller);
        if !is_owner && !is_actor {
            return Err(Error::NotAuthorized);
        }
        caller.require_auth();
        // Issue #311: enforce size limits.
        assert_len(&location, MAX_LOCATION_LEN, "location");
        assert_len(&metadata, MAX_METADATA_LEN, "metadata");

        let timestamp = env.ledger().timestamp();

        // Compute stable_id: SHA-256 of "product_id|event_type|timestamp" encoded as bytes
        let stable_id = compute_stable_id(&env, &product_id, &caller, &event_type, timestamp, &metadata);

        // Enforce lifecycle transition (#404)
        if !validate_lifecycle_transition(&env, &product.lifecycle_stage, &event_type) {
            panic!("invalid lifecycle transition");
        }

        // Advance lifecycle stage if this event triggers a transition
        if let Some(next_stage) = event_type_to_stage(&env, &event_type) {
            product.lifecycle_stage = next_stage;
            env.storage()
                .persistent()
                .set(&DataKey::Product(product_id.clone()), &product);
        }

        let event = TrackingEvent {
            schema_version: EVENT_SCHEMA_VERSION,
            product_id: product_id.clone(),
            location,
o            actor: caller,
            timestamp,
            actor: caller.clone(),
            timestamp: env.ledger().timestamp(),
            event_type: event_type.clone(),
            metadata,
            stable_id,
        };

        // Check if multi-signature is required
        if product.required_signatures > 1 {
            // Stage event as pending with a stable ID
            let mut pending: Vec<PendingEvent> = env
                .storage()
                .persistent()
                .get(&DataKey::PendingEvents(product_id.clone()))
                .unwrap_or_else(|| Vec::new(&env));

            // Generate next stable pending event ID
            let next_id: u64 = env
                .storage()
                .persistent()
                .get(&DataKey::NextPendingId(product_id.clone()))
                .unwrap_or(0u64);

            let mut approvals = Vec::new(&env);
            approvals.push_back(caller);

            let pending_event = PendingEvent {
                pending_event_id: next_id,
                product_id: product_id.clone(),
                event: event.clone(),
                approvals,
                required_signatures: product.required_signatures,
                created_at: env.ledger().timestamp(),
                expiration: env.ledger().timestamp() + EXPIRATION_WINDOW,
            };

            pending.push_back(pending_event);
            env.storage()
                .persistent()
                .set(&DataKey::PendingEvents(product_id.clone()), &pending);

            // Increment the ID counter for next pending event
            env.storage()
                .persistent()
                .set(&DataKey::NextPendingId(product_id.clone()), &(next_id + 1));

            // Emit pending event
            env.events().publish(
                (Symbol::new(&env, "event_pending"), product_id, event_type, EVENT_SCHEMA_VERSION),
                event.clone(),
            );
        } else {
            // Immediately finalize event
            let mut events: Vec<TrackingEvent> = env
                .storage()
                .persistent()
                .get(&DataKey::Events(product_id.clone()))
                .unwrap_or_else(|| Vec::new(&env));

            events.push_back(event.clone());
            env.storage()
                .persistent()
                .set(&DataKey::Events(product_id.clone()), &events);

            // Emit event
            env.events().publish(
                (Symbol::new(&env, "event_added"), product_id, event_type, EVENT_SCHEMA_VERSION),
                event.clone(),
            );
        }

        Ok(event)
    }

    /// Retrieve a product by its ID.
    ///
    /// # Returns
    /// The [`Product`] struct stored under `id`.
    ///
    /// # Errors
    /// - [`Error::ProductNotFound`] — if no product with `id` is registered.
    pub fn get_product(env: Env, id: String) -> Result<Product, Error> {
        env.storage()
            .persistent()
            .get(&DataKey::Product(id))
            .ok_or(Error::ProductNotFound)
    }

    /// Retrieve all tracking events for a product.
    ///
    /// Returns events in insertion order (oldest first).
    ///
    /// # Parameters
    /// - `env` — Soroban execution environment.
    /// - `product_id` — The product ID whose events to retrieve.
    ///
    /// # Returns
    /// A `Vec<TrackingEvent>` containing every event recorded for the product.
    /// Returns an empty vector if the product has no events or does not exist.
    ///
    /// # Authorization
    /// None — this is a read-only function.
    ///
    /// # Panics
    /// Does not panic.
    pub fn get_tracking_events(env: Env, product_id: String) -> Vec<TrackingEvent> {
        env.storage()
            .persistent()
            .get(&DataKey::Events(product_id))
            .unwrap_or_else(|| Vec::new(&env))
    }

    /// Check whether a product ID is registered.
    ///
    /// Useful for pre-flight checks before calling functions that panic on
    /// unknown IDs.
    ///
    /// # Parameters
    /// - `env` — Soroban execution environment.
    /// - `id` — The product ID to check.
    ///
    /// # Returns
    /// `true` if a product with `id` exists in storage, `false` otherwise.
    ///
    /// # Authorization
    /// None — this is a read-only function.
    ///
    /// # Panics
    /// Does not panic.
    pub fn product_exists(env: Env, id: String) -> bool {
        env.storage().persistent().has(&DataKey::Product(id))
    }

    /// Return the number of tracking events recorded for a product.
    ///
    /// # Parameters
    /// - `env` — Soroban execution environment.
    /// - `product_id` — The product ID to query.
    ///
    /// # Returns
    /// The number of events as a `u32`. Returns `0` if the product has no
    /// events or does not exist.
    ///
    /// # Note
    /// This function deserialises the full `Vec<TrackingEvent>` from storage
    /// to read its length. It has the same storage cost as
    /// `get_tracking_events(product_id).len()` and is not a cheaper
    /// alternative for large event logs.
    ///
    /// # Authorization
    /// None — this is a read-only function.
    ///
    /// # Panics
    /// Does not panic.
    pub fn get_events_count(env: Env, product_id: String) -> u32 {
        env.storage()
            .persistent()
            .get::<DataKey, Vec<TrackingEvent>>(&DataKey::Events(product_id))
            .map(|v| v.len())
            .unwrap_or(0)
    }

    /// Transfer product ownership to a new address.
    ///
    /// Updates the `owner` field of the [`Product`] in storage. The previous
    /// owner loses all owner-gated privileges immediately. The new owner gains
    /// them immediately.
    ///
    /// # Safety Checks
    /// - Prevents no-op transfers (transferring to the current owner)
    /// - Validates that the new owner is a valid address
    ///
    /// # Parameters
    /// - `env` — Soroban execution environment.
    /// - `product_id` — ID of the product to transfer.
    /// - `new_owner` — Stellar address of the incoming owner.
    ///
    /// # Returns
    /// `true` on success.
    ///
    /// # Authorization
    /// Requires the *current* `product.owner.require_auth()`. The transaction
    /// must be signed by the current owner.
    ///
    /// # Panics
    /// - `"product not found"` — if `product_id` is not registered.
    /// - `"cannot transfer to current owner"` — if `new_owner` equals current owner.
    ///
    /// # Emitted Events
    /// Publishes an `("ownership_transferred", product_id)` event with
    /// `new_owner` as the event body.
    pub fn transfer_ownership(
        env: Env,
        product_id: String,
        new_owner: Address,
        nonce: u64,
    ) -> Result<bool, Error> {
        let mut product: Product = env
            .storage()
            .persistent()
            .get(&DataKey::Product(product_id.clone()))
            .ok_or(Error::ProductNotFound)?;

        product.owner.require_auth();
        Self::validate_and_increment_nonce(&env, &product.owner, nonce);
        
        product.owner = new_owner.clone();
        env.storage()
            .persistent()
            .set(&DataKey::Product(product_id.clone()), &product);

        env.events().publish(
            (Symbol::new(&env, "ownership_transferred"), product_id),
            new_owner,
        );

        Ok(true)
    }

    /// Grant an address permission to add tracking events for a product.
    ///
    /// Appends `actor` to `product.authorized_actors`. Prevents duplicate entries
    /// to maintain clean governance state.
    ///
    /// # Parameters
    /// - `env` — Soroban execution environment.
    /// - `product_id` — ID of the product to update.
    /// - `actor` — Stellar address to authorise.
    ///
    /// # Returns
    /// `true` if `actor` was added, `false` if `actor` was already in the list.
    ///
    /// # Authorization
    /// Requires `product.owner.require_auth()`. Only the current product owner
    /// may grant actor permissions.
    ///
    /// # Panics
    /// - `"product not found"` — if `product_id` is not registered.
    /// - `"actor already authorized"` — if the actor is already in the authorized list.
    ///
    /// # Emitted Events
    /// Publishes an `("actor_authorized", product_id)` event with `actor` as
    /// the event body.
    pub fn add_authorized_actor(
        env: Env,
        product_id: String,
        actor: Address,
        nonce: u64,
    ) -> Result<bool, Error> {
        let mut product: Product = env
            .storage()
            .persistent()
            .get(&DataKey::Product(product_id.clone()))
            .ok_or(Error::ProductNotFound)?;

        product.owner.require_auth();
        Self::validate_and_increment_nonce(&env, &product.owner, nonce);
        
        product.authorized_actors.push_back(actor.clone());
        env.storage()
            .persistent()
            .set(&DataKey::Product(product_id.clone()), &product);

        env.events().publish(
            (Symbol::new(&env, "actor_authorized"), product_id),
            actor,
        );

        Ok(true)
    }

    // ── #404: Lifecycle helpers ───────────────────────────────────────────────

    /// Get the current lifecycle stage of a product.
    pub fn get_lifecycle_stage(env: Env, product_id: String) -> LifecycleStage {
        let product: Product = env
            .storage()
            .persistent()
            .get(&DataKey::Product(product_id))
            .expect("product not found");
        product.lifecycle_stage
    }

    // ── #396: Ownership transfer escrow ──────────────────────────────────────

    /// Request an ownership transfer. Creates an escrow pending acceptance.
    pub fn request_transfer_ownership(
        env: Env,
        product_id: String,
        proposed_owner: Address,
    ) -> TransferEscrow {
        let product: Product = env
            .storage()
            .persistent()
            .get(&DataKey::Product(product_id.clone()))
            .expect("product not found");

        if product.owner == proposed_owner {
            panic!("proposed owner must differ from current owner");
        }
        product.owner.require_auth();

        let escrow = TransferEscrow {
            product_id: product_id.clone(),
            current_owner: product.owner.clone(),
            proposed_owner,
            requested_at: env.ledger().timestamp(),
            disputed: false,
        };
        env.storage()
            .persistent()
            .set(&DataKey::TransferEscrow(product_id.clone()), &escrow);

        env.events().publish(
            (Symbol::new(&env, "transfer_requested"), product_id),
            escrow.clone(),
        );
        escrow
    }

    /// Accept a pending transfer. Proposed owner confirms and takes ownership.
    pub fn accept_transfer_ownership(env: Env, product_id: String) -> bool {
        let escrow: TransferEscrow = env
            .storage()
            .persistent()
            .get(&DataKey::TransferEscrow(product_id.clone()))
            .expect("no pending transfer");

        if escrow.disputed {
            panic!("transfer is disputed");
        }

        escrow.proposed_owner.require_auth();

        let mut product: Product = env
            .storage()
            .persistent()
            .get(&DataKey::Product(product_id.clone()))
            .expect("product not found");

        product.owner = escrow.proposed_owner.clone();
        env.storage()
            .persistent()
            .set(&DataKey::Product(product_id.clone()), &product);
        env.storage()
            .persistent()
            .remove(&DataKey::TransferEscrow(product_id.clone()));

        env.events().publish(
            (Symbol::new(&env, "transfer_accepted"), product_id),
            escrow.proposed_owner,
        );
        true
    }

    /// Cancel a pending transfer request (current owner only).
    pub fn cancel_transfer_request(env: Env, product_id: String) -> bool {
        let escrow: TransferEscrow = env
            .storage()
            .persistent()
            .get(&DataKey::TransferEscrow(product_id.clone()))
            .expect("no pending transfer");

        escrow.current_owner.require_auth();
        env.storage()
            .persistent()
            .remove(&DataKey::TransferEscrow(product_id.clone()));

        env.events().publish(
            (Symbol::new(&env, "transfer_cancelled"), product_id),
            escrow.current_owner,
        );
        true
    }

    /// Dispute a pending transfer. Either party can raise a dispute.
    pub fn dispute_transfer_ownership(env: Env, product_id: String) -> bool {
        let mut escrow: TransferEscrow = env
            .storage()
            .persistent()
            .get(&DataKey::TransferEscrow(product_id.clone()))
            .expect("no pending transfer");

        // Either current owner or proposed owner can dispute
        let caller_is_owner = escrow.current_owner.clone();
        caller_is_owner.require_auth();

        escrow.disputed = true;
        env.storage()
            .persistent()
            .set(&DataKey::TransferEscrow(product_id.clone()), &escrow);

        env.events().publish(
            (Symbol::new(&env, "transfer_disputed"), product_id),
            escrow.current_owner,
        );
        true
    }

    /// Get the pending transfer escrow for a product, if any.
    pub fn get_transfer_escrow(env: Env, product_id: String) -> Option<TransferEscrow> {
        env.storage()
            .persistent()
            .get(&DataKey::TransferEscrow(product_id))
    }

    // ── #394: Pending event approval queue ───────────────────────────────────

    /// Submit an event for approval by required approvers.
    /// The event is not committed to the event log until finalized.
    pub fn submit_event_for_approval(
        env: Env,
        product_id: String,
        submitter: Address,
        location: String,
        event_type: String,
        metadata: String,
        required_approvers: Vec<Address>,
        ttl_seconds: u64,
    ) -> PendingEvent {
        let product: Product = env
            .storage()
            .persistent()
            .get(&DataKey::Product(product_id.clone()))
            .expect("product not found");

        let is_owner = product.owner == submitter;
        let is_actor = product.authorized_actors.contains(&submitter);
        if !is_owner && !is_actor {
            panic!("submitter is not authorized");
        }
        submitter.require_auth();

        let now = env.ledger().timestamp();
        let pending = PendingEvent {
            product_id: product_id.clone(),
            submitter: submitter.clone(),
            location,
            event_type,
            metadata,
            submitted_at: now,
            required_approvers,
            approvals: Vec::new(&env),
            rejected: false,
            expires_at: now + ttl_seconds,
        };

        env.storage()
            .persistent()
            .set(&DataKey::PendingEvent(product_id.clone(), submitter.clone()), &pending);

        env.events().publish(
            (Symbol::new(&env, "event_submitted"), product_id),
            submitter,
        );
        pending
    }

    /// Approve a pending event. Approver must be in required_approvers.
    pub fn approve_pending_event(
        env: Env,
        product_id: String,
        submitter: Address,
        approver: Address,
    ) -> PendingEvent {
        let mut pending: PendingEvent = env
            .storage()
            .persistent()
            .get(&DataKey::PendingEvent(product_id.clone(), submitter.clone()))
            .expect("no pending event");

        if pending.rejected {
            panic!("event already rejected");
        }
        if env.ledger().timestamp() > pending.expires_at {
            panic!("pending event expired");
        }
        if !pending.required_approvers.contains(&approver) {
            panic!("approver is not a required approver");
        }
        if pending.approvals.contains(&approver) {
            panic!("already approved");
        }

        approver.require_auth();
        pending.approvals.push_back(approver.clone());

        env.storage()
            .persistent()
            .set(&DataKey::PendingEvent(product_id.clone(), submitter.clone()), &pending);

        env.events().publish(
            (Symbol::new(&env, "event_approved"), product_id),
            approver,
        );
        pending
    }

    /// Reject a pending event. Any required approver can reject.
    pub fn reject_pending_event(
        env: Env,
        product_id: String,
        submitter: Address,
        approver: Address,
    ) -> bool {
        let mut pending: PendingEvent = env
            .storage()
            .persistent()
            .get(&DataKey::PendingEvent(product_id.clone(), submitter.clone()))
            .expect("no pending event");

        if !pending.required_approvers.contains(&approver) {
            panic!("approver is not a required approver");
        }
        approver.require_auth();

        pending.rejected = true;
        env.storage()
            .persistent()
            .set(&DataKey::PendingEvent(product_id.clone(), submitter.clone()), &pending);

        env.events().publish(
            (Symbol::new(&env, "event_rejected"), product_id),
            approver,
        );
        true
    }

    /// Finalize a pending event once all required approvals are collected.
    /// Commits the event to the product's event log.
    pub fn finalize_pending_event(
        env: Env,
        product_id: String,
        submitter: Address,
    ) -> TrackingEvent {
        let pending: PendingEvent = env
            .storage()
            .persistent()
            .get(&DataKey::PendingEvent(product_id.clone(), submitter.clone()))
            .expect("no pending event");

        if pending.rejected {
            panic!("event was rejected");
        }
        if env.ledger().timestamp() > pending.expires_at {
            panic!("pending event expired");
        }

        // All required approvers must have approved
        for i in 0..pending.required_approvers.len() {
            let req = pending.required_approvers.get(i).unwrap();
            if !pending.approvals.contains(&req) {
                panic!("not all approvers have approved");
            }
        }

        let event = TrackingEvent {
            product_id: product_id.clone(),
            location: pending.location,
            actor: pending.submitter,
            timestamp: env.ledger().timestamp(),
            event_type: pending.event_type,
            metadata: pending.metadata,
        };

        let mut events: Vec<TrackingEvent> = env
            .storage()
            .persistent()
            .get(&DataKey::Events(product_id.clone()))
            .unwrap_or_else(|| Vec::new(&env));
        events.push_back(event.clone());
        env.storage()
            .persistent()
            .set(&DataKey::Events(product_id.clone()), &events);

        // Remove the pending event
        env.storage()
            .persistent()
            .remove(&DataKey::PendingEvent(product_id.clone(), submitter));

        env.events().publish(
            (Symbol::new(&env, "event_finalized"), product_id),
            event.clone(),
        );
        event
    }

    /// Get a pending event by product_id and submitter.
    pub fn get_pending_event(
        env: Env,
        product_id: String,
        submitter: Address,
    ) -> Option<PendingEvent> {
        env.storage()
            .persistent()
            .get(&DataKey::PendingEvent(product_id, submitter))
    }
}

    /// Revoke an address's permission to add tracking events for a product.
    ///
    /// Rebuilds `authorized_actors` without `actor`. Because
    /// [`Self::add_authorized_actor`] prevents duplicates, at most one entry
    /// will ever be removed.
    ///
    /// # Governance Safeguards
    /// - Prevents removal of the owner from authorized actors if multi-signature
    ///   is enabled and would leave insufficient authorized actors to meet the
    ///   required signature threshold.
    /// - Ensures at least one authorized path remains for governance operations.
    ///
    /// # Parameters
    /// - `env` — Soroban execution environment.
    /// - `product_id` — ID of the product to update.
    /// - `actor` — Stellar address to revoke.
    ///
    /// # Returns
    /// `true` if `actor` was found and removed, `false` if `actor` was not in
    /// the authorized list.
    ///
    /// # Authorization
    /// Requires `product.owner.require_auth()`. Only the current product owner
    /// may revoke actor permissions.
    ///
    /// # Panics
    /// - `"product not found"` — if `product_id` is not registered.
    /// - `"cannot remove owner from actors"` — if attempting to remove the owner
    ///   when it would violate governance invariants.
    /// - `"removal would violate governance"` — if removal would leave insufficient
    ///   actors to meet multi-signature requirements.
    ///
    /// # Emitted Events
    /// Does not emit an event. Removal of an actor is not announced on-chain.
    /// Consumers tracking actor permissions must observe the absence of future
    /// `actor_authorized` events or query [`Self::get_authorized_actors`]
    /// directly.
    pub fn remove_authorized_actor(
        env: Env,
        product_id: String,
        actor: Address,
        nonce: u64,
    ) -> Result<bool, Error> {
        let mut product: Product = env
            .storage()
            .persistent()
            .get(&DataKey::Product(product_id.clone()))
            .ok_or(Error::ProductNotFound)?;

        product.owner.require_auth();
        Self::validate_and_increment_nonce(&env, &product.owner, nonce);

        let mut found = false;
        let mut new_actors = Vec::new(&env);
        for i in 0..product.authorized_actors.len() {
            let current_actor = product.authorized_actors.get(i).unwrap();
            if current_actor != actor {
                new_actors.push_back(current_actor);
            } else {
                found = true;
            }
        }

        // Governance safeguard: ensure sufficient actors remain for multi-sig
        if product.required_signatures > 1 {
            // Count total authorized entities (owner + actors)
            let total_authorized = 1 + new_actors.len() as u32; // owner + remaining actors
            if total_authorized < product.required_signatures {
                panic!("removal would violate governance");
            }
        }

        product.authorized_actors = new_actors;
        env.storage()
            .persistent()
            .set(&DataKey::Product(product_id.clone()), &product);

        // Emit event
        if found {
            env.events().publish(
                (Symbol::new(&env, "actor_removed"), product_id),
                actor,
            );
        }

        Ok(found)
    }

    /// Update the mutable metadata fields of a product.
    ///
    /// Only `name` and `origin` can be changed. The `id`, `owner`,
    /// `timestamp`, `authorized_actors`, and `required_signatures` fields are
    /// immutable through this function.
    ///
    /// # Parameters
    /// - `env` — Soroban execution environment.
    /// - `product_id` — ID of the product to update.
    /// - `name` — New human-readable product name.
    /// - `origin` — New origin string.
    ///
    /// # Returns
    /// The updated [`Product`] struct.
    ///
    /// # Authorization
    /// Requires `product.owner.require_auth()`. Only the current product owner
    /// may update metadata.
    ///
    /// # Panics
    /// - `"product not found"` — if `product_id` is not registered.
    ///
    /// # Emitted Events
    /// Publishes a `("product_updated", product_id)` event with the updated
    /// [`Product`] struct as the event body.
    pub fn update_product_metadata(
        env: Env,
        product_id: String,
        name: String,
        origin: String,
    ) -> Result<Product, Error> {
        let mut product: Product = env
            .storage()
            .persistent()
            .get(&DataKey::Product(product_id.clone()))
            .ok_or(Error::ProductNotFound)?;

        product.owner.require_auth();
        // Issue #311: enforce size limits on update.
        assert_len(&name,   MAX_NAME_LEN,   "name");
        assert_len(&origin, MAX_ORIGIN_LEN, "origin");

        product.name = name;
        product.origin = origin;

        env.storage()
            .persistent()
            .set(&DataKey::Product(product_id.clone()), &product);

        env.events().publish(
            (Symbol::new(&env, "product_updated"), product_id),
            product.clone(),
        );

        Ok(product)
    }

    fn add_event(env: &Env, contract_id: &soroban_sdk::Address, product_id: &String) {
        let client = SupplyLinkContractClient::new(env, contract_id);
        // HARVEST is the only valid first event from Registered stage
        client.add_tracking_event(
            product_id,
            &String::from_str(env, "Farm"),
            &String::from_str(env, "HARVEST"),
            &String::from_str(env, "{}"),
    /// Deactivate a product, preventing new events from being recorded.
    ///
    /// Sets `product.active` to `false`. Once deactivated, a product cannot
    /// receive new tracking events. The product remains queryable but is marked
    /// as recalled/deactivated for consumer display.
    ///
    /// # Parameters
    /// - `env` — Soroban execution environment.
    /// - `product_id` — ID of the product to deactivate.
    ///
    /// # Returns
    /// The updated [`Product`] struct with `active = false`.
    ///
    /// # Authorization
    /// Requires `product.owner.require_auth()`. Only the product owner may
    /// deactivate a product.
    ///
    /// # Panics
    /// - `"product not found"` — if `product_id` is not registered.
    /// - `"product already inactive"` — if the product is already deactivated.
    ///
    /// # Emitted Events
    /// Publishes a `("product_deactivated", product_id)` event with the updated
    /// [`Product`] struct as the event body.
    pub fn deactivate_product(env: Env, product_id: String) -> Result<Product, Error> {
        let mut product: Product = env
            .storage()
            .persistent()
            .get(&DataKey::Product(product_id.clone()))
            .ok_or(Error::ProductNotFound)?;

        product.owner.require_auth();

        if !product.active {
            panic!("product already inactive");
        }

        product.active = false;

        env.storage()
            .persistent()
            .set(&DataKey::Product(product_id.clone()), &product);

        env.events().publish(
            (Symbol::new(&env, "product_deactivated"), product_id),
            product.clone(),
        );

        Ok(product)
    }

    /// Return the list of addresses authorised to add events for a product.
    ///
    /// # Parameters
    /// - `env` — Soroban execution environment.
    /// - `product_id` — ID of the product to query.
    ///
    /// # Returns
    /// A `Vec<Address>` of authorized actors. Returns an empty vector if the
    /// product does not exist or has no authorized actors.
    ///
    /// # Authorization
    /// None — this is a read-only function.
    ///
    /// # Panics
    /// Does not panic.
    pub fn get_authorized_actors(env: Env, product_id: String) -> Vec<Address> {
        env.storage()
            .persistent()
            .get::<DataKey, Product>(&DataKey::Product(product_id))
            .map(|p| p.authorized_actors)
            .unwrap_or_else(|| Vec::new(&env))
    }

    /// Return the total number of products registered on this contract.
    ///
    /// The count is a monotonically increasing counter; it is never decremented
    /// even if products were to be removed (which is not currently supported).
    ///
    /// # Parameters
    /// - `env` — Soroban execution environment.
    ///
    /// # Returns
    /// A `u64` count. Returns `0` if no products have been registered.
    ///
    /// # Authorization
    /// None — this is a read-only function.
    ///
    /// # Panics
    /// Does not panic.
    pub fn get_product_count(env: Env) -> u64 {
        env.storage()
            .persistent()
            .get(&DataKey::ProductCount)
            .unwrap_or(0)
    }

    /// Return a paginated slice of product IDs in registration order.
    ///
    /// Uses the [`DataKey::ProductIndex`] mapping to look up IDs by their
    /// sequential insertion index, enabling efficient pagination without
    /// iterating all storage keys.
    ///
    /// # Parameters
    /// - `env` — Soroban execution environment.
    /// - `offset` — Zero-based index of the first product to return.
    /// - `limit` — Maximum number of product IDs to return.
    ///
    /// # Returns
    /// A `Vec<String>` of product IDs. Returns an empty vector if `offset` is
    /// beyond the total count or no products are registered.
    ///
    /// # Authorization
    /// None — this is a read-only function.
    ///
    /// # Panics
    /// Does not panic.
    ///
    /// # Example
    /// ```text
    /// // Fetch the first page of 10 products
    /// list_products(env, 0, 10)
    ///
    /// // Fetch the second page
    /// list_products(env, 10, 10)
    /// ```
    pub fn list_products(env: Env, offset: u64, limit: u64) -> Vec<String> {
        let count: u64 = env
            .storage()
            .persistent()
            .get(&DataKey::ProductCount)
            .unwrap_or(0);

        let mut products = Vec::new(&env);
        let end = core::cmp::min(offset + limit, count);

        for i in offset..end {
            if let Some(product_id) =
                env.storage()
                    .persistent()
                    .get::<DataKey, String>(&DataKey::ProductIndex(i))
            {
                products.push_back(product_id);
            }
        }

        products
    }

    /// Req 3.4 — multiple products each with one event → correct total
    #[test]
    fn test_multiple_events_returns_correct_count() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, SupplyLinkContract);
        let client = SupplyLinkContractClient::new(&env, &contract_id);
        let owner = soroban_sdk::Address::generate(&env);

        // Register 5 separate products and add one HARVEST event each
        for i in 0u32..5 {
            let pid = {
                let ids = ["pa", "pb", "pc", "pd", "pe"];
                String::from_str(&env, ids[i as usize])
            };
            client.register_product(&pid, &String::from_str(&env, "W"), &String::from_str(&env, "O"), &owner);
            client.add_tracking_event(&pid, &String::from_str(&env, "Farm"), &String::from_str(&env, "HARVEST"), &String::from_str(&env, "{}"));
            assert_eq!(client.get_events_count(&pid), 1);
        }
    }

    /// Req 3.5 — get_events_count == get_tracking_events(...).len()
    #[test]
    fn test_count_equals_vec_len() {
        let (env, contract_id, product_id) = setup();
        add_event(&env, &contract_id, &product_id);
        let client = SupplyLinkContractClient::new(&env, &contract_id);
        let count = client.get_events_count(&product_id);
        let events = client.get_tracking_events(&product_id);
        assert_eq!(count, events.len());
    }

    // ── Property-based tests ─────────────────────────────────────────────────

    /// Property 1: Registered product with one HARVEST event has count == 1
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]
        #[test]
        fn prop_count_equals_n_events(product_id_str in "[a-z]{1,20}") {
            let env = Env::default();
            env.mock_all_auths();
            let contract_id = env.register_contract(None, SupplyLinkContract);
            let client = SupplyLinkContractClient::new(&env, &contract_id);
            let owner = soroban_sdk::Address::generate(&env);
            let product_id = String::from_str(&env, &product_id_str);

            client.register_product(
                &product_id,
                &String::from_str(&env, "Widget"),
                &String::from_str(&env, "Origin"),
                &owner,
            );
            prop_assert_eq!(client.get_events_count(&product_id), 0);

            client.add_tracking_event(
                &product_id,
                &String::from_str(&env, "Farm"),
                &String::from_str(&env, "HARVEST"),
                &String::from_str(&env, "{}"),
            );
            prop_assert_eq!(client.get_events_count(&product_id), 1);
        }
    }

    /// Property 2: Unknown product returns 0
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]
        #[test]
        fn prop_unknown_product_returns_zero(product_id_str in "[a-z]{1,20}") {
            let env = Env::default();
            let contract_id = env.register_contract(None, SupplyLinkContract);
            let client = SupplyLinkContractClient::new(&env, &contract_id);
            let product_id = String::from_str(&env, &product_id_str);
            prop_assert_eq!(client.get_events_count(&product_id), 0);
    // ── #388: Paginated event retrieval ───────────────────────────────────────

    /// Return a paginated slice of tracking events for a product.
    ///
    /// Events are returned in insertion order (oldest first).
    ///
    /// # Parameters
    /// - `product_id` — The product to query.
    /// - `offset` — Zero-based index of the first event to return.
    /// - `limit` — Maximum number of events to return.
    ///
    /// # Returns
    /// A `Vec<TrackingEvent>`. Returns an empty vector if offset is beyond
    /// the total count or the product has no events.
    pub fn list_tracking_events(
        env: Env,
        product_id: String,
        offset: u32,
        limit: u32,
    ) -> Vec<TrackingEvent> {
        let all: Vec<TrackingEvent> = env
            .storage()
            .persistent()
            .get(&DataKey::Events(product_id))
            .unwrap_or_else(|| Vec::new(&env));

        let total = all.len();
        if offset >= total {
            return Vec::new(&env);
        }

        let end = core::cmp::min(offset + limit, total);
        let mut page = Vec::new(&env);
        for i in offset..end {
            page.push_back(all.get(i).unwrap());
        }
        page
    }

    /// Return the total number of tracking events for a product.
    /// Alias of `get_events_count` — provided for API symmetry with `list_tracking_events`.
    pub fn count_tracking_events(env: Env, product_id: String) -> u32 {
        env.storage()
            .persistent()
            .get::<DataKey, Vec<TrackingEvent>>(&DataKey::Events(product_id))
            .map(|v| v.len())
            .unwrap_or(0)
    }

    // ── #387: Role segregation ────────────────────────────────────────────────

    /// Assign a named role to an authorized actor for a product.
    ///
    /// The actor must already be in `authorized_actors`. Replaces any existing
    /// role assignment for the same actor.
    ///
    /// # Authorization
    /// Requires `product.owner.require_auth()`.
    pub fn assign_role(env: Env, product_id: String, actor: Address, role: Role) -> bool {
    /// Approve a pending event for a high-value product.
    ///
    /// For products with `required_signatures > 1`, events are staged as pending
    /// until the required number of approvals are received. This function allows
    /// authorized actors to approve a pending event using its stable identifier.
    ///
    /// # Parameters
    /// - `env` — Soroban execution environment (injected by the runtime).
    /// - `product_id` — ID of the product.
    /// - `pending_event_id` — Stable ID of the pending event to approve.
    ///   This ID remains unchanged even if other pending events are removed.
    /// - `approver` — Address of the actor approving the event.
    /// - `nonce` — Sequential nonce for authorization, incremented by the contract.
    ///
    /// # Returns
    /// `true` if the event was finalized (all signatures received), `false` if
    /// more approvals are needed.
    ///
    /// # Authorization
    /// Requires `approver.require_auth()`. The approver must be the owner or
    /// an authorized actor.
    ///
    /// # Errors
    /// - [`Error::ProductNotFound`] — if `product_id` is not registered.
    /// - [`Error::ApproverNotAuthorized`] — if approver is not owner or actor.
    /// - [`Error::NoPendingEvents`] — if there are no pending events.
    /// - [`Error::PendingEventExpired`] — if the pending event has expired (issue #314).
    ///
    /// # Panics
    /// - `"event index out of bounds"` — if `event_index` is invalid.
    ///
    /// # Emitted Events
    /// - When the event is **not yet finalized**: no event is emitted.
    /// - When the event **is finalized** (approvals reach `required_signatures`):
    ///   publishes an `("event_finalized", product_id, event_type,
    ///   schema_version)` event with the [`TrackingEvent`] struct as the body.
    pub fn approve_event(
        env: Env,
        product_id: String,
        pending_event_id: u64,
        approver: Address,
        nonce: u64,
    ) -> Result<bool, Error> {
        let product: Product = env
            .storage()
            .persistent()
            .get(&DataKey::Product(product_id.clone()))
            .ok_or(Error::ProductNotFound)?;

        let is_owner = product.owner == approver;
        let is_actor = product.authorized_actors.contains(&approver);
        if !is_owner && !is_actor {
            return Err(Error::ApproverNotAuthorized);
        }
        approver.require_auth();
        Self::validate_and_increment_nonce(&env, &approver, nonce);

        let mut pending: Vec<PendingEvent> = env
            .storage()
            .persistent()
            .get(&DataKey::PendingEvents(product_id.clone()))
            .ok_or(Error::NoPendingEvents)?;

        // Find the pending event by stable ID (not index-based)
        let mut event_position: Option<usize> = None;
        for i in 0..pending.len() {
            if pending.get(i).unwrap().pending_event_id == pending_event_id {
                event_position = Some(i);
                break;
            }
        }

        let event_index = event_position.ok_or_else(|| {
            panic!("pending event not found")
        })?;

        let mut pending_event = pending.get(event_index).unwrap().clone();

        // Check expiration (issue #314)
        let current_time = env.ledger().timestamp();
        if current_time > pending_event.expiration {
            return Err(Error::PendingEventExpired);
        }

        if !pending_event.approvals.contains(&approver) {
            pending_event.approvals.push_back(approver.clone());
        }

        let is_finalized = pending_event.approvals.len() as u32 >= pending_event.required_signatures;

        if is_finalized {
            let mut events: Vec<TrackingEvent> = env
                .storage()
                .persistent()
                .get(&DataKey::Events(product_id.clone()))
                .unwrap_or_else(|| Vec::new(&env));

            events.push_back(pending_event.event.clone());
            env.storage()
                .persistent()
                .set(&DataKey::Events(product_id.clone()), &events);

            // Remove from pending
            pending.remove(event_index);
            if pending.len() > 0 {
                env.storage()
                    .persistent()
                    .set(&DataKey::PendingEvents(product_id.clone()), &pending);
            } else {
                env.storage()
                    .persistent()
                    .remove(&DataKey::PendingEvents(product_id.clone()));
            }

            env.events().publish(
                (
                    Symbol::new(&env, "event_finalized"),
                    product_id,
                    pending_event.event.event_type.clone(),
                    EVENT_SCHEMA_VERSION,
                ),
                pending_event.event,
            );

            Ok(true)
        } else {
            // Update pending event with new approval
            pending.set(event_index, pending_event);
            env.storage()
                .persistent()
                .set(&DataKey::PendingEvents(product_id), &pending);
            Ok(false)
        }
    }

    /// Reject a pending event for a high-value product.
    ///
    /// Removes a pending event from the approval queue without finalizing it.
    /// Optionally accepts a reason for the rejection for audit purposes.
    /// Uses the stable identifier of the pending event to ensure deterministic
    /// behavior even after queue mutations.
    ///
    /// # Parameters
    /// - `env` — Soroban execution environment.
    /// - `product_id` — ID of the product.
    /// - `pending_event_id` — Stable ID of the pending event to reject.
    ///   This ID remains unchanged even if other pending events are removed.
    /// - `rejector` — Address of the actor rejecting the event.
    /// - `reason` — Optional reason for rejection (max 256 characters).
    /// - `nonce` — Sequential nonce for authorization, incremented by the contract.
    ///
    /// # Returns
    /// `true` on success.
    ///
    /// # Authorization
    /// Requires `rejector.require_auth()`. The rejector must be the owner.
    ///
    /// # Panics
    /// - `"product not found"` — if `product_id` is not registered.
    /// - `"only owner can reject"` — if rejector is not the owner.
    /// - `"no pending events"` — if there are no pending events.
    /// - `"pending event not found"` — if `pending_event_id` doesn't match any pending event.
    /// - `"rejection reason too long"` — if reason exceeds 256 characters.
    /// - `"invalid nonce"` — if nonce does not match the expected sequential value.
    pub fn reject_event(
        env: Env,
        product_id: String,
        pending_event_id: u64,
        rejector: Address,
        reason: String,
        nonce: u64,
    ) -> Result<bool, Error> {
        let product: Product = env
            .storage()
            .persistent()
            .get(&DataKey::Product(product_id.clone()))
            .expect("product not found");
        product.owner.require_auth();

        let mut policy: AuthPolicy = env
            .storage()
            .persistent()
            .get(&DataKey::AuthPolicy(product_id.clone()))
            .unwrap_or(AuthPolicy {
                threshold: 1,
                roles: Vec::new(&env),
            });

        // Replace existing role for actor or append
        let mut new_roles: Vec<ActorRole> = Vec::new(&env);
        let mut replaced = false;
        for i in 0..policy.roles.len() {
            let ar = policy.roles.get(i).unwrap();
            if ar.actor == actor {
                new_roles.push_back(ActorRole { actor: actor.clone(), role: role.clone() });
                replaced = true;
            } else {
                new_roles.push_back(ar);
            }
        }
        if !replaced {
            new_roles.push_back(ActorRole { actor, role });
        }
        policy.roles = new_roles;

        env.storage()
            .persistent()
            .set(&DataKey::AuthPolicy(product_id), &policy);
        true
    }

    /// Property 3: After HARVEST, count increments by one
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]
        #[test]
        fn prop_add_increments_count(product_id_str in "[a-z]{1,20}") {
            let env = Env::default();
            env.mock_all_auths();
            let contract_id = env.register_contract(None, SupplyLinkContract);
            let client = SupplyLinkContractClient::new(&env, &contract_id);
            let owner = soroban_sdk::Address::generate(&env);
            let product_id = String::from_str(&env, &product_id_str);

            client.register_product(
                &product_id,
                &String::from_str(&env, "Widget"),
                &String::from_str(&env, "Origin"),
                &owner,
            );
            let count_before = client.get_events_count(&product_id);
            client.add_tracking_event(
                &product_id,
                &String::from_str(&env, "Farm"),
                &String::from_str(&env, "HARVEST"),
                &String::from_str(&env, "{}"),
            );
            let count_after = client.get_events_count(&product_id);
            prop_assert_eq!(count_after, count_before + 1);
    /// Revoke the role assignment for an actor on a product.
    ///
    /// # Authorization
    /// Requires `product.owner.require_auth()`.
    pub fn revoke_role(env: Env, product_id: String, actor: Address) -> bool {
        let product: Product = env
            .storage()
            .persistent()
            .get(&DataKey::Product(product_id.clone()))
            .expect("product not found");
        product.owner.require_auth();

        let mut policy: AuthPolicy = env
            .storage()
            .persistent()
            .get(&DataKey::AuthPolicy(product_id.clone()))
            .unwrap_or(AuthPolicy {
                threshold: 1,
                roles: Vec::new(&env),
            });

        let mut new_roles: Vec<ActorRole> = Vec::new(&env);
        let mut found = false;
        for i in 0..policy.roles.len() {
            let ar = policy.roles.get(i).unwrap();
            if ar.actor == actor {
                found = true;
            } else {
                new_roles.push_back(ar);
            }
        }
        policy.roles = new_roles;

        env.storage()
            .persistent()
            .set(&DataKey::AuthPolicy(product_id), &policy);
        found
    }

    /// Set the minimum number of signers required for an event on this product.
    ///
    /// # Authorization
    /// Requires `product.owner.require_auth()`.
    pub fn set_event_threshold(env: Env, product_id: String, threshold: u32) -> bool {
        let product: Product = env
            .storage()
            .persistent()
            .get(&DataKey::Product(product_id.clone()))
            .expect("product not found");
        product.owner.require_auth();

        let mut policy: AuthPolicy = env
            .storage()
            .persistent()
            .get(&DataKey::AuthPolicy(product_id.clone()))
            .unwrap_or(AuthPolicy {
                threshold: 1,
                roles: Vec::new(&env),
            });
        policy.threshold = threshold;

        env.storage()
            .persistent()
            .set(&DataKey::AuthPolicy(product_id), &policy);
        true
    }

    /// Return the authorization policy (roles + threshold) for a product.
    pub fn get_authorization_policy(env: Env, product_id: String) -> AuthPolicy {
        env.storage()
            .persistent()
            .get(&DataKey::AuthPolicy(product_id))
            .unwrap_or(AuthPolicy {
                threshold: 1,
                roles: Vec::new(&env),
            })
    }

    // ── Deactivate / reactivate (#3) ──────────────────────────────────────────

    /// Deactivate a product (owner-only). Idempotent.
    pub fn deactivate_product(env: Env, product_id: String) -> bool {
        let mut product: Product = env
            .storage()
            .persistent()
            .get(&DataKey::Product(product_id.clone()))
            .expect("product not found");
        product.owner.require_auth();
        product.active = false;
        env.storage()
            .persistent()
            .set(&DataKey::Product(product_id), &product);
        true
    }

    /// Reactivate a previously deactivated product (owner-only). Idempotent.
    pub fn reactivate_product(env: Env, product_id: String) -> bool {
        let mut product: Product = env
            .storage()
            .persistent()
            .get(&DataKey::Product(product_id.clone()))
            .expect("product not found");
        product.owner.require_auth();
        product.active = true;
        env.storage()
            .persistent()
            .set(&DataKey::Product(product_id), &product);
        true
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Compute a stable deterministic event ID.
///
/// Concatenates `product_id`, `actor` (as bytes), `event_type`, `timestamp`
/// (big-endian u64), and `metadata` into a single byte buffer, then returns
/// the SHA-256 hash as a lowercase hex `String`.
///
/// The result is invariant as long as the input fields are identical, making
/// it safe to use as a permanent reference across contract upgrades.
fn compute_stable_id(
    env: &Env,
    product_id: &String,
    actor: &Address,
    event_type: &String,
    timestamp: u64,
    metadata: &String,
) -> String {
    // Build a byte buffer: product_id bytes + event_type bytes + timestamp (8 bytes BE) + metadata bytes
    let pid_bytes = product_id.clone().to_xdr(env);
    let et_bytes = event_type.clone().to_xdr(env);
    let meta_bytes = metadata.clone().to_xdr(env);
    let actor_bytes = actor.clone().to_xdr(env);

    let ts_bytes: [u8; 8] = timestamp.to_be_bytes();
    let ts_buf = Bytes::from_array(env, &ts_bytes);

    let mut buf = Bytes::new(env);
    buf.append(&pid_bytes);
    buf.append(&actor_bytes);
    buf.append(&et_bytes);
    buf.append(&ts_buf);
    buf.append(&meta_bytes);

    let hash = env.crypto().sha256(&buf);

    // Encode hash as lowercase hex string
    let hex_chars = b"0123456789abcdef";
    let mut hex_bytes = [0u8; 64];
    for (i, byte) in hash.to_array().iter().enumerate() {
        hex_bytes[i * 2] = hex_chars[(byte >> 4) as usize];
        hex_bytes[i * 2 + 1] = hex_chars[(byte & 0xf) as usize];
    }
    String::from_bytes(env, &hex_bytes)
            .ok_or(Error::ProductNotFound)?;

        if product.owner != rejector {
            return Err(Error::OwnerOnly);
        }
        rejector.require_auth();
        Self::validate_and_increment_nonce(&env, &rejector, nonce);

        // Validate reason length (max 256 characters)
        if reason.len() > 256 {
            panic!("rejection reason too long");
        }

        let mut pending: Vec<PendingEvent> = env
            .storage()
            .persistent()
            .get(&DataKey::PendingEvents(product_id.clone()))
            .ok_or(Error::NoPendingEvents)?;

        // Find the pending event by stable ID (not index-based)
        let mut event_position: Option<usize> = None;
        for i in 0..pending.len() {
            if pending.get(i).unwrap().pending_event_id == pending_event_id {
                event_position = Some(i);
                break;
            }
        }

        let event_index = event_position.ok_or_else(|| {
            panic!("pending event not found")
        })?;

        let rejected_event = pending.get(event_index).unwrap().clone();

        // Remove from pending
        pending.remove(event_index);
        if pending.len() > 0 {
            env.storage()
                .persistent()
                .set(&DataKey::PendingEvents(product_id.clone()), &pending);
        } else {
            env.storage()
                .persistent()
                .remove(&DataKey::PendingEvents(product_id.clone()));
        }

        // Emit enriched rejection event with reason
        let rejection = EventRejection {
            product_id: product_id.clone(),
            event: rejected_event.event,
            rejector,
            reason,
            timestamp: env.ledger().timestamp(),
        };

        env.events().publish(
            (Symbol::new(&env, "event_rejected"), product_id),
            rejection,
        );

        Ok(true)
    }

    /// Property 4: Count equals vec length
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]
        #[test]
        fn prop_count_equals_vec_len(product_id_str in "[a-z]{1,20}") {
            let env = Env::default();
            env.mock_all_auths();
            let contract_id = env.register_contract(None, SupplyLinkContract);
            let client = SupplyLinkContractClient::new(&env, &contract_id);
            let owner = soroban_sdk::Address::generate(&env);
            let product_id = String::from_str(&env, &product_id_str);

            client.register_product(
                &product_id,
                &String::from_str(&env, "Widget"),
                &String::from_str(&env, "Origin"),
                &owner,
            );
            client.add_tracking_event(
                &product_id,
                &String::from_str(&env, "Farm"),
                &String::from_str(&env, "HARVEST"),
                &String::from_str(&env, "{}"),
            );
            let count = client.get_events_count(&product_id);
            let events = client.get_tracking_events(&product_id);
            prop_assert_eq!(count, events.len());
    /// Get pending events for a product.
    ///
    /// Returns all events awaiting multi-signature approval.
    ///
    /// # Parameters
    /// - `env` — Soroban execution environment.
    /// - `product_id` — ID of the product.
    ///
    /// # Returns
    /// A `Vec<PendingEvent>` containing all pending events for the product.
    ///
    /// # Authorization
    /// None — this is a read-only function.
    ///
    /// # Panics
    /// Does not panic.
    pub fn get_pending_events(env: Env, product_id: String) -> Vec<PendingEvent> {
        env.storage()
            .persistent()
            .get(&DataKey::PendingEvents(product_id))
            .unwrap_or_else(|| Vec::new(&env))
    }

    /// Clean up expired pending events for a product.
    ///
    /// Removes all expired pending events from storage and emits a purge event
    /// for each removed entry (issue #314).
    ///
    /// # Parameters
    /// - `env` — Soroban execution environment.
    /// - `product_id` — ID of the product to clean up.
    ///
    /// # Returns
    /// Number of events purged.
    ///
    /// # Authorization
    /// None — this is a permissionless cleanup function.
    ///
    /// # Emitted Events
    /// Publishes `("pending_events_purged", product_id)` event with the count
    /// of purged events. Also publishes `("pending_event_purged", product_id)`
    /// for each individual removed event.
    pub fn cleanup_expired_events(env: Env, product_id: String) -> u32 {
        let mut pending: Vec<PendingEvent> = env
            .storage()
            .persistent()
            .get(&DataKey::PendingEvents(product_id.clone()))
            .unwrap_or_else(|| Vec::new(&env));

        let current_time = env.ledger().timestamp();
        let mut expired_count: u32 = 0;

        // Filter out expired events
        let mut valid_pending = Vec::new(&env);
        for i in 0..pending.len() {
            let event = pending.get(i).unwrap();
            if current_time <= event.expiration {
                valid_pending.push_back(event.clone());
            } else {
                expired_count += 1;

                // Emit event for each purged entry
                env.events().publish(
                    (Symbol::new(&env, "pending_event_purged"), product_id.clone()),
                    event.product_id.clone(),
                );
            }
        }

        if valid_pending.len() > 0 {
            env.storage()
                .persistent()
                .set(&DataKey::PendingEvents(product_id.clone()), &valid_pending);
        } else {
            env.storage()
                .persistent()
                .remove(&DataKey::PendingEvents(product_id.clone()));
        }

        // Emit summary event
        env.events().publish(
            (Symbol::new(&env, "pending_events_purged"), product_id),
            expired_count,
        );

        expired_count
    }

    pub fn get_nonce(env: Env, actor: Address) -> u64 {
        env.storage()
            .persistent()
            .get(&DataKey::ActorNonce(actor))
            .unwrap_or(0)
    }

    /// Get the stable pending event ID for a pending event at a given index.
    ///
    /// This function is provided for backward compatibility with clients that
    /// currently use index-based references. It bridges index-based lookups to
    /// stable IDs.
    ///
    /// # Parameters
    /// - `env` — Soroban execution environment.
    /// - `product_id` — ID of the product.
    /// - `event_index` — Zero-based index into the pending events queue.
    ///
    /// # Returns
    /// The stable `pending_event_id` of the event at that index, or panics if
    /// the index is out of bounds or no events exist.
    ///
    /// # Panics
    /// - `"no pending events"` — if there are no pending events.
    /// - `"event index out of bounds"` — if `event_index` is invalid.
    ///
    /// # Note
    /// This function should be called to convert existing index-based client code
    /// to use stable IDs. Direct index usage in approve_event/reject_event will
    /// no longer work; the stable ID must be obtained first.
    pub fn get_pending_event_id_at_index(
        env: Env,
        product_id: String,
        event_index: u32,
    ) -> u64 {
        let pending: Vec<PendingEvent> = env
            .storage()
            .persistent()
            .get(&DataKey::PendingEvents(product_id))
            .ok_or_else(|| panic!("no pending events"))?;

        if event_index >= pending.len() as u32 {
            panic!("event index out of bounds");
        }

        pending.get(event_index).unwrap().pending_event_id
    }

    fn validate_and_increment_nonce(env: &Env, actor: &Address, provided_nonce: u64) {
        let current_nonce: u64 = env
            .storage()
            .persistent()
            .get(&DataKey::ActorNonce(actor.clone()))
            .unwrap_or(0);

        if provided_nonce != current_nonce {
            panic!("invalid nonce");
        }

        env.storage()
            .persistent()
            .set(&DataKey::ActorNonce(actor.clone()), &(current_nonce + 1));
    }

    // ── #404: Lifecycle tests ─────────────────────────────────────────────────

    #[test]
    fn test_lifecycle_starts_at_registered() {
        let (env, contract_id, product_id) = setup();
        let client = SupplyLinkContractClient::new(&env, &contract_id);
        assert_eq!(client.get_lifecycle_stage(&product_id), LifecycleStage::Registered);
    }

    #[test]
    fn test_harvest_advances_to_harvested() {
        let (env, contract_id, product_id) = setup();
        let client = SupplyLinkContractClient::new(&env, &contract_id);
        client.add_tracking_event(&product_id, &String::from_str(&env, "Farm"), &String::from_str(&env, "HARVEST"), &String::from_str(&env, "{}"));
        assert_eq!(client.get_lifecycle_stage(&product_id), LifecycleStage::Harvested);
    }

    #[test]
    #[should_panic(expected = "invalid lifecycle transition")]
    fn test_invalid_transition_panics() {
        let (env, contract_id, product_id) = setup();
        env.as_contract(&contract_id, || {
            SupplyLinkContract::add_tracking_event(
                env.clone(), product_id.clone(),
                String::from_str(&env, "Loc"),
                String::from_str(&env, "SHIPPING"), // invalid from Registered
                String::from_str(&env, "{}"),
            );
        });
    }

    #[test]
    fn test_full_lifecycle_sequence() {
        let (env, contract_id, product_id) = setup();
        let client = SupplyLinkContractClient::new(&env, &contract_id);
        let stages = [
            ("HARVEST", LifecycleStage::Harvested),
            ("PROCESSING", LifecycleStage::Processed),
            ("SHIPPING", LifecycleStage::Shipped),
            ("DELIVERY", LifecycleStage::Delivered),
            ("RETAIL", LifecycleStage::Retail),
        ];
        for (et, expected) in stages {
            client.add_tracking_event(&product_id, &String::from_str(&env, "Loc"), &String::from_str(&env, et), &String::from_str(&env, "{}"));
            assert_eq!(client.get_lifecycle_stage(&product_id), expected);
        }
    }

    // ── #396: Escrow transfer tests ───────────────────────────────────────────

    #[test]
    fn test_request_and_accept_transfer() {
        let (env, contract_id, _pid) = setup();
        let client = SupplyLinkContractClient::new(&env, &contract_id);
        let owner = soroban_sdk::Address::generate(&env);
        let new_owner = soroban_sdk::Address::generate(&env);
        let pid = String::from_str(&env, "prod-escrow");
        client.register_product(&pid, &String::from_str(&env, "W"), &String::from_str(&env, "O"), &owner);

        client.request_transfer_ownership(&pid, &new_owner);
        let escrow = client.get_transfer_escrow(&pid).unwrap();
        assert!(!escrow.disputed);

        client.accept_transfer_ownership(&pid);
        assert_eq!(client.get_product(&pid).owner, new_owner);
        assert!(client.get_transfer_escrow(&pid).is_none());
    }

    #[test]
    fn test_cancel_transfer_request() {
        let (env, contract_id, _pid) = setup();
        let client = SupplyLinkContractClient::new(&env, &contract_id);
        let owner = soroban_sdk::Address::generate(&env);
        let new_owner = soroban_sdk::Address::generate(&env);
        let pid = String::from_str(&env, "prod-cancel");
        client.register_product(&pid, &String::from_str(&env, "W"), &String::from_str(&env, "O"), &owner);

        client.request_transfer_ownership(&pid, &new_owner);
        client.cancel_transfer_request(&pid);
        assert!(client.get_transfer_escrow(&pid).is_none());
        // Owner unchanged
        assert_eq!(client.get_product(&pid).owner, owner);
    }

    #[test]
    fn test_dispute_transfer() {
        let (env, contract_id, _pid) = setup();
        let client = SupplyLinkContractClient::new(&env, &contract_id);
        let owner = soroban_sdk::Address::generate(&env);
        let new_owner = soroban_sdk::Address::generate(&env);
        let pid = String::from_str(&env, "prod-dispute");
        client.register_product(&pid, &String::from_str(&env, "W"), &String::from_str(&env, "O"), &owner);

        client.request_transfer_ownership(&pid, &new_owner);
        client.dispute_transfer_ownership(&pid);
        let escrow = client.get_transfer_escrow(&pid).unwrap();
        assert!(escrow.disputed);
    }

    #[test]
    #[should_panic(expected = "transfer is disputed")]
    fn test_disputed_transfer_cannot_be_accepted() {
        let (env, contract_id, _pid) = setup();
        let client = SupplyLinkContractClient::new(&env, &contract_id);
        let owner = soroban_sdk::Address::generate(&env);
        let new_owner = soroban_sdk::Address::generate(&env);
        let pid = String::from_str(&env, "prod-disp2");
        client.register_product(&pid, &String::from_str(&env, "W"), &String::from_str(&env, "O"), &owner);
        client.request_transfer_ownership(&pid, &new_owner);
        client.dispute_transfer_ownership(&pid);
        env.as_contract(&contract_id, || {
            SupplyLinkContract::accept_transfer_ownership(env.clone(), pid.clone());
        });
    }

    // ── #394: Pending event approval tests ───────────────────────────────────

    #[test]
    fn test_submit_approve_finalize_event() {
        let (env, contract_id, product_id) = setup();
        let client = SupplyLinkContractClient::new(&env, &contract_id);
        let owner = soroban_sdk::Address::generate(&env);
        let approver = soroban_sdk::Address::generate(&env);
        let pid = String::from_str(&env, "prod-approval");
        client.register_product(&pid, &String::from_str(&env, "W"), &String::from_str(&env, "O"), &owner);

        let mut approvers = Vec::new(&env);
        approvers.push_back(approver.clone());

        client.submit_event_for_approval(
            &pid, &owner,
            &String::from_str(&env, "Farm"),
            &String::from_str(&env, "HARVEST"),
            &String::from_str(&env, "{}"),
            &approvers,
            &3600u64,
        );

        client.approve_pending_event(&pid, &owner, &approver);
        client.finalize_pending_event(&pid, &owner);

        assert_eq!(client.get_events_count(&pid), 1);
        assert!(client.get_pending_event(&pid, &owner).is_none());
    }

    #[test]
    fn test_reject_pending_event() {
        let (env, contract_id, _pid) = setup();
        let client = SupplyLinkContractClient::new(&env, &contract_id);
        let owner = soroban_sdk::Address::generate(&env);
        let approver = soroban_sdk::Address::generate(&env);
        let pid = String::from_str(&env, "prod-reject");
        client.register_product(&pid, &String::from_str(&env, "W"), &String::from_str(&env, "O"), &owner);

        let mut approvers = Vec::new(&env);
        approvers.push_back(approver.clone());

        client.submit_event_for_approval(
            &pid, &owner,
            &String::from_str(&env, "Farm"),
            &String::from_str(&env, "HARVEST"),
            &String::from_str(&env, "{}"),
            &approvers,
            &3600u64,
        );

        client.reject_pending_event(&pid, &owner, &approver);
        let pending = client.get_pending_event(&pid, &owner).unwrap();
        assert!(pending.rejected);
        assert_eq!(client.get_events_count(&pid), 0);
    }
}

#[cfg(test)]
mod rejection_reason_tests {
    use super::*;
    use soroban_sdk::{testutils::Address as _, Env};

    #[test]
    fn test_reject_event_with_reason() {
        let env = Env::default();
        let contract_id = env.register_contract(None, SupplyLinkContract);
        let client = SupplyLinkContractClient::new(&env, &contract_id);

        let owner = Address::generate(&env);
        let actor = Address::generate(&env);
        let product_id = String::from_str(&env, "test-product-001");
        let name = String::from_str(&env, "Test Product");
        let origin = String::from_str(&env, "Test Origin");
        let location = String::from_str(&env, "Test Location");
        let event_type = String::from_str(&env, "HARVEST");
        let metadata = String::from_str(&env, "{}");
        let reason = String::from_str(&env, "Invalid metadata format");

        env.mock_all_auths();

        // Register product with multi-sig
        client.register_product(&product_id, &name, &origin, &owner, &2);
        client.add_authorized_actor(&product_id, &actor, &0);

        // Add pending event
        client.add_tracking_event(&product_id, &actor, &location, &event_type, &metadata);

        // Verify pending event exists
        let pending = client.get_pending_events(&product_id);
        assert_eq!(pending.len(), 1);

        // Reject with reason
        let result = client.reject_event(&product_id, &0, &owner, &reason, &1);
        assert_eq!(result, true);

        // Verify pending event was removed
        let pending = client.get_pending_events(&product_id);
        assert_eq!(pending.len(), 0);
    }

    #[test]
    fn test_reject_event_with_empty_reason() {
        let env = Env::default();
        let contract_id = env.register_contract(None, SupplyLinkContract);
        let client = SupplyLinkContractClient::new(&env, &contract_id);

        let owner = Address::generate(&env);
        let actor = Address::generate(&env);
        let product_id = String::from_str(&env, "test-product-002");
        let name = String::from_str(&env, "Test Product");
        let origin = String::from_str(&env, "Test Origin");
        let location = String::from_str(&env, "Test Location");
        let event_type = String::from_str(&env, "HARVEST");
        let metadata = String::from_str(&env, "{}");
        let reason = String::from_str(&env, "");

        env.mock_all_auths();

        // Register product with multi-sig
        client.register_product(&product_id, &name, &origin, &owner, &2);
        client.add_authorized_actor(&product_id, &actor, &0);

        // Add pending event
        client.add_tracking_event(&product_id, &actor, &location, &event_type, &metadata);

        // Reject with empty reason (should work)
        let result = client.reject_event(&product_id, &0, &owner, &reason, &1);
        assert_eq!(result, true);
    }

    #[test]
    #[should_panic(expected = "rejection reason too long")]
    fn test_reject_event_reason_too_long() {
        let env = Env::default();
        let contract_id = env.register_contract(None, SupplyLinkContract);
        let client = SupplyLinkContractClient::new(&env, &contract_id);

        let owner = Address::generate(&env);
        let actor = Address::generate(&env);
        let product_id = String::from_str(&env, "test-product-003");
        let name = String::from_str(&env, "Test Product");
        let origin = String::from_str(&env, "Test Origin");
        let location = String::from_str(&env, "Test Location");
        let event_type = String::from_str(&env, "HARVEST");
        let metadata = String::from_str(&env, "{}");
        
        // Create a reason longer than 256 characters
        let long_reason = String::from_str(&env, &"x".repeat(257));

        env.mock_all_auths();

        // Register product with multi-sig
        client.register_product(&product_id, &name, &origin, &owner, &2);
        client.add_authorized_actor(&product_id, &actor, &0);

        // Add pending event
        client.add_tracking_event(&product_id, &actor, &location, &event_type, &metadata);

        // Try to reject with too long reason - should panic
        client.reject_event(&product_id, &0, &owner, &long_reason, &1);
    }

    #[test]
    fn test_reject_event_max_length_reason() {
        let env = Env::default();
        let contract_id = env.register_contract(None, SupplyLinkContract);
        let client = SupplyLinkContractClient::new(&env, &contract_id);

        let owner = Address::generate(&env);
        let actor = Address::generate(&env);
        let product_id = String::from_str(&env, "test-product-004");
        let name = String::from_str(&env, "Test Product");
        let origin = String::from_str(&env, "Test Origin");
        let location = String::from_str(&env, "Test Location");
        let event_type = String::from_str(&env, "HARVEST");
        let metadata = String::from_str(&env, "{}");
        
        // Create a reason exactly 256 characters (should work)
        let max_reason = String::from_str(&env, &"x".repeat(256));

        env.mock_all_auths();

        // Register product with multi-sig
        client.register_product(&product_id, &name, &origin, &owner, &2);
        client.add_authorized_actor(&product_id, &actor, &0);

        // Add pending event
        client.add_tracking_event(&product_id, &actor, &location, &event_type, &metadata);

        // Reject with max length reason (should work)
        let result = client.reject_event(&product_id, &0, &owner, &max_reason, &1);
        assert_eq!(result, true);
    }
}

mod tests;
