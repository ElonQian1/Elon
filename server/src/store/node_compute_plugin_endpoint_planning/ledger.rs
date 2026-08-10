use anyhow::{bail, Result};
use chrono::{DateTime, Duration, SecondsFormat, Utc};
use rusqlite::{named_params, OptionalExtension, Transaction};

use super::{
    messages::{session_binding_fields, BuiltMessage},
    types::EndpointPlanningEventWrite,
};
use crate::store::{new_id, node_credentials::NodeEndpointSessionPermit};

const EVENT_SCHEMA: &str = "elon.node_compute_plugin.endpoint_planning_chain_event.v1";

macro_rules! event_params {
    ($event:expr) => {
        named_params! {
            ":event_id": &$event.event_id, ":event_schema": EVENT_SCHEMA,
            ":bootstrap_id": &$event.bootstrap_id, ":sequence": $event.message_sequence,
            ":kind": $event.message_kind, ":previous_sequence": $event.previous_message_sequence,
            ":previous_event": &$event.previous_event_id, ":next_sequence": $event.next_message_sequence,
            ":next_event": &$event.next_event_id, ":message_schema": $event.message_schema,
            ":message_json": &$event.message_json, ":message_digest": &$event.message_digest,
            ":previous_digest": &$event.previous_message_digest, ":delivery_id": &$event.delivery_id,
            ":agent": &$event.agent_id, ":owner": &$event.owner_user_id, ":install": &$event.install_id,
            ":installation": &$event.installation_binding_digest, ":credential": &$event.credential_id,
            ":plugin_installation": &$event.plugin_installation_identity_digest,
            ":credential_revision": $event.credential_revision,
            ":credential_digest": &$event.credential_digest, ":receipt": &$event.authentication_receipt_id,
            ":authentication_digest": &$event.authentication_digest, ":session": &$event.session_id,
            ":session_generation": $event.session_generation, ":server_instance": &$event.server_instance_id,
            ":agent_version": &$event.agent_version, ":authenticated_at": &$event.authenticated_at,
            ":expires_at": &$event.expires_at, ":protocol": $event.protocol_version,
            ":capability_count": $event.capability_count,
            ":capability_digest": &$event.capability_set_digest,
            ":consent": &$event.consent_receipt_id, ":policy_revision": $event.policy_revision,
            ":policy_digest": &$event.policy_digest,
            ":policy_snapshot_digest": &$event.policy_snapshot_digest,
            ":plugin_runtime_requested": $event.plugin_runtime_requested,
            ":sharing_delivery": &$event.sharing_delivery_id,
            ":sharing_observation": &$event.sharing_observation_id,
            ":sharing_observation_digest": &$event.sharing_observation_digest,
            ":preparation": &$event.preparation_id,
            ":preparation_delivery": &$event.preparation_delivery_id,
            ":preparation_request_digest": &$event.preparation_request_digest,
            ":preparation_observation": &$event.preparation_observation_id,
            ":preparation_observation_digest": &$event.preparation_observation_digest,
            ":planning_delivery": &$event.planning_delivery_id,
            ":planning_request_digest": &$event.planning_request_digest,
            ":planning_observation": &$event.planning_observation_event_id,
            ":planning_observation_digest": &$event.planning_observation_digest,
            ":accepted": $event.accepted, ":replayed": $event.replayed,
            ":snapshot_ready": $event.snapshot_ready, ":recorded_at": &$event.recorded_at,
        }
    };
}

pub(super) fn initial_sharing_event(
    permit: &NodeEndpointSessionPermit,
    built: &BuiltMessage<homecli_proto::NodeEndpointPlanningBootstrapSharingRequestV1>,
    sharing: &crate::store::NodeComputePluginSharingDispatchIntent,
    policy_snapshot_digest: &str,
) -> Result<EndpointPlanningEventWrite> {
    let session = session_binding_fields(permit)?;
    Ok(EndpointPlanningEventWrite {
        event_id: new_id("nepe"),
        bootstrap_id: built.binding.bootstrap_id.clone(),
        message_sequence: 1,
        message_kind: "sharing_request",
        previous_message_sequence: None,
        previous_event_id: None,
        next_message_sequence: None,
        next_event_id: None,
        message_schema: homecli_proto::NODE_ENDPOINT_PLANNING_BOOTSTRAP_SHARING_REQUEST_V1_SCHEMA,
        message_json: built.json.clone(),
        message_digest: built.digest.clone(),
        previous_message_digest: built.binding.previous_message_digest.clone(),
        delivery_id: built.binding.delivery_id.clone(),
        agent_id: session.agent_id,
        owner_user_id: session.owner_user_id,
        install_id: session.install_id,
        installation_binding_digest: session.installation_binding_digest,
        plugin_installation_identity_digest: sharing.installation_identity_digest.clone(),
        credential_id: session.credential_id,
        credential_revision: i64::try_from(session.credential_revision)?,
        credential_digest: session.credential_digest,
        authentication_receipt_id: session.authentication_receipt_id,
        authentication_digest: session.authentication_digest,
        session_id: session.session_id,
        session_generation: i64::try_from(session.session_generation)?,
        server_instance_id: session.server_instance_id,
        agent_version: session.agent_version,
        authenticated_at: session.authenticated_at,
        expires_at: session.expires_at,
        protocol_version: i64::try_from(permit.protocol_version())?,
        capability_count: i64::try_from(permit.capability_count())?,
        capability_set_digest: session.capability_set_digest,
        consent_receipt_id: sharing.consent_receipt_id.clone(),
        policy_revision: sharing.policy_revision,
        policy_digest: sharing.policy_digest.clone(),
        policy_snapshot_digest: policy_snapshot_digest.to_string(),
        plugin_runtime_requested: sharing.plugin_runtime_requested,
        sharing_delivery_id: sharing.delivery_id.clone(),
        sharing_observation_id: None,
        sharing_observation_digest: None,
        preparation_id: None,
        preparation_delivery_id: None,
        preparation_request_digest: None,
        preparation_observation_id: None,
        preparation_observation_digest: None,
        planning_delivery_id: None,
        planning_request_digest: None,
        planning_observation_event_id: None,
        planning_observation_digest: None,
        accepted: None,
        replayed: None,
        snapshot_ready: None,
        recorded_at: initial_event_time(permit)?,
    })
}

#[allow(clippy::too_many_arguments)]
pub(super) fn successor_event(
    prior: &EndpointPlanningEventWrite,
    event_id: String,
    sequence: i64,
    kind: &'static str,
    schema: &'static str,
    message_json: String,
    message_digest: String,
    delivery_id: String,
    next_event_id: Option<String>,
    accepted: Option<bool>,
    replayed: Option<bool>,
    snapshot_ready: Option<bool>,
) -> Result<EndpointPlanningEventWrite> {
    let mut event = prior.clone();
    event.event_id = event_id;
    event.message_sequence = sequence;
    event.message_kind = kind;
    event.previous_message_sequence = Some(prior.message_sequence);
    event.previous_event_id = Some(prior.event_id.clone());
    event.next_message_sequence = next_event_id.as_ref().map(|_| sequence + 1);
    event.next_event_id = next_event_id;
    event.message_schema = schema;
    event.message_json = message_json;
    event.message_digest = message_digest;
    event.previous_message_digest = prior.message_digest.clone();
    event.delivery_id = delivery_id;
    event.accepted = accepted;
    event.replayed = replayed;
    event.snapshot_ready = snapshot_ready;
    event.recorded_at = successor_event_time(prior)?;
    Ok(event)
}

pub(super) fn require_event_session_matches_permit(
    event: &EndpointPlanningEventWrite,
    permit: &NodeEndpointSessionPermit,
) -> Result<()> {
    let session = session_binding_fields(permit)?;
    if event.agent_id != session.agent_id
        || event.owner_user_id != session.owner_user_id
        || event.install_id != session.install_id
        || event.installation_binding_digest != session.installation_binding_digest
        || event.plugin_installation_identity_digest
            != crate::compute_plugin_sharing_directive::derive_compute_plugin_installation_identity_digest(
                permit.install_id(),
            )
            .map_err(|error| anyhow::anyhow!(error.code()))?
        || event.credential_id != session.credential_id
        || event.credential_revision != i64::try_from(session.credential_revision)?
        || event.credential_digest != session.credential_digest
        || event.authentication_receipt_id != session.authentication_receipt_id
        || event.authentication_digest != session.authentication_digest
        || event.session_id != session.session_id
        || event.session_generation != i64::try_from(session.session_generation)?
        || event.server_instance_id != session.server_instance_id
        || event.agent_version != session.agent_version
        || event.authenticated_at != session.authenticated_at
        || event.expires_at != session.expires_at
        || event.protocol_version != i64::try_from(permit.protocol_version())?
        || event.capability_count != i64::try_from(permit.capability_count())?
        || event.capability_set_digest != session.capability_set_digest
    {
        bail!("NODE_ENDPOINT_PLANNING_INTENT_SESSION_MISMATCH");
    }
    Ok(())
}

pub(super) fn append_event_on(
    transaction: &Transaction<'_>,
    event: &EndpointPlanningEventWrite,
) -> Result<()> {
    transaction.execute(
        r#"INSERT INTO node_compute_plugin_endpoint_planning_chain_events_v1 (
           event_id,event_schema,bootstrap_id,message_sequence,message_kind,
           previous_message_sequence,previous_event_id,next_message_sequence,next_event_id,
           message_schema,message_json,message_digest,previous_message_digest,delivery_id,
           agent_id,owner_user_id,install_id,installation_binding_digest,
           plugin_installation_identity_digest,
           credential_id,credential_revision,credential_digest,authentication_receipt_id,
           authentication_digest,session_id,session_generation,server_instance_id,
           agent_version,authenticated_at,expires_at,protocol_version,capability_count,
           capability_set_json,capability_set_digest,consent_receipt_id,policy_revision,
           policy_digest,policy_snapshot_digest,plugin_runtime_requested,
           sharing_delivery_id,sharing_observation_id,
           sharing_observation_digest,preparation_id,preparation_delivery_id,
           preparation_request_digest,preparation_observation_id,preparation_observation_digest,
           planning_delivery_id,planning_request_digest,planning_observation_event_id,
           planning_observation_digest,accepted,replayed,snapshot_ready,recorded_at
         ) VALUES (
           :event_id,:event_schema,:bootstrap_id,:sequence,:kind,
           :previous_sequence,:previous_event,:next_sequence,:next_event,
           :message_schema,:message_json,:message_digest,:previous_digest,:delivery_id,
           :agent,:owner,:install,:installation,:plugin_installation,:credential,:credential_revision,
           :credential_digest,:receipt,:authentication_digest,:session,:session_generation,
           :server_instance,:agent_version,:authenticated_at,:expires_at,:protocol,:capability_count,
           '["node_endpoint_planning_snapshot_bootstrap_v1"]',:capability_digest,:consent,
           :policy_revision,:policy_digest,:policy_snapshot_digest,:plugin_runtime_requested,
           :sharing_delivery,
           :sharing_observation,:sharing_observation_digest,:preparation,:preparation_delivery,
           :preparation_request_digest,:preparation_observation,:preparation_observation_digest,
           :planning_delivery,:planning_request_digest,:planning_observation,
           :planning_observation_digest,:accepted,:replayed,:snapshot_ready,:recorded_at
         )"#,
        event_params!(event),
    )?;
    require_exact_event_on(transaction, event)
}

pub(super) fn require_exact_event_on(
    transaction: &Transaction<'_>,
    event: &EndpointPlanningEventWrite,
) -> Result<()> {
    let stored = transaction
        .query_row(
            r#"SELECT 1 FROM node_compute_plugin_endpoint_planning_chain_events_v1
              WHERE event_id=:event_id AND event_schema=:event_schema
                AND bootstrap_id=:bootstrap_id AND message_sequence=:sequence
                AND message_kind=:kind AND previous_message_sequence IS :previous_sequence
                AND previous_event_id IS :previous_event AND next_message_sequence IS :next_sequence
                AND next_event_id IS :next_event AND message_schema=:message_schema
                AND message_json=:message_json AND message_digest=:message_digest
                AND previous_message_digest=:previous_digest AND delivery_id=:delivery_id
                AND agent_id=:agent AND owner_user_id=:owner AND install_id=:install
                AND installation_binding_digest=:installation AND credential_id=:credential
                AND plugin_installation_identity_digest=:plugin_installation
                AND credential_revision=:credential_revision
                AND credential_digest=:credential_digest AND authentication_receipt_id=:receipt
                AND authentication_digest=:authentication_digest AND session_id=:session
                AND session_generation=:session_generation AND server_instance_id=:server_instance
                AND agent_version=:agent_version AND authenticated_at=:authenticated_at
                AND expires_at=:expires_at AND protocol_version=:protocol
                AND capability_count=:capability_count
                AND capability_set_json='["node_endpoint_planning_snapshot_bootstrap_v1"]'
                AND capability_set_digest=:capability_digest AND consent_receipt_id=:consent
                AND policy_revision=:policy_revision AND policy_digest=:policy_digest
                AND policy_snapshot_digest=:policy_snapshot_digest
                AND plugin_runtime_requested=:plugin_runtime_requested
                AND sharing_delivery_id=:sharing_delivery
                AND sharing_observation_id IS :sharing_observation
                AND sharing_observation_digest IS :sharing_observation_digest
                AND preparation_id IS :preparation
                AND preparation_delivery_id IS :preparation_delivery
                AND preparation_request_digest IS :preparation_request_digest
                AND preparation_observation_id IS :preparation_observation
                AND preparation_observation_digest IS :preparation_observation_digest
                AND planning_delivery_id IS :planning_delivery
                AND planning_request_digest IS :planning_request_digest
                AND planning_observation_event_id IS :planning_observation
                AND planning_observation_digest IS :planning_observation_digest
                AND accepted IS :accepted AND replayed IS :replayed
                AND snapshot_ready IS :snapshot_ready AND recorded_at=:recorded_at"#,
            event_params!(event),
            |_| Ok(()),
        )
        .optional()?;
    if stored.is_none() {
        bail!("NODE_ENDPOINT_PLANNING_EVENT_EXACT_READBACK_MISMATCH");
    }
    Ok(())
}

fn initial_event_time(permit: &NodeEndpointSessionPermit) -> Result<String> {
    bounded_event_time(
        std::cmp::max(Utc::now(), permit.recorded_at()),
        permit.expires_at(),
    )
}

fn successor_event_time(prior: &EndpointPlanningEventWrite) -> Result<String> {
    let prior_at = parse_event_time(&prior.recorded_at)?;
    let next_floor = prior_at
        .checked_add_signed(Duration::nanoseconds(1))
        .ok_or_else(|| anyhow::anyhow!("NODE_ENDPOINT_PLANNING_EVENT_TIME_EXHAUSTED"))?;
    bounded_event_time(
        std::cmp::max(Utc::now(), next_floor),
        parse_event_time(&prior.expires_at)?,
    )
}

fn bounded_event_time(recorded_at: DateTime<Utc>, expires_at: DateTime<Utc>) -> Result<String> {
    if recorded_at >= expires_at {
        bail!("NODE_ENDPOINT_PLANNING_EVENT_TIME_EXPIRED");
    }
    Ok(recorded_at.to_rfc3339_opts(SecondsFormat::Nanos, true))
}

fn parse_event_time(value: &str) -> Result<DateTime<Utc>> {
    Ok(DateTime::parse_from_rfc3339(value)?.with_timezone(&Utc))
}
