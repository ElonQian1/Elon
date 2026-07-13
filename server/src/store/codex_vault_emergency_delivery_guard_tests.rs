use super::CodexVaultEmergencyCredentialDeliveryClaim;
use crate::store::{
    codex_vault_emergency::CodexVaultEmergencyLeaseCreate, CodexVaultSlotRecord, Store,
};

struct DeliveryFixture {
    store: Store,
    path: std::path::PathBuf,
    provider_id: String,
    consumer_id: String,
    grant_id: String,
    lease_id: String,
    lease_updated_at: String,
    lease_expires_at: String,
    slot: CodexVaultSlotRecord,
}

impl DeliveryFixture {
    fn claim(&self) -> CodexVaultEmergencyCredentialDeliveryClaim<'_> {
        CodexVaultEmergencyCredentialDeliveryClaim {
            lease_id: &self.lease_id,
            expected_lease_updated_at: &self.lease_updated_at,
            grant_id: &self.grant_id,
            provider_user_id: &self.provider_id,
            consumer_user_id: &self.consumer_id,
            consumer_node_id: "node-delivery",
            provider_slot_id: &self.slot.slot_id,
            credential_version: self.slot.credential_version,
            compute_call_id: None,
            cloud_control_deadline: &self.lease_expires_at,
        }
    }
}

fn fixture(label: &str) -> DeliveryFixture {
    let path = std::env::temp_dir().join(format!(
        "elon-codex-delivery-{label}-{}.sqlite",
        uuid::Uuid::new_v4().simple()
    ));
    let _ = std::fs::remove_file(&path);
    let store = Store::open(&path).unwrap();
    let provider = store
        .create_user(
            &format!("delivery-provider-{label}@example.com"),
            "secret1",
            None,
            None,
        )
        .unwrap();
    let consumer = store
        .create_user(
            &format!("delivery-consumer-{label}@example.com"),
            "secret1",
            None,
            None,
        )
        .unwrap();
    store
        .upsert_user_codex_credential(
            &provider.id,
            "chatgpt",
            Some("account-hash"),
            Some("test"),
            "ciphertext",
            "nonce",
        )
        .unwrap();
    let slot = store
        .select_user_codex_credential_slot(&provider.id, None)
        .unwrap()
        .unwrap();
    let grant = store
        .upsert_codex_vault_emergency_grant(
            &provider.id,
            &consumer.id,
            Some("delivery guard"),
            Some("unit_test"),
            Some(900),
            None,
            &provider.id,
        )
        .unwrap();
    let lease = store
        .create_codex_vault_emergency_lease(CodexVaultEmergencyLeaseCreate {
            grant_id: &grant.id,
            provider_user_id: &provider.id,
            consumer_user_id: &consumer.id,
            consumer_node_id: "node-delivery",
            provider_slot_id: &slot.slot_id,
            account_hint_hash: slot.account_hint_hash.as_deref(),
            purpose: Some("unit_test"),
            failure_reason: None,
            max_lease_seconds: 900,
        })
        .unwrap();
    DeliveryFixture {
        store,
        path,
        provider_id: provider.id,
        consumer_id: consumer.id,
        grant_id: grant.id,
        lease_id: lease.id,
        lease_updated_at: lease.updated_at,
        lease_expires_at: lease.expires_at,
        slot,
    }
}

#[test]
fn active_exact_delivery_claim_is_one_shot() {
    let fixture = fixture("active");
    assert!(fixture
        .store
        .claim_codex_vault_emergency_credential_delivery(fixture.claim())
        .unwrap());
    assert!(!fixture
        .store
        .claim_codex_vault_emergency_credential_delivery(fixture.claim())
        .unwrap());
    assert_eq!(
        fixture
            .store
            .get_codex_vault_emergency_lease(&fixture.lease_id)
            .unwrap()
            .unwrap()
            .status,
        "active"
    );
    drop(fixture.store);
    let _ = std::fs::remove_file(fixture.path);
}

#[test]
fn revoke_committed_before_delivery_claim_fails_closed() {
    let fixture = fixture("revoked");
    fixture
        .store
        .revoke_codex_vault_emergency_grant(&fixture.grant_id, &fixture.provider_id)
        .unwrap()
        .expect("grant should revoke");
    assert!(!fixture
        .store
        .claim_codex_vault_emergency_credential_delivery(fixture.claim())
        .unwrap());
    drop(fixture.store);
    let _ = std::fs::remove_file(fixture.path);
}

#[test]
fn superseded_lease_cannot_claim_credential_delivery() {
    let fixture = fixture("superseded");
    fixture
        .store
        .create_codex_vault_emergency_lease(CodexVaultEmergencyLeaseCreate {
            grant_id: &fixture.grant_id,
            provider_user_id: &fixture.provider_id,
            consumer_user_id: &fixture.consumer_id,
            consumer_node_id: "node-delivery",
            provider_slot_id: &fixture.slot.slot_id,
            account_hint_hash: fixture.slot.account_hint_hash.as_deref(),
            purpose: Some("supersede_test"),
            failure_reason: None,
            max_lease_seconds: 900,
        })
        .unwrap();
    assert!(!fixture
        .store
        .claim_codex_vault_emergency_credential_delivery(fixture.claim())
        .unwrap());
    drop(fixture.store);
    let _ = std::fs::remove_file(fixture.path);
}
