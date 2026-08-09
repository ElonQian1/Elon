use anyhow::{bail, Result};
use rusqlite::{named_params, params, OptionalExtension, Transaction};

use crate::store::node_compute_plugin_install_plan_planning::{
    digest::{hashed_snapshot_json, planning_observed_json_and_digest},
    types::{
        DurableComputePluginInstallPlanPlanningSnapshotV2,
        NodeComputePluginInstallPlanPlanningDispatchIntentV2,
    },
    validation::validate_planning_observation,
    PLANNING_DELIVERY_REQUEST_SCHEMA_V2,
};

pub(super) fn validate_delivery_intent_readback(
    tx: &Transaction<'_>,
    intent: &NodeComputePluginInstallPlanPlanningDispatchIntentV2,
) -> Result<()> {
    let (total, exact) = tx.query_row(
        "SELECT COUNT(*), COALESCE(SUM(CASE WHEN
                  cloud_session_id=?2 AND source_sharing_delivery_id=?3
                  AND source_preparation_id=?4 AND source_preparation_delivery_id=?5
                  AND source_preparation_observation_id=?6
                  AND source_preparation_observation_digest=?7
                  AND source_preparation_request_digest=?8
                  AND source_bootstrap_instance_id=?9
                  AND source_configuration_generation=?10
                  AND source_cancellation_generation=?11
                  AND request_schema=?12 AND request_json=?13 AND request_digest=?14
                  AND node_id=?15 AND owner_user_id=?16 AND consent_receipt_id=?17
                  AND installation_identity_digest=?18 AND policy_revision=?19
                  AND policy_digest=?20 AND policy_snapshot_digest=?21
                  AND authorization_ref=?22 AND authorization_revision=?23
                  AND authorization_digest=?24 AND event_sequence=1
                  AND event_kind='intent_committed' AND observed_json IS NULL
                  AND observed_digest IS NULL AND observed_snapshot_ready IS NULL
                  AND observed_snapshot_json IS NULL AND observed_snapshot_digest IS NULL
                  AND detail_code IS NULL
                THEN 1 ELSE 0 END), 0)
           FROM node_compute_plugin_install_plan_planning_delivery_events_v2
          WHERE planning_delivery_id=?1",
        params![
            intent.planning_delivery_id,
            intent.cloud_session_id,
            intent.source_sharing_delivery_id,
            intent.source_preparation_id,
            intent.source_preparation_delivery_id,
            intent.source_preparation_observation_id,
            intent.source_preparation_observation_digest,
            intent.source_preparation_request_digest,
            intent.source_bootstrap_instance_id,
            i64::try_from(intent.source_configuration_generation)?,
            i64::try_from(intent.source_cancellation_generation)?,
            PLANNING_DELIVERY_REQUEST_SCHEMA_V2,
            intent.request_json,
            intent.request_digest,
            intent.request.node_id,
            intent.request.owner_user_id,
            intent.consent_receipt_id,
            intent.request.installation_identity_digest,
            i64::try_from(intent.request.policy_revision)?,
            intent.request.policy_digest,
            intent.request.policy_snapshot_digest,
            intent.request.authorization.authorization_ref,
            i64::try_from(intent.request.authorization.revision)?,
            intent.request.authorization.digest,
        ],
        |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
    )?;
    if total < 1 || exact != 1 {
        bail!("算力插件 Planning Snapshot V2 delivery intent exact readback 失败");
    }
    Ok(())
}

pub(super) fn validate_delivery_outcome_readback(
    tx: &Transaction<'_>,
    intent: &NodeComputePluginInstallPlanPlanningDispatchIntentV2,
    event_kind: &str,
    observed_json: Option<&str>,
    observed_digest: Option<&str>,
    observed_snapshot_ready: Option<bool>,
    observed_snapshot_json: Option<&str>,
    observed_snapshot_digest: Option<&str>,
    detail_code: Option<&str>,
) -> Result<()> {
    validate_delivery_intent_readback(tx, intent)?;
    let exact = tx.query_row(
        "SELECT COUNT(*)
           FROM node_compute_plugin_install_plan_planning_delivery_events_v2
          WHERE planning_delivery_id=?1 AND cloud_session_id=?2
            AND source_sharing_delivery_id=?3 AND source_preparation_id=?4
            AND source_preparation_delivery_id=?5
            AND source_preparation_observation_id=?6
            AND source_preparation_observation_digest=?7
            AND source_preparation_request_digest=?8
            AND source_bootstrap_instance_id=?9
            AND source_configuration_generation=?10
            AND source_cancellation_generation=?11
            AND request_schema=?12 AND request_json=?13 AND request_digest=?14
            AND node_id=?15 AND owner_user_id=?16 AND consent_receipt_id=?17
            AND installation_identity_digest=?18 AND policy_revision=?19
            AND policy_digest=?20 AND policy_snapshot_digest=?21
            AND authorization_ref=?22 AND authorization_revision=?23
            AND authorization_digest=?24 AND event_sequence=2 AND event_kind=?25
            AND observed_json IS ?26 AND observed_digest IS ?27
            AND observed_snapshot_ready IS ?28 AND observed_snapshot_json IS ?29
            AND observed_snapshot_digest IS ?30 AND detail_code IS ?31",
        params![
            intent.planning_delivery_id,
            intent.cloud_session_id,
            intent.source_sharing_delivery_id,
            intent.source_preparation_id,
            intent.source_preparation_delivery_id,
            intent.source_preparation_observation_id,
            intent.source_preparation_observation_digest,
            intent.source_preparation_request_digest,
            intent.source_bootstrap_instance_id,
            i64::try_from(intent.source_configuration_generation)?,
            i64::try_from(intent.source_cancellation_generation)?,
            PLANNING_DELIVERY_REQUEST_SCHEMA_V2,
            intent.request_json,
            intent.request_digest,
            intent.request.node_id,
            intent.request.owner_user_id,
            intent.consent_receipt_id,
            intent.request.installation_identity_digest,
            i64::try_from(intent.request.policy_revision)?,
            intent.request.policy_digest,
            intent.request.policy_snapshot_digest,
            intent.request.authorization.authorization_ref,
            i64::try_from(intent.request.authorization.revision)?,
            intent.request.authorization.digest,
            event_kind,
            observed_json,
            observed_digest,
            observed_snapshot_ready,
            observed_snapshot_json,
            observed_snapshot_digest,
            detail_code,
        ],
        |row| row.get::<_, i64>(0),
    )?;
    let total = tx.query_row(
        "SELECT COUNT(*) FROM node_compute_plugin_install_plan_planning_delivery_events_v2
          WHERE planning_delivery_id=?1",
        params![intent.planning_delivery_id],
        |row| row.get::<_, i64>(0),
    )?;
    if total != 2 || exact != 1 {
        bail!("算力插件 Planning Snapshot V2 delivery outcome exact readback 失败");
    }
    Ok(())
}

pub(super) fn read_planning_snapshot(
    tx: &Transaction<'_>,
    intent: &NodeComputePluginInstallPlanPlanningDispatchIntentV2,
) -> Result<Option<DurableComputePluginInstallPlanPlanningSnapshotV2>> {
    let observed_json = tx
        .query_row(
            "SELECT observed_json
               FROM node_compute_plugin_install_plan_planning_delivery_events_v2
              WHERE planning_delivery_id=?1 AND event_sequence=2 AND event_kind='observed'",
            params![intent.planning_delivery_id],
            |row| row.get::<_, String>(0),
        )
        .optional()?;
    let Some(observed_json) = observed_json else {
        return Ok(None);
    };
    let observed: homecli_proto::ComputePluginInstallPlanPlanningSnapshotObservedV2 =
        serde_json::from_str(&observed_json)?;
    validate_planning_observation(intent, &observed)?;
    let (canonical_observed, observed_digest) = planning_observed_json_and_digest(&observed)?;
    if canonical_observed != observed_json {
        bail!("算力插件 Planning Snapshot V2 observation 规范 readback 失败");
    }
    let observed_snapshot_json = observed
        .snapshot
        .as_ref()
        .map(hashed_snapshot_json)
        .transpose()?;
    let observed_snapshot_digest = observed
        .snapshot
        .as_ref()
        .map(|snapshot| snapshot.snapshot_digest.as_str());
    validate_delivery_outcome_readback(
        tx,
        intent,
        "observed",
        Some(&observed_json),
        Some(&observed_digest),
        Some(observed.snapshot_ready),
        observed_snapshot_json.as_deref(),
        observed_snapshot_digest,
        observed.error_code.as_deref(),
    )?;
    let Some(expected) = observed.snapshot.as_ref() else {
        let count = tx.query_row(
            "SELECT COUNT(*) FROM node_compute_plugin_install_plan_planning_snapshots_v2
              WHERE planning_delivery_id=?1",
            params![intent.planning_delivery_id],
            |row| row.get::<_, i64>(0),
        )?;
        if count != 0 {
            bail!("算力插件 Planning Snapshot V2 未就绪 ACK 出现快照行");
        }
        return Ok(None);
    };
    let row = tx
        .query_row(
            "SELECT snapshot_id, snapshot_json
               FROM node_compute_plugin_install_plan_planning_snapshots_v2
              WHERE planning_delivery_id=?1",
            params![intent.planning_delivery_id],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )
        .optional()?;
    let Some((snapshot_id, snapshot_json)) = row else {
        bail!("算力插件 Planning Snapshot V2 ready ACK 缺少快照行");
    };
    let stored: homecli_proto::HashedComputePluginInstallPlanPlanningSnapshotV2 =
        serde_json::from_str(&snapshot_json)?;
    if &stored != expected || hashed_snapshot_json(&stored)? != snapshot_json {
        bail!("算力插件 Planning Snapshot V2 快照 JSON/digest readback 失败");
    }
    let durable = DurableComputePluginInstallPlanPlanningSnapshotV2 {
        snapshot_id,
        snapshot: stored,
        snapshot_json,
        planning_delivery_id: intent.planning_delivery_id.clone(),
        consent_receipt_id: intent.consent_receipt_id.clone(),
        source_preparation_observation_id: intent.source_preparation_observation_id.clone(),
        source_preparation_request_digest: intent.source_preparation_request_digest.clone(),
    };
    validate_durable_snapshot_readback(tx, &durable)?;
    Ok(Some(durable))
}

pub(super) fn validate_durable_snapshot_readback(
    tx: &Transaction<'_>,
    durable: &DurableComputePluginInstallPlanPlanningSnapshotV2,
) -> Result<()> {
    let hashed = &durable.snapshot;
    if hashed_snapshot_json(hashed)? != durable.snapshot_json {
        bail!("算力插件 Planning Snapshot V2 durable evidence 不是 exact JCS");
    }
    let value = &hashed.snapshot;
    let exact = tx.query_row(
        "SELECT COUNT(*) FROM node_compute_plugin_install_plan_planning_snapshots_v2
          WHERE snapshot_id=:snapshot_id AND snapshot_schema=:snapshot_schema
            AND snapshot_json=:snapshot_json AND snapshot_digest=:snapshot_digest
            AND planning_delivery_id=:delivery AND cloud_session_id=:session
            AND source_preparation_id=:preparation
            AND source_preparation_delivery_id=:source_delivery
            AND source_preparation_observation_id=:source_observation
            AND source_preparation_observation_digest=:source_observed_digest
            AND source_preparation_request_digest=:source_request_digest
            AND node_id=:node AND owner_user_id=:owner AND consent_receipt_id=:consent
            AND installation_identity_digest=:installation
            AND policy_revision=:policy_revision AND policy_digest=:policy_digest
            AND policy_snapshot_digest=:policy_snapshot_digest
            AND authorization_ref=:authorization_ref
            AND authorization_revision=:authorization_revision
            AND authorization_digest=:authorization_digest
            AND bootstrap_instance_id=:bootstrap
            AND configuration_generation=:configuration_generation
            AND cancellation_generation=:cancellation_generation
            AND policy_binding_receipt_digest=:binding_receipt
            AND policy_capability_revocation_receipt_digest=:revocation_receipt
            AND policy_binding_authority_epoch=:binding_authority_epoch
            AND policy_binding_process_owner_epoch=:binding_process_epoch
            AND authority_state_revision=:state_revision AND authority_epoch=:authority_epoch
            AND process_owner_epoch=:process_epoch AND clock_epoch_digest=:clock_epoch
            AND trusted_time_high_water_ms=:high_water AND captured_at_ms=:captured
            AND expires_at_ms=:expires AND rollback_anchor_witness_digest=:rollback
            AND inventory_revision=:inventory_revision AND inventory_digest=:inventory_digest
            AND node_profile_digest=:profile AND manifest_catalog_revision=:catalog_revision
            AND manifest_catalog_digest=:catalog_digest
            AND keyring_bundle_revision=:bundle_revision
            AND publisher_keyring_revision=:publisher_revision
            AND publisher_keyring_digest=:publisher_digest
            AND control_keyring_revision=:control_revision
            AND control_keyring_digest=:control_digest
            AND target_id=:target AND host_api_protocol_id=:host_protocol
            AND host_api_revision=:host_revision AND installed_record_count=:record_count",
        named_params! {
            ":snapshot_id": &durable.snapshot_id,
            ":snapshot_schema": &hashed.schema,
            ":snapshot_json": &durable.snapshot_json,
            ":snapshot_digest": &hashed.snapshot_digest,
            ":delivery": &durable.planning_delivery_id,
            ":session": &value.cloud_session_id,
            ":preparation": &value.preparation_id,
            ":source_delivery": &value.source_preparation_delivery_id,
            ":source_observation": &durable.source_preparation_observation_id,
            ":source_observed_digest": &value.source_preparation_observation_digest,
            ":source_request_digest": &durable.source_preparation_request_digest,
            ":node": &value.node_id,
            ":owner": &value.owner_user_id,
            ":consent": &durable.consent_receipt_id,
            ":installation": &value.installation_identity_digest,
            ":policy_revision": i64::try_from(value.policy_revision)?,
            ":policy_digest": &value.policy_digest,
            ":policy_snapshot_digest": &value.policy_snapshot_digest,
            ":authorization_ref": &value.authorization.authorization_ref,
            ":authorization_revision": i64::try_from(value.authorization.revision)?,
            ":authorization_digest": &value.authorization.digest,
            ":bootstrap": &value.bootstrap_instance_id,
            ":configuration_generation": i64::try_from(value.configuration_generation)?,
            ":cancellation_generation": i64::try_from(value.cancellation_generation)?,
            ":binding_receipt": &value.policy_binding_receipt_digest,
            ":revocation_receipt": &value.policy_capability_revocation_receipt_digest,
            ":binding_authority_epoch": i64::try_from(value.policy_binding_authority_epoch)?,
            ":binding_process_epoch": i64::try_from(value.policy_binding_process_owner_epoch)?,
            ":state_revision": i64::try_from(value.authority_state_revision)?,
            ":authority_epoch": i64::try_from(value.authority_epoch)?,
            ":process_epoch": i64::try_from(value.process_owner_epoch)?,
            ":clock_epoch": &value.clock_epoch_digest,
            ":high_water": i64::try_from(value.trusted_time_high_water_ms)?,
            ":captured": i64::try_from(value.captured_at_ms)?,
            ":expires": i64::try_from(value.expires_at_ms)?,
            ":rollback": &value.rollback_anchor_witness_digest,
            ":inventory_revision": i64::try_from(value.inventory_revision)?,
            ":inventory_digest": &value.inventory_digest,
            ":profile": &value.node_profile_digest,
            ":catalog_revision": i64::try_from(value.manifest_catalog_revision)?,
            ":catalog_digest": &value.manifest_catalog_digest,
            ":bundle_revision": i64::try_from(value.keyring_bundle_revision)?,
            ":publisher_revision": i64::try_from(value.publisher_keyring.revision)?,
            ":publisher_digest": &value.publisher_keyring.digest,
            ":control_revision": i64::try_from(value.control_keyring.revision)?,
            ":control_digest": &value.control_keyring.digest,
            ":target": &value.target_id,
            ":host_protocol": &value.host_api_protocol_id,
            ":host_revision": i64::from(value.host_api_revision),
            ":record_count": i64::try_from(value.installed_records.len())?,
        },
        |row| row.get::<_, i64>(0),
    )?;
    if exact != 1 {
        bail!("算力插件 Planning Snapshot V2 冗余列 exact readback 失败");
    }
    Ok(())
}
