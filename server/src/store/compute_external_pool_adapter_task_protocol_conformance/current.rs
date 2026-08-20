use anyhow::{bail, Result};
use chrono::{SecondsFormat, Utc};
use rusqlite::{params, OptionalExtension, Transaction, TransactionBehavior};

use crate::{
    compute_federation::{
        external_pool_adapter_installation::PreparedExternalPoolAdapterInstallation,
        external_pool_adapter_task_protocol_conformance::*,
    },
    store::Store,
};

use super::{
    error::ExternalPoolAdapterTaskProtocolConformanceStoreError as StoreError,
    read::{identifier, run_by_id_on, run_head_by_release_on},
    roots::{canonical_time, current_roots_for_receipt_on, CurrentTaskProtocolConformanceRoots},
    runtime::ExternalPoolAdapterTaskProtocolConformanceRuntime,
    types::*,
};

pub(super) struct RelationalCurrentness {
    head_status: String,
    revocation_status: String,
    ttl_status: String,
    registry_release_status: String,
    vulnerability_reattestation_status: String,
    sandbox_reattestation_status: String,
    sandbox_verifier_key_status: String,
    runtime_compatibility_verification_status: String,
    task_protocol_profile_status: String,
    fixture_catalog_status: String,
    canonical_receipt_integrity_status: String,
    receipt_integrity_status: String,
    process_custody_status: String,
    prepared_reproof_status: String,
    pub(super) current_status: String,
}

impl Store {
    /// Collection diagnostic for the structural head. It never opens a carrier and therefore can
    /// never return or mint consumable authority.
    pub(crate) fn external_pool_adapter_task_protocol_conformance_currentness(
        &self,
        registry_release_id: &str,
        runtime: Option<&ExternalPoolAdapterTaskProtocolConformanceRuntime>,
    ) -> std::result::Result<
        Option<ExternalPoolAdapterTaskProtocolConformanceCurrentness>,
        StoreError,
    > {
        identifier(registry_release_id).map_err(StoreError::conflict)?;
        let mut conn = self.conn().map_err(StoreError::storage)?;
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Deferred)
            .map_err(StoreError::storage)?;
        let output = (|| -> Result<_> {
            let Some(stored) = run_head_by_release_on(&tx, registry_release_id)? else {
                return Ok(None);
            };
            let relational = relational_currentness_on(&tx, &stored.receipt.run_receipt_id)?
                .ok_or_else(|| {
                    anyhow::anyhow!("task-protocol conformance diagnostic view lost its head")
                })?;
            let process_seal = runtime
                .map(|runtime| {
                    runtime
                        .process_custody()
                        .attests_task_protocol_conformance_seal(
                            &stored.receipt.run_receipt_id,
                            &stored.receipt_integrity_digest,
                            &stored.receipt.run_receipt_digest,
                            &stored.runtime_custody_epoch_digest,
                            &stored.process_hmac_seal,
                            &stored.receipt.run.expires_at,
                        )
                })
                .transpose()?
                .unwrap_or(false);
            let r = &stored.receipt.run;
            Ok(Some(
                ExternalPoolAdapterTaskProtocolConformanceCurrentness {
                    schema: TASK_PROTOCOL_CONFORMANCE_CURRENTNESS_SCHEMA.into(),
                    run: stored.receipt.clone(),
                    currentness_status: relational.current_status,
                    head_status: relational.head_status,
                    revocation_status: relational.revocation_status,
                    ttl_status: relational.ttl_status,
                    registry_release_status: relational.registry_release_status,
                    vulnerability_reattestation_status: relational
                        .vulnerability_reattestation_status,
                    sandbox_reattestation_status: relational.sandbox_reattestation_status,
                    sandbox_verifier_key_status: relational.sandbox_verifier_key_status,
                    runtime_compatibility_verification_status: relational
                        .runtime_compatibility_verification_status,
                    task_protocol_profile_status: relational.task_protocol_profile_status,
                    fixture_catalog_status: relational.fixture_catalog_status,
                    canonical_receipt_integrity_status: relational
                        .canonical_receipt_integrity_status,
                    receipt_integrity_status: relational.receipt_integrity_status,
                    process_custody_status: if process_seal {
                        "same_process_committed_seal_present_but_prepared_reproof_required".into()
                    } else {
                        relational.process_custody_status
                    },
                    prepared_reproof_status: relational.prepared_reproof_status,
                    checked_at: now(),
                    effects: r.effects.clone(),
                    readiness: task_protocol_conformance_no_readiness(),
                },
            ))
        })()
        .map_err(StoreError::storage)?;
        tx.commit().map_err(StoreError::storage)?;
        Ok(output)
    }

    pub(crate) fn external_pool_adapter_task_protocol_conformance_run_exists(
        &self,
        registry_release_id: &str,
        run_receipt_id: &str,
    ) -> std::result::Result<bool, StoreError> {
        identifier(registry_release_id).map_err(StoreError::conflict)?;
        identifier(run_receipt_id).map_err(StoreError::conflict)?;
        let conn = self.conn().map_err(StoreError::storage)?;
        Ok(run_by_id_on(&conn, run_receipt_id)
            .map_err(StoreError::storage)?
            .is_some_and(|stored| {
                stored.receipt.run.registry_release.registry_release_id == registry_release_id
            }))
    }
}

/// Only this same-transaction path can turn a durable V272 receipt into consumable Store-private
/// authority. Diagnostic currentness cannot call it because a fresh Prepared carrier is required.
#[allow(clippy::too_many_arguments)]
pub(in crate::store) fn current_external_pool_adapter_task_protocol_conformance_authority_on<
    'tx,
    'conn,
>(
    transaction: &'tx Transaction<'conn>,
    run_receipt_id: &str,
    expected_run_receipt_digest: &str,
    provider_binding_id: &str,
    expected_provider_binding_digest: &str,
    expected_installation_receipt_id: &str,
    expected_installation_receipt_digest: &str,
    prepared: PreparedExternalPoolAdapterInstallation,
    runtime: &ExternalPoolAdapterTaskProtocolConformanceRuntime,
    checked_at: &str,
) -> Result<Option<CurrentExternalPoolAdapterTaskProtocolConformanceAuthority<'tx, 'conn>>> {
    let Some(stored) = run_by_id_on(transaction, run_receipt_id)? else {
        return Ok(None);
    };
    if stored.receipt.run_receipt_digest != expected_run_receipt_digest {
        bail!("task-protocol conformance expected receipt digest is not exact")
    }
    let relational = relational_currentness_on(transaction, run_receipt_id)?
        .ok_or_else(|| anyhow::anyhow!("task-protocol conformance current view disappeared"))?;
    let r = &stored.receipt.run;
    if relational.current_status != TASK_PROTOCOL_CONFORMANCE_RELATIONAL_CURRENT_STATUS
        || canonical_time(&r.post_cleanup_checked_at)? > canonical_time(checked_at)?
        || canonical_time(&r.expires_at)? <= canonical_time(checked_at)?
        || !runtime
            .process_custody()
            .attests_task_protocol_conformance_seal(
                &stored.receipt.run_receipt_id,
                &stored.receipt_integrity_digest,
                &stored.receipt.run_receipt_digest,
                &stored.runtime_custody_epoch_digest,
                &stored.process_hmac_seal,
                &r.expires_at,
            )?
    {
        bail!("task-protocol conformance receipt is historical or lacks exact process custody")
    }
    let roots = current_roots_for_receipt_on(
        transaction,
        &stored.receipt,
        provider_binding_id,
        expected_provider_binding_digest,
        expected_installation_receipt_id,
        expected_installation_receipt_digest,
        prepared,
        checked_at,
    )?;
    let CurrentTaskProtocolConformanceRoots {
        carrier,
        vulnerability,
        sandbox,
        runtime_compatibility,
    } = roots;
    Ok(Some(
        CurrentExternalPoolAdapterTaskProtocolConformanceAuthority::new(
            transaction,
            stored.receipt,
            carrier,
            vulnerability,
            sandbox,
            runtime_compatibility,
            checked_at.into(),
        ),
    ))
}

pub(super) fn relational_currentness_on(
    conn: &rusqlite::Connection,
    run_receipt_id: &str,
) -> Result<Option<RelationalCurrentness>> {
    conn.query_row(
        "SELECT head_status,revocation_status,ttl_status,registry_release_status,
                vulnerability_reattestation_status,sandbox_reattestation_status,
                sandbox_verifier_key_status,runtime_compatibility_verification_status,
                task_protocol_profile_status,fixture_catalog_status,
                canonical_receipt_integrity_status,receipt_integrity_status,
                process_custody_status,
                prepared_reproof_status,current_status
           FROM compute_external_pool_adapter_task_protocol_conformance_current
          WHERE run_receipt_id=?1",
        params![run_receipt_id],
        |row| {
            Ok(RelationalCurrentness {
                head_status: row.get(0)?,
                revocation_status: row.get(1)?,
                ttl_status: row.get(2)?,
                registry_release_status: row.get(3)?,
                vulnerability_reattestation_status: row.get(4)?,
                sandbox_reattestation_status: row.get(5)?,
                sandbox_verifier_key_status: row.get(6)?,
                runtime_compatibility_verification_status: row.get(7)?,
                task_protocol_profile_status: row.get(8)?,
                fixture_catalog_status: row.get(9)?,
                canonical_receipt_integrity_status: row.get(10)?,
                receipt_integrity_status: row.get(11)?,
                process_custody_status: row.get(12)?,
                prepared_reproof_status: row.get(13)?,
                current_status: row.get(14)?,
            })
        },
    )
    .optional()
    .map_err(Into::into)
}

fn now() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Nanos, true)
}
