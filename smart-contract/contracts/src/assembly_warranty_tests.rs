/// assembly_warranty_tests.rs
///
/// Tests for product assembly relationships and warranty lifecycle.

#![cfg(test)]
use soroban_sdk::{testutils::Address as _, Address, Env, String, Vec};

use crate::{
    assembly_warranty::{ClaimStatus, ProductAssembly, WarrantyClaim, WarrantyInfo},
    SupplyLinkContract, SupplyLinkContractClient,
};

// ── Helpers ───────────────────────────────────────────────────────────────────

fn setup() -> (Env, Address, SupplyLinkContractClient<'static>) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, SupplyLinkContract);
    let owner = Address::generate(&env);
    let client = SupplyLinkContractClient::new(&env, &contract_id);
    (env, owner, client)
}

fn s(env: &Env, val: &str) -> String {
    String::from_str(env, val)
}

fn register(client: &SupplyLinkContractClient, env: &Env, id: &str, owner: &Address) {
    client.register_product(
        &s(env, id),
        &s(env, &format!("Product {}", id)),
        &s(env, "Origin"),
        owner,
        &0u32,
        &s(env, "other"),
        &s(env, "general"),
    );
}

fn add_harvest_event(client: &SupplyLinkContractClient, env: &Env, product_id: &str, owner: &Address) {
    client.add_tracking_event(
        &s(env, product_id),
        owner,
        &s(env, "Farm"),
        &s(env, "HARVEST"),
        &s(env, "{}"),
    );
}

// ── Assembly tests ────────────────────────────────────────────────────────────

#[test]
fn test_register_assembly_success() {
    let (env, owner, client) = setup();

    // Register parent and two components
    register(&client, &env, "parent-001", &owner);
    register(&client, &env, "comp-001", &owner);
    register(&client, &env, "comp-002", &owner);

    let mut components = Vec::new(&env);
    components.push_back(s(&env, "comp-001"));
    components.push_back(s(&env, "comp-002"));

    let assembly = client
        .register_assembly(
            &s(&env, "parent-001"),
            &components,
            &s(&env, "Assembled from two components"),
            &owner,
        )
        .unwrap();

    assert_eq!(assembly.parent_id, s(&env, "parent-001"));
    assert_eq!(assembly.component_ids.len(), 2);
    assert_eq!(assembly.description, s(&env, "Assembled from two components"));
}

#[test]
fn test_get_assembly_returns_none_when_not_registered() {
    let (env, owner, client) = setup();
    register(&client, &env, "solo-001", &owner);

    let result = client.get_assembly(&s(&env, "solo-001"));
    assert!(result.is_none());
}

#[test]
fn test_get_assembly_returns_registered_assembly() {
    let (env, owner, client) = setup();

    register(&client, &env, "parent-002", &owner);
    register(&client, &env, "comp-003", &owner);

    let mut components = Vec::new(&env);
    components.push_back(s(&env, "comp-003"));

    client
        .register_assembly(
            &s(&env, "parent-002"),
            &components,
            &s(&env, "Single component assembly"),
            &owner,
        )
        .unwrap();

    let assembly = client.get_assembly(&s(&env, "parent-002")).unwrap();
    assert_eq!(assembly.component_ids.len(), 1);
    assert_eq!(assembly.component_ids.get(0).unwrap(), s(&env, "comp-003"));
}

#[test]
fn test_register_assembly_replaces_existing() {
    let (env, owner, client) = setup();

    register(&client, &env, "parent-003", &owner);
    register(&client, &env, "comp-a", &owner);
    register(&client, &env, "comp-b", &owner);
    register(&client, &env, "comp-c", &owner);

    let mut v1 = Vec::new(&env);
    v1.push_back(s(&env, "comp-a"));
    client
        .register_assembly(&s(&env, "parent-003"), &v1, &s(&env, "v1"), &owner)
        .unwrap();

    let mut v2 = Vec::new(&env);
    v2.push_back(s(&env, "comp-b"));
    v2.push_back(s(&env, "comp-c"));
    client
        .register_assembly(&s(&env, "parent-003"), &v2, &s(&env, "v2"), &owner)
        .unwrap();

    let assembly = client.get_assembly(&s(&env, "parent-003")).unwrap();
    assert_eq!(assembly.component_ids.len(), 2);
    assert_eq!(assembly.description, s(&env, "v2"));
}

#[test]
fn test_get_parents_of_component() {
    let (env, owner, client) = setup();

    register(&client, &env, "parent-p1", &owner);
    register(&client, &env, "parent-p2", &owner);
    register(&client, &env, "shared-comp", &owner);

    let mut c1 = Vec::new(&env);
    c1.push_back(s(&env, "shared-comp"));
    client
        .register_assembly(&s(&env, "parent-p1"), &c1, &s(&env, ""), &owner)
        .unwrap();

    let mut c2 = Vec::new(&env);
    c2.push_back(s(&env, "shared-comp"));
    client
        .register_assembly(&s(&env, "parent-p2"), &c2, &s(&env, ""), &owner)
        .unwrap();

    let mut candidates = Vec::new(&env);
    candidates.push_back(s(&env, "parent-p1"));
    candidates.push_back(s(&env, "parent-p2"));

    let parents = client.get_parents_of_component(&s(&env, "shared-comp"), &candidates);
    assert_eq!(parents.len(), 2);
}

#[test]
fn test_get_parents_of_component_returns_empty_when_no_match() {
    let (env, owner, client) = setup();

    register(&client, &env, "parent-x", &owner);
    register(&client, &env, "comp-x", &owner);
    register(&client, &env, "unrelated", &owner);

    let mut c = Vec::new(&env);
    c.push_back(s(&env, "comp-x"));
    client
        .register_assembly(&s(&env, "parent-x"), &c, &s(&env, ""), &owner)
        .unwrap();

    let mut candidates = Vec::new(&env);
    candidates.push_back(s(&env, "parent-x"));

    let parents = client.get_parents_of_component(&s(&env, "unrelated"), &candidates);
    assert_eq!(parents.len(), 0);
}

#[test]
#[should_panic(expected = "assembly exceeds maximum component count")]
fn test_register_assembly_exceeds_max_components() {
    let (env, owner, client) = setup();
    register(&client, &env, "big-parent", &owner);

    // Build a Vec of 51 IDs — count check fires before product-existence check
    let mut components = Vec::new(&env);
    for i in 0..51u32 {
        // Use a simple pattern; these don't need to be registered products
        // because the count guard panics first
        let id_str = match i {
            0 => "x00", 1 => "x01", 2 => "x02", 3 => "x03", 4 => "x04",
            5 => "x05", 6 => "x06", 7 => "x07", 8 => "x08", 9 => "x09",
            10 => "x10", 11 => "x11", 12 => "x12", 13 => "x13", 14 => "x14",
            15 => "x15", 16 => "x16", 17 => "x17", 18 => "x18", 19 => "x19",
            20 => "x20", 21 => "x21", 22 => "x22", 23 => "x23", 24 => "x24",
            25 => "x25", 26 => "x26", 27 => "x27", 28 => "x28", 29 => "x29",
            30 => "x30", 31 => "x31", 32 => "x32", 33 => "x33", 34 => "x34",
            35 => "x35", 36 => "x36", 37 => "x37", 38 => "x38", 39 => "x39",
            40 => "x40", 41 => "x41", 42 => "x42", 43 => "x43", 44 => "x44",
            45 => "x45", 46 => "x46", 47 => "x47", 48 => "x48", 49 => "x49",
            _ => "x50",
        };
        components.push_back(s(&env, id_str));
    }

    client
        .register_assembly(&s(&env, "big-parent"), &components, &s(&env, ""), &owner)
        .unwrap();
}

#[test]
fn test_register_assembly_unauthorized() {
    let (env, owner, client) = setup();
    let stranger = Address::generate(&env);

    register(&client, &env, "parent-auth", &owner);
    register(&client, &env, "comp-auth", &owner);

    let mut components = Vec::new(&env);
    components.push_back(s(&env, "comp-auth"));

    let result = client.register_assembly(
        &s(&env, "parent-auth"),
        &components,
        &s(&env, ""),
        &stranger,
    );
    assert!(result.is_err());
}

// ── Warranty tests ────────────────────────────────────────────────────────────

#[test]
fn test_register_warranty_success() {
    let (env, owner, client) = setup();
    register(&client, &env, "warr-001", &owner);

    let warranty = client
        .register_warranty(
            &s(&env, "warr-001"),
            &(365 * 24 * 3600u64), // 1 year
            &s(&env, "Standard 1-year warranty"),
            &s(&env, "ipfs://QmWarrantyDoc"),
            &owner,
        )
        .unwrap();

    assert_eq!(warranty.product_id, s(&env, "warr-001"));
    assert_eq!(warranty.duration_seconds, 365 * 24 * 3600u64);
    assert!(!warranty.voided);
    assert_eq!(warranty.voided_at, 0);
}

#[test]
fn test_get_warranty_returns_none_when_not_registered() {
    let (env, owner, client) = setup();
    register(&client, &env, "no-warr", &owner);

    let result = client.get_warranty(&s(&env, "no-warr"));
    assert!(result.is_none());
}

#[test]
fn test_get_warranty_returns_registered_warranty() {
    let (env, owner, client) = setup();
    register(&client, &env, "warr-002", &owner);

    client
        .register_warranty(
            &s(&env, "warr-002"),
            &0u64, // lifetime
            &s(&env, "Lifetime warranty"),
            &s(&env, ""),
            &owner,
        )
        .unwrap();

    let warranty = client.get_warranty(&s(&env, "warr-002")).unwrap();
    assert_eq!(warranty.duration_seconds, 0);
    assert_eq!(warranty.terms, s(&env, "Lifetime warranty"));
}

#[test]
fn test_is_warranty_active_returns_true_for_lifetime() {
    let (env, owner, client) = setup();
    register(&client, &env, "warr-lifetime", &owner);

    client
        .register_warranty(
            &s(&env, "warr-lifetime"),
            &0u64,
            &s(&env, "Lifetime"),
            &s(&env, ""),
            &owner,
        )
        .unwrap();

    assert!(client.is_warranty_active(&s(&env, "warr-lifetime")));
}

#[test]
fn test_is_warranty_active_returns_false_when_no_warranty() {
    let (env, owner, client) = setup();
    register(&client, &env, "no-warr-2", &owner);

    assert!(!client.is_warranty_active(&s(&env, "no-warr-2")));
}

#[test]
fn test_void_warranty_success() {
    let (env, owner, client) = setup();
    register(&client, &env, "warr-void", &owner);

    client
        .register_warranty(
            &s(&env, "warr-void"),
            &(365 * 24 * 3600u64),
            &s(&env, "1 year"),
            &s(&env, ""),
            &owner,
        )
        .unwrap();

    let voided = client.void_warranty(&s(&env, "warr-void"), &owner).unwrap();
    assert!(voided.voided);
    assert!(voided.voided_at > 0);
}

#[test]
fn test_is_warranty_active_returns_false_after_void() {
    let (env, owner, client) = setup();
    register(&client, &env, "warr-void-2", &owner);

    client
        .register_warranty(
            &s(&env, "warr-void-2"),
            &0u64,
            &s(&env, "Lifetime"),
            &s(&env, ""),
            &owner,
        )
        .unwrap();

    client.void_warranty(&s(&env, "warr-void-2"), &owner).unwrap();

    assert!(!client.is_warranty_active(&s(&env, "warr-void-2")));
}

#[test]
fn test_void_warranty_unauthorized() {
    let (env, owner, client) = setup();
    let stranger = Address::generate(&env);
    register(&client, &env, "warr-unauth", &owner);

    client
        .register_warranty(
            &s(&env, "warr-unauth"),
            &0u64,
            &s(&env, "Lifetime"),
            &s(&env, ""),
            &owner,
        )
        .unwrap();

    let result = client.void_warranty(&s(&env, "warr-unauth"), &stranger);
    assert!(result.is_err());
}

// ── Warranty claim tests ──────────────────────────────────────────────────────

#[test]
fn test_file_warranty_claim_success() {
    let (env, owner, client) = setup();
    let claimant = Address::generate(&env);
    register(&client, &env, "claim-prod", &owner);

    client
        .register_warranty(
            &s(&env, "claim-prod"),
            &0u64,
            &s(&env, "Lifetime"),
            &s(&env, ""),
            &owner,
        )
        .unwrap();

    let claim = client
        .file_warranty_claim(
            &s(&env, "claim-prod"),
            &s(&env, "claim-001"),
            &s(&env, "Product stopped working after 3 months"),
            &s(&env, "ipfs://QmProofDoc"),
            &claimant,
        )
        .unwrap();

    assert_eq!(claim.claim_id, s(&env, "claim-001"));
    assert_eq!(claim.product_id, s(&env, "claim-prod"));
    assert_eq!(claim.status, ClaimStatus::Pending);
    assert!(claim.filed_at > 0);
}

#[test]
fn test_list_warranty_claims_returns_all_claims() {
    let (env, owner, client) = setup();
    let claimant = Address::generate(&env);
    register(&client, &env, "multi-claim", &owner);

    client
        .register_warranty(
            &s(&env, "multi-claim"),
            &0u64,
            &s(&env, "Lifetime"),
            &s(&env, ""),
            &owner,
        )
        .unwrap();

    client
        .file_warranty_claim(
            &s(&env, "multi-claim"),
            &s(&env, "c-001"),
            &s(&env, "Issue 1"),
            &s(&env, ""),
            &claimant,
        )
        .unwrap();

    client
        .file_warranty_claim(
            &s(&env, "multi-claim"),
            &s(&env, "c-002"),
            &s(&env, "Issue 2"),
            &s(&env, ""),
            &claimant,
        )
        .unwrap();

    let claims = client.list_warranty_claims(&s(&env, "multi-claim"));
    assert_eq!(claims.len(), 2);
}

#[test]
fn test_update_claim_status_to_approved() {
    let (env, owner, client) = setup();
    let claimant = Address::generate(&env);
    register(&client, &env, "status-prod", &owner);

    client
        .register_warranty(
            &s(&env, "status-prod"),
            &0u64,
            &s(&env, "Lifetime"),
            &s(&env, ""),
            &owner,
        )
        .unwrap();

    client
        .file_warranty_claim(
            &s(&env, "status-prod"),
            &s(&env, "claim-upd"),
            &s(&env, "Defective unit"),
            &s(&env, ""),
            &claimant,
        )
        .unwrap();

    let updated = client
        .update_claim_status(
            &s(&env, "status-prod"),
            &s(&env, "claim-upd"),
            &ClaimStatus::Approved,
            &owner,
        )
        .unwrap();

    assert_eq!(updated.status, ClaimStatus::Approved);
}

#[test]
fn test_update_claim_status_to_resolved() {
    let (env, owner, client) = setup();
    let claimant = Address::generate(&env);
    register(&client, &env, "resolve-prod", &owner);

    client
        .register_warranty(
            &s(&env, "resolve-prod"),
            &0u64,
            &s(&env, "Lifetime"),
            &s(&env, ""),
            &owner,
        )
        .unwrap();

    client
        .file_warranty_claim(
            &s(&env, "resolve-prod"),
            &s(&env, "claim-res"),
            &s(&env, "Broken screen"),
            &s(&env, "ipfs://QmEvidence"),
            &claimant,
        )
        .unwrap();

    let resolved = client
        .update_claim_status(
            &s(&env, "resolve-prod"),
            &s(&env, "claim-res"),
            &ClaimStatus::Resolved,
            &owner,
        )
        .unwrap();

    assert_eq!(resolved.status, ClaimStatus::Resolved);
    assert!(resolved.updated_at > 0);
}

#[test]
fn test_update_claim_status_unauthorized() {
    let (env, owner, client) = setup();
    let claimant = Address::generate(&env);
    let stranger = Address::generate(&env);
    register(&client, &env, "auth-claim-prod", &owner);

    client
        .register_warranty(
            &s(&env, "auth-claim-prod"),
            &0u64,
            &s(&env, "Lifetime"),
            &s(&env, ""),
            &owner,
        )
        .unwrap();

    client
        .file_warranty_claim(
            &s(&env, "auth-claim-prod"),
            &s(&env, "claim-auth"),
            &s(&env, "Issue"),
            &s(&env, ""),
            &claimant,
        )
        .unwrap();

    let result = client.update_claim_status(
        &s(&env, "auth-claim-prod"),
        &s(&env, "claim-auth"),
        &ClaimStatus::Rejected,
        &stranger,
    );
    assert!(result.is_err());
}

#[test]
#[should_panic(expected = "warranty is voided")]
fn test_file_claim_on_voided_warranty_panics() {
    let (env, owner, client) = setup();
    let claimant = Address::generate(&env);
    register(&client, &env, "voided-claim", &owner);

    client
        .register_warranty(
            &s(&env, "voided-claim"),
            &0u64,
            &s(&env, "Lifetime"),
            &s(&env, ""),
            &owner,
        )
        .unwrap();

    client.void_warranty(&s(&env, "voided-claim"), &owner).unwrap();

    // This should panic
    client
        .file_warranty_claim(
            &s(&env, "voided-claim"),
            &s(&env, "claim-v"),
            &s(&env, "Too late"),
            &s(&env, ""),
            &claimant,
        )
        .unwrap();
}

#[test]
fn test_list_warranty_claims_empty_when_none_filed() {
    let (env, owner, client) = setup();
    register(&client, &env, "empty-claims", &owner);

    client
        .register_warranty(
            &s(&env, "empty-claims"),
            &0u64,
            &s(&env, "Lifetime"),
            &s(&env, ""),
            &owner,
        )
        .unwrap();

    let claims = client.list_warranty_claims(&s(&env, "empty-claims"));
    assert_eq!(claims.len(), 0);
}

// ── Provenance aggregation test ───────────────────────────────────────────────

/// Verify that component products retain their own event histories after
/// being registered as part of an assembly.
#[test]
fn test_component_provenance_preserved_after_assembly() {
    let (env, owner, client) = setup();

    register(&client, &env, "assembled-parent", &owner);
    register(&client, &env, "comp-prov-1", &owner);
    register(&client, &env, "comp-prov-2", &owner);

    // Add events to components
    add_harvest_event(&client, &env, "comp-prov-1", &owner);
    client.add_tracking_event(
        &s(&env, "comp-prov-2"),
        &owner,
        &s(&env, "Factory B"),
        &s(&env, "PROCESSING"),
        &s(&env, "{}"),
    );

    // Register assembly
    let mut components = Vec::new(&env);
    components.push_back(s(&env, "comp-prov-1"));
    components.push_back(s(&env, "comp-prov-2"));

    client
        .register_assembly(
            &s(&env, "assembled-parent"),
            &components,
            &s(&env, "Final assembly"),
            &owner,
        )
        .unwrap();

    // Component events are still accessible
    let events1 = client.get_tracking_events(&s(&env, "comp-prov-1"));
    let events2 = client.get_tracking_events(&s(&env, "comp-prov-2"));
    assert_eq!(events1.len(), 1);
    assert_eq!(events2.len(), 1);
    assert_eq!(events1.get(0).unwrap().event_type, s(&env, "HARVEST"));
    assert_eq!(events2.get(0).unwrap().event_type, s(&env, "PROCESSING"));

    // Assembly record references both components
    let assembly = client.get_assembly(&s(&env, "assembled-parent")).unwrap();
    assert_eq!(assembly.component_ids.len(), 2);
}
