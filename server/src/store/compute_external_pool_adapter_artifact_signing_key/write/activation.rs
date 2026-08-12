use anyhow::{bail, Result};
use rusqlite::{params, TransactionBehavior};

use crate::{
    compute_federation::external_pool_adapter_artifact_signing_key::{
        canonical_signing_key_activation_json_and_digest, signing_key_activation_material_digest,
        validate_signing_key_activation_receipt, ExternalPoolAdapterArtifactSigningKeyActivation,
        ExternalPoolAdapterArtifactSigningKeyActivationReceipt,
        SIGNING_KEY_ACTIVATION_CONFIRMATION, SIGNING_KEY_ACTIVATION_RECEIPT_SCHEMA,
        SIGNING_KEY_ACTOR_KIND, SIGNING_KEY_ADAPTER_EFFECT_NONE, SIGNING_KEY_ARTIFACT_EFFECT_NONE,
        SIGNING_KEY_CANONICALIZATION, SIGNING_KEY_DIGEST_ALGORITHM, SIGNING_KEY_ROUTE_EFFECT_NONE,
        SIGNING_KEY_STATUS_ACTIVE,
    },
    store::{new_id, Store},
};

use super::super::{
    read::{
        activation_by_idempotency_on, activation_by_key_on, currentness_on, record_by_id_on,
        revocation_by_key_on, validate_digest, validate_exact,
    },
    types::{
        ActivateExternalPoolAdapterArtifactSigningKey,
        ExternalPoolAdapterArtifactSigningKeyActivationWriteReceipt, StoredSigningKeyActivation,
        StoredSigningKeyRecord,
    },
};
use super::common::now;

impl Store {
    pub(crate) fn activate_external_pool_adapter_artifact_signing_key(
        &self,
        input: ActivateExternalPoolAdapterArtifactSigningKey,
    ) -> Result<ExternalPoolAdapterArtifactSigningKeyActivationWriteReceipt> {
        validate_input(&input)?;
        let mut connection = self.conn()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;

        if let Some(stored) = activation_by_idempotency_on(
            &transaction,
            &input.idempotency_scope,
            &input.idempotency_key,
        )? {
            let record = record_by_id_on(&transaction, &stored.receipt.activation.key_record_id)?
                .ok_or_else(|| anyhow::anyhow!("signing-key activation lost its root"))?;
            ensure_replay(&stored, &record, &input)?;
            let receipt = write_receipt(&record, &stored, true);
            transaction.commit()?;
            return Ok(receipt);
        }

        let record = record_by_id_on(&transaction, &input.key_record_id)?
            .ok_or_else(|| anyhow::anyhow!("signing-key record was not found"))?;
        ensure_exact_root(&record, &input)?;
        if activation_by_key_on(&transaction, &input.key_record_id)?.is_some()
            || revocation_by_key_on(&transaction, &input.key_record_id)?.is_some()
        {
            bail!("signing-key record is not pending activation");
        }

        let timestamp = now();
        let registration = &record.record.registration;
        let activation = ExternalPoolAdapterArtifactSigningKeyActivation {
            key_record_id: record.record.key_record_id.clone(),
            key_record_digest: record.record.key_record_digest.clone(),
            key_id: registration.key_id.clone(),
            source_operator: registration.source_operator.clone(),
            actor_kind: SIGNING_KEY_ACTOR_KIND.to_string(),
            activated_by_admin_user_id: input.activated_by_admin_user_id,
            confirmation: input.confirmation,
            idempotency_scope: input.idempotency_scope,
            idempotency_key: input.idempotency_key,
            occurred_at: timestamp.clone(),
            recorded_at: timestamp,
            currentness_effect: SIGNING_KEY_STATUS_ACTIVE.to_string(),
            artifact_signature_effect: SIGNING_KEY_ARTIFACT_EFFECT_NONE.to_string(),
            adapter_effect: SIGNING_KEY_ADAPTER_EFFECT_NONE.to_string(),
            route_effect: SIGNING_KEY_ROUTE_EFFECT_NONE.to_string(),
        };
        let mut receipt = ExternalPoolAdapterArtifactSigningKeyActivationReceipt {
            schema: SIGNING_KEY_ACTIVATION_RECEIPT_SCHEMA.to_string(),
            activation_receipt_id: new_id("external_pool_adapter_artifact_signing_key_activation"),
            activation_receipt_digest: String::new(),
            activation_material_digest: signing_key_activation_material_digest(&activation)?,
            canonicalization: SIGNING_KEY_CANONICALIZATION.to_string(),
            digest_algorithm: SIGNING_KEY_DIGEST_ALGORITHM.to_string(),
            activation,
        };
        receipt.activation_receipt_digest =
            canonical_signing_key_activation_json_and_digest(&receipt)?.1;
        let (receipt_json, digest) = canonical_signing_key_activation_json_and_digest(&receipt)?;
        if digest != receipt.activation_receipt_digest {
            bail!("signing-key activation digest changed before persistence");
        }
        validate_signing_key_activation_receipt(&receipt)?;
        insert_activation(&transaction, &receipt, &receipt_json)?;
        let stored = activation_by_key_on(&transaction, &input.key_record_id)?
            .ok_or_else(|| anyhow::anyhow!("signing-key activation is absent after insert"))?;
        if stored.receipt != receipt || stored.receipt_json != receipt_json {
            bail!("signing-key activation changed during exact readback");
        }
        if currentness_on(&transaction, &input.key_record_id)?
            .map(|value| value.current_status)
            .as_deref()
            != Some(SIGNING_KEY_STATUS_ACTIVE)
        {
            bail!("signing-key activation currentness is not exact");
        }
        let result = write_receipt(&record, &stored, false);
        transaction.commit()?;
        Ok(result)
    }
}

fn insert_activation(
    transaction: &rusqlite::Transaction<'_>,
    receipt: &ExternalPoolAdapterArtifactSigningKeyActivationReceipt,
    receipt_json: &str,
) -> Result<()> {
    let activation = &receipt.activation;
    transaction.execute(
        "INSERT INTO compute_external_pool_adapter_artifact_signing_key_activations (
            activation_receipt_id, activation_receipt_schema, activation_receipt_digest,
            activation_receipt_json, activation_material_digest, canonicalization,
            digest_algorithm, key_record_id, key_record_digest, key_id, source_operator,
            actor_kind, activated_by_admin_user_id, confirmation, idempotency_scope,
            idempotency_key, occurred_at, recorded_at, currentness_effect,
            artifact_signature_effect, adapter_effect, route_effect
         ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,
                   ?17,?18,?19,?20,?21,?22)",
        params![
            receipt.activation_receipt_id,
            receipt.schema,
            receipt.activation_receipt_digest,
            receipt_json,
            receipt.activation_material_digest,
            receipt.canonicalization,
            receipt.digest_algorithm,
            activation.key_record_id,
            activation.key_record_digest,
            activation.key_id,
            activation.source_operator,
            activation.actor_kind,
            activation.activated_by_admin_user_id,
            activation.confirmation,
            activation.idempotency_scope,
            activation.idempotency_key,
            activation.occurred_at,
            activation.recorded_at,
            activation.currentness_effect,
            activation.artifact_signature_effect,
            activation.adapter_effect,
            activation.route_effect,
        ],
    )?;
    Ok(())
}

fn ensure_exact_root(
    record: &StoredSigningKeyRecord,
    input: &ActivateExternalPoolAdapterArtifactSigningKey,
) -> Result<()> {
    if record.record.key_record_digest != input.expected_key_record_digest {
        bail!("signing-key activation expected root digest is stale");
    }
    if record.record.registration.created_by_admin_user_id == input.activated_by_admin_user_id {
        bail!("signing-key activation requires a distinct platform administrator");
    }
    Ok(())
}

fn ensure_replay(
    stored: &StoredSigningKeyActivation,
    record: &StoredSigningKeyRecord,
    input: &ActivateExternalPoolAdapterArtifactSigningKey,
) -> Result<()> {
    let activation = &stored.receipt.activation;
    ensure_exact_root(record, input)?;
    if activation.key_record_id != input.key_record_id
        || activation.key_record_digest != input.expected_key_record_digest
        || activation.activated_by_admin_user_id != input.activated_by_admin_user_id
        || activation.confirmation != input.confirmation
        || activation.idempotency_scope != input.idempotency_scope
        || activation.idempotency_key != input.idempotency_key
    {
        bail!("signing-key activation replay conflicts with immutable history");
    }
    Ok(())
}

fn write_receipt(
    record: &StoredSigningKeyRecord,
    activation: &StoredSigningKeyActivation,
    replayed: bool,
) -> ExternalPoolAdapterArtifactSigningKeyActivationWriteReceipt {
    ExternalPoolAdapterArtifactSigningKeyActivationWriteReceipt {
        key_record: record.summary(),
        activation: activation.summary(),
        replayed,
    }
}

fn validate_input(input: &ActivateExternalPoolAdapterArtifactSigningKey) -> Result<()> {
    validate_exact(&input.key_record_id, "key record ID", 160)?;
    validate_digest(
        &input.expected_key_record_digest,
        "expected key record digest",
    )?;
    validate_exact(
        &input.activated_by_admin_user_id,
        "activation actor ID",
        160,
    )?;
    validate_exact(
        &input.idempotency_scope,
        "activation idempotency scope",
        200,
    )?;
    validate_exact(&input.idempotency_key, "activation idempotency key", 160)?;
    if input.confirmation != SIGNING_KEY_ACTIVATION_CONFIRMATION {
        bail!("signing-key activation confirmation is not exact");
    }
    Ok(())
}
