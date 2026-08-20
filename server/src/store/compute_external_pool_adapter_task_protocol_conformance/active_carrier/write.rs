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
    super::{
        error::ExternalPoolAdapterTaskProtocolConformanceStoreError as StoreError,
        persistence::insert_run,
        read::{run_by_id_on, run_by_idempotency_on, run_head_by_release_on},
        roots::canonical_time,
        run::execute_external_pool_adapter_task_protocol_conformance,
        runtime::ExternalPoolAdapterTaskProtocolConformanceRuntime,
        types::*,
        write::{
            replay::{
                ensure_create_replay, ensure_fresh_readback, promote_exact_pending_replay,
                replay_output,
            },
            validation::{
                classify_execution_error, classify_installation_error, ensure_predecessor,
                validate_create_input,
            },
        },
    },
    roots::{
        current_active_roots_for_create_on, into_projected_execution_input, projected_domain_roots,
        projected_expiry,
    },
};

struct FreshActiveRunCommit {
    output: ExternalPoolAdapterTaskProtocolConformanceRunWriteReceipt,
    run_receipt_id: String,
    receipt_integrity_digest: String,
}

/// Reuses V272's Provider-neutral execution and durable receipt ABI while replacing only its
/// registering carrier with a current V277 projected-active historical carrier.
pub(in crate::store) fn create_external_pool_adapter_task_protocol_conformance_run_for_projected_active(
    store: &Store,
    input: CreateExternalPoolAdapterTaskProtocolConformanceRun,
    reopen_prepared: &mut ExternalPoolAdapterInstallationReopener<'_>,
    runtime: &ExternalPoolAdapterTaskProtocolConformanceRuntime,
) -> std::result::Result<ExternalPoolAdapterTaskProtocolConformanceRunWriteReceipt, StoreError> {
    validate_create_input(&input).map_err(StoreError::conflict)?;
    let preflight_prepared = reopen_prepared().map_err(classify_installation_error)?;
    let execution_input = {
        let mut conn = store.conn().map_err(StoreError::storage)?;
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(StoreError::storage)?;
        if let Some(stored) =
            run_by_idempotency_on(&tx, &input.idempotency_scope, &input.idempotency_key)
                .map_err(StoreError::storage)?
        {
            ensure_create_replay(&input, &stored).map_err(StoreError::conflict)?;
            let roots = current_active_roots_for_create_on(&tx, &input, preflight_prepared, &now())
                .map_err(StoreError::classify_write)?;
            drop(roots);
            let output = replay_output(&stored);
            let receipt_id = stored.receipt.run_receipt_id.clone();
            let integrity = stored.receipt_integrity_digest.clone();
            tx.commit().map_err(StoreError::storage)?;
            promote_exact_pending_replay(runtime, &receipt_id, &integrity)?;
            return Ok(output);
        }
        let checked_at = now();
        let roots =
            current_active_roots_for_create_on(&tx, &input, preflight_prepared, &checked_at)
                .map_err(StoreError::classify_write)?;
        let head =
            run_head_by_release_on(&tx, &input.registry_release_id).map_err(StoreError::storage)?;
        ensure_predecessor(&input, head.as_ref()).map_err(StoreError::conflict)?;
        let execution =
            into_projected_execution_input(roots).map_err(StoreError::classify_write)?;
        tx.commit().map_err(StoreError::storage)?;
        execution
    };

    let evidence =
        execute_external_pool_adapter_task_protocol_conformance(execution_input, runtime)
            .map_err(classify_execution_error)?;
    let final_prepared = reopen_prepared().map_err(classify_installation_error)?;
    let finalized = {
        let mut conn = store.conn().map_err(StoreError::storage)?;
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(StoreError::storage)?;
        if let Some(stored) =
            run_by_idempotency_on(&tx, &input.idempotency_scope, &input.idempotency_key)
                .map_err(StoreError::storage)?
        {
            ensure_create_replay(&input, &stored).map_err(StoreError::conflict)?;
            let roots = current_active_roots_for_create_on(&tx, &input, final_prepared, &now())
                .map_err(StoreError::classify_write)?;
            drop(roots);
            let output = replay_output(&stored);
            let receipt_id = stored.receipt.run_receipt_id.clone();
            let integrity = stored.receipt_integrity_digest.clone();
            tx.commit().map_err(StoreError::storage)?;
            promote_exact_pending_replay(runtime, &receipt_id, &integrity)?;
            return Ok(output);
        }
        let checked_at = now();
        let roots = current_active_roots_for_create_on(&tx, &input, final_prepared, &checked_at)
            .map_err(StoreError::classify_write)?;
        let projected = projected_domain_roots(&roots).map_err(StoreError::classify_write)?;
        let expires_at =
            projected_expiry(&checked_at, &roots).map_err(StoreError::classify_write)?;
        let head =
            run_head_by_release_on(&tx, &input.registry_release_id).map_err(StoreError::storage)?;
        ensure_predecessor(&input, head.as_ref()).map_err(StoreError::conflict)?;
        let sequence = head
            .as_ref()
            .map_or(Ok(1), |stored| {
                stored
                    .receipt
                    .run
                    .sequence
                    .checked_add(1)
                    .ok_or_else(|| anyhow::anyhow!("projected-active V272 sequence overflow"))
            })
            .map_err(StoreError::conflict)?;
        let predecessor =
            head.as_ref().map(
                |stored| ExternalPoolAdapterTaskProtocolConformancePredecessor {
                    run_receipt_id: stored.receipt.run_receipt_id.clone(),
                    run_receipt_digest: stored.receipt.run_receipt_digest.clone(),
                },
            );
        if canonical_time(&expires_at).map_err(StoreError::conflict)? <= Utc::now() {
            return Err(StoreError::conflict(anyhow::anyhow!(
                "projected-active V272 roots expired before append"
            )));
        }
        let material = build_external_pool_adapter_task_protocol_conformance_run_material(
            projected,
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
        let run = &receipt.run;
        let seal = runtime
            .process_custody()
            .seal_task_protocol_conformance(&ExternalPoolAdapterTaskProtocolConformanceSealInput {
                run_receipt_digest: &receipt.run_receipt_digest,
                task_observation_root: &run.task_observation_root,
                session_roots_digest: &run.session_roots_digest,
                session_transcript_digest: &run.session_transcript_digest,
                delivery_inventory_digest: &run.delivery_inventory_digest,
                exchange_inventory_digest: &run.exchange_inventory_digest,
                post_cleanup_checked_at: &run.post_cleanup_checked_at,
                expires_at: &run.expires_at,
            })
            .map_err(StoreError::storage)?;
        let epoch = runtime.custody_epoch_digest().to_owned();
        let process_hmac = seal.seal_hex().to_owned();
        let integrity = task_protocol_conformance_receipt_integrity_digest(
            &receipt.run_receipt_digest,
            &epoch,
            &process_hmac,
        )
        .map_err(StoreError::storage)?;
        runtime
            .process_custody()
            .remember_pending_task_protocol_conformance_seal(
                &receipt.run_receipt_id,
                &integrity,
                &seal,
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
                runtime_custody_epoch_digest: &epoch,
                process_hmac_seal: &process_hmac,
                receipt_integrity_digest: &integrity,
            },
        )
        .map_err(StoreError::classify_write)?;
        let stored = run_by_id_on(&tx, &receipt.run_receipt_id)
            .map_err(StoreError::storage)?
            .ok_or_else(|| StoreError::storage(anyhow::anyhow!("V272 active row disappeared")))?;
        ensure_fresh_readback(&input, &receipt, &stored, &epoch, &process_hmac, &integrity)
            .map_err(StoreError::classify_write)?;
        if canonical_time(&receipt.run.expires_at).map_err(StoreError::conflict)? <= Utc::now() {
            return Err(StoreError::conflict(anyhow::anyhow!(
                "projected-active V272 receipt expired before commit"
            )));
        }
        let fresh = FreshActiveRunCommit {
            output: ExternalPoolAdapterTaskProtocolConformanceRunWriteReceipt {
                run: stored.receipt,
                replayed: false,
            },
            run_receipt_id: receipt.run_receipt_id,
            receipt_integrity_digest: integrity,
        };
        tx.commit().map_err(StoreError::storage)?;
        fresh
    };
    if !runtime
        .process_custody()
        .promote_task_protocol_conformance_seal(
            &finalized.run_receipt_id,
            &finalized.receipt_integrity_digest,
        )
        .map_err(StoreError::storage)?
    {
        return Err(StoreError::storage(anyhow::anyhow!(
            "projected-active V272 row lost its pending process seal"
        )));
    }
    Ok(finalized.output)
}

fn now() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Nanos, true)
}
