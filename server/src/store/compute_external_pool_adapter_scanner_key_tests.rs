use std::{
    path::{Path, PathBuf},
    sync::{Arc, Barrier},
};

use rsa::{
    pkcs8::{EncodePublicKey, LineEnding},
    rand_core::OsRng,
    RsaPrivateKey,
};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::{
    compute_federation::external_pool_adapter_artifact_signing_key::SIGNING_KEY_REGISTRATION_CONFIRMATION,
    compute_federation::external_pool_adapter_scanner_key::{
        SCANNER_KEY_ACTIVATE_CONFIRMATION, SCANNER_KEY_REGISTER_CONFIRMATION,
        SCANNER_KEY_REVOKE_CONFIRMATION,
    },
    store::{
        ActivateExternalPoolAdapterScannerKey, RegisterExternalPoolAdapterArtifactSigningKey,
        RegisterExternalPoolAdapterScannerKey, RevokeExternalPoolAdapterScannerKey, Store,
    },
};

#[test]
fn scanner_key_lifecycle_survives_replay_reopen_and_role_separation() {
    let path = temp_path("lifecycle");
    let store = Store::open(&path).unwrap();
    let (pem, key_id) = key_fixture();
    let created = store
        .register_external_pool_adapter_scanner_key(registration(&pem, &key_id, "register-one"))
        .unwrap();
    assert!(!created.replayed);
    assert_eq!(
        store
            .external_pool_adapter_scanner_key_currentness(&created.key_record.key_record_id)
            .unwrap()
            .unwrap()
            .current_status,
        "pending_activation"
    );
    assert!(store
        .register_external_pool_adapter_scanner_key(registration(&pem, &key_id, "register-two"))
        .is_err());
    assert!(store
        .activate_external_pool_adapter_scanner_key(activation(
            &created.key_record.key_record_id,
            &created.key_record.key_record_digest,
            "admin-register",
            "self"
        ))
        .is_err());
    let active = store
        .activate_external_pool_adapter_scanner_key(activation(
            &created.key_record.key_record_id,
            &created.key_record.key_record_digest,
            "admin-review",
            "active",
        ))
        .unwrap();
    assert!(!active.replayed);
    let revoked = store
        .revoke_external_pool_adapter_scanner_key(revocation(
            &created.key_record.key_record_id,
            &created.key_record.key_record_digest,
            "revoke",
        ))
        .unwrap();
    assert!(!revoked.replayed);
    drop(store);
    let reopened = Store::open(&path).unwrap();
    let current = reopened
        .external_pool_adapter_scanner_key_currentness(&created.key_record.key_record_id)
        .unwrap()
        .unwrap();
    assert_eq!(current.current_status, "revoked");
    assert!(current.activation.is_some());
    assert!(current.revocation.is_some());
    drop(reopened);
    remove_store(&path);
}

#[test]
fn scanner_key_activation_is_linearized_and_sql_is_immutable() {
    let path = temp_path("linear");
    let store = Store::open(&path).unwrap();
    let (pem, key_id) = key_fixture();
    let created = store
        .register_external_pool_adapter_scanner_key(registration(&pem, &key_id, "register-linear"))
        .unwrap();
    let connection = store.conn().unwrap();
    assert!(connection.execute("UPDATE compute_external_pool_adapter_scanner_keys SET scanner_product='mutated' WHERE key_record_id=?1",[&created.key_record.key_record_id]).is_err());
    drop(connection);
    drop(store);
    let barrier = Arc::new(Barrier::new(3));
    let mut workers = Vec::new();
    for i in 0..2 {
        let path = path.clone();
        let barrier = barrier.clone();
        let id = created.key_record.key_record_id.clone();
        let digest = created.key_record.key_record_digest.clone();
        workers.push(std::thread::spawn(move || {
            let store = Store::open(&path).unwrap();
            barrier.wait();
            store
                .activate_external_pool_adapter_scanner_key(activation(
                    &id,
                    &digest,
                    &format!("admin-{i}"),
                    &format!("activate-{i}"),
                ))
                .is_ok()
        }));
    }
    barrier.wait();
    assert_eq!(
        workers
            .into_iter()
            .map(|worker| worker.join().unwrap())
            .filter(|succeeded| *succeeded)
            .count(),
        1
    );
    drop(Store::open(&path).unwrap());
    remove_store(&path);
}

#[test]
fn scanner_key_and_supplier_signing_key_roles_are_separated_both_ways() {
    let (pem, key_id) = key_fixture();

    let supplier_first_path = temp_path("supplier-first");
    let supplier_first = Store::open(&supplier_first_path).unwrap();
    supplier_first
        .register_external_pool_adapter_artifact_signing_key(supplier_registration(
            &pem,
            &key_id,
            "supplier-first",
        ))
        .unwrap();
    assert!(supplier_first
        .register_external_pool_adapter_scanner_key(registration(
            &pem,
            &key_id,
            "scanner-after-supplier"
        ))
        .is_err());
    drop(supplier_first);
    remove_store(&supplier_first_path);

    let scanner_first_path = temp_path("scanner-first");
    let scanner_first = Store::open(&scanner_first_path).unwrap();
    scanner_first
        .register_external_pool_adapter_scanner_key(registration(&pem, &key_id, "scanner-first"))
        .unwrap();
    assert!(scanner_first
        .register_external_pool_adapter_artifact_signing_key(supplier_registration(
            &pem,
            &key_id,
            "supplier-after-scanner"
        ))
        .is_err());
    drop(scanner_first);
    remove_store(&scanner_first_path);
}

fn registration(pem: &str, key_id: &str, key: &str) -> RegisterExternalPoolAdapterScannerKey {
    RegisterExternalPoolAdapterScannerKey {
        scanner_operator: "fixture-security-lab".into(),
        scanner_product: "fixture-adapter-scanner-v1".into(),
        key_id: key_id.into(),
        public_key_pem: pem.into(),
        created_by_admin_user_id: "admin-register".into(),
        confirmation: SCANNER_KEY_REGISTER_CONFIRMATION.into(),
        idempotency_scope: "test-scanner-key-register:admin-register".into(),
        idempotency_key: key.into(),
    }
}
fn supplier_registration(
    pem: &str,
    key_id: &str,
    key: &str,
) -> RegisterExternalPoolAdapterArtifactSigningKey {
    RegisterExternalPoolAdapterArtifactSigningKey {
        source_operator: "fixture-pool-operator".into(),
        key_id: key_id.into(),
        public_key_pem: pem.into(),
        created_by_admin_user_id: "admin-register".into(),
        confirmation: SIGNING_KEY_REGISTRATION_CONFIRMATION.into(),
        idempotency_scope: "test-scanner-role-separation:admin-register".into(),
        idempotency_key: key.into(),
    }
}
fn activation(
    id: &str,
    digest: &str,
    actor: &str,
    key: &str,
) -> ActivateExternalPoolAdapterScannerKey {
    ActivateExternalPoolAdapterScannerKey {
        key_record_id: id.into(),
        expected_key_record_digest: digest.into(),
        activated_by_admin_user_id: actor.into(),
        confirmation: SCANNER_KEY_ACTIVATE_CONFIRMATION.into(),
        idempotency_scope: format!("test-scanner-key-activate:{actor}"),
        idempotency_key: key.into(),
    }
}
fn revocation(id: &str, digest: &str, key: &str) -> RevokeExternalPoolAdapterScannerKey {
    RevokeExternalPoolAdapterScannerKey {
        key_record_id: id.into(),
        expected_key_record_digest: digest.into(),
        revoked_by_admin_user_id: "admin-revoke".into(),
        reason: "fixture scanner key intentionally retired".into(),
        confirmation: SCANNER_KEY_REVOKE_CONFIRMATION.into(),
        idempotency_scope: "test-scanner-key-revoke:admin-revoke".into(),
        idempotency_key: key.into(),
    }
}
fn key_fixture() -> (String, String) {
    let private = RsaPrivateKey::new(&mut OsRng, 2048).unwrap();
    let public = private.to_public_key();
    let pem = public.to_public_key_pem(LineEnding::LF).unwrap();
    let der = public.to_public_key_der().unwrap();
    (pem, hex::encode(Sha256::digest(der.as_bytes())))
}
fn temp_path(label: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "elon_scanner_key_{label}_{}.db",
        Uuid::new_v4().simple()
    ))
}
fn remove_store(path: &Path) {
    for item in [
        path.to_path_buf(),
        PathBuf::from(format!("{}-wal", path.display())),
        PathBuf::from(format!("{}-shm", path.display())),
    ] {
        if item.exists() {
            std::fs::remove_file(item).unwrap()
        }
    }
}
