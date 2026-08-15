use anyhow::{bail, Result};
use chrono::{SecondsFormat, Utc};
use rusqlite::{params, TransactionBehavior};

use crate::{
    compute_federation::external_pool_adapter_runtime_compatibility_verification::*,
    store::{new_id, Store},
};

use super::{
    error::ExternalPoolAdapterRuntimeCompatibilityVerificationStoreError as StoreError,
    persistence::insert_revocation,
    read::{
        identifier, revocation_by_idempotency_on, revocation_by_verification_on,
        verification_by_id_on, verification_head_by_release_on,
    },
    types::ExternalPoolAdapterRuntimeCompatibilityVerificationRevocationWriteReceipt,
};

impl Store {
    pub(crate) fn revoke_external_pool_adapter_runtime_compatibility_verification(
        &self,
        admin_user_id: &str,
        input: RevokeExternalPoolAdapterRuntimeCompatibilityVerificationReceiptInput,
    ) -> std::result::Result<
        ExternalPoolAdapterRuntimeCompatibilityVerificationRevocationWriteReceipt,
        StoreError,
    > {
        identifier(admin_user_id).map_err(StoreError::conflict)?;
        validate_revoke_runtime_compatibility_verification_input(&input)
            .map_err(StoreError::conflict)?;
        let scope = format!("v268:runtime-compatibility-revoke:{admin_user_id}");
        identifier(&scope).map_err(StoreError::conflict)?;
        let mut conn = self.conn().map_err(StoreError::storage)?;
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| StoreError::storage(error.into()))?;
        let result = (|| -> Result<_> {
            if let Some(revocation) =
                revocation_by_idempotency_on(&tx, &scope, &input.idempotency_key)?
            {
                let verification = verification_by_id_on(
                    &tx,
                    &revocation.receipt.revocation.verification_receipt_id,
                )?
                .ok_or_else(|| anyhow::anyhow!("V268 replay lost its verification"))?;
                ensure_replay(
                    admin_user_id,
                    &scope,
                    &input,
                    &verification.receipt,
                    &revocation.receipt,
                )?;
                return Ok(
                    ExternalPoolAdapterRuntimeCompatibilityVerificationRevocationWriteReceipt {
                        verification: verification.receipt,
                        revocation: revocation.receipt,
                        replayed: true,
                    },
                );
            }
            require_current_admin(&tx, admin_user_id)?;
            let verification = verification_by_id_on(&tx, &input.verification_receipt_id)?
                .ok_or_else(|| anyhow::anyhow!("V268 verification was not found"))?;
            if verification.receipt.verification_receipt_digest
                != input.expected_verification_receipt_digest
            {
                bail!("V268 expected verification-receipt digest is not exact");
            }
            let material = &verification.receipt.verification;
            let head = verification_head_by_release_on(
                &tx,
                &material.registry_release.registry_release_id,
            )?
            .ok_or_else(|| anyhow::anyhow!("V268 verification lineage head disappeared"))?;
            if head.receipt.verification_receipt_id != verification.receipt.verification_receipt_id
                || head.receipt.verification_receipt_digest
                    != verification.receipt.verification_receipt_digest
                || revocation_by_verification_on(
                    &tx,
                    &verification.receipt.verification_receipt_id,
                )?
                .is_some()
            {
                bail!("only the exact unrevoked V268 verification head may be revoked");
            }
            let revoked_at = Utc::now().to_rfc3339_opts(SecondsFormat::Nanos, true);
            let revocation_material = ExternalPoolAdapterRuntimeCompatibilityRevocationMaterial {
                verification_receipt_id: verification.receipt.verification_receipt_id.clone(),
                verification_receipt_digest: verification
                    .receipt
                    .verification_receipt_digest
                    .clone(),
                registry_release_id: material.registry_release.registry_release_id.clone(),
                registry_release_digest: material.registry_release.registry_release_digest.clone(),
                revoked_by_admin_user_id: admin_user_id.into(),
                reason: input.reason.clone(),
                confirmation: input.confirmation.clone(),
                idempotency_scope: scope.clone(),
                idempotency_key: input.idempotency_key.clone(),
                revoked_at: revoked_at.clone(),
                recorded_at: revoked_at,
                revocation_status: RUNTIME_COMPATIBILITY_VERIFICATION_REVOCATION_STATUS.into(),
                effects: runtime_compatibility_no_effects(),
                readiness: runtime_compatibility_no_readiness(),
            };
            let receipt = build_runtime_compatibility_revocation_receipt(
                new_id("external_pool_adapter_runtime_compatibility_revocation"),
                revocation_material,
            )?;
            insert_revocation(&tx, &receipt)?;
            let stored =
                revocation_by_verification_on(&tx, &verification.receipt.verification_receipt_id)?
                    .ok_or_else(|| anyhow::anyhow!("V268 revocation disappeared after insert"))?;
            if stored.receipt != receipt {
                bail!("V268 revocation readback drifted");
            }
            Ok(
                ExternalPoolAdapterRuntimeCompatibilityVerificationRevocationWriteReceipt {
                    verification: verification.receipt,
                    revocation: stored.receipt,
                    replayed: false,
                },
            )
        })()
        .map_err(StoreError::classify_write)?;
        tx.commit()
            .map_err(|error| StoreError::storage(error.into()))?;
        Ok(result)
    }
}

fn ensure_replay(
    admin: &str,
    scope: &str,
    input: &RevokeExternalPoolAdapterRuntimeCompatibilityVerificationReceiptInput,
    verification: &ExternalPoolAdapterRuntimeCompatibilityVerificationReceipt,
    revocation: &ExternalPoolAdapterRuntimeCompatibilityRevocationReceipt,
) -> Result<()> {
    let r = &revocation.revocation;
    if verification.verification_receipt_id != input.verification_receipt_id
        || verification.verification_receipt_digest != input.expected_verification_receipt_digest
        || r.verification_receipt_id != verification.verification_receipt_id
        || r.verification_receipt_digest != verification.verification_receipt_digest
        || r.revoked_by_admin_user_id != admin
        || r.reason != input.reason
        || r.idempotency_scope != scope
        || r.idempotency_key != input.idempotency_key
        || r.confirmation != input.confirmation
    {
        bail!("V268 revocation idempotency replay conflicts with sealed input");
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
