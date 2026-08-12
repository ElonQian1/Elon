use anyhow::{bail, Result};
use rusqlite::{params, OptionalExtension, TransactionBehavior};

use crate::{
    compute_federation::external_pool_adapter_scanner_key::{
        scanner_key_record_json_and_digest, scanner_key_registration_digest,
        validate_scanner_key_record, ExternalPoolAdapterScannerKeyRecord,
        ExternalPoolAdapterScannerKeyRegistration, SCANNER_KEY_ACTOR_KIND, SCANNER_KEY_ALGORITHM,
        SCANNER_KEY_CANONICALIZATION, SCANNER_KEY_DIGEST_ALGORITHM, SCANNER_KEY_NO_EFFECT,
        SCANNER_KEY_RECORD_SCHEMA, SCANNER_KEY_REGISTER_CONFIRMATION, SCANNER_KEY_STATUS_PENDING,
    },
    store::{new_id, Store},
};

use super::super::{
    read::{
        record_by_id_on, record_by_idempotency_on, record_by_key_id_on, validate_digest,
        validate_exact,
    },
    types::{
        ExternalPoolAdapterScannerKeyRegistrationWriteReceipt,
        RegisterExternalPoolAdapterScannerKey, StoredScannerKeyRecord,
    },
};
use super::now;

impl Store {
    pub(crate) fn register_external_pool_adapter_scanner_key(
        &self,
        input: RegisterExternalPoolAdapterScannerKey,
    ) -> Result<ExternalPoolAdapterScannerKeyRegistrationWriteReceipt> {
        validate_input(&input)?;
        let mut connection = self.conn()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(stored) = record_by_idempotency_on(
            &transaction,
            &input.idempotency_scope,
            &input.idempotency_key,
        )? {
            ensure_replay(&stored, &input)?;
            let result = result(&stored, true);
            transaction.commit()?;
            return Ok(result);
        }
        if record_by_key_id_on(&transaction, &input.key_id)?.is_some()
            || transaction
                .query_row(
                    "SELECT 1 FROM compute_external_pool_adapter_artifact_signing_keys WHERE key_id=?1",
                    [&input.key_id],
                    |_| Ok(()),
                )
                .optional()?
                .is_some()
        {
            bail!("scanner key is already assigned to another immutable trust role");
        }
        let timestamp = now();
        let registration = ExternalPoolAdapterScannerKeyRegistration {
            scanner_operator: input.scanner_operator,
            scanner_product: input.scanner_product,
            key_id: input.key_id,
            algorithm: SCANNER_KEY_ALGORITHM.to_string(),
            public_key_pem: input.public_key_pem,
            actor_kind: SCANNER_KEY_ACTOR_KIND.to_string(),
            created_by_admin_user_id: input.created_by_admin_user_id,
            confirmation: input.confirmation,
            idempotency_scope: input.idempotency_scope,
            idempotency_key: input.idempotency_key,
            created_at: timestamp.clone(),
            recorded_at: timestamp,
            currentness_effect: SCANNER_KEY_STATUS_PENDING.to_string(),
            vulnerability_report_effect: SCANNER_KEY_NO_EFFECT.to_string(),
            artifact_security_effect: SCANNER_KEY_NO_EFFECT.to_string(),
            conformance_effect: SCANNER_KEY_NO_EFFECT.to_string(),
            adapter_effect: SCANNER_KEY_NO_EFFECT.to_string(),
            route_effect: SCANNER_KEY_NO_EFFECT.to_string(),
        };
        let mut record = ExternalPoolAdapterScannerKeyRecord {
            schema: SCANNER_KEY_RECORD_SCHEMA.to_string(),
            key_record_id: new_id("external_pool_adapter_scanner_key"),
            key_record_digest: String::new(),
            registration_material_digest: scanner_key_registration_digest(&registration)?,
            canonicalization: SCANNER_KEY_CANONICALIZATION.to_string(),
            digest_algorithm: SCANNER_KEY_DIGEST_ALGORITHM.to_string(),
            registration,
        };
        record.key_record_digest = scanner_key_record_json_and_digest(&record)?.1;
        validate_scanner_key_record(&record)?;
        let (json, digest) = scanner_key_record_json_and_digest(&record)?;
        if digest != record.key_record_digest {
            bail!("scanner-key record digest changed before persistence");
        }
        insert(&transaction, &record, &json)?;
        let stored = record_by_id_on(&transaction, &record.key_record_id)?
            .ok_or_else(|| anyhow::anyhow!("scanner-key record disappeared after insert"))?;
        if stored.record != record || stored.json != json {
            bail!("scanner-key record changed during exact readback");
        }
        let result = result(&stored, false);
        transaction.commit()?;
        Ok(result)
    }
}

fn insert(
    transaction: &rusqlite::Transaction<'_>,
    record: &ExternalPoolAdapterScannerKeyRecord,
    json: &str,
) -> Result<()> {
    let item = &record.registration;
    transaction.execute(
        "INSERT INTO compute_external_pool_adapter_scanner_keys(
          key_record_id,key_record_schema,key_record_digest,key_record_json,
          registration_material_digest,canonicalization,digest_algorithm,
          scanner_operator,scanner_product,key_id,algorithm,public_key_pem,actor_kind,
          created_by_admin_user_id,confirmation,idempotency_scope,idempotency_key,
          created_at,recorded_at,currentness_effect,vulnerability_report_effect,
          artifact_security_effect,conformance_effect,adapter_effect,route_effect)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,
                 ?18,?19,?20,?21,?22,?23,?24,?25)",
        params![
            record.key_record_id,
            record.schema,
            record.key_record_digest,
            json,
            record.registration_material_digest,
            record.canonicalization,
            record.digest_algorithm,
            item.scanner_operator,
            item.scanner_product,
            item.key_id,
            item.algorithm,
            item.public_key_pem,
            item.actor_kind,
            item.created_by_admin_user_id,
            item.confirmation,
            item.idempotency_scope,
            item.idempotency_key,
            item.created_at,
            item.recorded_at,
            item.currentness_effect,
            item.vulnerability_report_effect,
            item.artifact_security_effect,
            item.conformance_effect,
            item.adapter_effect,
            item.route_effect
        ],
    )?;
    Ok(())
}

fn ensure_replay(
    stored: &StoredScannerKeyRecord,
    input: &RegisterExternalPoolAdapterScannerKey,
) -> Result<()> {
    let item = &stored.record.registration;
    if item.scanner_operator != input.scanner_operator
        || item.scanner_product != input.scanner_product
        || item.key_id != input.key_id
        || item.public_key_pem != input.public_key_pem
        || item.created_by_admin_user_id != input.created_by_admin_user_id
        || item.confirmation != input.confirmation
        || item.idempotency_scope != input.idempotency_scope
        || item.idempotency_key != input.idempotency_key
    {
        bail!("scanner-key registration replay conflicts with immutable history");
    }
    Ok(())
}

fn result(
    stored: &StoredScannerKeyRecord,
    replayed: bool,
) -> ExternalPoolAdapterScannerKeyRegistrationWriteReceipt {
    ExternalPoolAdapterScannerKeyRegistrationWriteReceipt {
        key_record: stored.summary(),
        replayed,
    }
}

fn validate_input(input: &RegisterExternalPoolAdapterScannerKey) -> Result<()> {
    validate_exact(&input.scanner_operator, 160)?;
    validate_exact(&input.scanner_product, 160)?;
    validate_digest(&input.key_id)?;
    validate_exact(&input.created_by_admin_user_id, 160)?;
    validate_exact(&input.idempotency_scope, 200)?;
    validate_exact(&input.idempotency_key, 160)?;
    if input.confirmation != SCANNER_KEY_REGISTER_CONFIRMATION
        || input.public_key_pem.is_empty()
        || input.public_key_pem.len() > 16 * 1024
    {
        bail!("scanner-key registration input is invalid");
    }
    Ok(())
}
