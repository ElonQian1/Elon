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
    compute_federation::external_pool_adapter_artifact_signing_key::{
        SIGNING_KEY_ACTIVATION_CONFIRMATION, SIGNING_KEY_REGISTRATION_CONFIRMATION,
        SIGNING_KEY_REVOCATION_CONFIRMATION,
    },
    store::{
        ActivateExternalPoolAdapterArtifactSigningKey,
        RegisterExternalPoolAdapterArtifactSigningKey, RevokeExternalPoolAdapterArtifactSigningKey,
        Store,
    },
};

#[test]
fn external_pool_adapter_artifact_signing_key_lifecycle_survives_replay_and_reopen() {
    let path = temporary_path("lifecycle");
    let store = Store::open(&path).unwrap();
    let (pem, key_id) = signing_key_fixture();
    let registration = registration_input(&pem, &key_id, "register-one");
    let created = store
        .register_external_pool_adapter_artifact_signing_key(registration)
        .unwrap();
    assert!(!created.replayed);
    assert_eq!(created.key_record.key_id, key_id);
    assert_eq!(
        store
            .external_pool_adapter_artifact_signing_key_currentness(
                &created.key_record.key_record_id
            )
            .unwrap()
            .unwrap()
            .current_status,
        "pending_activation"
    );

    let replayed = store
        .register_external_pool_adapter_artifact_signing_key(registration_input(
            &pem,
            &key_id,
            "register-one",
        ))
        .unwrap();
    assert!(replayed.replayed);
    assert_eq!(replayed.key_record, created.key_record);

    let self_activation = activation_input(
        &created.key_record.key_record_id,
        &created.key_record.key_record_digest,
        "admin-register",
        "activate-self",
    );
    assert!(store
        .activate_external_pool_adapter_artifact_signing_key(self_activation)
        .is_err());

    let activated = store
        .activate_external_pool_adapter_artifact_signing_key(activation_input(
            &created.key_record.key_record_id,
            &created.key_record.key_record_digest,
            "admin-activate",
            "activate-one",
        ))
        .unwrap();
    assert!(!activated.replayed);
    assert_eq!(
        store
            .external_pool_adapter_artifact_signing_key_currentness(
                &created.key_record.key_record_id
            )
            .unwrap()
            .unwrap()
            .current_status,
        "active"
    );

    let revoked = store
        .revoke_external_pool_adapter_artifact_signing_key(revocation_input(
            &created.key_record.key_record_id,
            &created.key_record.key_record_digest,
            "revoke-one",
        ))
        .unwrap();
    assert!(!revoked.replayed);

    let historical_activation = store
        .activate_external_pool_adapter_artifact_signing_key(activation_input(
            &created.key_record.key_record_id,
            &created.key_record.key_record_digest,
            "admin-activate",
            "activate-one",
        ))
        .unwrap();
    assert!(historical_activation.replayed);
    assert!(store
        .activate_external_pool_adapter_artifact_signing_key(activation_input(
            &created.key_record.key_record_id,
            &created.key_record.key_record_digest,
            "admin-third",
            "activate-after-revoke",
        ))
        .is_err());
    drop(store);

    let reopened = Store::open(&path).unwrap();
    let current = reopened
        .external_pool_adapter_artifact_signing_key_currentness(&created.key_record.key_record_id)
        .unwrap()
        .unwrap();
    assert_eq!(current.current_status, "revoked");
    assert_eq!(
        current.activation.unwrap().activation_receipt_id,
        activated.activation.activation_receipt_id
    );
    assert_eq!(
        current.revocation.unwrap().revocation_receipt_id,
        revoked.revocation.revocation_receipt_id
    );
    drop(reopened);
    remove_store_files(&path);
}

#[test]
fn external_pool_adapter_artifact_signing_key_rejects_duplicate_material_and_sql_mutation() {
    let path = temporary_path("guards");
    let store = Store::open(&path).unwrap();
    let (pem, key_id) = signing_key_fixture();
    let created = store
        .register_external_pool_adapter_artifact_signing_key(registration_input(
            &pem,
            &key_id,
            "register-guard",
        ))
        .unwrap();
    let mut duplicate = registration_input(&pem, &key_id, "register-duplicate");
    duplicate.source_operator = "another-operator".to_string();
    assert!(store
        .register_external_pool_adapter_artifact_signing_key(duplicate)
        .is_err());

    let connection = store.conn().unwrap();
    assert!(connection
        .execute(
            "UPDATE compute_external_pool_adapter_artifact_signing_keys
                SET source_operator='mutated' WHERE key_record_id=?1",
            [&created.key_record.key_record_id],
        )
        .is_err());
    assert!(connection
        .execute(
            "DELETE FROM compute_external_pool_adapter_artifact_signing_keys
              WHERE key_record_id=?1",
            [&created.key_record.key_record_id],
        )
        .is_err());
    drop(connection);
    drop(store);
    remove_store_files(&path);
}

#[test]
fn external_pool_adapter_artifact_signing_key_activation_is_linearized() {
    let path = temporary_path("concurrency");
    let store = Store::open(&path).unwrap();
    let (pem, key_id) = signing_key_fixture();
    let created = store
        .register_external_pool_adapter_artifact_signing_key(registration_input(
            &pem,
            &key_id,
            "register-concurrency",
        ))
        .unwrap();
    drop(store);

    let barrier = Arc::new(Barrier::new(3));
    let mut workers = Vec::new();
    for index in 0..2 {
        let path = path.clone();
        let barrier = barrier.clone();
        let id = created.key_record.key_record_id.clone();
        let digest = created.key_record.key_record_digest.clone();
        workers.push(std::thread::spawn(move || {
            let store = Store::open(&path).unwrap();
            barrier.wait();
            store
                .activate_external_pool_adapter_artifact_signing_key(activation_input(
                    &id,
                    &digest,
                    &format!("admin-concurrent-{index}"),
                    &format!("activate-concurrent-{index}"),
                ))
                .is_ok()
        }));
    }
    barrier.wait();
    let successes = workers
        .into_iter()
        .map(|worker| worker.join().unwrap())
        .filter(|success| *success)
        .count();
    assert_eq!(successes, 1);
    let store = Store::open(&path).unwrap();
    assert_eq!(
        store
            .external_pool_adapter_artifact_signing_key_currentness(
                &created.key_record.key_record_id
            )
            .unwrap()
            .unwrap()
            .current_status,
        "active"
    );
    drop(store);
    remove_store_files(&path);
}

fn registration_input(
    pem: &str,
    key_id: &str,
    idempotency_key: &str,
) -> RegisterExternalPoolAdapterArtifactSigningKey {
    RegisterExternalPoolAdapterArtifactSigningKey {
        source_operator: "fixture-pool-operator".to_string(),
        key_id: key_id.to_string(),
        public_key_pem: pem.to_string(),
        created_by_admin_user_id: "admin-register".to_string(),
        confirmation: SIGNING_KEY_REGISTRATION_CONFIRMATION.to_string(),
        idempotency_scope: "test-signing-key-register:admin-register".to_string(),
        idempotency_key: idempotency_key.to_string(),
    }
}

fn activation_input(
    id: &str,
    digest: &str,
    actor: &str,
    key: &str,
) -> ActivateExternalPoolAdapterArtifactSigningKey {
    ActivateExternalPoolAdapterArtifactSigningKey {
        key_record_id: id.to_string(),
        expected_key_record_digest: digest.to_string(),
        activated_by_admin_user_id: actor.to_string(),
        confirmation: SIGNING_KEY_ACTIVATION_CONFIRMATION.to_string(),
        idempotency_scope: format!("test-signing-key-activate:{actor}"),
        idempotency_key: key.to_string(),
    }
}

fn revocation_input(
    id: &str,
    digest: &str,
    key: &str,
) -> RevokeExternalPoolAdapterArtifactSigningKey {
    RevokeExternalPoolAdapterArtifactSigningKey {
        key_record_id: id.to_string(),
        expected_key_record_digest: digest.to_string(),
        revoked_by_admin_user_id: "admin-revoke".to_string(),
        reason: "fixture trust root has been intentionally retired".to_string(),
        confirmation: SIGNING_KEY_REVOCATION_CONFIRMATION.to_string(),
        idempotency_scope: "test-signing-key-revoke:admin-revoke".to_string(),
        idempotency_key: key.to_string(),
    }
}

fn signing_key_fixture() -> (String, String) {
    let private = RsaPrivateKey::new(&mut OsRng, 2_048).unwrap();
    let public = private.to_public_key();
    let pem = public.to_public_key_pem(LineEnding::LF).unwrap();
    let der = public.to_public_key_der().unwrap();
    (pem, hex::encode(Sha256::digest(der.as_bytes())))
}

fn temporary_path(label: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "elon_external_pool_adapter_signing_key_{label}_{}.db",
        Uuid::new_v4().simple()
    ))
}

fn remove_store_files(path: &Path) {
    for candidate in [
        path.to_path_buf(),
        PathBuf::from(format!("{}-wal", path.display())),
        PathBuf::from(format!("{}-shm", path.display())),
    ] {
        if candidate.exists() {
            std::fs::remove_file(candidate).unwrap();
        }
    }
}
