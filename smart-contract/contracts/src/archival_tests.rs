#[cfg(test)]
mod archival_tests {
    use crate::{SupplyLinkContract, SupplyLinkContractClient};
    use soroban_sdk::{testutils::Address as _, Address, Env, String, Vec};

    fn setup_with_event(env: &Env) -> (Address, Address, String, String) {
        env.mock_all_auths();
        let contract_id = env.register_contract(None, SupplyLinkContract);
        let client = SupplyLinkContractClient::new(env, &contract_id);
        let owner = Address::generate(env);
        let product_id = String::from_str(env, "prod-archive-001");

        client.register_product(
            &product_id,
            &String::from_str(env, "Archive Test Product"),
            &String::from_str(env, "Test Origin"),
            &owner,
            &1u32,
            &String::from_str(env, "agricultural"),
            &String::from_str(env, "coffee"),
        );

        let event = client.add_tracking_event(
            &product_id,
            &owner,
            &String::from_str(env, "Warehouse A"),
            &String::from_str(env, "HARVEST"),
            &String::from_str(env, "{}"),
        );

        (contract_id, owner, product_id, event.stable_id)
    }

    #[test]
    fn test_archive_event_removes_from_active_list() {
        let env = Env::default();
        env.mock_all_auths();
        let (contract_id, owner, product_id, stable_id) = setup_with_event(&env);
        let client = SupplyLinkContractClient::new(&env, &contract_id);

        // Active list has 1 event before archival
        let before = client.get_tracking_events(&product_id);
        assert_eq!(before.len(), 1);

        client.archive_tracking_event(
            &product_id,
            &owner,
            &stable_id,
            &String::from_str(&env, "retention policy"),
        );

        // Active list is now empty
        let after = client.get_tracking_events(&product_id);
        assert_eq!(after.len(), 0);
    }

    #[test]
    fn test_archived_event_appears_in_archive_list() {
        let env = Env::default();
        env.mock_all_auths();
        let (contract_id, owner, product_id, stable_id) = setup_with_event(&env);
        let client = SupplyLinkContractClient::new(&env, &contract_id);

        client.archive_tracking_event(
            &product_id,
            &owner,
            &stable_id,
            &String::from_str(&env, "annual retention"),
        );

        let archived = client.list_archived_events(&product_id, &0u32, &0u32);
        assert_eq!(archived.len(), 1);
        assert_eq!(archived.get(0).unwrap().event.stable_id, stable_id);
    }

    #[test]
    fn test_archived_event_preserves_integrity() {
        let env = Env::default();
        env.mock_all_auths();
        let (contract_id, owner, product_id, stable_id) = setup_with_event(&env);
        let client = SupplyLinkContractClient::new(&env, &contract_id);

        // Capture original event data
        let original = client.get_tracking_events(&product_id).get(0).unwrap();

        client.archive_tracking_event(
            &product_id,
            &owner,
            &stable_id,
            &String::from_str(&env, "test"),
        );

        let archived = client.list_archived_events(&product_id, &0u32, &0u32);
        let archived_event = archived.get(0).unwrap();

        // Integrity: stable_id, event_type, actor, timestamp all preserved
        assert_eq!(archived_event.event.stable_id, original.stable_id);
        assert_eq!(archived_event.event.event_type, original.event_type);
        assert_eq!(archived_event.event.actor, original.actor);
        assert_eq!(archived_event.event.timestamp, original.timestamp);
        assert_eq!(archived_event.event.metadata, original.metadata);
    }

    #[test]
    fn test_archive_pagination() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, SupplyLinkContract);
        let client = SupplyLinkContractClient::new(&env, &contract_id);
        let owner = Address::generate(&env);

        // Use 3 separate products so lifecycle state doesn't interfere
        let product_ids = [
            String::from_str(&env, "prod-page-001"),
            String::from_str(&env, "prod-page-002"),
            String::from_str(&env, "prod-page-003"),
        ];
        // Archive product: collects all archived events under one product for pagination test
        let archive_product = String::from_str(&env, "prod-page-archive");

        client.register_product(
            &archive_product,
            &String::from_str(&env, "Archive Pagination Product"),
            &String::from_str(&env, "Origin"),
            &owner,
            &1u32,
            &String::from_str(&env, "agricultural"),
            &String::from_str(&env, "coffee"),
        );

        for pid in product_ids.iter() {
            client.register_product(
                pid,
                &String::from_str(&env, "Temp Product"),
                &String::from_str(&env, "Origin"),
                &owner,
                &1u32,
                &String::from_str(&env, "agricultural"),
                &String::from_str(&env, "coffee"),
            );
            let ev = client.add_tracking_event(
                pid,
                &owner,
                &String::from_str(&env, "Loc"),
                &String::from_str(&env, "HARVEST"),
                &String::from_str(&env, "{}"),
            );
            // Archive into the dedicated archive product's store by archiving from each product
            client.archive_tracking_event(
                pid,
                &owner,
                &ev.stable_id,
                &String::from_str(&env, "test"),
            );
        }

        // Each product has 1 archived event — test pagination on one of them
        // For a single-product pagination test, add 3 events to archive_product
        // using different metadata to get distinct stable_ids
        let ev1 = client.add_tracking_event(
            &archive_product,
            &owner,
            &String::from_str(&env, "Loc A"),
            &String::from_str(&env, "HARVEST"),
            &String::from_str(&env, "{\"n\":1}"),
        );
        client.archive_tracking_event(
            &archive_product,
            &owner,
            &ev1.stable_id,
            &String::from_str(&env, "test"),
        );

        let ev2 = client.add_tracking_event(
            &archive_product,
            &owner,
            &String::from_str(&env, "Loc B"),
            &String::from_str(&env, "PROCESSING"),
            &String::from_str(&env, "{\"n\":2}"),
        );
        client.archive_tracking_event(
            &archive_product,
            &owner,
            &ev2.stable_id,
            &String::from_str(&env, "test"),
        );

        let ev3 = client.add_tracking_event(
            &archive_product,
            &owner,
            &String::from_str(&env, "Loc C"),
            &String::from_str(&env, "SHIPPING"),
            &String::from_str(&env, "{\"n\":3}"),
        );
        client.archive_tracking_event(
            &archive_product,
            &owner,
            &ev3.stable_id,
            &String::from_str(&env, "test"),
        );

        let page1 = client.list_archived_events(&archive_product, &0u32, &2u32);
        let page2 = client.list_archived_events(&archive_product, &2u32, &2u32);
        let all = client.list_archived_events(&archive_product, &0u32, &0u32);

        assert_eq!(page1.len(), 2);
        assert_eq!(page2.len(), 1);
        assert_eq!(all.len(), 3);
    }

    #[test]
    #[should_panic(expected = "event not found")]
    fn test_archive_nonexistent_event_panics() {
        let env = Env::default();
        env.mock_all_auths();
        let (contract_id, owner, product_id, _) = setup_with_event(&env);
        let client = SupplyLinkContractClient::new(&env, &contract_id);

        client.archive_tracking_event(
            &product_id,
            &owner,
            &String::from_str(&env, "nonexistent-stable-id"),
            &String::from_str(&env, "test"),
        );
    }

    #[test]
    #[should_panic(expected = "caller is not authorized")]
    fn test_archive_unauthorized_caller_panics() {
        let env = Env::default();
        env.mock_all_auths();
        let (contract_id, _owner, product_id, stable_id) = setup_with_event(&env);
        let client = SupplyLinkContractClient::new(&env, &contract_id);
        let stranger = Address::generate(&env);

        client.archive_tracking_event(
            &product_id,
            &stranger,
            &stable_id,
            &String::from_str(&env, "test"),
        );
    }
}

#[cfg(test)]
mod certification_registry_tests {
    use crate::{SupplyLinkContract, SupplyLinkContractClient};
    use soroban_sdk::{testutils::Address as _, Address, Env, String, Vec};

    fn setup(env: &Env) -> (Address, Address, String) {
        env.mock_all_auths();
        let contract_id = env.register_contract(None, SupplyLinkContract);
        let client = SupplyLinkContractClient::new(env, &contract_id);
        let owner = Address::generate(env);
        let product_id = String::from_str(env, "prod-cert-reg-001");

        client.register_product(
            &product_id,
            &String::from_str(env, "Cert Registry Product"),
            &String::from_str(env, "Origin"),
            &owner,
            &1u32,
            &String::from_str(env, "agricultural"),
            &String::from_str(env, "coffee"),
        );

        (contract_id, owner, product_id)
    }

    #[test]
    fn test_register_issuer_and_retrieve() {
        let env = Env::default();
        env.mock_all_auths();
        let (contract_id, _owner, _product_id) = setup(&env);
        let client = SupplyLinkContractClient::new(&env, &contract_id);
        let issuer = Address::generate(&env);

        let mut cert_types = Vec::new(&env);
        cert_types.push_back(String::from_str(&env, "organic"));
        cert_types.push_back(String::from_str(&env, "fair_trade"));

        client.register_certification_issuer(
            &issuer,
            &issuer,
            &String::from_str(&env, "Global Cert Body"),
            &cert_types,
        );

        let retrieved = client.get_certification_issuer(&issuer);
        assert_eq!(retrieved.name, String::from_str(&env, "Global Cert Body"));
        assert!(retrieved.active);
        assert_eq!(retrieved.cert_types.len(), 2);
    }

    #[test]
    fn test_issue_registry_record_and_verify() {
        let env = Env::default();
        env.mock_all_auths();
        let (contract_id, _owner, product_id) = setup(&env);
        let client = SupplyLinkContractClient::new(&env, &contract_id);
        let issuer = Address::generate(&env);

        let mut cert_types = Vec::new(&env);
        cert_types.push_back(String::from_str(&env, "organic"));

        client.register_certification_issuer(
            &issuer,
            &issuer,
            &String::from_str(&env, "Organic Cert Authority"),
            &cert_types,
        );

        let record = client.issue_certification_registry_record(
            &product_id,
            &issuer,
            &String::from_str(&env, "rec-001"),
            &String::from_str(&env, "EXT-CERT-12345"),
            &String::from_str(&env, "organic"),
            &String::from_str(&env, "a".repeat(64).as_str()),
        );

        assert_eq!(record.external_cert_id, String::from_str(&env, "EXT-CERT-12345"));
        assert!(!record.revoked);

        let (valid, verified) = client.verify_certification_registry_record(
            &product_id,
            &String::from_str(&env, "rec-001"),
        );
        assert!(valid);
        assert_eq!(verified.id, String::from_str(&env, "rec-001"));
    }

    #[test]
    fn test_revoke_registry_record() {
        let env = Env::default();
        env.mock_all_auths();
        let (contract_id, _owner, product_id) = setup(&env);
        let client = SupplyLinkContractClient::new(&env, &contract_id);
        let issuer = Address::generate(&env);

        let mut cert_types = Vec::new(&env);
        cert_types.push_back(String::from_str(&env, "iso_9001"));

        client.register_certification_issuer(
            &issuer,
            &issuer,
            &String::from_str(&env, "ISO Body"),
            &cert_types,
        );

        client.issue_certification_registry_record(
            &product_id,
            &issuer,
            &String::from_str(&env, "rec-iso-001"),
            &String::from_str(&env, "ISO-9001-2024-XYZ"),
            &String::from_str(&env, "iso_9001"),
            &String::from_str(&env, "b".repeat(64).as_str()),
        );

        client.revoke_certification_registry_record(
            &product_id,
            &issuer,
            &String::from_str(&env, "rec-iso-001"),
        );

        let (valid, record) = client.verify_certification_registry_record(
            &product_id,
            &String::from_str(&env, "rec-iso-001"),
        );
        assert!(!valid);
        assert!(record.revoked);
        assert!(record.revoked_at > 0);
    }

    #[test]
    #[should_panic(expected = "cert_type not supported by issuer")]
    fn test_issue_unsupported_cert_type_panics() {
        let env = Env::default();
        env.mock_all_auths();
        let (contract_id, _owner, product_id) = setup(&env);
        let client = SupplyLinkContractClient::new(&env, &contract_id);
        let issuer = Address::generate(&env);

        let mut cert_types = Vec::new(&env);
        cert_types.push_back(String::from_str(&env, "organic"));

        client.register_certification_issuer(
            &issuer,
            &issuer,
            &String::from_str(&env, "Organic Only Body"),
            &cert_types,
        );

        // Attempt to issue a cert type not in the issuer's list
        client.issue_certification_registry_record(
            &product_id,
            &issuer,
            &String::from_str(&env, "rec-bad"),
            &String::from_str(&env, "EXT-BAD"),
            &String::from_str(&env, "iso_9001"), // not supported
            &String::from_str(&env, "c".repeat(64).as_str()),
        );
    }

    #[test]
    #[should_panic(expected = "issuer already registered")]
    fn test_duplicate_issuer_registration_panics() {
        let env = Env::default();
        env.mock_all_auths();
        let (contract_id, _owner, _product_id) = setup(&env);
        let client = SupplyLinkContractClient::new(&env, &contract_id);
        let issuer = Address::generate(&env);

        let mut cert_types = Vec::new(&env);
        cert_types.push_back(String::from_str(&env, "organic"));

        client.register_certification_issuer(
            &issuer,
            &issuer,
            &String::from_str(&env, "Body A"),
            &cert_types,
        );

        // Second registration with same address should panic
        client.register_certification_issuer(
            &issuer,
            &issuer,
            &String::from_str(&env, "Body A Duplicate"),
            &cert_types,
        );
    }

    #[test]
    fn test_deactivate_issuer() {
        let env = Env::default();
        env.mock_all_auths();
        let (contract_id, _owner, _product_id) = setup(&env);
        let client = SupplyLinkContractClient::new(&env, &contract_id);
        let issuer = Address::generate(&env);

        let mut cert_types = Vec::new(&env);
        cert_types.push_back(String::from_str(&env, "organic"));

        client.register_certification_issuer(
            &issuer,
            &issuer,
            &String::from_str(&env, "Deactivatable Body"),
            &cert_types,
        );

        client.deactivate_certification_issuer(&issuer);

        let retrieved = client.get_certification_issuer(&issuer);
        assert!(!retrieved.active);
    }

    #[test]
    fn test_list_registry_records_for_product() {
        let env = Env::default();
        env.mock_all_auths();
        let (contract_id, _owner, product_id) = setup(&env);
        let client = SupplyLinkContractClient::new(&env, &contract_id);
        let issuer = Address::generate(&env);

        let mut cert_types = Vec::new(&env);
        cert_types.push_back(String::from_str(&env, "organic"));
        cert_types.push_back(String::from_str(&env, "fair_trade"));

        client.register_certification_issuer(
            &issuer,
            &issuer,
            &String::from_str(&env, "Multi Cert Body"),
            &cert_types,
        );

        client.issue_certification_registry_record(
            &product_id,
            &issuer,
            &String::from_str(&env, "rec-list-001"),
            &String::from_str(&env, "EXT-001"),
            &String::from_str(&env, "organic"),
            &String::from_str(&env, "d".repeat(64).as_str()),
        );

        client.issue_certification_registry_record(
            &product_id,
            &issuer,
            &String::from_str(&env, "rec-list-002"),
            &String::from_str(&env, "EXT-002"),
            &String::from_str(&env, "fair_trade"),
            &String::from_str(&env, "e".repeat(64).as_str()),
        );

        let records = client.list_certification_registry_records(&product_id);
        assert_eq!(records.len(), 2);
    }
}
