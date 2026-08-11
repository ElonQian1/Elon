use anyhow::{bail, Result};
use rusqlite::{params, TransactionBehavior};
use sha2::{Digest, Sha256};

use crate::{
    compute_federation::external_pool_onboarding::{
        canonical_external_pool_onboarding_request_json_and_digest,
        validate_external_pool_onboarding_request_envelope,
    },
    compute_plugin_sharing_directive::canonical_compute_plugin_ijson_and_sha256,
};

use super::{
    read::{request_by_id_on, request_by_idempotency_on, request_receipt},
    types::{ExternalPoolOnboardingRequestReceipt, SubmitExternalPoolOnboardingRequest},
};
use crate::store::{compute_provider_registry::validate_compute_provider_contract, Store};

const MAX_TARGET_PROVIDER_JSON_BYTES: usize = 256 * 1024;

impl Store {
    pub(crate) fn submit_external_pool_onboarding_request(
        &self,
        input: SubmitExternalPoolOnboardingRequest,
    ) -> Result<ExternalPoolOnboardingRequestReceipt> {
        validate_external_pool_onboarding_request_envelope(&input.request)?;
        validate_idempotency(&input)?;
        validate_compute_provider_contract(&input.request.request.target_provider)?;
        let (request_json, request_digest) =
            canonical_external_pool_onboarding_request_json_and_digest(&input.request)?;
        if request_digest != input.request.request_digest {
            bail!("external-pool onboarding request digest changed before persistence");
        }
        let target_provider_registry_json =
            serde_json::to_string(&input.request.request.target_provider)?;
        let target_provider_digest =
            hex::encode(Sha256::digest(target_provider_registry_json.as_bytes()));
        let (target_provider_jcs, _) = canonical_compute_plugin_ijson_and_sha256(
            &input.request.request.target_provider,
            MAX_TARGET_PROVIDER_JSON_BYTES,
        )?;

        let mut connection = self.conn()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(stored) = request_by_idempotency_on(
            &transaction,
            &input.idempotency_scope,
            &input.idempotency_key,
        )? {
            ensure_replay(&stored, &input)?;
            let receipt = request_receipt(stored, true);
            transaction.commit()?;
            return Ok(receipt);
        }
        if let Some(stored) = request_by_id_on(&transaction, &input.request.request_id)? {
            ensure_replay(&stored, &input)?;
            let receipt = request_receipt(stored, true);
            transaction.commit()?;
            return Ok(receipt);
        }

        let request = &input.request.request;
        let provider = &request.target_provider;
        let adapter = &request.adapter_intent;
        let credential = &request.credential_intent;
        transaction.execute(
            "INSERT INTO compute_external_pool_onboarding_requests (
                request_id, request_schema, request_digest, request_json,
                canonicalization, digest_algorithm, target_provider_policy_revision,
                target_provider_digest, target_provider_jcs, target_provider_registry_json,
                provider_id, provider_kind,
                provider_owner_account_id, settlement_account_id, adapter_id,
                adapter_release_version, adapter_config_revision, adapter_config_digest,
                non_bearer_credential_ref, credential_hint, external_evidence_ref,
                external_evidence_sha256, confirmation, owner_note,
                requested_by_owner_user_id, requested_at,
                status, reviewed_by_user_id, reviewed_at, canceled_by_owner_user_id,
                canceled_at, applied_by_user_id, applied_at, idempotency_scope,
                idempotency_key, created_at, updated_at
             ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, 1, ?7, ?8, ?9, ?10, ?11, ?12, ?13,
                ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25,
                'submitted', NULL, NULL, NULL, NULL, NULL, NULL, ?26, ?27, ?25, ?25
             )",
            params![
                input.request.request_id,
                input.request.schema,
                input.request.request_digest,
                request_json,
                input.request.canonicalization,
                input.request.digest_algorithm,
                target_provider_digest,
                target_provider_jcs,
                target_provider_registry_json,
                provider.provider_id,
                provider.provider_kind,
                provider.owner_account_id,
                provider.settlement_account_id,
                adapter.expected_adapter_id,
                adapter.expected_release_version,
                adapter.expected_config_revision,
                adapter.expected_config_digest,
                credential.non_bearer_credential_ref,
                credential.credential_hint,
                request.external_evidence_ref,
                request.external_evidence_sha256,
                request.confirmation,
                request.owner_note,
                request.requested_by_owner_user_id,
                request.submitted_at,
                input.idempotency_scope,
                input.idempotency_key,
            ],
        )?;
        let stored =
            request_by_id_on(&transaction, &input.request.request_id)?.ok_or_else(|| {
                anyhow::anyhow!("external-pool onboarding request is absent after insert")
            })?;
        let receipt = request_receipt(stored, false);
        transaction.commit()?;
        Ok(receipt)
    }
}

fn validate_idempotency(input: &SubmitExternalPoolOnboardingRequest) -> Result<()> {
    super::review::validate_exact(&input.idempotency_scope, "request idempotency scope", 200)?;
    super::review::validate_exact(&input.idempotency_key, "request idempotency key", 160)?;
    if input.idempotency_key != input.request.request.idempotency_key {
        bail!("external-pool owner request and Store idempotency key differ");
    }
    Ok(())
}

fn ensure_replay(
    stored: &super::types::StoredRequest,
    input: &SubmitExternalPoolOnboardingRequest,
) -> Result<()> {
    if stored.envelope.request_digest != input.request.request_digest
        || stored.idempotency_scope != input.idempotency_scope
        || stored.idempotency_key != input.idempotency_key
    {
        bail!("external-pool onboarding request replay conflicts with immutable history");
    }
    Ok(())
}
