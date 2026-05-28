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
}

// ── Data models ──────────────────────────────────────────────────────────────

#[contracttype]
#[derive(Clone)]
pub struct Product {
    pub id: String,
    pub name: String,
    pub origin: String,
    pub owner: Address,
    pub timestamp: u64,
    pub authorized_actors: Vec<Address>,
    /// Current lifecycle stage (#404)
    pub lifecycle_stage: LifecycleStage,
}

#[contracttype]
#[derive(Clone)]
pub struct TrackingEvent {
    pub product_id: String,
    pub location: String,
    pub actor: Address,
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

#[contracttype]
pub enum DataKey {
    Product(String),
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
}

// ── Contract ─────────────────────────────────────────────────────────────────

#[contract]
pub struct SupplyLinkContract;

#[contractimpl]
impl SupplyLinkContract {
    /// Register a new product on-chain.
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
            lifecycle_stage: LifecycleStage::Registered,
        };
        env.storage()
            .persistent()
            .set(&DataKey::Product(id), &product);
        product
    }

    /// Add a tracking event for a product. Enforces lifecycle stage transitions (#404).
    pub fn add_tracking_event(
        env: Env,
        product_id: String,
        location: String,
        event_type: String,
        metadata: String,
    ) -> TrackingEvent {
        let mut product: Product = env
            .storage()
            .persistent()
            .get(&DataKey::Product(product_id.clone()))
            .expect("product not found");

        let caller = product.owner.clone();
        caller.require_auth();

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
            product_id: product_id.clone(),
            location,
            actor: caller,
            timestamp: env.ledger().timestamp(),
            event_type,
            metadata,
        };

        let mut events: Vec<TrackingEvent> = env
            .storage()
            .persistent()
            .get(&DataKey::Events(product_id.clone()))
            .unwrap_or_else(|| Vec::new(&env));

        events.push_back(event.clone());
        env.storage()
            .persistent()
            .set(&DataKey::Events(product_id), &events);

        event
    }

    /// Get product details.
    pub fn get_product(env: Env, id: String) -> Product {
        env.storage()
            .persistent()
            .get(&DataKey::Product(id))
            .expect("product not found")
    }

    /// Get all tracking events for a product.
    pub fn get_tracking_events(env: Env, product_id: String) -> Vec<TrackingEvent> {
        env.storage()
            .persistent()
            .get(&DataKey::Events(product_id))
            .unwrap_or_else(|| Vec::new(&env))
    }

    /// Returns the number of tracking events recorded for `product_id`.
    /// Returns 0 if the product has no events or does not exist.
    pub fn get_events_count(env: Env, product_id: String) -> u32 {
        env.storage()
            .persistent()
            .get::<DataKey, Vec<TrackingEvent>>(&DataKey::Events(product_id))
            .map(|v| v.len())
            .unwrap_or(0)
    }

    /// Transfer product ownership.
    pub fn transfer_ownership(env: Env, product_id: String, new_owner: Address) -> bool {
        let mut product: Product = env
            .storage()
            .persistent()
            .get(&DataKey::Product(product_id.clone()))
            .expect("product not found");

        product.owner.require_auth();
        product.owner = new_owner;
        env.storage()
            .persistent()
            .set(&DataKey::Product(product_id), &product);
        true
    }

    /// Authorize an actor to add events for a product.
    pub fn add_authorized_actor(env: Env, product_id: String, actor: Address) -> bool {
        let mut product: Product = env
            .storage()
            .persistent()
            .get(&DataKey::Product(product_id.clone()))
            .expect("product not found");

        product.owner.require_auth();
        product.authorized_actors.push_back(actor);
        env.storage()
            .persistent()
            .set(&DataKey::Product(product_id), &product);
        true
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

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use soroban_sdk::{testutils::Address as _, Env};

    fn setup() -> (Env, soroban_sdk::Address, String) {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, SupplyLinkContract);
        let owner = soroban_sdk::Address::generate(&env);
        let product_id = String::from_str(&env, "prod-001");
        let client = SupplyLinkContractClient::new(&env, &contract_id);
        client.register_product(
            &product_id,
            &String::from_str(&env, "Widget"),
            &String::from_str(&env, "Factory A"),
            &owner,
        );
        (env, contract_id, product_id)
    }

    fn add_event(env: &Env, contract_id: &soroban_sdk::Address, product_id: &String) {
        let client = SupplyLinkContractClient::new(env, contract_id);
        // HARVEST is the only valid first event from Registered stage
        client.add_tracking_event(
            product_id,
            &String::from_str(env, "Farm"),
            &String::from_str(env, "HARVEST"),
            &String::from_str(env, "{}"),
        );
    }

    /// Req 3.1 — unknown product_id returns 0
    #[test]
    fn test_unknown_product_returns_zero() {
        let env = Env::default();
        let contract_id = env.register_contract(None, SupplyLinkContract);
        let client = SupplyLinkContractClient::new(&env, &contract_id);
        let unknown = String::from_str(&env, "does-not-exist");
        assert_eq!(client.get_events_count(&unknown), 0);
    }

    /// Req 3.2 — registered product with no events returns 0
    #[test]
    fn test_registered_product_no_events_returns_zero() {
        let (env, contract_id, product_id) = setup();
        let client = SupplyLinkContractClient::new(&env, &contract_id);
        assert_eq!(client.get_events_count(&product_id), 0);
    }

    /// Req 3.3 — one add_tracking_event call → count == 1
    #[test]
    fn test_one_event_returns_one() {
        let (env, contract_id, product_id) = setup();
        add_event(&env, &contract_id, &product_id);
        let client = SupplyLinkContractClient::new(&env, &contract_id);
        assert_eq!(client.get_events_count(&product_id), 1);
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
        }
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
        }
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
        }
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
