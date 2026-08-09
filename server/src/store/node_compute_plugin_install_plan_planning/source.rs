use anyhow::{bail, Result};
use rusqlite::{params, OptionalExtension, Transaction};

use super::{
    digest::{source_preparation_observed_digest, source_preparation_request_digest},
    types::PlanningSourceV2,
    validation::{validate_source_preparation_observation, validate_source_sharing_observation},
};

pub(super) fn resolve_exact_planning_source(
    tx: &Transaction<'_>,
    request: &homecli_proto::ComputePluginInstallPlanPlanningSnapshotRequestV2,
) -> Result<PlanningSourceV2> {
    let row = tx
        .query_row(
            "SELECT delivery.sharing_delivery_id, observation.id, request.request_digest,
                    request.consent_receipt_id, observation.observed_json,
                    observation.observed_digest, sharing_observation.observed_json
               FROM node_compute_plugin_install_plan_preparation_requests request
               JOIN node_compute_plugin_install_plan_preparation_delivery_events delivery
                 ON delivery.preparation_id=request.preparation_id
                AND delivery.node_id=request.node_id
                AND delivery.consent_receipt_id=request.consent_receipt_id
                AND delivery.policy_revision=request.policy_revision
                AND delivery.policy_digest=request.policy_digest
               JOIN node_compute_plugin_install_plan_preparation_observations observation
                 ON observation.delivery_id=delivery.delivery_id
                AND observation.preparation_id=request.preparation_id
                AND observation.node_id=request.node_id
                AND observation.consent_receipt_id=request.consent_receipt_id
                AND observation.policy_revision=request.policy_revision
                AND observation.policy_digest=request.policy_digest
                AND observation.policy_snapshot_digest=request.policy_snapshot_digest
               JOIN node_compute_plugin_sharing_consents consent
                 ON consent.receipt_id=request.consent_receipt_id
                AND consent.node_id=request.node_id
                AND consent.owner_user_id=request.owner_user_id
                AND consent.installation_identity_digest=request.installation_identity_digest
                AND consent.policy_revision=request.policy_revision
                AND consent.policy_digest=request.policy_digest
                AND consent.authorization_ref=request.authorization_ref
                AND consent.authorization_revision=request.authorization_revision
                AND consent.authorization_digest=request.authorization_digest
               JOIN node_compute_plugin_sharing_delivery_events sharing_delivery
                 ON sharing_delivery.delivery_id=delivery.sharing_delivery_id
                AND sharing_delivery.node_id=request.node_id
                AND sharing_delivery.consent_receipt_id=request.consent_receipt_id
                AND sharing_delivery.policy_revision=request.policy_revision
                AND sharing_delivery.policy_digest=request.policy_digest
               JOIN node_compute_plugin_sharing_observations sharing_observation
                 ON sharing_observation.delivery_id=sharing_delivery.delivery_id
                AND sharing_observation.node_id=request.node_id
                AND sharing_observation.consent_receipt_id=request.consent_receipt_id
                AND sharing_observation.policy_revision=request.policy_revision
                AND sharing_observation.policy_digest=request.policy_digest
               JOIN node_compute_sharing_policies policy
                 ON policy.node_id=request.node_id
                AND policy.owner_user_id=request.owner_user_id
                AND policy.plugin_consent_receipt_id=request.consent_receipt_id
                AND policy.plugin_installation_identity_digest=request.installation_identity_digest
                AND policy.plugin_policy_revision=request.policy_revision
                AND policy.plugin_policy_digest=request.policy_digest
                AND policy.plugin_authorization_ref=request.authorization_ref
                AND policy.plugin_authorization_revision=request.authorization_revision
                AND policy.plugin_authorization_digest=request.authorization_digest
               WHERE request.preparation_id=?1
                 AND request.request_schema=?14
                 AND request.node_id=?2 AND request.owner_user_id=?3
                AND request.installation_identity_digest=?4
                AND request.policy_revision=?5 AND request.policy_digest=?6
                AND request.policy_snapshot_digest=?7
                AND request.authorization_ref=?8
                AND request.authorization_revision=?9
                AND request.authorization_digest=?10
                AND delivery.delivery_id=?11
                AND delivery.event_sequence=2 AND delivery.event_kind='dispatched'
                AND delivery.detail_code IS NULL
                AND observation.observed_digest=?12 AND observation.accepted=1
                AND consent.plugin_runtime_requested=1
                AND consent.consent_schema=?13
                AND policy.plugin_consent_schema=consent.consent_schema
                AND policy.plugin_runtime_requested=consent.plugin_runtime_requested
                AND policy.enabled=consent.plugin_runtime_requested
                AND policy.allowed_model_ids_json=consent.allowed_model_ids_json
                AND policy.max_concurrent_runs=consent.max_concurrent_runs
                AND policy.daily_token_limit=consent.daily_token_limit
                AND sharing_delivery.event_sequence=2
                AND sharing_delivery.event_kind='dispatched'
                AND sharing_delivery.detail_code IS NULL
                AND sharing_observation.accepted=1
                AND policy.enabled=1 AND policy.plugin_runtime_requested=1
                AND (SELECT COUNT(*) FROM node_compute_plugin_install_plan_preparation_delivery_events d2
                      WHERE d2.delivery_id=delivery.delivery_id)=2
                AND (SELECT COUNT(*) FROM node_compute_plugin_install_plan_preparation_observations o2
                      WHERE o2.delivery_id=delivery.delivery_id)=1
                AND (SELECT COUNT(*) FROM node_compute_plugin_sharing_delivery_events s2
                      WHERE s2.delivery_id=sharing_delivery.delivery_id)=2
                AND (SELECT COUNT(*) FROM node_compute_plugin_sharing_observations so2
                      WHERE so2.delivery_id=sharing_delivery.delivery_id)=1",
            params![
                request.preparation_id,
                request.node_id,
                request.owner_user_id,
                request.installation_identity_digest,
                i64::try_from(request.policy_revision)?,
                request.policy_digest,
                request.policy_snapshot_digest,
                request.authorization.authorization_ref,
                i64::try_from(request.authorization.revision)?,
                request.authorization.digest,
                request.source_preparation_delivery_id,
                request.source_preparation_observation_digest,
                crate::store::NODE_COMPUTE_PLUGIN_SHARING_CONSENT_SCHEMA_V1,
                homecli_proto::COMPUTE_PLUGIN_INSTALL_PLAN_PREPARATION_REQUEST_V1_SCHEMA,
            ],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                ))
            },
        )
        .optional()?;
    let Some((
        sharing_delivery_id,
        observation_id,
        request_digest,
        consent_id,
        json,
        digest,
        sharing_json,
    )) = row
    else {
        bail!("算力插件 Planning Snapshot V2 缺少 exact v209 会话来源");
    };
    if digest != request.source_preparation_observation_digest
        || source_preparation_observed_digest(&json)? != digest
        || source_preparation_request_digest(request) != request_digest
    {
        bail!("算力插件 Planning Snapshot V2 来源 request/observation 摘要损坏");
    }
    let preparation_observed = validate_source_preparation_observation(request, &json)?;
    validate_source_sharing_observation(request, &sharing_json)?;
    Ok(PlanningSourceV2 {
        source_sharing_delivery_id: sharing_delivery_id,
        source_preparation_observation_id: observation_id,
        source_preparation_request_digest: request_digest,
        consent_receipt_id: consent_id,
        source_bootstrap_instance_id: preparation_observed.bootstrap_instance_id,
        source_configuration_generation: preparation_observed.configuration_generation,
        source_cancellation_generation: preparation_observed.cancellation_generation,
    })
}
