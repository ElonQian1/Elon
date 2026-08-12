use base64::{engine::general_purpose::STANDARD, Engine as _};
use rsa::{
    pkcs1v15::SigningKey,
    pkcs8::{EncodePublicKey, LineEnding},
    rand_core::OsRng,
    signature::{SignatureEncoding, Signer},
    RsaPrivateKey,
};
use sha2::{Digest, Sha256};

use crate::{
    compute_federation::external_pool_adapter_artifact_signed_provenance::ARTIFACT_SIGNED_PROVENANCE_CONFIRMATION,
    compute_federation::external_pool_adapter_artifact_signing_key::{
        SIGNING_KEY_ACTIVATION_CONFIRMATION, SIGNING_KEY_REGISTRATION_CONFIRMATION,
        SIGNING_KEY_REVOCATION_CONFIRMATION,
    },
    store::{
        ActivateExternalPoolAdapterArtifactSigningKey,
        CreateExternalPoolAdapterArtifactSignedProvenance,
        GetExternalPoolAdapterArtifactSignatureChallenge,
        RegisterExternalPoolAdapterArtifactSigningKey, RevokeExternalPoolAdapterArtifactSigningKey,
    },
};

use super::{lifecycle_support::*, *};

#[tokio::test]
async fn signed_provenance_verifies_replays_reopens_and_becomes_historical_after_revocation() {
    let (store, database_path, data_dir) = temporary_lifecycle_store();
    let release = stage_release(&store, "signed-adapter", "1.0.0", "signed-provenance");
    let source = record_artifact(&store, &data_dir, &release, "signed-source").await;
    let (private, key_id, key_record_id, key_record_digest) = active_signing_key(&store);
    let challenge = store
        .external_pool_adapter_artifact_signature_challenge(challenge_input(
            &release,
            &source.source_receipt_digest,
            &key_record_id,
            &key_record_digest,
            &key_id,
        ))
        .unwrap();
    assert!(!challenge
        .signature_message_base64
        .contains("artifact-ref:signed-provenance"));
    let signature = sign(&private, &challenge.signature_message_base64);
    let input = provenance_input(
        &release,
        &source.source_receipt_digest,
        &key_record_id,
        &key_record_digest,
        &key_id,
        &challenge.signature_message_digest,
        &signature,
        "provenance-one",
    );

    let mut tampered = provenance_input(
        &release,
        &source.source_receipt_digest,
        &key_record_id,
        &key_record_digest,
        &key_id,
        &challenge.signature_message_digest,
        &STANDARD.encode([7_u8; 256]),
        "provenance-tampered",
    );
    assert!(store
        .create_external_pool_adapter_artifact_signed_provenance(tampered)
        .is_err());
    assert_eq!(provenance_count(&store), 0);

    let created = store
        .create_external_pool_adapter_artifact_signed_provenance(input)
        .unwrap();
    assert!(!created.replayed);
    assert_eq!(
        created.provenance.binding.artifact_sha256,
        release.declared_sha256
    );
    assert_eq!(
        created.provenance.binding.artifact_size_bytes,
        release.artifact_bytes.len() as u64
    );
    assert_eq!(
        store
            .external_pool_adapter_artifact_signed_provenance_currentness(&release.admission_id)
            .unwrap()
            .unwrap()
            .current_status,
        "verified_current"
    );

    let replay = store
        .create_external_pool_adapter_artifact_signed_provenance(provenance_input(
            &release,
            &source.source_receipt_digest,
            &key_record_id,
            &key_record_digest,
            &key_id,
            &challenge.signature_message_digest,
            &signature,
            "provenance-one",
        ))
        .unwrap();
    assert!(replay.replayed);
    assert_eq!(replay.provenance, created.provenance);

    store
        .revoke_external_pool_adapter_artifact_signing_key(
            RevokeExternalPoolAdapterArtifactSigningKey {
                key_record_id: key_record_id.clone(),
                expected_key_record_digest: key_record_digest.clone(),
                revoked_by_admin_user_id: "admin-revoke-signer".to_string(),
                reason: "fixture intentionally retires the signing key".to_string(),
                confirmation: SIGNING_KEY_REVOCATION_CONFIRMATION.to_string(),
                idempotency_scope: "test-signed-provenance-revoke".to_string(),
                idempotency_key: "revoke-signer".to_string(),
            },
        )
        .unwrap();
    let historical = store
        .external_pool_adapter_artifact_signed_provenance_currentness(&release.admission_id)
        .unwrap()
        .unwrap();
    assert_eq!(historical.current_status, "historical_only");
    assert_eq!(historical.signer_current_status, "revoked");
    assert!(store
        .external_pool_adapter_artifact_signature_challenge(challenge_input(
            &release,
            &source.source_receipt_digest,
            &key_record_id,
            &key_record_digest,
            &key_id,
        ))
        .is_err());

    let connection = store.conn().unwrap();
    assert!(connection
        .execute(
            "UPDATE compute_external_pool_adapter_artifact_signed_provenance_receipts
                SET source_operator='mutated' WHERE admission_id=?1",
            [&release.admission_id],
        )
        .is_err());
    assert!(connection
        .execute(
            "DELETE FROM compute_external_pool_adapter_artifact_signed_provenance_receipts
              WHERE admission_id=?1",
            [&release.admission_id],
        )
        .is_err());
    drop(connection);
    drop(store);

    let reopened = Store::open(&database_path).unwrap();
    let current = reopened
        .external_pool_adapter_artifact_signed_provenance_currentness(&release.admission_id)
        .unwrap()
        .unwrap();
    assert_eq!(current.current_status, "historical_only");
    assert_eq!(current.provenance, created.provenance);
    drop(reopened);
    cleanup_lifecycle_files(&database_path, &data_dir);
}

#[tokio::test]
async fn terminal_admission_rejects_a_previously_issued_signature_challenge() {
    let (store, database_path, data_dir) = temporary_lifecycle_store();
    let release = stage_release(
        &store,
        "terminal-signed-adapter",
        "2.0.0",
        "terminal-signed",
    );
    let source = record_artifact(&store, &data_dir, &release, "terminal-source").await;
    let (private, key_id, key_record_id, key_record_digest) = active_signing_key(&store);
    let challenge = store
        .external_pool_adapter_artifact_signature_challenge(challenge_input(
            &release,
            &source.source_receipt_digest,
            &key_record_id,
            &key_record_digest,
            &key_id,
        ))
        .unwrap();
    let signature = sign(&private, &challenge.signature_message_base64);
    store
        .create_external_pool_adapter_release_admission_terminal(terminal_input(
            &release,
            "withdrawn",
            "terminal-before-provenance",
        ))
        .unwrap();
    assert!(store
        .create_external_pool_adapter_artifact_signed_provenance(provenance_input(
            &release,
            &source.source_receipt_digest,
            &key_record_id,
            &key_record_digest,
            &key_id,
            &challenge.signature_message_digest,
            &signature,
            "terminal-provenance",
        ))
        .is_err());
    assert_eq!(provenance_count(&store), 0);
    drop(store);
    cleanup_lifecycle_files(&database_path, &data_dir);
}

fn active_signing_key(store: &Store) -> (RsaPrivateKey, String, String, String) {
    let private = RsaPrivateKey::new(&mut OsRng, 2_048).unwrap();
    let public = private.to_public_key();
    let pem = public.to_public_key_pem(LineEnding::LF).unwrap();
    let key_id = hex::encode(Sha256::digest(
        public.to_public_key_der().unwrap().as_bytes(),
    ));
    let registered = store
        .register_external_pool_adapter_artifact_signing_key(
            RegisterExternalPoolAdapterArtifactSigningKey {
                source_operator: "signed-provenance-fixture-pool".to_string(),
                key_id: key_id.clone(),
                public_key_pem: pem,
                created_by_admin_user_id: "admin-register-signer".to_string(),
                confirmation: SIGNING_KEY_REGISTRATION_CONFIRMATION.to_string(),
                idempotency_scope: "test-signed-provenance-register".to_string(),
                idempotency_key: "register-signer".to_string(),
            },
        )
        .unwrap();
    store
        .activate_external_pool_adapter_artifact_signing_key(
            ActivateExternalPoolAdapterArtifactSigningKey {
                key_record_id: registered.key_record.key_record_id.clone(),
                expected_key_record_digest: registered.key_record.key_record_digest.clone(),
                activated_by_admin_user_id: "admin-activate-signer".to_string(),
                confirmation: SIGNING_KEY_ACTIVATION_CONFIRMATION.to_string(),
                idempotency_scope: "test-signed-provenance-activate".to_string(),
                idempotency_key: "activate-signer".to_string(),
            },
        )
        .unwrap();
    (
        private,
        key_id,
        registered.key_record.key_record_id,
        registered.key_record.key_record_digest,
    )
}

fn challenge_input(
    release: &StagedRelease,
    source_digest: &str,
    key_record_id: &str,
    key_record_digest: &str,
    key_id: &str,
) -> GetExternalPoolAdapterArtifactSignatureChallenge {
    GetExternalPoolAdapterArtifactSignatureChallenge {
        admission_id: release.admission_id.clone(),
        expected_admission_digest: release.admission_digest.clone(),
        expected_source_receipt_digest: source_digest.to_string(),
        key_record_id: key_record_id.to_string(),
        expected_key_record_digest: key_record_digest.to_string(),
        expected_key_id: key_id.to_string(),
    }
}

#[allow(clippy::too_many_arguments)]
fn provenance_input(
    release: &StagedRelease,
    source_digest: &str,
    key_record_id: &str,
    key_record_digest: &str,
    key_id: &str,
    message_digest: &str,
    signature: &str,
    idempotency_key: &str,
) -> CreateExternalPoolAdapterArtifactSignedProvenance {
    CreateExternalPoolAdapterArtifactSignedProvenance {
        admission_id: release.admission_id.clone(),
        expected_admission_digest: release.admission_digest.clone(),
        expected_source_receipt_digest: source_digest.to_string(),
        key_record_id: key_record_id.to_string(),
        expected_key_record_digest: key_record_digest.to_string(),
        expected_key_id: key_id.to_string(),
        expected_signature_message_digest: message_digest.to_string(),
        signature_base64: signature.to_string(),
        verified_by_admin_user_id: "admin-verify-provenance".to_string(),
        confirmation: ARTIFACT_SIGNED_PROVENANCE_CONFIRMATION.to_string(),
        idempotency_scope: "test-signed-provenance-record".to_string(),
        idempotency_key: idempotency_key.to_string(),
    }
}

fn sign(private: &RsaPrivateKey, message_base64: &str) -> String {
    let message = STANDARD.decode(message_base64).unwrap();
    let signature = SigningKey::<Sha256>::new(private.clone()).sign(&message);
    STANDARD.encode(signature.to_bytes())
}

fn provenance_count(store: &Store) -> i64 {
    store
        .conn()
        .unwrap()
        .query_row(
            "SELECT COUNT(*) FROM compute_external_pool_adapter_artifact_signed_provenance_receipts",
            [],
            |row| row.get(0),
        )
        .unwrap()
}
