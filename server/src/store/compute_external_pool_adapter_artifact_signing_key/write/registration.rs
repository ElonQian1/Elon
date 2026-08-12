use anyhow::{bail, Result};
use rusqlite::{params, TransactionBehavior};

use crate::{
    compute_federation::external_pool_adapter_artifact_signing_key::{
        canonical_signing_key_record_json_and_digest, signing_key_registration_material_digest,
        validate_signing_key_record, ExternalPoolAdapterArtifactSigningKeyRecord,
        ExternalPoolAdapterArtifactSigningKeyRegistration, SIGNING_KEY_ACTOR_KIND,
        SIGNING_KEY_ADAPTER_EFFECT_NONE, SIGNING_KEY_ALGORITHM, SIGNING_KEY_ARTIFACT_EFFECT_NONE,
        SIGNING_KEY_CANONICALIZATION, SIGNING_KEY_DIGEST_ALGORITHM, SIGNING_KEY_RECORD_SCHEMA,
        SIGNING_KEY_REGISTRATION_CONFIRMATION, SIGNING_KEY_ROUTE_EFFECT_NONE,
        SIGNING_KEY_STATUS_PENDING_ACTIVATION,
    },
    store::{new_id, Store},
};

use super::super::{
    read::{record_by_idempotency_on, record_by_key_id_on, validate_digest, validate_exact},
    types::{
        ExternalPoolAdapterArtifactSigningKeyRegistrationWriteReceipt,
        RegisterExternalPoolAdapterArtifactSigningKey, StoredSigningKeyRecord,
    },
};
use super::common::now;

impl Store {
    pub(crate) fn register_external_pool_adapter_artifact_signing_key(
        &self,
        input: RegisterExternalPoolAdapterArtifactSigningKey,
    ) -> Result<ExternalPoolAdapterArtifactSigningKeyRegistrationWriteReceipt> {
        validate_input(&input)?;
        let mut connection = self.conn()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;

        if let Some(stored) = record_by_idempotency_on(
            &transaction,
            &input.idempotency_scope,
            &input.idempotency_key,
        )? {
            ensure_replay(&stored, &input)?;
            let receipt = ExternalPoolAdapterArtifactSigningKeyRegistrationWriteReceipt {
                key_record: stored.summary(),
                replayed: true,
            };
            transaction.commit()?;
            return Ok(receipt);
        }
        if record_by_key_id_on(&transaction, &input.key_id)?.is_some() {
            bail!("RSA public key is already registered under another immutable request");
        }

        let timestamp = now();
        let registration = ExternalPoolAdapterArtifactSigningKeyRegistration {
            source_operator: input.source_operator,
            key_id: input.key_id,
            algorithm: SIGNING_KEY_ALGORITHM.to_string(),
            public_key_pem: input.public_key_pem,
            actor_kind: SIGNING_KEY_ACTOR_KIND.to_string(),
            created_by_admin_user_id: input.created_by_admin_user_id,
            confirmation: input.confirmation,
            idempotency_scope: input.idempotency_scope,
            idempotency_key: input.idempotency_key,
            created_at: timestamp.clone(),
            recorded_at: timestamp,
            currentness_effect: SIGNING_KEY_STATUS_PENDING_ACTIVATION.to_string(),
            artifact_signature_effect: SIGNING_KEY_ARTIFACT_EFFECT_NONE.to_string(),
            adapter_effect: SIGNING_KEY_ADAPTER_EFFECT_NONE.to_string(),
            route_effect: SIGNING_KEY_ROUTE_EFFECT_NONE.to_string(),
        };
        let mut record = ExternalPoolAdapterArtifactSigningKeyRecord {
            schema: SIGNING_KEY_RECORD_SCHEMA.to_string(),
            key_record_id: new_id("external_pool_adapter_artifact_signing_key"),
            key_record_digest: String::new(),
            registration_material_digest: signing_key_registration_material_digest(&registration)?,
            canonicalization: SIGNING_KEY_CANONICALIZATION.to_string(),
            digest_algorithm: SIGNING_KEY_DIGEST_ALGORITHM.to_string(),
            registration,
        };
        record.key_record_digest = canonical_signing_key_record_json_and_digest(&record)?.1;
        let (record_json, digest) = canonical_signing_key_record_json_and_digest(&record)?;
        if digest != record.key_record_digest {
            bail!("signing-key record digest changed before persistence");
        }
        validate_signing_key_record(&record)?;
        insert_record(&transaction, &record, &record_json)?;
        let stored = super::super::read::record_by_id_on(&transaction, &record.key_record_id)?
            .ok_or_else(|| anyhow::anyhow!("signing-key record is absent after insert"))?;
        if stored.record != record || stored.record_json != record_json {
            bail!("signing-key record changed during exact readback");
        }
        let receipt = ExternalPoolAdapterArtifactSigningKeyRegistrationWriteReceipt {
            key_record: stored.summary(),
            replayed: false,
        };
        transaction.commit()?;
        Ok(receipt)
    }
}

fn insert_record(
    transaction: &rusqlite::Transaction<'_>,
    record: &ExternalPoolAdapterArtifactSigningKeyRecord,
    record_json: &str,
) -> Result<()> {
    let registration = &record.registration;
    transaction.execute(
        "INSERT INTO compute_external_pool_adapter_artifact_signing_keys (
            key_record_id, key_record_schema, key_record_digest, key_record_json,
            registration_material_digest, canonicalization, digest_algorithm,
            source_operator, key_id, algorithm, public_key_pem, actor_kind,
            created_by_admin_user_id, confirmation, idempotency_scope, idempotency_key,
            created_at, recorded_at, currentness_effect, artifact_signature_effect,
            adapter_effect, route_effect
         ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,
                   ?17,?18,?19,?20,?21,?22)",
        params![
            record.key_record_id,
            record.schema,
            record.key_record_digest,
            record_json,
            record.registration_material_digest,
            record.canonicalization,
            record.digest_algorithm,
            registration.source_operator,
            registration.key_id,
            registration.algorithm,
            registration.public_key_pem,
            registration.actor_kind,
            registration.created_by_admin_user_id,
            registration.confirmation,
            registration.idempotency_scope,
            registration.idempotency_key,
            registration.created_at,
            registration.recorded_at,
            registration.currentness_effect,
            registration.artifact_signature_effect,
            registration.adapter_effect,
            registration.route_effect,
        ],
    )?;
    Ok(())
}

fn ensure_replay(
    stored: &StoredSigningKeyRecord,
    input: &RegisterExternalPoolAdapterArtifactSigningKey,
) -> Result<()> {
    let registration = &stored.record.registration;
    if registration.source_operator != input.source_operator
        || registration.key_id != input.key_id
        || registration.public_key_pem != input.public_key_pem
        || registration.created_by_admin_user_id != input.created_by_admin_user_id
        || registration.confirmation != input.confirmation
        || registration.idempotency_scope != input.idempotency_scope
        || registration.idempotency_key != input.idempotency_key
    {
        bail!("signing-key registration replay conflicts with immutable history");
    }
    Ok(())
}

fn validate_input(input: &RegisterExternalPoolAdapterArtifactSigningKey) -> Result<()> {
    validate_exact(&input.source_operator, "source operator", 160)?;
    validate_digest(&input.key_id, "key ID")?;
    validate_exact(
        &input.created_by_admin_user_id,
        "registration actor ID",
        160,
    )?;
    validate_exact(
        &input.idempotency_scope,
        "registration idempotency scope",
        200,
    )?;
    validate_exact(&input.idempotency_key, "registration idempotency key", 160)?;
    if input.public_key_pem.is_empty() || input.public_key_pem.len() > 16 * 1024 {
        bail!("normalized RSA public key PEM length is invalid");
    }
    if input.confirmation != SIGNING_KEY_REGISTRATION_CONFIRMATION {
        bail!("signing-key registration confirmation is not exact");
    }
    Ok(())
}
