use anyhow::{bail, Result};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use chrono::{Duration, SecondsFormat, Utc};
use rusqlite::{params, TransactionBehavior};
use sha2::{Digest, Sha256};

use crate::{
    compute_federation::external_pool_adapter_runtime_compatibility_verification::*,
    store::{
        compute_external_pool_adapter_registry::current_external_pool_adapter_registry_release_authority_on,
        compute_external_pool_adapter_sandbox_verifier_key::current_sandbox_verifier_key_authority_on,
        new_id, Store,
    },
};

use super::{
    error::ExternalPoolAdapterRuntimeCompatibilityVerificationStoreError as StoreError,
    persistence::insert_verification,
    read::{
        challenge_by_id_on, identifier, run_observation_by_id_on, verification_by_id_on,
        verification_by_idempotency_on, verification_head_by_release_on,
    },
    types::ExternalPoolAdapterRuntimeCompatibilityVerificationWriteReceipt,
};

impl Store {
    pub(crate) fn record_external_pool_adapter_runtime_compatibility_verification(
        &self,
        admin_user_id: &str,
        input: RecordExternalPoolAdapterRuntimeCompatibilityVerificationReceiptInput,
    ) -> std::result::Result<
        ExternalPoolAdapterRuntimeCompatibilityVerificationWriteReceipt,
        StoreError,
    > {
        identifier(admin_user_id).map_err(StoreError::conflict)?;
        validate_record_runtime_compatibility_verification_input(&input)
            .map_err(StoreError::conflict)?;
        let scope = format!("v268:runtime-compatibility-verify:{admin_user_id}");
        identifier(&scope).map_err(StoreError::conflict)?;
        let mut conn = self.conn().map_err(StoreError::storage)?;
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(StoreError::storage)?;
        let result = (|| -> Result<_> {
            if let Some(stored) =
                verification_by_idempotency_on(&tx, &scope, &input.idempotency_key)?
            {
                ensure_replay(admin_user_id, &scope, &input, &stored.receipt)?;
                return Ok(
                    ExternalPoolAdapterRuntimeCompatibilityVerificationWriteReceipt {
                        verification: stored.receipt,
                        replayed: true,
                    },
                );
            }
            require_current_admin(&tx, admin_user_id)?;
            let observation = run_observation_by_id_on(&tx, &input.run_observation_id)?
                .ok_or_else(|| anyhow::anyhow!("V268 run observation was not found"))?;
            if observation.receipt.run_observation_digest != input.expected_run_observation_digest {
                bail!("V268 expected run-observation digest is not exact");
            }
            let challenge = challenge_by_id_on(&tx, &observation.receipt.observation.challenge_id)?
                .ok_or_else(|| anyhow::anyhow!("V268 observation lost its challenge"))?;
            let signature_challenge = runtime_compatibility_signature_challenge(
                &challenge.receipt,
                &observation.receipt,
            )?;
            if signature_challenge.signature_message_digest
                != input.expected_signature_message_digest
            {
                bail!("V268 expected signature-message digest is not exact");
            }
            let checked_at = Utc::now().to_rfc3339_opts(SecondsFormat::Nanos, true);
            if checked_at >= challenge.receipt.challenge.expires_at {
                bail!("V268 challenge expired before signature recording");
            }
            let selected = &challenge.receipt.challenge;
            current_external_pool_adapter_registry_release_authority_on(
                &tx,
                &selected.registry_release.registry_release_id,
                &selected.registry_release.registry_release_digest,
                &checked_at,
            )?
            .ok_or_else(|| anyhow::anyhow!("V268 registry release is no longer current"))?;
            validate_runtime_compatibility_challenge_current_roots(selected)?;
            let key = current_sandbox_verifier_key_authority_on(
                &tx,
                &selected.sandbox_verifier_key_record_id,
                &selected.sandbox_verifier_key_record_digest,
                &selected.sandbox_verifier_key_id,
            )?
            .ok_or_else(|| anyhow::anyhow!("V268 verifier key is no longer current"))?;
            if key.verifier_operator() != selected.sandbox_verifier_operator
                || key.verifier_product() != selected.sandbox_verifier_product
            {
                bail!("V268 verifier key operator/product roots drifted");
            }
            verify_runtime_compatibility_signature(
                key.public_key_pem(),
                &signature_challenge,
                &input.signature_base64,
            )?;
            let verified = Utc::now();
            let verified_at = verified.to_rfc3339_opts(SecondsFormat::Nanos, true);
            if verified_at >= challenge.receipt.challenge.expires_at {
                bail!("V268 challenge expired during signature verification");
            }
            current_external_pool_adapter_registry_release_authority_on(
                &tx,
                &selected.registry_release.registry_release_id,
                &selected.registry_release.registry_release_digest,
                &verified_at,
            )?
            .ok_or_else(|| anyhow::anyhow!("V268 registry release drifted during verification"))?;
            current_sandbox_verifier_key_authority_on(
                &tx,
                &selected.sandbox_verifier_key_record_id,
                &selected.sandbox_verifier_key_record_digest,
                &selected.sandbox_verifier_key_id,
            )?
            .ok_or_else(|| anyhow::anyhow!("V268 verifier key drifted during verification"))?;
            ensure_lineage(&tx, selected)?;
            let signature = STANDARD.decode(&input.signature_base64)?;
            let material = ExternalPoolAdapterRuntimeCompatibilityVerificationMaterial {
                runner_execution_id: observation.receipt.observation.runner_execution_id.clone(),
                challenge_id: selected.challenge_id.clone(),
                challenge_digest: challenge.receipt.challenge_digest.clone(),
                run_observation_id: observation.receipt.run_observation_id.clone(),
                run_observation_digest: observation.receipt.run_observation_digest.clone(),
                run_observation_material_digest: observation
                    .receipt
                    .run_observation_material_digest
                    .clone(),
                registry_release: selected.registry_release.clone(),
                profile_id: selected.profile_id.clone(),
                profile_revision: selected.profile_revision,
                profile_digest: selected.profile_digest.clone(),
                runner_policy_digest: selected.runner_policy.policy_digest.clone(),
                fixture_catalog_digest: selected.fixture_catalog.policy_digest.clone(),
                public_fixture_delivery_root: observation
                    .receipt
                    .observation
                    .public_fixture_delivery_root
                    .clone(),
                sandbox_verifier_key_record_id: selected.sandbox_verifier_key_record_id.clone(),
                sandbox_verifier_key_record_digest: selected
                    .sandbox_verifier_key_record_digest
                    .clone(),
                sandbox_verifier_key_id: selected.sandbox_verifier_key_id.clone(),
                sandbox_verifier_operator: selected.sandbox_verifier_operator.clone(),
                sandbox_verifier_product: selected.sandbox_verifier_product.clone(),
                signature_algorithm: RUNTIME_COMPATIBILITY_VERIFICATION_SIGNATURE_ALGORITHM.into(),
                sequence: selected.sequence,
                predecessor_verification_receipt_id: selected
                    .predecessor_verification_receipt_id
                    .clone(),
                predecessor_verification_receipt_digest: selected
                    .predecessor_verification_receipt_digest
                    .clone(),
                signature_message_digest: signature_challenge.signature_message_digest,
                signature_base64: input.signature_base64.clone(),
                signature_digest: hex::encode(Sha256::digest(&signature)),
                verified_by_admin_user_id: admin_user_id.into(),
                confirmation: input.confirmation.clone(),
                idempotency_scope: scope.clone(),
                idempotency_key: input.idempotency_key.clone(),
                verified_at: verified_at.clone(),
                recorded_at: verified_at,
                expires_at: (verified
                    + Duration::hours(RUNTIME_COMPATIBILITY_VERIFICATION_RECEIPT_VALIDITY_HOURS))
                .to_rfc3339_opts(SecondsFormat::Nanos, true),
                evidence_scope: RUNTIME_COMPATIBILITY_VERIFICATION_EVIDENCE_SCOPE.into(),
                receipt_status: RUNTIME_COMPATIBILITY_VERIFICATION_SIGNED_RECEIPT_STATUS.into(),
                effects: runtime_compatibility_no_effects(),
                readiness: runtime_compatibility_no_readiness(),
            };
            let receipt = build_runtime_compatibility_verification_receipt(
                new_id("external_pool_adapter_runtime_compatibility_verification"),
                material,
                &challenge.receipt,
                &observation.receipt,
            )?;
            insert_verification(&tx, &receipt)?;
            let stored = verification_by_id_on(&tx, &receipt.verification_receipt_id)?
                .ok_or_else(|| anyhow::anyhow!("V268 verification disappeared after insert"))?;
            if stored.receipt != receipt {
                bail!("V268 verification readback drifted");
            }
            Ok(
                ExternalPoolAdapterRuntimeCompatibilityVerificationWriteReceipt {
                    verification: stored.receipt,
                    replayed: false,
                },
            )
        })()
        .map_err(StoreError::classify_write)?;
        tx.commit().map_err(StoreError::storage)?;
        Ok(result)
    }
}

fn ensure_lineage(
    conn: &rusqlite::Connection,
    selected: &ExternalPoolAdapterRuntimeCompatibilityChallengeMaterial,
) -> Result<()> {
    let head =
        verification_head_by_release_on(conn, &selected.registry_release.registry_release_id)?;
    match (
        head,
        selected.predecessor_verification_receipt_id.as_deref(),
        selected.predecessor_verification_receipt_digest.as_deref(),
    ) {
        (None, None, None) if selected.sequence == 1 => Ok(()),
        (Some(head), Some(id), Some(digest))
            if head.receipt.verification_receipt_id == id
                && head.receipt.verification_receipt_digest == digest
                && head.receipt.verification.sequence.checked_add(1) == Some(selected.sequence) =>
        {
            Ok(())
        }
        _ => bail!("V268 challenge no longer targets the exact verification head"),
    }
}

fn ensure_replay(
    admin: &str,
    scope: &str,
    input: &RecordExternalPoolAdapterRuntimeCompatibilityVerificationReceiptInput,
    receipt: &ExternalPoolAdapterRuntimeCompatibilityVerificationReceipt,
) -> Result<()> {
    let v = &receipt.verification;
    if v.run_observation_id != input.run_observation_id
        || v.run_observation_digest != input.expected_run_observation_digest
        || v.signature_message_digest != input.expected_signature_message_digest
        || v.signature_base64 != input.signature_base64
        || v.verified_by_admin_user_id != admin
        || v.idempotency_scope != scope
        || v.idempotency_key != input.idempotency_key
        || v.confirmation != input.confirmation
    {
        bail!("V268 verification idempotency replay conflicts with sealed input");
    }
    Ok(())
}

fn require_current_admin(conn: &rusqlite::Connection, admin: &str) -> Result<()> {
    let current: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM users WHERE id=?1 AND role IN ('admin','owner') AND status='active')",
        params![admin],
        |row| row.get(0),
    )?;
    if !current {
        bail!("V268 actor is not a current administrator");
    }
    Ok(())
}
