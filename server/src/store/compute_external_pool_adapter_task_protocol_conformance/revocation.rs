use anyhow::{bail, Result};
use chrono::{SecondsFormat, Utc};
use rusqlite::TransactionBehavior;

use crate::{
    compute_federation::external_pool_adapter_task_protocol_conformance::*,
    store::{new_id, Store},
};

use super::{
    error::ExternalPoolAdapterTaskProtocolConformanceStoreError as StoreError,
    persistence::insert_revocation,
    read::{
        identifier, revocation_by_idempotency_on, revocation_by_run_on, run_by_id_on,
        run_head_by_release_on,
    },
    roots::require_current_admin,
    types::*,
};

impl Store {
    pub(crate) fn revoke_external_pool_adapter_task_protocol_conformance_run(
        &self,
        input: RevokeExternalPoolAdapterTaskProtocolConformanceRun,
    ) -> std::result::Result<
        ExternalPoolAdapterTaskProtocolConformanceRevocationWriteReceipt,
        StoreError,
    > {
        validate_revoke_input(&input).map_err(StoreError::conflict)?;
        let mut conn = self.conn().map_err(StoreError::storage)?;
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(StoreError::storage)?;
        let output = (|| -> Result<_> {
            if let Some(stored) =
                revocation_by_idempotency_on(&tx, &input.idempotency_scope, &input.idempotency_key)?
            {
                let run = run_by_id_on(&tx, &stored.receipt.revocation.run_receipt_id)?
                    .ok_or_else(|| {
                        anyhow::anyhow!("task-protocol conformance revocation replay lost its run")
                    })?;
                ensure_revocation_replay(&input, &run, &stored)?;
                return Ok(
                    ExternalPoolAdapterTaskProtocolConformanceRevocationWriteReceipt {
                        run: run.receipt,
                        revocation: stored.receipt,
                        replayed: true,
                    },
                );
            }
            require_current_admin(&tx, &input.revoked_by_admin_user_id)?;
            let run = run_by_id_on(&tx, &input.run_receipt_id)?
                .ok_or_else(|| anyhow::anyhow!("task-protocol conformance run was not found"))?;
            ensure_revocation_target(&input, &run)?;
            let head =
                run_head_by_release_on(&tx, &input.registry_release_id)?.ok_or_else(|| {
                    anyhow::anyhow!("task-protocol conformance lineage head vanished")
                })?;
            if head.receipt.run_receipt_id != run.receipt.run_receipt_id
                || head.receipt.run_receipt_digest != run.receipt.run_receipt_digest
                || revocation_by_run_on(&tx, &input.run_receipt_id)?.is_some()
            {
                bail!("only the latest unrevoked task-protocol conformance run may be revoked")
            }
            let revoked_at = std::cmp::max(now(), run.receipt.run.recorded_at.clone());
            let material =
                build_external_pool_adapter_task_protocol_conformance_revocation_material(
                    &run.receipt,
                    input.reason.clone(),
                    revoked_at,
                )?;
            let receipt = build_external_pool_adapter_task_protocol_conformance_revocation_receipt(
                new_id("external_pool_adapter_task_protocol_conformance_revocation"),
                material,
            )?;
            insert_revocation(
                &tx,
                &receipt,
                &TaskProtocolConformanceRevocationPrivateFields {
                    revoked_by_admin_user_id: &input.revoked_by_admin_user_id,
                    idempotency_scope: &input.idempotency_scope,
                    idempotency_key: &input.idempotency_key,
                    confirmation: &input.confirmation,
                },
            )?;
            let stored = revocation_by_run_on(&tx, &input.run_receipt_id)?.ok_or_else(|| {
                anyhow::anyhow!("task-protocol conformance revocation disappeared after append")
            })?;
            ensure_revocation_readback(&input, &run, &receipt, &stored)?;
            Ok(
                ExternalPoolAdapterTaskProtocolConformanceRevocationWriteReceipt {
                    run: run.receipt,
                    revocation: stored.receipt,
                    replayed: false,
                },
            )
        })()
        .map_err(StoreError::classify_write)?;
        tx.commit().map_err(StoreError::storage)?;
        Ok(output)
    }
}

fn validate_revoke_input(
    input: &RevokeExternalPoolAdapterTaskProtocolConformanceRun,
) -> Result<()> {
    for value in [
        &input.registry_release_id,
        &input.run_receipt_id,
        &input.revoked_by_admin_user_id,
        &input.idempotency_key,
    ] {
        identifier(value)?;
    }
    if input.expected_run_receipt_digest.len() != 64
        || !input
            .expected_run_receipt_digest
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
        || input.reason.trim() != input.reason
        || !(12..=500).contains(&input.reason.chars().count())
        || input.reason.chars().any(char::is_control)
    {
        bail!("task-protocol conformance revocation target or reason is invalid")
    }
    let expected_scope = format!(
        "v272:task-protocol-conformance:revoke:{}",
        input.revoked_by_admin_user_id
    );
    if input.idempotency_scope != expected_scope
        || input.confirmation != TASK_PROTOCOL_CONFORMANCE_REVOCATION_CONFIRMATION
    {
        bail!("task-protocol conformance revocation actor-bound metadata is invalid")
    }
    Ok(())
}

fn ensure_revocation_target(
    input: &RevokeExternalPoolAdapterTaskProtocolConformanceRun,
    run: &StoredTaskProtocolConformanceRun,
) -> Result<()> {
    if run.receipt.run_receipt_id != input.run_receipt_id
        || run.receipt.run_receipt_digest != input.expected_run_receipt_digest
        || run.receipt.run.registry_release.registry_release_id != input.registry_release_id
    {
        bail!("task-protocol conformance revocation target is not exact")
    }
    Ok(())
}

fn ensure_revocation_replay(
    input: &RevokeExternalPoolAdapterTaskProtocolConformanceRun,
    run: &StoredTaskProtocolConformanceRun,
    stored: &StoredTaskProtocolConformanceRevocation,
) -> Result<()> {
    ensure_revocation_target(input, run)?;
    let r = &stored.receipt.revocation;
    if r.run_receipt_id != run.receipt.run_receipt_id
        || r.run_receipt_digest != run.receipt.run_receipt_digest
        || r.registry_release_id != run.receipt.run.registry_release.registry_release_id
        || r.registry_release_digest != run.receipt.run.registry_release.registry_release_digest
        || r.reason != input.reason
        || stored.revoked_by_admin_user_id != input.revoked_by_admin_user_id
        || stored.idempotency_scope != input.idempotency_scope
        || stored.idempotency_key != input.idempotency_key
        || stored.confirmation != input.confirmation
    {
        bail!("task-protocol conformance revocation replay conflicts with immutable input")
    }
    Ok(())
}

fn ensure_revocation_readback(
    input: &RevokeExternalPoolAdapterTaskProtocolConformanceRun,
    run: &StoredTaskProtocolConformanceRun,
    receipt: &ExternalPoolAdapterTaskProtocolConformanceRevocationReceipt,
    stored: &StoredTaskProtocolConformanceRevocation,
) -> Result<()> {
    ensure_revocation_replay(input, run, stored)?;
    if stored.receipt != *receipt {
        bail!("task-protocol conformance revocation durable readback drifted")
    }
    Ok(())
}

fn now() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Nanos, true)
}
