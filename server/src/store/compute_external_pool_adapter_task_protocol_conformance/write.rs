use chrono::{SecondsFormat, Utc};
use rusqlite::TransactionBehavior;

use crate::{
    compute_federation::external_pool_adapter_task_protocol_conformance::*,
    store::{
        compute_external_pool_adapter_runtime_bundle::ExternalPoolAdapterTaskProtocolConformanceSealInput,
        compute_external_pool_adapter_upstream_transport_target::ExternalPoolAdapterInstallationReopener,
        new_id, Store,
    },
};

use super::{
    error::ExternalPoolAdapterTaskProtocolConformanceStoreError as StoreError,
    persistence::insert_run,
    read::{run_by_id_on, run_by_idempotency_on, run_head_by_release_on},
    roots::{
        canonical_time, current_roots_for_create_on, domain_roots, into_execution_input,
        task_protocol_conformance_expires_at,
    },
    run::execute_external_pool_adapter_task_protocol_conformance,
    runtime::ExternalPoolAdapterTaskProtocolConformanceRuntime,
    types::*,
};

pub(super) mod replay;
pub(super) mod validation;

use replay::{
    ensure_create_replay, ensure_fresh_readback, promote_exact_pending_replay, replay_output,
};
use validation::{
    classify_execution_error, classify_installation_error, ensure_predecessor,
    validate_create_input,
};

struct FreshRunCommit {
    output: ExternalPoolAdapterTaskProtocolConformanceRunWriteReceipt,
    run_receipt_id: String,
    receipt_integrity_digest: String,
}

impl Store {
    /// Runs the exact V272 stateful matrix outside SQLite transactions, then appends its
    /// Provider-neutral receipt under a second fresh currentness proof.
    pub(crate) fn create_external_pool_adapter_task_protocol_conformance_run(
        &self,
        input: CreateExternalPoolAdapterTaskProtocolConformanceRun,
        reopen_prepared: &mut ExternalPoolAdapterInstallationReopener<'_>,
        runtime: &ExternalPoolAdapterTaskProtocolConformanceRuntime,
    ) -> std::result::Result<ExternalPoolAdapterTaskProtocolConformanceRunWriteReceipt, StoreError>
    {
        validate_create_input(&input).map_err(StoreError::conflict)?;

        // The filesystem audit happens before BEGIN IMMEDIATE. The resulting Prepared handles are
        // consumed by the transaction-bound V249 carrier authority and then by the pure runner.
        let preflight_prepared = reopen_prepared().map_err(classify_installation_error)?;
        let execution_input = {
            let mut conn = self.conn().map_err(StoreError::storage)?;
            let tx = conn
                .transaction_with_behavior(TransactionBehavior::Immediate)
                .map_err(|error| StoreError::storage(error))?;
            if let Some(stored) =
                run_by_idempotency_on(&tx, &input.idempotency_scope, &input.idempotency_key)
                    .map_err(StoreError::storage)?
            {
                ensure_create_replay(&input, &stored).map_err(StoreError::conflict)?;
                let checked_at = now();
                let current =
                    current_roots_for_create_on(&tx, &input, preflight_prepared, &checked_at)
                        .map_err(StoreError::classify_write)?;
                drop(current);
                let output = replay_output(&stored);
                let receipt_id = stored.receipt.run_receipt_id.clone();
                let integrity = stored.receipt_integrity_digest.clone();
                tx.commit().map_err(|error| StoreError::storage(error))?;
                promote_exact_pending_replay(runtime, &receipt_id, &integrity)?;
                return Ok(output);
            }
            let checked_at = now();
            let roots = current_roots_for_create_on(&tx, &input, preflight_prepared, &checked_at)
                .map_err(StoreError::classify_write)?;
            let head = run_head_by_release_on(&tx, &input.registry_release_id)
                .map_err(StoreError::storage)?;
            ensure_predecessor(&input, head.as_ref()).map_err(StoreError::conflict)?;
            let execution_input =
                into_execution_input(roots).map_err(StoreError::classify_write)?;
            tx.commit().map_err(|error| StoreError::storage(error))?;
            execution_input
        };

        let evidence =
            execute_external_pool_adapter_task_protocol_conformance(execution_input, runtime)
                .map_err(classify_execution_error)?;

        // Cleanup is part of evidence. A new reopener call is mandatory after cleanup so neither
        // the first Prepared value nor a cached diagnostic can authorize the append.
        let final_prepared = reopen_prepared().map_err(classify_installation_error)?;
        let finalized = {
            let mut conn = self.conn().map_err(StoreError::storage)?;
            let tx = conn
                .transaction_with_behavior(TransactionBehavior::Immediate)
                .map_err(|error| StoreError::storage(error))?;
            if let Some(stored) =
                run_by_idempotency_on(&tx, &input.idempotency_scope, &input.idempotency_key)
                    .map_err(StoreError::storage)?
            {
                ensure_create_replay(&input, &stored).map_err(StoreError::conflict)?;
                let checked_at = now();
                let current = current_roots_for_create_on(&tx, &input, final_prepared, &checked_at)
                    .map_err(StoreError::classify_write)?;
                drop(current);
                let output = replay_output(&stored);
                let receipt_id = stored.receipt.run_receipt_id.clone();
                let integrity = stored.receipt_integrity_digest.clone();
                tx.commit().map_err(|error| StoreError::storage(error))?;
                promote_exact_pending_replay(runtime, &receipt_id, &integrity)?;
                return Ok(output);
            }

            let checked_at = now();
            let current = current_roots_for_create_on(&tx, &input, final_prepared, &checked_at)
                .map_err(StoreError::classify_write)?;
            let roots = domain_roots(&current).map_err(StoreError::classify_write)?;
            let head = run_head_by_release_on(&tx, &input.registry_release_id)
                .map_err(StoreError::storage)?;
            ensure_predecessor(&input, head.as_ref()).map_err(StoreError::conflict)?;
            let sequence = head
                .as_ref()
                .map_or(Ok(1), |stored| {
                    stored.receipt.run.sequence.checked_add(1).ok_or_else(|| {
                        anyhow::anyhow!("task-protocol conformance sequence overflow")
                    })
                })
                .map_err(StoreError::conflict)?;
            let predecessor =
                head.as_ref().map(
                    |stored| ExternalPoolAdapterTaskProtocolConformancePredecessor {
                        run_receipt_id: stored.receipt.run_receipt_id.clone(),
                        run_receipt_digest: stored.receipt.run_receipt_digest.clone(),
                    },
                );
            let expires_at = task_protocol_conformance_expires_at(&checked_at, &roots)
                .map_err(StoreError::classify_write)?;
            if canonical_time(&expires_at).map_err(StoreError::conflict)? <= Utc::now() {
                return Err(StoreError::conflict(anyhow::anyhow!(
                    "task-protocol conformance roots expired before final append"
                )));
            }
            let material = build_external_pool_adapter_task_protocol_conformance_run_material(
                roots,
                evidence,
                sequence,
                predecessor,
                checked_at,
                expires_at,
            )
            .map_err(StoreError::classify_write)?;
            let receipt = build_external_pool_adapter_task_protocol_conformance_run_receipt(
                new_id("external_pool_adapter_task_protocol_conformance_run"),
                material,
            )
            .map_err(StoreError::classify_write)?;
            let r = &receipt.run;
            let process_seal = runtime
                .process_custody()
                .seal_task_protocol_conformance(
                    &ExternalPoolAdapterTaskProtocolConformanceSealInput {
                        run_receipt_digest: &receipt.run_receipt_digest,
                        task_observation_root: &r.task_observation_root,
                        session_roots_digest: &r.session_roots_digest,
                        session_transcript_digest: &r.session_transcript_digest,
                        delivery_inventory_digest: &r.delivery_inventory_digest,
                        exchange_inventory_digest: &r.exchange_inventory_digest,
                        post_cleanup_checked_at: &r.post_cleanup_checked_at,
                        expires_at: &r.expires_at,
                    },
                )
                .map_err(StoreError::storage)?;
            let custody_epoch_digest = runtime.custody_epoch_digest().to_owned();
            let process_hmac_seal = process_seal.seal_hex().to_owned();
            let receipt_integrity_digest = task_protocol_conformance_receipt_integrity_digest(
                &receipt.run_receipt_digest,
                &custody_epoch_digest,
                &process_hmac_seal,
            )
            .map_err(StoreError::storage)?;

            // Only a freshly minted opaque seal can enter pending state, and pending state is
            // deliberately established before the append. Rollback never promotes it.
            runtime
                .process_custody()
                .remember_pending_task_protocol_conformance_seal(
                    &receipt.run_receipt_id,
                    &receipt_integrity_digest,
                    &process_seal,
                )
                .map_err(StoreError::storage)?;
            insert_run(
                &tx,
                &receipt,
                &TaskProtocolConformanceRunPrivateFields {
                    recorded_by_admin_user_id: &input.recorded_by_admin_user_id,
                    idempotency_scope: &input.idempotency_scope,
                    idempotency_key: &input.idempotency_key,
                    confirmation: &input.confirmation,
                    runtime_custody_epoch_digest: &custody_epoch_digest,
                    process_hmac_seal: &process_hmac_seal,
                    receipt_integrity_digest: &receipt_integrity_digest,
                },
            )
            .map_err(StoreError::classify_write)?;
            let stored = run_by_id_on(&tx, &receipt.run_receipt_id)
                .map_err(StoreError::storage)?
                .ok_or_else(|| {
                    StoreError::storage(anyhow::anyhow!(
                        "task-protocol conformance run disappeared after append"
                    ))
                })?;
            ensure_fresh_readback(
                &input,
                &receipt,
                &stored,
                &custody_epoch_digest,
                &process_hmac_seal,
                &receipt_integrity_digest,
            )
            .map_err(StoreError::classify_write)?;
            if canonical_time(&receipt.run.expires_at).map_err(StoreError::conflict)? <= Utc::now()
            {
                return Err(StoreError::conflict(anyhow::anyhow!(
                    "task-protocol conformance receipt expired before commit"
                )));
            }
            let fresh = FreshRunCommit {
                output: ExternalPoolAdapterTaskProtocolConformanceRunWriteReceipt {
                    run: stored.receipt,
                    replayed: false,
                },
                run_receipt_id: receipt.run_receipt_id,
                receipt_integrity_digest,
            };
            tx.commit().map_err(|error| StoreError::storage(error))?;
            fresh
        };

        let promoted = runtime
            .process_custody()
            .promote_task_protocol_conformance_seal(
                &finalized.run_receipt_id,
                &finalized.receipt_integrity_digest,
            )
            .map_err(StoreError::storage)?;
        if !promoted {
            return Err(StoreError::storage(anyhow::anyhow!(
                "fresh task-protocol conformance row lost its pending process seal"
            )));
        }
        Ok(finalized.output)
    }
}

fn now() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Nanos, true)
}
