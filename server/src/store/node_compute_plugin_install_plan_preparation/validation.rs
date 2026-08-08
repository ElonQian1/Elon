use anyhow::{bail, Result};
use rusqlite::{params, OptionalExtension, Transaction};

use super::{
    digest::{preparation_intent_request_digest, preparation_request_digest},
    NodeComputePluginInstallPlanPreparationDispatchIntent,
};
use crate::store::{
    NodeComputePluginSharingAuthorization, NodeComputePluginSharingDispatchIntent,
    NODE_COMPUTE_PLUGIN_SHARING_CONSENT_SCHEMA_V1,
};

pub(super) fn validate_sharing_binding(
    sharing: &NodeComputePluginSharingDispatchIntent,
    authorization: &NodeComputePluginSharingAuthorization,
    policy_snapshot_digest: &str,
) -> Result<()> {
    if !bounded_identifier(&sharing.node_id)
        || !bounded_identifier(&sharing.owner_user_id)
        || !bounded_identifier(&sharing.consent_receipt_id)
        || !bounded_identifier(&authorization.authorization_ref)
        || sharing.policy_revision <= 0
        || authorization.revision != sharing.policy_revision
        || !is_sha256(&sharing.installation_identity_digest)
        || !is_sha256(&sharing.policy_digest)
        || authorization.digest != sharing.policy_digest
        || !is_sha256(policy_snapshot_digest)
    {
        bail!("算力插件 InstallPlan 准备请求绑定无效");
    }
    Ok(())
}

pub(super) fn current_sharing_head_matches(
    tx: &Transaction<'_>,
    sharing: &NodeComputePluginSharingDispatchIntent,
    authorization: &NodeComputePluginSharingAuthorization,
) -> Result<bool> {
    current_head_query(
        tx,
        &sharing.node_id,
        &sharing.owner_user_id,
        &sharing.consent_receipt_id,
        &sharing.installation_identity_digest,
        sharing.policy_revision,
        &sharing.policy_digest,
        &authorization.authorization_ref,
        authorization.revision,
        &authorization.digest,
    )
}

/// Proves the caller's public intent came through this exact durable policy delivery and that the
/// node accepted the exact snapshot. This keeps preparation causally behind the sharing ACK even
/// when another crate-local caller obtains or constructs a dispatch intent.
pub(super) fn accepted_sharing_ack_matches(
    tx: &Transaction<'_>,
    sharing: &NodeComputePluginSharingDispatchIntent,
    authorization: &NodeComputePluginSharingAuthorization,
    policy_snapshot_digest: &str,
) -> Result<bool> {
    let Some(expected_snapshot_digest) = expected_policy_snapshot_digest(sharing, authorization)
    else {
        return Ok(false);
    };
    if expected_snapshot_digest != policy_snapshot_digest {
        return Ok(false);
    }

    let (event_count, intent_count, dispatched_count) = tx.query_row(
        "SELECT COUNT(*),
                COALESCE(SUM(CASE WHEN node_id=?2 AND consent_receipt_id=?3
                  AND policy_revision=?4 AND policy_digest=?5
                  AND event_sequence=1 AND event_kind='intent_committed'
                  AND detail_code IS NULL THEN 1 ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN node_id=?2 AND consent_receipt_id=?3
                  AND policy_revision=?4 AND policy_digest=?5
                  AND event_sequence=2 AND event_kind='dispatched'
                  AND detail_code IS NULL THEN 1 ELSE 0 END), 0)
           FROM node_compute_plugin_sharing_delivery_events
          WHERE delivery_id=?1",
        params![
            sharing.delivery_id,
            sharing.node_id,
            sharing.consent_receipt_id,
            sharing.policy_revision,
            sharing.policy_digest
        ],
        |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, i64>(2)?,
            ))
        },
    )?;
    if (event_count, intent_count, dispatched_count) != (2, 1, 1) {
        return Ok(false);
    }

    let observation_count = tx.query_row(
        "SELECT COUNT(*) FROM node_compute_plugin_sharing_observations WHERE delivery_id=?1",
        params![sharing.delivery_id],
        |row| row.get::<_, i64>(0),
    )?;
    if observation_count != 1 {
        return Ok(false);
    }
    let observed_json = tx
        .query_row(
            "SELECT observed_json
               FROM node_compute_plugin_sharing_observations
              WHERE delivery_id=?1 AND node_id=?2 AND consent_receipt_id=?3
                AND policy_revision=?4 AND policy_digest=?5 AND accepted=1",
            params![
                sharing.delivery_id,
                sharing.node_id,
                sharing.consent_receipt_id,
                sharing.policy_revision,
                sharing.policy_digest
            ],
            |row| row.get::<_, String>(0),
        )
        .optional()?;
    let Some(observed_json) = observed_json else {
        return Ok(false);
    };
    exact_accepted_observation(&observed_json, sharing, policy_snapshot_digest)
}

fn expected_policy_snapshot_digest(
    sharing: &NodeComputePluginSharingDispatchIntent,
    authorization: &NodeComputePluginSharingAuthorization,
) -> Option<String> {
    let policy_revision = u64::try_from(sharing.policy_revision).ok()?;
    let authorization_revision = u64::try_from(authorization.revision).ok()?;
    let snapshot =
        crate::compute_plugin_sharing_directive::build_compute_plugin_sharing_policy_snapshot_v1(
            sharing.node_id.clone(),
            sharing.owner_user_id.clone(),
            sharing.installation_identity_digest.clone(),
            policy_revision,
            sharing.policy_digest.clone(),
            true,
            Some(homecli_proto::ComputePluginSharingAuthorizationBindingV1 {
                authorization_ref: authorization.authorization_ref.clone(),
                revision: authorization_revision,
                digest: authorization.digest.clone(),
            }),
        )
        .ok()?;
    crate::compute_plugin_sharing_directive::compute_plugin_sharing_policy_snapshot_digest(
        &snapshot,
    )
    .ok()
}

fn exact_accepted_observation(
    observed_json: &str,
    sharing: &NodeComputePluginSharingDispatchIntent,
    policy_snapshot_digest: &str,
) -> Result<bool> {
    let raw: serde_json::Value = serde_json::from_str(observed_json)?;
    let observed: homecli_proto::ComputePluginSharingPolicyObservedV1 =
        serde_json::from_value(raw.clone())?;
    if serde_json::to_value(&observed)? != raw {
        return Ok(false);
    }
    Ok(
        observed.schema == homecli_proto::COMPUTE_PLUGIN_SHARING_POLICY_OBSERVED_V1_SCHEMA
            && observed.node_id == sharing.node_id
            && observed.owner_user_id == sharing.owner_user_id
            && observed.installation_identity_digest.as_deref()
                == Some(sharing.installation_identity_digest.as_str())
            && observed.accepted
            && observed.observed_policy_revision == u64::try_from(sharing.policy_revision).ok()
            && observed.observed_policy_digest.as_deref() == Some(sharing.policy_digest.as_str())
            && observed.observed_snapshot_digest.as_deref() == Some(policy_snapshot_digest)
            && observed.phase == "blocked"
            && !observed.side_effects_started
            && observed.error_code.is_none(),
    )
}

pub(super) fn resolve_existing_preparation_id(
    tx: &Transaction<'_>,
    sharing: &NodeComputePluginSharingDispatchIntent,
    authorization: &NodeComputePluginSharingAuthorization,
    policy_snapshot_digest: &str,
) -> Result<Option<String>> {
    let stored = tx
        .query_row(
            "SELECT preparation_id, request_schema, request_digest, node_id, owner_user_id,
                    consent_receipt_id, installation_identity_digest, policy_revision,
                    policy_digest, policy_snapshot_digest, authorization_ref,
                    authorization_revision, authorization_digest
               FROM node_compute_plugin_install_plan_preparation_requests
              WHERE node_id=?1 AND consent_receipt_id=?2
                AND policy_revision=?3 AND policy_digest=?4",
            params![
                sharing.node_id,
                sharing.consent_receipt_id,
                sharing.policy_revision,
                sharing.policy_digest
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
                    row.get::<_, i64>(7)?,
                    row.get::<_, String>(8)?,
                    row.get::<_, String>(9)?,
                    row.get::<_, String>(10)?,
                    row.get::<_, i64>(11)?,
                    row.get::<_, String>(12)?,
                ))
            },
        )
        .optional()?;
    let Some(stored) = stored else {
        return Ok(None);
    };
    let expected_digest =
        preparation_request_digest(&stored.0, sharing, authorization, policy_snapshot_digest);
    if stored.1 != homecli_proto::COMPUTE_PLUGIN_INSTALL_PLAN_PREPARATION_REQUEST_V1_SCHEMA
        || stored.2 != expected_digest
        || stored.3 != sharing.node_id
        || stored.4 != sharing.owner_user_id
        || stored.5 != sharing.consent_receipt_id
        || stored.6 != sharing.installation_identity_digest
        || stored.7 != sharing.policy_revision
        || stored.8 != sharing.policy_digest
        || stored.9 != policy_snapshot_digest
        || stored.10 != authorization.authorization_ref
        || stored.11 != authorization.revision
        || stored.12 != authorization.digest
    {
        bail!("算力插件 InstallPlan 准备请求的不可变事实不一致");
    }
    Ok(Some(stored.0))
}

pub(super) fn delivery_binding_is_committed(
    tx: &Transaction<'_>,
    intent: &NodeComputePluginInstallPlanPreparationDispatchIntent,
) -> Result<bool> {
    binding_query(tx, intent, false)
}

pub(super) fn observation_binding_is_current(
    tx: &Transaction<'_>,
    intent: &NodeComputePluginInstallPlanPreparationDispatchIntent,
) -> Result<bool> {
    binding_query(tx, intent, true)
}

fn binding_query(
    tx: &Transaction<'_>,
    intent: &NodeComputePluginInstallPlanPreparationDispatchIntent,
    require_current_head: bool,
) -> Result<bool> {
    if intent.request_digest != preparation_intent_request_digest(intent) {
        return Ok(false);
    }
    let committed = tx
        .query_row(
            "SELECT 1
               FROM node_compute_plugin_install_plan_preparation_delivery_events d
               JOIN node_compute_plugin_install_plan_preparation_requests r
                 ON r.preparation_id=d.preparation_id AND r.node_id=d.node_id
                AND r.consent_receipt_id=d.consent_receipt_id
                AND r.policy_revision=d.policy_revision AND r.policy_digest=d.policy_digest
              WHERE d.delivery_id=?1 AND d.sharing_delivery_id=?2
                AND d.preparation_id=?3 AND d.node_id=?4
                AND d.consent_receipt_id=?5 AND d.policy_revision=?6 AND d.policy_digest=?7
                AND d.event_sequence=1 AND d.event_kind='intent_committed'
                AND r.request_schema=?8 AND r.owner_user_id=?9
                AND r.installation_identity_digest=?10 AND r.policy_snapshot_digest=?11
                AND r.authorization_ref=?12 AND r.authorization_revision=?13
                AND r.authorization_digest=?14 AND r.request_digest=?15",
            params![
                intent.delivery_id,
                intent.sharing_delivery_id,
                intent.preparation_id,
                intent.node_id,
                intent.consent_receipt_id,
                intent.policy_revision,
                intent.policy_digest,
                homecli_proto::COMPUTE_PLUGIN_INSTALL_PLAN_PREPARATION_REQUEST_V1_SCHEMA,
                intent.owner_user_id,
                intent.installation_identity_digest,
                intent.policy_snapshot_digest,
                intent.authorization.authorization_ref,
                intent.authorization.revision,
                intent.authorization.digest,
                intent.request_digest
            ],
            |_| Ok(()),
        )
        .optional()?
        .is_some();
    if !committed || !require_current_head {
        return Ok(committed);
    }
    let dispatched = tx
        .query_row(
            "SELECT 1
              FROM node_compute_plugin_install_plan_preparation_delivery_events
              WHERE delivery_id=?1 AND sharing_delivery_id=?2 AND preparation_id=?3 AND node_id=?4
                AND consent_receipt_id=?5 AND policy_revision=?6 AND policy_digest=?7
                AND event_sequence=2 AND event_kind='dispatched' AND detail_code IS NULL",
            params![
                intent.delivery_id,
                intent.sharing_delivery_id,
                intent.preparation_id,
                intent.node_id,
                intent.consent_receipt_id,
                intent.policy_revision,
                intent.policy_digest
            ],
            |_| Ok(()),
        )
        .optional()?
        .is_some();
    if !dispatched {
        return Ok(false);
    }
    current_head_query(
        tx,
        &intent.node_id,
        &intent.owner_user_id,
        &intent.consent_receipt_id,
        &intent.installation_identity_digest,
        intent.policy_revision,
        &intent.policy_digest,
        &intent.authorization.authorization_ref,
        intent.authorization.revision,
        &intent.authorization.digest,
    )
}

#[allow(clippy::too_many_arguments)]
fn current_head_query(
    tx: &Transaction<'_>,
    node_id: &str,
    owner_user_id: &str,
    consent_receipt_id: &str,
    installation_identity_digest: &str,
    policy_revision: i64,
    policy_digest: &str,
    authorization_ref: &str,
    authorization_revision: i64,
    authorization_digest: &str,
) -> Result<bool> {
    Ok(tx
        .query_row(
            "SELECT 1
               FROM node_compute_sharing_policies p
               JOIN node_compute_plugin_sharing_consents c
                 ON c.receipt_id=p.plugin_consent_receipt_id
                AND c.node_id=p.node_id AND c.owner_user_id=p.owner_user_id
                AND c.consent_schema=p.plugin_consent_schema
                AND c.installation_identity_digest=p.plugin_installation_identity_digest
                AND c.policy_revision=p.plugin_policy_revision
                AND c.policy_digest=p.plugin_policy_digest
                AND c.plugin_runtime_requested=p.plugin_runtime_requested
                AND c.plugin_runtime_requested=p.enabled
                AND c.allowed_model_ids_json=p.allowed_model_ids_json
                AND c.max_concurrent_runs=p.max_concurrent_runs
                AND c.daily_token_limit=p.daily_token_limit
                AND c.authorization_ref=p.plugin_authorization_ref
                AND c.authorization_revision=p.plugin_authorization_revision
                AND c.authorization_digest=p.plugin_authorization_digest
              WHERE p.node_id=?1 AND p.owner_user_id=?2
                AND p.enabled=1 AND p.plugin_runtime_requested=1
                AND p.plugin_consent_schema=?3 AND p.plugin_consent_receipt_id=?4
                AND p.plugin_installation_identity_digest=?5
                AND p.plugin_policy_revision=?6 AND p.plugin_policy_digest=?7
                AND p.plugin_authorization_ref=?8
                AND p.plugin_authorization_revision=?9
                AND p.plugin_authorization_digest=?10",
            params![
                node_id,
                owner_user_id,
                NODE_COMPUTE_PLUGIN_SHARING_CONSENT_SCHEMA_V1,
                consent_receipt_id,
                installation_identity_digest,
                policy_revision,
                policy_digest,
                authorization_ref,
                authorization_revision,
                authorization_digest
            ],
            |_| Ok(()),
        )
        .optional()?
        .is_some())
}

fn bounded_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 256
        && value.trim() == value
        && !value.chars().any(char::is_control)
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}
