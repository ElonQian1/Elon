use anyhow::{bail, Result};
use rusqlite::{params, OptionalExtension, Transaction};

use crate::store::node_compute_plugin_install_plan_planning::{
    digest::{generation_outcome_json_and_digest, generation_request_json_and_digest},
    types::{
        ComputePluginInstallPlanGenerationOutcomeV1, ComputePluginInstallPlanGenerationRequestV1,
        DurableComputePluginInstallPlanGenerationOutcomeV1,
        DurableComputePluginInstallPlanGenerationRequestV1,
        DurableComputePluginInstallPlanPlanningSnapshotV2,
    },
    validation::{validate_generation_outcome, validate_generation_request},
};

pub(in crate::store::node_compute_plugin_install_plan_planning) fn read_generation_request(
    tx: &Transaction<'_>,
    snapshot_id: &str,
) -> Result<Option<DurableComputePluginInstallPlanGenerationRequestV1>> {
    let row = tx
        .query_row(
            "SELECT request_json, request_digest
               FROM node_compute_plugin_install_plan_generation_requests_v1
              WHERE snapshot_id=?1",
            params![snapshot_id],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )
        .optional()?;
    let Some((request_json, request_digest)) = row else {
        return Ok(None);
    };
    let request: ComputePluginInstallPlanGenerationRequestV1 = serde_json::from_str(&request_json)?;
    validate_generation_request(&request)?;
    let (canonical, digest) = generation_request_json_and_digest(&request)?;
    if canonical != request_json || digest != request_digest {
        bail!("算力插件 InstallPlan generation request JSON/digest readback 失败");
    }
    let durable = DurableComputePluginInstallPlanGenerationRequestV1 {
        request,
        request_json,
        request_digest,
    };
    validate_generation_request_readback(tx, &durable)?;
    Ok(Some(durable))
}

pub(in crate::store::node_compute_plugin_install_plan_planning) fn validate_generation_request_readback(
    tx: &Transaction<'_>,
    durable: &DurableComputePluginInstallPlanGenerationRequestV1,
) -> Result<()> {
    let request = &durable.request;
    let exact = tx.query_row(
        "SELECT COUNT(*) FROM node_compute_plugin_install_plan_generation_requests_v1
          WHERE generation_request_id=?1 AND request_schema=?2 AND request_json=?3
            AND request_digest=?4 AND snapshot_id=?5 AND snapshot_digest=?6
            AND node_id=?7 AND owner_user_id=?8 AND installation_identity_digest=?9
            AND policy_revision=?10 AND policy_digest=?11 AND authorization_ref=?12
            AND authorization_revision=?13 AND authorization_digest=?14
            AND requested_control_keyring_revision=?15
            AND requested_control_keyring_digest=?16 AND signer_profile=?17
            AND requested_at_ms=?18",
        params![
            request.generation_request_id,
            request.schema,
            durable.request_json,
            durable.request_digest,
            request.snapshot_id,
            request.snapshot_digest,
            request.node_id,
            request.owner_user_id,
            request.installation_identity_digest,
            i64::try_from(request.policy_revision)?,
            request.policy_digest,
            request.authorization_ref,
            i64::try_from(request.authorization_revision)?,
            request.authorization_digest,
            i64::try_from(request.requested_control_keyring_revision)?,
            request.requested_control_keyring_digest,
            request.signer_profile,
            i64::try_from(request.requested_at_ms)?,
        ],
        |row| row.get::<_, i64>(0),
    )?;
    if exact != 1 {
        bail!("算力插件 InstallPlan generation request 冗余列 exact readback 失败");
    }
    Ok(())
}

pub(in crate::store::node_compute_plugin_install_plan_planning) fn validate_generation_snapshot_authority(
    tx: &Transaction<'_>,
    durable: &DurableComputePluginInstallPlanPlanningSnapshotV2,
) -> Result<()> {
    let snapshot = &durable.snapshot.snapshot;
    let exact = tx.query_row(
        "SELECT COUNT(*)
           FROM node_compute_sharing_policies policy
           JOIN node_compute_plugin_sharing_consents consent
             ON consent.receipt_id=?3
            AND consent.node_id=policy.node_id
            AND consent.owner_user_id=policy.owner_user_id
            AND consent.installation_identity_digest=?4
            AND consent.policy_revision=?5 AND consent.policy_digest=?6
            AND consent.authorization_ref=?7
            AND consent.authorization_revision=?8
            AND consent.authorization_digest=?9
          WHERE policy.node_id=?1 AND policy.owner_user_id=?2
            AND policy.enabled=1 AND policy.plugin_runtime_requested=1
            AND policy.plugin_consent_receipt_id=consent.receipt_id
            AND policy.plugin_installation_identity_digest=consent.installation_identity_digest
            AND policy.plugin_policy_revision=consent.policy_revision
            AND policy.plugin_policy_digest=consent.policy_digest
            AND policy.plugin_authorization_ref=consent.authorization_ref
            AND policy.plugin_authorization_revision=consent.authorization_revision
            AND policy.plugin_authorization_digest=consent.authorization_digest
            AND policy.plugin_consent_schema=consent.consent_schema
            AND policy.allowed_model_ids_json=consent.allowed_model_ids_json
            AND policy.max_concurrent_runs=consent.max_concurrent_runs
            AND policy.daily_token_limit=consent.daily_token_limit
            AND consent.plugin_runtime_requested=1 AND consent.consent_schema=?10",
        params![
            snapshot.node_id,
            snapshot.owner_user_id,
            durable.consent_receipt_id,
            snapshot.installation_identity_digest,
            i64::try_from(snapshot.policy_revision)?,
            snapshot.policy_digest,
            snapshot.authorization.authorization_ref,
            i64::try_from(snapshot.authorization.revision)?,
            snapshot.authorization.digest,
            crate::store::NODE_COMPUTE_PLUGIN_SHARING_CONSENT_SCHEMA_V1,
        ],
        |row| row.get::<_, i64>(0),
    )?;
    if exact != 1 {
        bail!("算力插件 Planning Snapshot V2 的当前策略或不可变 consent 已漂移");
    }
    Ok(())
}

pub(in crate::store::node_compute_plugin_install_plan_planning) fn read_generation_outcome(
    tx: &Transaction<'_>,
    generation_request_id: &str,
) -> Result<Option<DurableComputePluginInstallPlanGenerationOutcomeV1>> {
    let row = tx
        .query_row(
            "SELECT outcome_json, outcome_digest
               FROM node_compute_plugin_install_plan_generation_outcomes_v1
              WHERE generation_request_id=?1",
            params![generation_request_id],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )
        .optional()?;
    let Some((outcome_json, outcome_digest)) = row else {
        return Ok(None);
    };
    let outcome: ComputePluginInstallPlanGenerationOutcomeV1 = serde_json::from_str(&outcome_json)?;
    validate_generation_outcome(&outcome)?;
    let (canonical, digest) = generation_outcome_json_and_digest(&outcome)?;
    if canonical != outcome_json || digest != outcome_digest {
        bail!("算力插件 InstallPlan generation outcome JSON/digest readback 失败");
    }
    let durable = DurableComputePluginInstallPlanGenerationOutcomeV1 {
        outcome,
        outcome_json,
        outcome_digest,
    };
    validate_generation_outcome_readback(tx, &durable)?;
    Ok(Some(durable))
}

pub(in crate::store::node_compute_plugin_install_plan_planning) fn validate_generation_outcome_readback(
    tx: &Transaction<'_>,
    durable: &DurableComputePluginInstallPlanGenerationOutcomeV1,
) -> Result<()> {
    let outcome = &durable.outcome;
    let exact = tx.query_row(
        "SELECT COUNT(*) FROM node_compute_plugin_install_plan_generation_outcomes_v1
          WHERE outcome_id=?1 AND outcome_schema=?2 AND outcome_json=?3
            AND outcome_digest=?4 AND generation_request_id=?5
            AND generation_request_digest=?6 AND outcome_kind=?7
            AND detail_code=?8 AND retryable=?9",
        params![
            outcome.outcome_id,
            outcome.schema,
            durable.outcome_json,
            durable.outcome_digest,
            outcome.generation_request_id,
            outcome.generation_request_digest,
            outcome.outcome_kind,
            outcome.detail_code,
            outcome.retryable,
        ],
        |row| row.get::<_, i64>(0),
    )?;
    if exact != 1 {
        bail!("算力插件 InstallPlan generation outcome 冗余列 exact readback 失败");
    }
    Ok(())
}
