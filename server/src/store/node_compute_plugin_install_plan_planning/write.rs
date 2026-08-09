use anyhow::{bail, Result};
use rusqlite::{named_params, params, OptionalExtension, Transaction, TransactionBehavior};

mod generation;

use generation::ensure_signer_unavailable_in_transaction;

use super::{
    digest::{
        hashed_snapshot_json, planning_observed_json_and_digest, planning_request_json_and_digest,
    },
    readback::{
        read_planning_snapshot, validate_delivery_intent_readback,
        validate_delivery_outcome_readback,
    },
    source::resolve_exact_planning_source,
    types::{
        NodeComputePluginInstallPlanPlanningDispatchIntentV2, PlanningDeliveryRequestEnvelopeV2,
        PlanningSnapshotObservationCommitV2, PlanningSourceV2,
    },
    validation::{validate_planning_observation, validate_planning_request},
    PLANNING_DELIVERY_REQUEST_SCHEMA_V2,
};
use crate::store::{new_id, now, Store};

impl Store {
    pub(crate) fn prepare_node_compute_plugin_install_plan_planning_delivery_v2(
        &self,
        request: homecli_proto::ComputePluginInstallPlanPlanningSnapshotRequestV2,
    ) -> Result<NodeComputePluginInstallPlanPlanningDispatchIntentV2> {
        validate_planning_request(&request)?;
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let source = resolve_exact_planning_source(&tx, &request)?;
        if let Some(delivery_id) = tx
            .query_row(
                "SELECT planning_delivery_id
                   FROM node_compute_plugin_install_plan_planning_delivery_events_v2
                  WHERE source_preparation_delivery_id=?1 AND event_sequence=1",
                params![request.source_preparation_delivery_id],
                |row| row.get::<_, String>(0),
            )
            .optional()?
        {
            let mut intent = build_intent(delivery_id, request, source, true)?;
            validate_delivery_intent_readback(&tx, &intent)?;
            let event_count = delivery_event_count(&tx, &intent.planning_delivery_id)?;
            if !(1..=2).contains(&event_count) {
                bail!("算力插件 Planning Snapshot V2 delivery event 链损坏");
            }
            intent.dispatchable = event_count == 1;
            tx.commit()?;
            return Ok(intent);
        }

        let intent = build_intent(new_id("cpv2d"), request, source, false)?;
        insert_delivery_event(
            &tx,
            &intent,
            1,
            "intent_committed",
            None,
            None,
            None,
            None,
            None,
            None,
        )?;
        validate_delivery_intent_readback(&tx, &intent)?;
        tx.commit()?;
        Ok(intent)
    }

    pub(crate) fn record_node_compute_plugin_install_plan_planning_delivery_failure_v2(
        &self,
        intent: &NodeComputePluginInstallPlanPlanningDispatchIntentV2,
        event_kind: &str,
        detail_code: &str,
    ) -> Result<()> {
        if !matches!(
            event_kind,
            "capability_missing"
                | "agent_offline"
                | "session_replaced"
                | "writer_closed"
                | "ack_timeout"
                | "dispatch_failed"
        ) || !stable_code(detail_code)
        {
            bail!("算力插件 Planning Snapshot V2 delivery failure 无效");
        }
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        validate_delivery_intent_readback(&tx, intent)?;
        if delivery_event_count(&tx, &intent.planning_delivery_id)? == 1 {
            insert_delivery_event(
                &tx,
                intent,
                2,
                event_kind,
                None,
                None,
                None,
                None,
                None,
                Some(detail_code),
            )?;
        }
        validate_delivery_outcome_readback(
            &tx,
            intent,
            event_kind,
            None,
            None,
            None,
            None,
            None,
            Some(detail_code),
        )?;
        tx.commit()?;
        Ok(())
    }

    pub(crate) fn record_node_compute_plugin_install_plan_planning_observation_v2(
        &self,
        intent: &NodeComputePluginInstallPlanPlanningDispatchIntentV2,
        observed: &homecli_proto::ComputePluginInstallPlanPlanningSnapshotObservedV2,
    ) -> Result<PlanningSnapshotObservationCommitV2> {
        validate_planning_observation(intent, observed)?;
        let (observed_json, observed_digest) = planning_observed_json_and_digest(observed)?;
        let observed_snapshot_json = observed
            .snapshot
            .as_ref()
            .map(hashed_snapshot_json)
            .transpose()?;
        let observed_snapshot_digest = observed
            .snapshot
            .as_ref()
            .map(|snapshot| snapshot.snapshot_digest.as_str());
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        validate_source_matches_intent(&tx, intent)?;
        validate_delivery_intent_readback(&tx, intent)?;
        if delivery_event_count(&tx, &intent.planning_delivery_id)? == 1 {
            insert_delivery_event(
                &tx,
                intent,
                2,
                "observed",
                Some(&observed_json),
                Some(&observed_digest),
                Some(observed.snapshot_ready),
                observed_snapshot_json.as_deref(),
                observed_snapshot_digest,
                observed.error_code.as_deref(),
            )?;
            if let Some(hashed) = observed.snapshot.as_ref() {
                insert_planning_snapshot(&tx, intent, hashed)?;
            }
        }
        validate_delivery_outcome_readback(
            &tx,
            intent,
            "observed",
            Some(&observed_json),
            Some(&observed_digest),
            Some(observed.snapshot_ready),
            observed_snapshot_json.as_deref(),
            observed_snapshot_digest,
            observed.error_code.as_deref(),
        )?;
        let snapshot = read_planning_snapshot(&tx, intent)?;
        if let Some(snapshot) = snapshot.as_ref() {
            ensure_signer_unavailable_in_transaction(&tx, snapshot)?;
        }
        tx.commit()?;
        Ok(match snapshot {
            Some(snapshot) => PlanningSnapshotObservationCommitV2::Snapshot(snapshot),
            None => PlanningSnapshotObservationCommitV2::ObservedWithoutSnapshot,
        })
    }
}

fn build_intent(
    delivery_id: String,
    request: homecli_proto::ComputePluginInstallPlanPlanningSnapshotRequestV2,
    source: PlanningSourceV2,
    replayed: bool,
) -> Result<NodeComputePluginInstallPlanPlanningDispatchIntentV2> {
    let envelope = PlanningDeliveryRequestEnvelopeV2 {
        schema: PLANNING_DELIVERY_REQUEST_SCHEMA_V2.to_string(),
        planning_delivery_id: delivery_id.clone(),
        cloud_session_id: request.cloud_session_id.clone(),
        source_sharing_delivery_id: source.source_sharing_delivery_id.clone(),
        source_preparation_observation_id: source.source_preparation_observation_id.clone(),
        source_preparation_request_digest: source.source_preparation_request_digest.clone(),
        source_bootstrap_instance_id: source.source_bootstrap_instance_id.clone(),
        source_configuration_generation: source.source_configuration_generation,
        source_cancellation_generation: source.source_cancellation_generation,
        consent_receipt_id: source.consent_receipt_id.clone(),
        request: request.clone(),
    };
    let (request_json, request_digest) = planning_request_json_and_digest(&envelope)?;
    Ok(NodeComputePluginInstallPlanPlanningDispatchIntentV2 {
        planning_delivery_id: delivery_id,
        cloud_session_id: request.cloud_session_id.clone(),
        source_sharing_delivery_id: source.source_sharing_delivery_id,
        source_preparation_id: request.preparation_id.clone(),
        source_preparation_delivery_id: request.source_preparation_delivery_id.clone(),
        source_preparation_observation_id: source.source_preparation_observation_id,
        source_preparation_observation_digest: request
            .source_preparation_observation_digest
            .clone(),
        source_preparation_request_digest: source.source_preparation_request_digest,
        source_bootstrap_instance_id: source.source_bootstrap_instance_id,
        source_configuration_generation: source.source_configuration_generation,
        source_cancellation_generation: source.source_cancellation_generation,
        request,
        request_json,
        request_digest,
        consent_receipt_id: source.consent_receipt_id,
        replayed,
        dispatchable: !replayed,
    })
}

fn insert_delivery_event(
    tx: &Transaction<'_>,
    intent: &NodeComputePluginInstallPlanPlanningDispatchIntentV2,
    sequence: i64,
    kind: &str,
    observed_json: Option<&str>,
    observed_digest: Option<&str>,
    observed_snapshot_ready: Option<bool>,
    observed_snapshot_json: Option<&str>,
    observed_snapshot_digest: Option<&str>,
    detail_code: Option<&str>,
) -> Result<()> {
    tx.execute(
        "INSERT INTO node_compute_plugin_install_plan_planning_delivery_events_v2 (
           id, planning_delivery_id, cloud_session_id, source_sharing_delivery_id,
           source_preparation_id, source_preparation_delivery_id,
           source_preparation_observation_id, source_preparation_observation_digest,
           source_preparation_request_digest, source_bootstrap_instance_id,
           source_configuration_generation, source_cancellation_generation,
           request_schema, request_json, request_digest,
           node_id, owner_user_id, consent_receipt_id, installation_identity_digest,
           policy_revision, policy_digest, policy_snapshot_digest, authorization_ref,
           authorization_revision, authorization_digest, event_sequence, event_kind,
           observed_json, observed_digest, observed_snapshot_ready,
           observed_snapshot_json, observed_snapshot_digest, detail_code, created_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13,
                   ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25,
                   ?26, ?27, ?28, ?29, ?30, ?31, ?32, ?33, ?34)",
        params![
            new_id("cpv2e"),
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
            sequence,
            kind,
            observed_json,
            observed_digest,
            observed_snapshot_ready,
            observed_snapshot_json,
            observed_snapshot_digest,
            detail_code,
            now(),
        ],
    )?;
    Ok(())
}

fn insert_planning_snapshot(
    tx: &Transaction<'_>,
    intent: &NodeComputePluginInstallPlanPlanningDispatchIntentV2,
    hashed: &homecli_proto::HashedComputePluginInstallPlanPlanningSnapshotV2,
) -> Result<()> {
    let snapshot_json = hashed_snapshot_json(hashed)?;
    let value = &hashed.snapshot;
    tx.execute(
        "INSERT INTO node_compute_plugin_install_plan_planning_snapshots_v2 (
           snapshot_id, snapshot_schema, snapshot_json, snapshot_digest,
           planning_delivery_id, cloud_session_id, source_preparation_id,
           source_preparation_delivery_id, source_preparation_observation_id,
           source_preparation_observation_digest, source_preparation_request_digest,
           node_id, owner_user_id, consent_receipt_id, installation_identity_digest,
           policy_revision, policy_digest, policy_snapshot_digest, authorization_ref,
           authorization_revision, authorization_digest, bootstrap_instance_id,
           configuration_generation, cancellation_generation, policy_binding_receipt_digest,
           policy_capability_revocation_receipt_digest, policy_binding_authority_epoch,
           policy_binding_process_owner_epoch, authority_state_revision, authority_epoch,
           process_owner_epoch, clock_epoch_digest, trusted_time_high_water_ms,
           captured_at_ms, expires_at_ms, rollback_anchor_witness_digest,
           inventory_revision, inventory_digest, node_profile_digest,
           manifest_catalog_revision, manifest_catalog_digest, keyring_bundle_revision,
           publisher_keyring_revision, publisher_keyring_digest,
           control_keyring_revision, control_keyring_digest, target_id,
           host_api_protocol_id, host_api_revision, installed_record_count, created_at
         ) VALUES (
           :snapshot_id, :snapshot_schema, :snapshot_json, :snapshot_digest,
           :delivery, :session, :preparation, :source_delivery, :source_observation,
           :source_observed_digest, :source_request_digest, :node, :owner, :consent,
           :installation, :policy_revision, :policy_digest, :policy_snapshot_digest,
           :authorization_ref, :authorization_revision, :authorization_digest, :bootstrap,
           :configuration_generation, :cancellation_generation, :binding_receipt,
           :revocation_receipt, :binding_authority_epoch, :binding_process_epoch,
           :state_revision, :authority_epoch, :process_epoch, :clock_epoch, :high_water,
           :captured, :expires, :rollback, :inventory_revision, :inventory_digest, :profile,
           :catalog_revision, :catalog_digest, :bundle_revision, :publisher_revision,
           :publisher_digest, :control_revision, :control_digest, :target, :host_protocol,
           :host_revision, :record_count, :created_at)",
        named_params! {
            ":snapshot_id": new_id("cpv2s"),
            ":snapshot_schema": &hashed.schema,
            ":snapshot_json": &snapshot_json,
            ":snapshot_digest": &hashed.snapshot_digest,
            ":delivery": &intent.planning_delivery_id,
            ":session": &intent.cloud_session_id,
            ":preparation": &value.preparation_id,
            ":source_delivery": &value.source_preparation_delivery_id,
            ":source_observation": &intent.source_preparation_observation_id,
            ":source_observed_digest": &value.source_preparation_observation_digest,
            ":source_request_digest": &intent.source_preparation_request_digest,
            ":node": &value.node_id,
            ":owner": &value.owner_user_id,
            ":consent": &intent.consent_receipt_id,
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
            ":created_at": now(),
        },
    )?;
    Ok(())
}

fn validate_source_matches_intent(
    tx: &Transaction<'_>,
    intent: &NodeComputePluginInstallPlanPlanningDispatchIntentV2,
) -> Result<()> {
    let source = resolve_exact_planning_source(tx, &intent.request)?;
    if source.source_sharing_delivery_id != intent.source_sharing_delivery_id
        || source.source_preparation_observation_id != intent.source_preparation_observation_id
        || source.source_preparation_request_digest != intent.source_preparation_request_digest
        || source.consent_receipt_id != intent.consent_receipt_id
        || source.source_bootstrap_instance_id != intent.source_bootstrap_instance_id
        || source.source_configuration_generation != intent.source_configuration_generation
        || source.source_cancellation_generation != intent.source_cancellation_generation
    {
        bail!("算力插件 Planning Snapshot V2 source session 已漂移");
    }
    Ok(())
}

fn delivery_event_count(tx: &Transaction<'_>, delivery_id: &str) -> Result<i64> {
    Ok(tx.query_row(
        "SELECT COUNT(*) FROM node_compute_plugin_install_plan_planning_delivery_events_v2
          WHERE planning_delivery_id=?1",
        params![delivery_id],
        |row| row.get::<_, i64>(0),
    )?)
}

fn stable_code(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 256
        && value
            .bytes()
            .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'_')
}
