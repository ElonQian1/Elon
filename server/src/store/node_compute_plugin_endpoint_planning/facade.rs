use anyhow::{bail, Result};
use chrono::Utc;
use rusqlite::TransactionBehavior;

use super::{
    ledger, legacy,
    messages::{self, BuiltMessageRef},
    types::{
        NodeComputePluginEndpointPlanningBootstrapPreparationIntentV1,
        NodeComputePluginEndpointPlanningBootstrapSharingIntentV1,
        NodeComputePluginEndpointPlanningBootstrapSnapshotIntentV1,
        NodeComputePluginEndpointPlanningBootstrapTerminalV1,
    },
};
use crate::store::{
    new_id,
    node_compute_plugin_install_plan_planning::{
        prepare_node_compute_plugin_install_plan_planning_delivery_v2_on,
        record_node_compute_plugin_install_plan_planning_terminal_observation_v2_on,
    },
    node_compute_plugin_install_plan_preparation::{
        prepare_node_compute_plugin_install_plan_preparation_delivery_on,
        record_node_compute_plugin_install_plan_preparation_delivery_on,
        record_node_compute_plugin_install_plan_preparation_observation_on,
    },
    node_compute_plugin_sharing::{
        prepare_node_compute_plugin_sharing_session_delivery_on,
        record_node_compute_plugin_sharing_delivery_on,
        record_node_compute_plugin_sharing_observation_on,
    },
    node_credentials::{require_current_node_endpoint_session_on, NodeEndpointSessionPermit},
    Store,
};

impl Store {
    pub(crate) fn prepare_node_compute_plugin_endpoint_planning_bootstrap_v1(
        &self,
        permit: &NodeEndpointSessionPermit,
    ) -> Result<Option<NodeComputePluginEndpointPlanningBootstrapSharingIntentV1>> {
        permit.require_planning_bootstrap_v14()?;
        let mut connection = self.conn()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        require_current_node_endpoint_session_on(&transaction, permit, Utc::now())?;
        let existing_chain_count = transaction.query_row(
            "SELECT COUNT(*)
               FROM node_compute_plugin_endpoint_planning_chain_events_v1
              WHERE authentication_receipt_id=?1 AND message_sequence=1",
            rusqlite::params![permit.binding().authentication_receipt_id()],
            |row| row.get::<_, i64>(0),
        )?;
        if existing_chain_count != 0 {
            bail!("NODE_ENDPOINT_PLANNING_SESSION_CHAIN_ALREADY_EXISTS");
        }
        let Some(sharing) = prepare_node_compute_plugin_sharing_session_delivery_on(
            &transaction,
            permit.binding().agent_id(),
        )?
        else {
            require_current_node_endpoint_session_on(&transaction, permit, Utc::now())?;
            bail!("NODE_ENDPOINT_PLANNING_SHARING_SOURCE_MISSING");
        };
        let (snapshot, policy_snapshot_digest) = legacy::sharing_snapshot(permit, &sharing)?;
        let built = messages::build_sharing_request(
            permit,
            &new_id("nepb"),
            &sharing.delivery_id,
            snapshot,
        )?;
        let event =
            ledger::initial_sharing_event(permit, &built, &sharing, &policy_snapshot_digest)?;
        ledger::append_event_on(&transaction, &event)?;
        require_current_node_endpoint_session_on(&transaction, permit, Utc::now())?;
        transaction.commit()?;
        Ok(Some(
            NodeComputePluginEndpointPlanningBootstrapSharingIntentV1 {
                message: built.value,
                event,
                sharing,
            },
        ))
    }

    pub(crate) fn observe_node_compute_plugin_endpoint_planning_bootstrap_sharing_v1(
        &self,
        permit: &NodeEndpointSessionPermit,
        intent: &NodeComputePluginEndpointPlanningBootstrapSharingIntentV1,
        observed: &homecli_proto::NodeEndpointPlanningBootstrapSharingObservedV1,
    ) -> Result<Option<NodeComputePluginEndpointPlanningBootstrapPreparationIntentV1>> {
        ledger::require_event_session_matches_permit(&intent.event, permit)?;
        let validated =
            messages::validate_sharing_observed(permit, &message_ref(&intent.event), observed)?;
        legacy::validate_sharing_observation(
            &intent.sharing,
            &intent.event.policy_snapshot_digest,
            &validated.payload,
        )?;
        let (legacy_json, legacy_digest) =
            legacy::canonical_sharing_observation(&validated.payload)?;

        let mut connection = self.conn()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        require_current_node_endpoint_session_on(&transaction, permit, Utc::now())?;
        ledger::require_exact_event_on(&transaction, &intent.event)?;
        record_node_compute_plugin_sharing_delivery_on(
            &transaction,
            &intent.sharing,
            "dispatched",
            None,
        )?;
        let observation_id = record_node_compute_plugin_sharing_observation_on(
            &transaction,
            &intent.sharing,
            validated.payload.accepted,
            &legacy_json,
        )?;

        let preparation = if intent.sharing.plugin_runtime_requested && validated.payload.accepted {
            Some(
                prepare_node_compute_plugin_install_plan_preparation_delivery_on(
                    &transaction,
                    &intent.sharing,
                    &intent.event.policy_snapshot_digest,
                )?
                .ok_or_else(|| {
                    anyhow::anyhow!("NODE_ENDPOINT_PLANNING_PREPARATION_NEXT_INTENT_MISSING")
                })?,
            )
        } else {
            None
        };
        let next_event_id = preparation.as_ref().map(|_| new_id("nepe"));
        let mut observation_event = ledger::successor_event(
            &intent.event,
            new_id("nepe"),
            2,
            "sharing_observed",
            homecli_proto::NODE_ENDPOINT_PLANNING_BOOTSTRAP_SHARING_OBSERVED_V1_SCHEMA,
            validated.json,
            validated.digest,
            validated.binding.delivery_id,
            next_event_id.clone(),
            Some(validated.payload.accepted),
            Some(validated.payload.replayed),
            None,
        )?;
        observation_event.sharing_observation_id = Some(observation_id);
        observation_event.sharing_observation_digest = Some(legacy_digest);

        let next = match (preparation, next_event_id) {
            (Some(preparation), Some(next_event_id)) => {
                let request = legacy::preparation_request(&preparation)?;
                let built = messages::build_preparation_request(
                    permit,
                    &intent.event.bootstrap_id,
                    &preparation.delivery_id,
                    &observation_event.message_digest,
                    request,
                )?;
                let mut event = ledger::successor_event(
                    &observation_event,
                    next_event_id,
                    3,
                    "preparation_request",
                    homecli_proto::NODE_ENDPOINT_PLANNING_BOOTSTRAP_PREPARATION_REQUEST_V1_SCHEMA,
                    built.json,
                    built.digest,
                    built.binding.delivery_id,
                    None,
                    None,
                    None,
                    None,
                )?;
                event.preparation_id = Some(preparation.preparation_id.clone());
                event.preparation_delivery_id = Some(preparation.delivery_id.clone());
                event.preparation_request_digest = Some(preparation.request_digest.clone());
                Some(
                    NodeComputePluginEndpointPlanningBootstrapPreparationIntentV1 {
                        message: built.value,
                        event,
                        preparation,
                    },
                )
            }
            (None, None) => None,
            _ => bail!("NODE_ENDPOINT_PLANNING_SHARING_NEXT_EVENT_MISMATCH"),
        };
        ledger::append_event_on(&transaction, &observation_event)?;
        if let Some(next) = next.as_ref() {
            ledger::append_event_on(&transaction, &next.event)?;
        }
        require_current_node_endpoint_session_on(&transaction, permit, Utc::now())?;
        transaction.commit()?;
        Ok(next)
    }

    pub(crate) fn observe_node_compute_plugin_endpoint_planning_bootstrap_preparation_v1(
        &self,
        permit: &NodeEndpointSessionPermit,
        intent: &NodeComputePluginEndpointPlanningBootstrapPreparationIntentV1,
        observed: &homecli_proto::NodeEndpointPlanningBootstrapPreparationObservedV1,
    ) -> Result<Option<NodeComputePluginEndpointPlanningBootstrapSnapshotIntentV1>> {
        ledger::require_event_session_matches_permit(&intent.event, permit)?;
        let validated =
            messages::validate_preparation_observed(permit, &message_ref(&intent.event), observed)?;
        let observed_value = serde_json::to_value(&validated.payload)?;

        let mut connection = self.conn()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        require_current_node_endpoint_session_on(&transaction, permit, Utc::now())?;
        ledger::require_exact_event_on(&transaction, &intent.event)?;
        record_node_compute_plugin_install_plan_preparation_delivery_on(
            &transaction,
            &intent.preparation,
            "dispatched",
            None,
        )?;
        let (observation_id, observation_digest) =
            record_node_compute_plugin_install_plan_preparation_observation_on(
                &transaction,
                &intent.preparation,
                validated.payload.accepted,
                validated.payload.replayed,
                validated.payload.context_ready,
                None,
                &validated.payload.bootstrap_instance_id,
                &observed_value,
            )?;

        let planning = if validated.payload.accepted {
            let request =
                legacy::planning_request(permit, &intent.preparation, &observation_digest)?;
            Some(
                prepare_node_compute_plugin_install_plan_planning_delivery_v2_on(
                    &transaction,
                    request,
                )?,
            )
        } else {
            None
        };
        if planning
            .as_ref()
            .is_some_and(|value| value.replayed || !value.dispatchable)
        {
            bail!("NODE_ENDPOINT_PLANNING_SNAPSHOT_NEXT_INTENT_NOT_FRESH");
        }
        let next_event_id = planning.as_ref().map(|_| new_id("nepe"));
        let mut observation_event = ledger::successor_event(
            &intent.event,
            new_id("nepe"),
            4,
            "preparation_observed",
            homecli_proto::NODE_ENDPOINT_PLANNING_BOOTSTRAP_PREPARATION_OBSERVED_V1_SCHEMA,
            validated.json,
            validated.digest,
            validated.binding.delivery_id,
            next_event_id.clone(),
            Some(validated.payload.accepted),
            Some(validated.payload.replayed),
            None,
        )?;
        observation_event.preparation_observation_id = Some(observation_id);
        observation_event.preparation_observation_digest = Some(observation_digest);

        let next = match (planning, next_event_id) {
            (Some(planning), Some(next_event_id)) => {
                let built = messages::build_snapshot_request(
                    permit,
                    &intent.event.bootstrap_id,
                    &planning.planning_delivery_id,
                    &observation_event.message_digest,
                    planning.request.clone(),
                )?;
                let mut event = ledger::successor_event(
                    &observation_event,
                    next_event_id,
                    5,
                    "snapshot_request",
                    homecli_proto::NODE_ENDPOINT_PLANNING_BOOTSTRAP_SNAPSHOT_REQUEST_V1_SCHEMA,
                    built.json,
                    built.digest,
                    built.binding.delivery_id,
                    None,
                    None,
                    None,
                    None,
                )?;
                event.planning_delivery_id = Some(planning.planning_delivery_id.clone());
                event.planning_request_digest = Some(planning.request_digest.clone());
                Some(NodeComputePluginEndpointPlanningBootstrapSnapshotIntentV1 {
                    message: built.value,
                    event,
                    planning,
                })
            }
            (None, None) => None,
            _ => bail!("NODE_ENDPOINT_PLANNING_PREPARATION_NEXT_EVENT_MISMATCH"),
        };
        ledger::append_event_on(&transaction, &observation_event)?;
        if let Some(next) = next.as_ref() {
            ledger::append_event_on(&transaction, &next.event)?;
        }
        require_current_node_endpoint_session_on(&transaction, permit, Utc::now())?;
        transaction.commit()?;
        Ok(next)
    }

    pub(crate) fn observe_node_compute_plugin_endpoint_planning_bootstrap_snapshot_v1(
        &self,
        permit: &NodeEndpointSessionPermit,
        intent: &NodeComputePluginEndpointPlanningBootstrapSnapshotIntentV1,
        observed: &homecli_proto::NodeEndpointPlanningBootstrapSnapshotObservedV1,
    ) -> Result<NodeComputePluginEndpointPlanningBootstrapTerminalV1> {
        ledger::require_event_session_matches_permit(&intent.event, permit)?;
        let validated =
            messages::validate_snapshot_observed(permit, &message_ref(&intent.event), observed)?;
        if validated.payload.snapshot_ready || validated.payload.snapshot.is_some() {
            bail!("NODE_ENDPOINT_PLANNING_SNAPSHOT_READY_FORBIDDEN");
        }
        let mut connection = self.conn()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        require_current_node_endpoint_session_on(&transaction, permit, Utc::now())?;
        ledger::require_exact_event_on(&transaction, &intent.event)?;
        let (observation_event_id, observation_digest) =
            record_node_compute_plugin_install_plan_planning_terminal_observation_v2_on(
                &transaction,
                &intent.planning,
                &validated.payload,
            )?;
        let mut event = ledger::successor_event(
            &intent.event,
            new_id("nepe"),
            6,
            "snapshot_observed",
            homecli_proto::NODE_ENDPOINT_PLANNING_BOOTSTRAP_SNAPSHOT_OBSERVED_V1_SCHEMA,
            validated.json,
            validated.digest,
            validated.binding.delivery_id,
            None,
            Some(validated.payload.accepted),
            Some(validated.payload.replayed),
            Some(false),
        )?;
        event.planning_observation_event_id = Some(observation_event_id);
        event.planning_observation_digest = Some(observation_digest);
        ledger::append_event_on(&transaction, &event)?;
        require_current_node_endpoint_session_on(&transaction, permit, Utc::now())?;
        transaction.commit()?;
        Ok(NodeComputePluginEndpointPlanningBootstrapTerminalV1 { _sealed: () })
    }
}

fn message_ref(event: &super::types::EndpointPlanningEventWrite) -> BuiltMessageRef<'_> {
    BuiltMessageRef {
        bootstrap_id: &event.bootstrap_id,
        delivery_id: &event.delivery_id,
        message_digest: &event.message_digest,
    }
}
