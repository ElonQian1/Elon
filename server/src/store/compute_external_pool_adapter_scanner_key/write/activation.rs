use anyhow::{bail, Result};
use rusqlite::{params, TransactionBehavior};

use crate::{
    compute_federation::external_pool_adapter_scanner_key::{
        scanner_key_activation_digest, scanner_key_activation_json_and_digest,
        validate_scanner_key_activation, ExternalPoolAdapterScannerKeyActivation,
        ExternalPoolAdapterScannerKeyActivationReceipt, SCANNER_KEY_ACTIVATE_CONFIRMATION,
        SCANNER_KEY_ACTIVATION_RECEIPT_SCHEMA, SCANNER_KEY_ACTOR_KIND,
        SCANNER_KEY_CANONICALIZATION, SCANNER_KEY_DIGEST_ALGORITHM, SCANNER_KEY_NO_EFFECT,
        SCANNER_KEY_STATUS_ACTIVE, SCANNER_KEY_STATUS_PENDING,
    },
    store::{new_id, Store},
};

use super::super::{
    read::{
        activation_by_idempotency_on, activation_by_key_on, currentness_on, record_by_id_on,
        revocation_by_key_on, validate_digest, validate_exact,
    },
    types::{
        ActivateExternalPoolAdapterScannerKey, ExternalPoolAdapterScannerKeyActivationWriteReceipt,
        StoredScannerKeyActivation, StoredScannerKeyRecord,
    },
};
use super::now;

impl Store {
    pub(crate) fn activate_external_pool_adapter_scanner_key(
        &self,
        input: ActivateExternalPoolAdapterScannerKey,
    ) -> Result<ExternalPoolAdapterScannerKeyActivationWriteReceipt> {
        validate_input(&input)?;
        let mut connection = self.conn()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(stored) = activation_by_idempotency_on(
            &transaction,
            &input.idempotency_scope,
            &input.idempotency_key,
        )? {
            let root = record_by_id_on(&transaction, &stored.receipt.activation.key_record_id)?
                .ok_or_else(|| anyhow::anyhow!("scanner-key activation lost its root"))?;
            ensure_replay(&stored, &root, &input)?;
            let result = result(&root, &stored, true);
            transaction.commit()?;
            return Ok(result);
        }
        let root = record_by_id_on(&transaction, &input.key_record_id)?
            .ok_or_else(|| anyhow::anyhow!("scanner-key record was not found"))?;
        if root.record.key_record_digest != input.expected_key_record_digest
            || root.record.registration.created_by_admin_user_id == input.activated_by_admin_user_id
            || activation_by_key_on(&transaction, &input.key_record_id)?.is_some()
            || revocation_by_key_on(&transaction, &input.key_record_id)?.is_some()
            || currentness_on(&transaction, &input.key_record_id)?
                .map(|x| x.current_status)
                .as_deref()
                != Some(SCANNER_KEY_STATUS_PENDING)
        {
            bail!(
                "scanner-key activation requires a current pending root and another administrator"
            );
        }
        let timestamp = now();
        let registration = &root.record.registration;
        let activation = ExternalPoolAdapterScannerKeyActivation {
            key_record_id: root.record.key_record_id.clone(),
            key_record_digest: root.record.key_record_digest.clone(),
            key_id: registration.key_id.clone(),
            scanner_operator: registration.scanner_operator.clone(),
            scanner_product: registration.scanner_product.clone(),
            actor_kind: SCANNER_KEY_ACTOR_KIND.to_string(),
            activated_by_admin_user_id: input.activated_by_admin_user_id,
            confirmation: input.confirmation,
            idempotency_scope: input.idempotency_scope,
            idempotency_key: input.idempotency_key,
            occurred_at: timestamp.clone(),
            recorded_at: timestamp,
            currentness_effect: SCANNER_KEY_STATUS_ACTIVE.to_string(),
            vulnerability_report_effect: SCANNER_KEY_NO_EFFECT.to_string(),
            artifact_security_effect: SCANNER_KEY_NO_EFFECT.to_string(),
            conformance_effect: SCANNER_KEY_NO_EFFECT.to_string(),
            adapter_effect: SCANNER_KEY_NO_EFFECT.to_string(),
            route_effect: SCANNER_KEY_NO_EFFECT.to_string(),
        };
        let mut receipt = ExternalPoolAdapterScannerKeyActivationReceipt {
            schema: SCANNER_KEY_ACTIVATION_RECEIPT_SCHEMA.to_string(),
            activation_receipt_id: new_id("external_pool_adapter_scanner_key_activation"),
            activation_receipt_digest: String::new(),
            activation_material_digest: scanner_key_activation_digest(&activation)?,
            canonicalization: SCANNER_KEY_CANONICALIZATION.to_string(),
            digest_algorithm: SCANNER_KEY_DIGEST_ALGORITHM.to_string(),
            activation,
        };
        receipt.activation_receipt_digest = scanner_key_activation_json_and_digest(&receipt)?.1;
        validate_scanner_key_activation(&receipt)?;
        let (json, digest) = scanner_key_activation_json_and_digest(&receipt)?;
        if digest != receipt.activation_receipt_digest {
            bail!("scanner-key activation digest drifted")
        }
        insert(&transaction, &receipt, &json)?;
        let stored = activation_by_key_on(&transaction, &input.key_record_id)?
            .ok_or_else(|| anyhow::anyhow!("scanner-key activation disappeared after insert"))?;
        if stored.receipt != receipt || stored.json != json {
            bail!("scanner-key activation changed during readback")
        }
        let result = result(&root, &stored, false);
        transaction.commit()?;
        Ok(result)
    }
}

fn insert(
    tx: &rusqlite::Transaction<'_>,
    receipt: &ExternalPoolAdapterScannerKeyActivationReceipt,
    json: &str,
) -> Result<()> {
    let item = &receipt.activation;
    tx.execute("INSERT INTO compute_external_pool_adapter_scanner_key_activations(
      activation_receipt_id,activation_receipt_schema,activation_receipt_digest,activation_receipt_json,
      activation_material_digest,canonicalization,digest_algorithm,key_record_id,key_record_digest,key_id,
      scanner_operator,scanner_product,actor_kind,activated_by_admin_user_id,confirmation,idempotency_scope,
      idempotency_key,occurred_at,recorded_at,currentness_effect,vulnerability_report_effect,
      artifact_security_effect,conformance_effect,adapter_effect,route_effect)
      VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,?22,?23,?24,?25)",
      params![receipt.activation_receipt_id,receipt.schema,receipt.activation_receipt_digest,json,
        receipt.activation_material_digest,receipt.canonicalization,receipt.digest_algorithm,item.key_record_id,
        item.key_record_digest,item.key_id,item.scanner_operator,item.scanner_product,item.actor_kind,
        item.activated_by_admin_user_id,item.confirmation,item.idempotency_scope,item.idempotency_key,
        item.occurred_at,item.recorded_at,item.currentness_effect,item.vulnerability_report_effect,
        item.artifact_security_effect,item.conformance_effect,item.adapter_effect,item.route_effect])?;
    Ok(())
}

fn ensure_replay(
    stored: &StoredScannerKeyActivation,
    root: &StoredScannerKeyRecord,
    input: &ActivateExternalPoolAdapterScannerKey,
) -> Result<()> {
    let item = &stored.receipt.activation;
    if root.record.key_record_digest != input.expected_key_record_digest
        || item.key_record_id != input.key_record_id
        || item.activated_by_admin_user_id != input.activated_by_admin_user_id
        || item.confirmation != input.confirmation
        || item.idempotency_scope != input.idempotency_scope
        || item.idempotency_key != input.idempotency_key
    {
        bail!("scanner-key activation replay conflicts with immutable history");
    }
    Ok(())
}

fn result(
    root: &StoredScannerKeyRecord,
    stored: &StoredScannerKeyActivation,
    replayed: bool,
) -> ExternalPoolAdapterScannerKeyActivationWriteReceipt {
    ExternalPoolAdapterScannerKeyActivationWriteReceipt {
        key_record: root.summary(),
        activation: stored.summary(),
        replayed,
    }
}

fn validate_input(input: &ActivateExternalPoolAdapterScannerKey) -> Result<()> {
    validate_exact(&input.key_record_id, 160)?;
    validate_digest(&input.expected_key_record_digest)?;
    validate_exact(&input.activated_by_admin_user_id, 160)?;
    validate_exact(&input.idempotency_scope, 200)?;
    validate_exact(&input.idempotency_key, 160)?;
    if input.confirmation != SCANNER_KEY_ACTIVATE_CONFIRMATION {
        bail!("scanner-key activation confirmation is invalid")
    }
    Ok(())
}
