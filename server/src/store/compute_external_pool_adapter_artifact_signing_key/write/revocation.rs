use anyhow::{bail, Result};
use rusqlite::{params, TransactionBehavior};

use crate::{
    compute_federation::external_pool_adapter_artifact_signing_key::{
        canonical_signing_key_revocation_json_and_digest, signing_key_revocation_material_digest,
        validate_signing_key_revocation_receipt, ExternalPoolAdapterArtifactSigningKeyRevocation,
        ExternalPoolAdapterArtifactSigningKeyRevocationReceipt, SIGNING_KEY_ACTOR_KIND,
        SIGNING_KEY_ADAPTER_EFFECT_NONE, SIGNING_KEY_ARTIFACT_EFFECT_NONE,
        SIGNING_KEY_CANONICALIZATION, SIGNING_KEY_DIGEST_ALGORITHM,
        SIGNING_KEY_REVOCATION_CONFIRMATION, SIGNING_KEY_REVOCATION_RECEIPT_SCHEMA,
        SIGNING_KEY_ROUTE_EFFECT_NONE, SIGNING_KEY_STATUS_ACTIVE, SIGNING_KEY_STATUS_REVOKED,
    },
    store::{new_id, Store},
};

use super::super::{
    read::{
        activation_by_key_on, currentness_on, record_by_id_on, revocation_by_idempotency_on,
        revocation_by_key_on, validate_digest, validate_exact,
    },
    types::{
        ExternalPoolAdapterArtifactSigningKeyRevocationWriteReceipt,
        RevokeExternalPoolAdapterArtifactSigningKey, StoredSigningKeyRecord,
        StoredSigningKeyRevocation,
    },
};
use super::common::now;

impl Store {
    pub(crate) fn revoke_external_pool_adapter_artifact_signing_key(
        &self,
        input: RevokeExternalPoolAdapterArtifactSigningKey,
    ) -> Result<ExternalPoolAdapterArtifactSigningKeyRevocationWriteReceipt> {
        validate_input(&input)?;
        let mut connection = self.conn()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;

        if let Some(stored) = revocation_by_idempotency_on(
            &transaction,
            &input.idempotency_scope,
            &input.idempotency_key,
        )? {
            let record = record_by_id_on(&transaction, &stored.receipt.revocation.key_record_id)?
                .ok_or_else(|| anyhow::anyhow!("signing-key revocation lost its root"))?;
            ensure_replay(&stored, &record, &input)?;
            let receipt = write_receipt(&record, &stored, true);
            transaction.commit()?;
            return Ok(receipt);
        }

        let record = record_by_id_on(&transaction, &input.key_record_id)?
            .ok_or_else(|| anyhow::anyhow!("signing-key record was not found"))?;
        if record.record.key_record_digest != input.expected_key_record_digest {
            bail!("signing-key revocation expected root digest is stale");
        }
        if revocation_by_key_on(&transaction, &input.key_record_id)?.is_some()
            || activation_by_key_on(&transaction, &input.key_record_id)?.is_none()
            || currentness_on(&transaction, &input.key_record_id)?
                .map(|value| value.current_status)
                .as_deref()
                != Some(SIGNING_KEY_STATUS_ACTIVE)
        {
            bail!("signing-key record is not active");
        }

        let timestamp = now();
        let registration = &record.record.registration;
        let revocation = ExternalPoolAdapterArtifactSigningKeyRevocation {
            key_record_id: record.record.key_record_id.clone(),
            key_record_digest: record.record.key_record_digest.clone(),
            key_id: registration.key_id.clone(),
            source_operator: registration.source_operator.clone(),
            actor_kind: SIGNING_KEY_ACTOR_KIND.to_string(),
            revoked_by_admin_user_id: input.revoked_by_admin_user_id,
            reason: input.reason,
            confirmation: input.confirmation,
            idempotency_scope: input.idempotency_scope,
            idempotency_key: input.idempotency_key,
            occurred_at: timestamp.clone(),
            recorded_at: timestamp,
            currentness_effect: SIGNING_KEY_STATUS_REVOKED.to_string(),
            artifact_signature_effect: SIGNING_KEY_ARTIFACT_EFFECT_NONE.to_string(),
            adapter_effect: SIGNING_KEY_ADAPTER_EFFECT_NONE.to_string(),
            route_effect: SIGNING_KEY_ROUTE_EFFECT_NONE.to_string(),
        };
        let mut receipt = ExternalPoolAdapterArtifactSigningKeyRevocationReceipt {
            schema: SIGNING_KEY_REVOCATION_RECEIPT_SCHEMA.to_string(),
            revocation_receipt_id: new_id("external_pool_adapter_artifact_signing_key_revocation"),
            revocation_receipt_digest: String::new(),
            revocation_material_digest: signing_key_revocation_material_digest(&revocation)?,
            canonicalization: SIGNING_KEY_CANONICALIZATION.to_string(),
            digest_algorithm: SIGNING_KEY_DIGEST_ALGORITHM.to_string(),
            revocation,
        };
        receipt.revocation_receipt_digest =
            canonical_signing_key_revocation_json_and_digest(&receipt)?.1;
        let (receipt_json, digest) = canonical_signing_key_revocation_json_and_digest(&receipt)?;
        if digest != receipt.revocation_receipt_digest {
            bail!("signing-key revocation digest changed before persistence");
        }
        validate_signing_key_revocation_receipt(&receipt)?;
        insert_revocation(&transaction, &receipt, &receipt_json)?;
        let stored = revocation_by_key_on(&transaction, &input.key_record_id)?
            .ok_or_else(|| anyhow::anyhow!("signing-key revocation is absent after insert"))?;
        if stored.receipt != receipt || stored.receipt_json != receipt_json {
            bail!("signing-key revocation changed during exact readback");
        }
        if currentness_on(&transaction, &input.key_record_id)?
            .map(|value| value.current_status)
            .as_deref()
            != Some(SIGNING_KEY_STATUS_REVOKED)
        {
            bail!("signing-key revocation currentness is not exact");
        }
        let result = write_receipt(&record, &stored, false);
        transaction.commit()?;
        Ok(result)
    }
}

fn insert_revocation(
    transaction: &rusqlite::Transaction<'_>,
    receipt: &ExternalPoolAdapterArtifactSigningKeyRevocationReceipt,
    receipt_json: &str,
) -> Result<()> {
    let revocation = &receipt.revocation;
    transaction.execute(
        "INSERT INTO compute_external_pool_adapter_artifact_signing_key_revocations (
            revocation_receipt_id, revocation_receipt_schema, revocation_receipt_digest,
            revocation_receipt_json, revocation_material_digest, canonicalization,
            digest_algorithm, key_record_id, key_record_digest, key_id, source_operator,
            actor_kind, revoked_by_admin_user_id, reason, confirmation, idempotency_scope,
            idempotency_key, occurred_at, recorded_at, currentness_effect,
            artifact_signature_effect, adapter_effect, route_effect
         ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,
                   ?17,?18,?19,?20,?21,?22,?23)",
        params![
            receipt.revocation_receipt_id,
            receipt.schema,
            receipt.revocation_receipt_digest,
            receipt_json,
            receipt.revocation_material_digest,
            receipt.canonicalization,
            receipt.digest_algorithm,
            revocation.key_record_id,
            revocation.key_record_digest,
            revocation.key_id,
            revocation.source_operator,
            revocation.actor_kind,
            revocation.revoked_by_admin_user_id,
            revocation.reason,
            revocation.confirmation,
            revocation.idempotency_scope,
            revocation.idempotency_key,
            revocation.occurred_at,
            revocation.recorded_at,
            revocation.currentness_effect,
            revocation.artifact_signature_effect,
            revocation.adapter_effect,
            revocation.route_effect,
        ],
    )?;
    Ok(())
}

fn ensure_replay(
    stored: &StoredSigningKeyRevocation,
    record: &StoredSigningKeyRecord,
    input: &RevokeExternalPoolAdapterArtifactSigningKey,
) -> Result<()> {
    let revocation = &stored.receipt.revocation;
    if record.record.key_record_digest != input.expected_key_record_digest
        || revocation.key_record_id != input.key_record_id
        || revocation.revoked_by_admin_user_id != input.revoked_by_admin_user_id
        || revocation.reason != input.reason
        || revocation.confirmation != input.confirmation
        || revocation.idempotency_scope != input.idempotency_scope
        || revocation.idempotency_key != input.idempotency_key
    {
        bail!("signing-key revocation replay conflicts with immutable history");
    }
    Ok(())
}

fn write_receipt(
    record: &StoredSigningKeyRecord,
    revocation: &StoredSigningKeyRevocation,
    replayed: bool,
) -> ExternalPoolAdapterArtifactSigningKeyRevocationWriteReceipt {
    ExternalPoolAdapterArtifactSigningKeyRevocationWriteReceipt {
        key_record: record.summary(),
        revocation: revocation.summary(),
        replayed,
    }
}

fn validate_input(input: &RevokeExternalPoolAdapterArtifactSigningKey) -> Result<()> {
    validate_exact(&input.key_record_id, "key record ID", 160)?;
    validate_digest(
        &input.expected_key_record_digest,
        "expected key record digest",
    )?;
    validate_exact(&input.revoked_by_admin_user_id, "revocation actor ID", 160)?;
    validate_exact(
        &input.idempotency_scope,
        "revocation idempotency scope",
        200,
    )?;
    validate_exact(&input.idempotency_key, "revocation idempotency key", 160)?;
    validate_exact(&input.reason, "revocation reason", 2_000)?;
    if input.reason.chars().count() < 8 {
        bail!("signing-key revocation reason is too short");
    }
    if input.confirmation != SIGNING_KEY_REVOCATION_CONFIRMATION {
        bail!("signing-key revocation confirmation is not exact");
    }
    Ok(())
}
