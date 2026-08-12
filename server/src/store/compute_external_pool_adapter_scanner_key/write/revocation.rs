use anyhow::{bail, Result};
use rusqlite::{params, TransactionBehavior};

use crate::{
    compute_federation::external_pool_adapter_scanner_key::{
        scanner_key_revocation_digest, scanner_key_revocation_json_and_digest,
        validate_scanner_key_revocation, ExternalPoolAdapterScannerKeyRevocation,
        ExternalPoolAdapterScannerKeyRevocationReceipt, SCANNER_KEY_ACTOR_KIND,
        SCANNER_KEY_CANONICALIZATION, SCANNER_KEY_DIGEST_ALGORITHM, SCANNER_KEY_NO_EFFECT,
        SCANNER_KEY_REVOCATION_RECEIPT_SCHEMA, SCANNER_KEY_REVOKE_CONFIRMATION,
        SCANNER_KEY_STATUS_ACTIVE, SCANNER_KEY_STATUS_REVOKED,
    },
    store::{new_id, Store},
};

use super::super::{
    read::{
        activation_by_key_on, currentness_on, record_by_id_on, revocation_by_idempotency_on,
        revocation_by_key_on, validate_digest, validate_exact,
    },
    types::{
        ExternalPoolAdapterScannerKeyRevocationWriteReceipt, RevokeExternalPoolAdapterScannerKey,
        StoredScannerKeyRecord, StoredScannerKeyRevocation,
    },
};
use super::now;

impl Store {
    pub(crate) fn revoke_external_pool_adapter_scanner_key(
        &self,
        input: RevokeExternalPoolAdapterScannerKey,
    ) -> Result<ExternalPoolAdapterScannerKeyRevocationWriteReceipt> {
        validate_input(&input)?;
        let mut connection = self.conn()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(stored) = revocation_by_idempotency_on(
            &transaction,
            &input.idempotency_scope,
            &input.idempotency_key,
        )? {
            let root = record_by_id_on(&transaction, &stored.receipt.revocation.key_record_id)?
                .ok_or_else(|| anyhow::anyhow!("scanner-key revocation lost its root"))?;
            ensure_replay(&stored, &root, &input)?;
            let result = result(&root, &stored, true);
            transaction.commit()?;
            return Ok(result);
        }
        let root = record_by_id_on(&transaction, &input.key_record_id)?
            .ok_or_else(|| anyhow::anyhow!("scanner-key record was not found"))?;
        if root.record.key_record_digest != input.expected_key_record_digest
            || activation_by_key_on(&transaction, &input.key_record_id)?.is_none()
            || revocation_by_key_on(&transaction, &input.key_record_id)?.is_some()
            || currentness_on(&transaction, &input.key_record_id)?
                .map(|x| x.current_status)
                .as_deref()
                != Some(SCANNER_KEY_STATUS_ACTIVE)
        {
            bail!("scanner-key root is not active and exact");
        }
        let timestamp = now();
        let registration = &root.record.registration;
        let revocation = ExternalPoolAdapterScannerKeyRevocation {
            key_record_id: root.record.key_record_id.clone(),
            key_record_digest: root.record.key_record_digest.clone(),
            key_id: registration.key_id.clone(),
            scanner_operator: registration.scanner_operator.clone(),
            scanner_product: registration.scanner_product.clone(),
            actor_kind: SCANNER_KEY_ACTOR_KIND.to_string(),
            revoked_by_admin_user_id: input.revoked_by_admin_user_id,
            reason: input.reason,
            confirmation: input.confirmation,
            idempotency_scope: input.idempotency_scope,
            idempotency_key: input.idempotency_key,
            occurred_at: timestamp.clone(),
            recorded_at: timestamp,
            currentness_effect: SCANNER_KEY_STATUS_REVOKED.to_string(),
            vulnerability_report_effect: SCANNER_KEY_NO_EFFECT.to_string(),
            artifact_security_effect: SCANNER_KEY_NO_EFFECT.to_string(),
            conformance_effect: SCANNER_KEY_NO_EFFECT.to_string(),
            adapter_effect: SCANNER_KEY_NO_EFFECT.to_string(),
            route_effect: SCANNER_KEY_NO_EFFECT.to_string(),
        };
        let mut receipt = ExternalPoolAdapterScannerKeyRevocationReceipt {
            schema: SCANNER_KEY_REVOCATION_RECEIPT_SCHEMA.to_string(),
            revocation_receipt_id: new_id("external_pool_adapter_scanner_key_revocation"),
            revocation_receipt_digest: String::new(),
            revocation_material_digest: scanner_key_revocation_digest(&revocation)?,
            canonicalization: SCANNER_KEY_CANONICALIZATION.to_string(),
            digest_algorithm: SCANNER_KEY_DIGEST_ALGORITHM.to_string(),
            revocation,
        };
        receipt.revocation_receipt_digest = scanner_key_revocation_json_and_digest(&receipt)?.1;
        validate_scanner_key_revocation(&receipt)?;
        let (json, digest) = scanner_key_revocation_json_and_digest(&receipt)?;
        if digest != receipt.revocation_receipt_digest {
            bail!("scanner-key revocation digest drifted")
        }
        insert(&transaction, &receipt, &json)?;
        let stored = revocation_by_key_on(&transaction, &input.key_record_id)?
            .ok_or_else(|| anyhow::anyhow!("scanner-key revocation disappeared"))?;
        if stored.receipt != receipt || stored.json != json {
            bail!("scanner-key revocation changed during readback")
        }
        let result = result(&root, &stored, false);
        transaction.commit()?;
        Ok(result)
    }
}

fn insert(
    tx: &rusqlite::Transaction<'_>,
    receipt: &ExternalPoolAdapterScannerKeyRevocationReceipt,
    json: &str,
) -> Result<()> {
    let item = &receipt.revocation;
    tx.execute("INSERT INTO compute_external_pool_adapter_scanner_key_revocations(
      revocation_receipt_id,revocation_receipt_schema,revocation_receipt_digest,revocation_receipt_json,
      revocation_material_digest,canonicalization,digest_algorithm,key_record_id,key_record_digest,key_id,
      scanner_operator,scanner_product,actor_kind,revoked_by_admin_user_id,reason,confirmation,idempotency_scope,
      idempotency_key,occurred_at,recorded_at,currentness_effect,vulnerability_report_effect,
      artifact_security_effect,conformance_effect,adapter_effect,route_effect)
      VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,?22,?23,?24,?25,?26)",
      params![receipt.revocation_receipt_id,receipt.schema,receipt.revocation_receipt_digest,json,
        receipt.revocation_material_digest,receipt.canonicalization,receipt.digest_algorithm,item.key_record_id,
        item.key_record_digest,item.key_id,item.scanner_operator,item.scanner_product,item.actor_kind,
        item.revoked_by_admin_user_id,item.reason,item.confirmation,item.idempotency_scope,item.idempotency_key,
        item.occurred_at,item.recorded_at,item.currentness_effect,item.vulnerability_report_effect,
        item.artifact_security_effect,item.conformance_effect,item.adapter_effect,item.route_effect])?;
    Ok(())
}

fn ensure_replay(
    stored: &StoredScannerKeyRevocation,
    root: &StoredScannerKeyRecord,
    input: &RevokeExternalPoolAdapterScannerKey,
) -> Result<()> {
    let item = &stored.receipt.revocation;
    if root.record.key_record_digest != input.expected_key_record_digest
        || item.key_record_id != input.key_record_id
        || item.revoked_by_admin_user_id != input.revoked_by_admin_user_id
        || item.reason != input.reason
        || item.confirmation != input.confirmation
        || item.idempotency_scope != input.idempotency_scope
        || item.idempotency_key != input.idempotency_key
    {
        bail!("scanner-key revocation replay conflicts with immutable history")
    }
    Ok(())
}

fn result(
    root: &StoredScannerKeyRecord,
    stored: &StoredScannerKeyRevocation,
    replayed: bool,
) -> ExternalPoolAdapterScannerKeyRevocationWriteReceipt {
    ExternalPoolAdapterScannerKeyRevocationWriteReceipt {
        key_record: root.summary(),
        revocation: stored.summary(),
        replayed,
    }
}

fn validate_input(input: &RevokeExternalPoolAdapterScannerKey) -> Result<()> {
    validate_exact(&input.key_record_id, 160)?;
    validate_digest(&input.expected_key_record_digest)?;
    validate_exact(&input.revoked_by_admin_user_id, 160)?;
    validate_exact(&input.reason, 2_000)?;
    validate_exact(&input.idempotency_scope, 200)?;
    validate_exact(&input.idempotency_key, 160)?;
    if input.reason.chars().count() < 8 || input.confirmation != SCANNER_KEY_REVOKE_CONFIRMATION {
        bail!("scanner-key revocation input is invalid")
    }
    Ok(())
}
