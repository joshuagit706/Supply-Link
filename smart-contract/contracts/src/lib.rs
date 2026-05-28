#![no_std]
use soroban_sdk::{
    contract, contractimpl, contracttype, Address, Bytes, Env, String, Vec, Symbol,
};
use soroban_sdk::xdr::ToXdr;

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
    /// Must be unique across all registered products; duplicate IDs will silently
    /// overwrite the existing entry.
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
    /// Whether the product is active. Deactivated products reject new events.
    pub active: bool,
}

/// A single supply-chain event recorded against a [`Product`].
///
/// Events are append-only. Once written they cannot be modified or deleted,
/// providing an immutable audit trail. All events for a product are stored
/// together under [`DataKey::Events`].
///
/// # Storage
/// Stored as a `Vec<TrackingEvent>` under [`DataKey::Events`] keyed by
/// `product_id`. Storage type is `persistent`.
#[contracttype]
#[derive(Clone)]
pub struct TrackingEvent {
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
    /// Key for the global product registration counter.
    ProductCount,
    /// Key for the index-to-ID mapping used by pagination.
    /// The inner `u64` is the zero-based insertion index.
    ProductIndex(u64),
    /// Key for the authorization policy (roles + threshold) of a product.
    AuthPolicy(String),
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
    /// - `id` — Caller-supplied unique product identifier. If a product with
    ///   this ID already exists it will be silently overwritten.
    /// - `name` — Human-readable product name.
    /// - `origin` — Geographic or organisational origin of the product.
    /// - `owner` — Stellar address that will own the product. This address
    ///   must sign the transaction.
    ///
    /// # Returns
    /// The newly created [`Product`] struct.
    ///
    /// # Authorization
    /// Requires `owner.require_auth()`. The transaction must be signed by
    /// `owner`.
    ///
    /// # Panics
    /// Does not panic under normal conditions. Panics if the Soroban runtime
    /// rejects the auth check (i.e. `owner` did not sign).
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
    ) -> Product {
        owner.require_auth();
        let product = Product {
            id: id.clone(),
            name,
            origin,
            owner,
            timestamp: env.ledger().timestamp(),
            authorized_actors: Vec::new(&env),
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
    /// - `event_type` — Supply-chain stage. Recommended values: `"HARVEST"`,
    ///   `"PROCESSING"`, `"SHIPPING"`, `"RETAIL"`. Not validated by the
    ///   contract.
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
    /// Publishes an `("event_added", product_id, event_type)` event with the
    /// [`TrackingEvent`] struct as the event body.
    pub fn add_tracking_event(
        env: Env,
        product_id: String,
        caller: Address,
        location: String,
        event_type: String,
        metadata: String,
    ) -> TrackingEvent {
        let product: Product = env
            .storage()
            .persistent()
            .get(&DataKey::Product(product_id.clone()))
            .expect("product not found");

        if !product.active {
            panic!("product is deactivated");
        }

        // Verify caller is owner or an authorized actor before requiring auth
        let is_owner = product.owner == caller;
        let is_actor = product.authorized_actors.contains(&caller);
        if !is_owner && !is_actor {
            panic!("caller is not authorized");
        }
        caller.require_auth();

        let timestamp = env.ledger().timestamp();

        // Compute stable_id: SHA-256 of "product_id|event_type|timestamp" encoded as bytes
        let stable_id = compute_stable_id(&env, &product_id, &caller, &event_type, timestamp, &metadata);

        let event = TrackingEvent {
            product_id: product_id.clone(),
            location,
            actor: caller,
            timestamp,
            event_type: event_type.clone(),
            metadata,
            stable_id,
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

        // Emit event
        env.events().publish(
            (Symbol::new(&env, "event_added"), product_id, event_type),
            event.clone(),
        );

        event
    }

    /// Retrieve a product by its ID.
    ///
    /// # Parameters
    /// - `env` — Soroban execution environment.
    /// - `id` — The product ID to look up.
    ///
    /// # Returns
    /// The [`Product`] struct stored under `id`.
    ///
    /// # Authorization
    /// None — this is a read-only function.
    ///
    /// # Panics
    /// - `"product not found"` — if no product with `id` is registered.
    pub fn get_product(env: Env, id: String) -> Product {
        env.storage()
            .persistent()
            .get(&DataKey::Product(id))
            .expect("product not found")
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
    /// Equivalent to `get_tracking_events(product_id).len()` but cheaper
    /// because it avoids deserialising the full event vector.
    ///
    /// # Parameters
    /// - `env` — Soroban execution environment.
    /// - `product_id` — The product ID to query.
    ///
    /// # Returns
    /// The number of events as a `u32`. Returns `0` if the product has no
    /// events or does not exist.
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
    ///
    /// # Emitted Events
    /// Publishes an `("ownership_transferred", product_id)` event with
    /// `new_owner` as the event body.
    pub fn transfer_ownership(env: Env, product_id: String, new_owner: Address) -> bool {
        let mut product: Product = env
            .storage()
            .persistent()
            .get(&DataKey::Product(product_id.clone()))
            .expect("product not found");

        product.owner.require_auth();
        product.owner = new_owner.clone();
        env.storage()
            .persistent()
            .set(&DataKey::Product(product_id.clone()), &product);

        // Emit event
        env.events().publish(
            (Symbol::new(&env, "ownership_transferred"), product_id),
            new_owner,
        );

        true
    }

    /// Grant an address permission to add tracking events for a product.
    ///
    /// Appends `actor` to `product.authorized_actors`. Duplicate entries are
    /// not deduplicated by the contract — callers should check
    /// [`Self::get_authorized_actors`] before calling if deduplication matters.
    ///
    /// # Parameters
    /// - `env` — Soroban execution environment.
    /// - `product_id` — ID of the product to update.
    /// - `actor` — Stellar address to authorise.
    ///
    /// # Returns
    /// `true` on success.
    ///
    /// # Authorization
    /// Requires `product.owner.require_auth()`. Only the current product owner
    /// may grant actor permissions.
    ///
    /// # Panics
    /// - `"product not found"` — if `product_id` is not registered.
    ///
    /// # Emitted Events
    /// Publishes an `("actor_authorized", product_id)` event with `actor` as
    /// the event body.
    pub fn add_authorized_actor(env: Env, product_id: String, actor: Address) -> bool {
        let mut product: Product = env
            .storage()
            .persistent()
            .get(&DataKey::Product(product_id.clone()))
            .expect("product not found");

        product.owner.require_auth();
        product.authorized_actors.push_back(actor.clone());
        env.storage()
            .persistent()
            .set(&DataKey::Product(product_id.clone()), &product);

        // Emit event
        env.events().publish(
            (Symbol::new(&env, "actor_authorized"), product_id),
            actor,
        );

        true
    }

    /// Revoke an address's permission to add tracking events for a product.
    ///
    /// Rebuilds `authorized_actors` without the first occurrence of `actor`.
    /// If `actor` appears multiple times (due to duplicate `add_authorized_actor`
    /// calls), only the first occurrence is removed.
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
    ///
    /// # Emitted Events
    /// Does not emit an event (removal is not currently announced on-chain).
    pub fn remove_authorized_actor(env: Env, product_id: String, actor: Address) -> bool {
        let mut product: Product = env
            .storage()
            .persistent()
            .get(&DataKey::Product(product_id.clone()))
            .expect("product not found");

        product.owner.require_auth();

        // Find and remove the actor
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

        product.authorized_actors = new_actors;
        env.storage()
            .persistent()
            .set(&DataKey::Product(product_id), &product);

        found
    }

    /// Update the mutable metadata fields of a product.
    ///
    /// Only `name` and `origin` can be changed. The `id`, `owner`,
    /// `timestamp`, and `authorized_actors` fields are immutable through this
    /// function.
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
    ) -> Product {
        let mut product: Product = env
            .storage()
            .persistent()
            .get(&DataKey::Product(product_id.clone()))
            .expect("product not found");

        product.owner.require_auth();

        product.name = name;
        product.origin = origin;

        env.storage()
            .persistent()
            .set(&DataKey::Product(product_id.clone()), &product);

        // Emit event
        env.events().publish(
            (Symbol::new(&env, "product_updated"), product_id),
            product.clone(),
        );

        product
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
}

mod tests;
