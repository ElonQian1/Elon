use chrono::{SecondsFormat, Utc};
use rusqlite::TransactionBehavior;

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
        compute_external_pool_adapter_registry::current_external_pool_adapter_registry_provider_binding_authority_on,
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

        let private = self.run_external_pool_adapter_runtime_compatibility_verification_challenge(
            challenge_id,
            expected_challenge_digest,
            prepared,
            cgroup_parent,
        )?;
        let mut conn = self.conn().map_err(StoreError::storage)?;
        let tx = conn.transaction().map_err(StoreError::storage)?;
        let challenge = challenge_by_id_on(&tx, challenge_id)
            .map_err(StoreError::storage)?
            .ok_or_else(|| {
                StoreError::storage(anyhow::anyhow!("V269 durable challenge disappeared"))
            })?;
        let observation = run_observation_by_challenge_on(&tx, challenge_id)
            .map_err(StoreError::storage)?
            .ok_or_else(|| {
                StoreError::storage(anyhow::anyhow!("V269 durable observation disappeared"))
            })?;
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
                sandbox_verifier_key_record_digest: selected
                    .sandbox_verifier_key_record_digest
                    .clone(),
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
}

fn now() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Nanos, true)
}
