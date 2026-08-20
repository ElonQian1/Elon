use chrono::{SecondsFormat, Utc};
use rusqlite::{params, OptionalExtension, TransactionBehavior};

use crate::{
    compute_federation::{
        external_pool_adapter_installation::PreparedExternalPoolAdapterInstallation,
        external_pool_adapter_linux_supervisor::ExternalPoolAdapterSupervisorCgroupParent,
        external_pool_adapter_runtime_compatibility_verification::{
            runtime_compatibility_signature_challenge,
            ExternalPoolAdapterRuntimeCompatibilitySignerPayload,
            ExternalPoolAdapterRuntimeCompatibilitySigningHandoff,
            ExternalPoolAdapterRuntimeCompatibilitySigningHandoffRecordBinding,
            RUNTIME_COMPATIBILITY_SIGNER_PAYLOAD_SCHEMA,
            RUNTIME_COMPATIBILITY_SIGNING_HANDOFF_SCHEMA,
        },
    },
    store::{
        compute_external_pool_adapter_credential_reattestation::current_external_pool_adapter_projected_active_credential_reattestation_authority_on,
        compute_external_pool_adapter_provider_active_successor::historical_external_pool_adapter_atomic_activation_for_binding_on,
        compute_external_pool_adapter_registry::{
            current_external_pool_adapter_registry_provider_binding_authority_on,
            current_external_pool_adapter_registry_release_authority_on,
        },
        Store,
    },
};

use super::{
    error::ExternalPoolAdapterRuntimeCompatibilityVerificationStoreError as StoreError,
    read::{challenge_by_id_on, identifier, run_observation_by_challenge_on},
    run::require_fresh_current_authority,
};

impl Store {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn run_external_pool_adapter_runtime_compatibility_signing_handoff(
        &self,
        expected_registry_release_id: &str,
        challenge_id: &str,
        expected_challenge_digest: &str,
        provider_binding_id: &str,
        expected_provider_binding_digest: &str,
        expected_installation_receipt_id: &str,
        expected_installation_receipt_digest: &str,
        prepared: PreparedExternalPoolAdapterInstallation,
        cgroup_parent: &ExternalPoolAdapterSupervisorCgroupParent,
    ) -> std::result::Result<ExternalPoolAdapterRuntimeCompatibilitySigningHandoff, StoreError>
    {
        for value in [
            expected_registry_release_id,
            challenge_id,
            provider_binding_id,
            expected_installation_receipt_id,
        ] {
            identifier(value).map_err(StoreError::conflict)?;
        }
        let prepared = {
            let mut conn = self.conn().map_err(StoreError::storage)?;
            let tx = conn
                .transaction_with_behavior(TransactionBehavior::Immediate)
                .map_err(StoreError::storage)?;
            let challenge = challenge_by_id_on(&tx, challenge_id)
                .map_err(StoreError::storage)?
                .ok_or_else(|| {
                    StoreError::conflict(anyhow::anyhow!("V269 challenge was not found"))
                })?;
            let selected = &challenge.receipt.challenge;
            if challenge.receipt.challenge_digest != expected_challenge_digest
                || selected.registry_release.registry_release_id != expected_registry_release_id
            {
                return Err(StoreError::conflict(anyhow::anyhow!(
                    "V269 challenge and URL release roots are not exact"
                )));
            }
            let checked_at = now();
            require_fresh_current_authority(&tx, &challenge.receipt, &checked_at)
                .map_err(StoreError::classify_write)?;
            let authority = current_external_pool_adapter_registry_provider_binding_authority_on(
                &tx,
                provider_binding_id,
                prepared,
                &checked_at,
            )
            .map_err(StoreError::classify_write)?
            .ok_or_else(|| {
                StoreError::conflict(anyhow::anyhow!(
                    "V269 current Provider binding was not found"
                ))
            })?;
            let release = authority.release();
            let binding = authority.binding();
            let binding_material = &binding.binding;
            if binding.provider_binding_id != provider_binding_id
                || binding.provider_binding_digest != expected_provider_binding_digest
                || binding_material.registry_release_id != expected_registry_release_id
                || binding_material.registry_release_digest
                    != selected.registry_release.registry_release_digest
                || release != &selected.registry_release
                || binding_material.installation_receipt_id != expected_installation_receipt_id
                || binding_material.installation_receipt_digest
                    != expected_installation_receipt_digest
                || binding_material.installation_content_digest
                    != selected
                        .registry_release
                        .release
                        .installation_content_digest
            {
                return Err(StoreError::conflict(anyhow::anyhow!(
                    "V269 Provider binding execution roots are not exact"
                )));
            }
            let prepared = authority.into_prepared();
            tx.commit().map_err(StoreError::storage)?;
            prepared
        };

        finish_signing_handoff(
            self,
            expected_registry_release_id,
            challenge_id,
            expected_challenge_digest,
            prepared,
            cgroup_parent,
        )
    }

    /// Runs the unchanged provider-neutral V268 fixture runner under a durable V277 projected
    /// active subject. It never asks a registering V249 Provider-binding current wrapper to lie.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::store) fn run_external_pool_adapter_runtime_compatibility_signing_handoff_for_projected_active(
        &self,
        expected_registry_release_id: &str,
        challenge_id: &str,
        expected_challenge_digest: &str,
        provider_binding_id: &str,
        expected_provider_binding_digest: &str,
        expected_installation_receipt_id: &str,
        expected_installation_receipt_digest: &str,
        prepared: PreparedExternalPoolAdapterInstallation,
        cgroup_parent: &ExternalPoolAdapterSupervisorCgroupParent,
    ) -> std::result::Result<ExternalPoolAdapterRuntimeCompatibilitySigningHandoff, StoreError>
    {
        let prepared = {
            let mut conn = self.conn().map_err(StoreError::storage)?;
            let tx = conn
                .transaction_with_behavior(TransactionBehavior::Immediate)
                .map_err(StoreError::storage)?;
            let challenge = challenge_by_id_on(&tx, challenge_id)
                .map_err(StoreError::storage)?
                .ok_or_else(|| StoreError::conflict(anyhow::anyhow!("V269 challenge missing")))?;
            let selected = &challenge.receipt.challenge;
            let checked_at = now();
            require_fresh_current_authority(&tx, &challenge.receipt, &checked_at)
                .map_err(StoreError::classify_write)?;
            let activation = historical_external_pool_adapter_atomic_activation_for_binding_on(
                &tx,
                provider_binding_id,
                &checked_at,
            )
            .map_err(StoreError::classify_write)?
            .ok_or_else(|| {
                StoreError::conflict(anyhow::anyhow!(
                    "projected-active V269 lacks durable V277 history"
                ))
            })?;
            let root = &activation.activation_root().activation_root;
            let (credential_id, credential_digest) = tx
                .query_row(
                    "SELECT reattestation_receipt_id,reattestation_receipt_digest
                       FROM compute_external_pool_adapter_credential_reattestation_current
                      WHERE provider_binding_id=?1 AND current_status='verified_current'
                        AND provider_revision_status='witnessed_projected_active'",
                    params![provider_binding_id],
                    |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
                )
                .optional()
                .map_err(StoreError::storage)?
                .ok_or_else(|| {
                    StoreError::conflict(anyhow::anyhow!(
                        "projected-active V269 lacks current V253"
                    ))
                })?;
            current_external_pool_adapter_projected_active_credential_reattestation_authority_on(
                &tx,
                provider_binding_id,
                &credential_id,
                &credential_digest,
                &checked_at,
            )
            .map_err(StoreError::classify_write)?
            .ok_or_else(|| {
                StoreError::conflict(anyhow::anyhow!("projected-active V269 V253 reproof failed"))
            })?;
            current_external_pool_adapter_registry_release_authority_on(
                &tx,
                &root.registry_release_id,
                &root.registry_release_digest,
                &checked_at,
            )
            .map_err(StoreError::classify_write)?
            .ok_or_else(|| {
                StoreError::conflict(anyhow::anyhow!(
                    "projected-active V269 release is no longer current"
                ))
            })?;
            let binding = prepared.binding();
            if challenge.receipt.challenge_digest != expected_challenge_digest
                || selected.registry_release.registry_release_id != expected_registry_release_id
                || root.provider_binding_id != provider_binding_id
                || root.provider_binding_digest != expected_provider_binding_digest
                || root.registry_release_id != expected_registry_release_id
                || root.installation_receipt_id != expected_installation_receipt_id
                || root.installation_receipt_digest != expected_installation_receipt_digest
                || binding.provider_id != root.provider_id
                || binding.provider_owner_account_id != root.provider_owner_account_id
                || binding.adapter_id != root.logical_adapter_id
                || binding.installation_content_digest != root.installation_content_digest
            {
                return Err(StoreError::conflict(anyhow::anyhow!(
                    "projected-active V269 execution roots are not exact"
                )));
            }
            tx.commit().map_err(StoreError::storage)?;
            prepared
        };
        finish_signing_handoff(
            self,
            expected_registry_release_id,
            challenge_id,
            expected_challenge_digest,
            prepared,
            cgroup_parent,
        )
    }
}

fn finish_signing_handoff(
    store: &Store,
    expected_registry_release_id: &str,
    challenge_id: &str,
    expected_challenge_digest: &str,
    prepared: PreparedExternalPoolAdapterInstallation,
    cgroup_parent: &ExternalPoolAdapterSupervisorCgroupParent,
) -> std::result::Result<ExternalPoolAdapterRuntimeCompatibilitySigningHandoff, StoreError> {
    let private = store.run_external_pool_adapter_runtime_compatibility_verification_challenge(
        challenge_id,
        expected_challenge_digest,
        prepared,
        cgroup_parent,
    )?;
    let mut conn = store.conn().map_err(StoreError::storage)?;
    let tx = conn.transaction().map_err(StoreError::storage)?;
    let challenge = challenge_by_id_on(&tx, challenge_id)
        .map_err(StoreError::storage)?
        .ok_or_else(|| StoreError::storage(anyhow::anyhow!("V269 challenge disappeared")))?;
    let observation = run_observation_by_challenge_on(&tx, challenge_id)
        .map_err(StoreError::storage)?
        .ok_or_else(|| StoreError::storage(anyhow::anyhow!("V269 observation disappeared")))?;
    let selected = &challenge.receipt.challenge;
    if challenge.receipt.challenge_digest != expected_challenge_digest
        || selected.registry_release.registry_release_id != expected_registry_release_id
    {
        return Err(StoreError::conflict(anyhow::anyhow!(
            "V269 durable challenge roots drifted after execution"
        )));
    }
    require_fresh_current_authority(&tx, &challenge.receipt, &now())
        .map_err(StoreError::classify_write)?;
    let signer =
        runtime_compatibility_signature_challenge(&challenge.receipt, &observation.receipt)
            .map_err(StoreError::storage)?;
    if private.run_observation != observation.receipt || private.signature_challenge != signer {
        return Err(StoreError::storage(anyhow::anyhow!(
            "V269 durable signing handoff readback drifted"
        )));
    }
    let output = ExternalPoolAdapterRuntimeCompatibilitySigningHandoff {
        schema: RUNTIME_COMPATIBILITY_SIGNING_HANDOFF_SCHEMA,
        record_binding: ExternalPoolAdapterRuntimeCompatibilitySigningHandoffRecordBinding {
            run_observation_id: observation.receipt.run_observation_id.clone(),
            run_observation_digest: observation.receipt.run_observation_digest.clone(),
        },
        signer_payload: ExternalPoolAdapterRuntimeCompatibilitySignerPayload {
            schema: RUNTIME_COMPATIBILITY_SIGNER_PAYLOAD_SCHEMA,
            signature_algorithm: signer.signature_algorithm,
            sandbox_verifier_key_record_id: selected.sandbox_verifier_key_record_id.clone(),
            sandbox_verifier_key_record_digest: selected.sandbox_verifier_key_record_digest.clone(),
            sandbox_verifier_key_id: selected.sandbox_verifier_key_id.clone(),
            signature_message_base64: signer.signature_message_base64,
            signature_message_digest: signer.signature_message_digest,
            expires_at: selected.expires_at.clone(),
        },
        replayed: private.replayed,
    };
    tx.commit().map_err(StoreError::storage)?;
    Ok(output)
}

fn now() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Nanos, true)
}
