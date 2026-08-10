use tokio::sync::mpsc;

use super::{
    failure, install_plan_planning_snapshot, AgentManager, AgentToServer,
    ComputePluginSharingDispatchFailure,
};
use crate::{
    node_registry::AgentProcessSessionKey,
    store::NodeComputePluginInstallPlanPreparationDispatchIntent, types::AppState,
};

impl AgentManager {
    pub(crate) async fn dispatch_compute_plugin_install_plan_preparation(
        &self,
        agent_id: &str,
        req_id: &str,
        expected_process_session: &AgentProcessSessionKey,
        request: homecli_proto::ComputePluginInstallPlanPreparationRequestV1,
    ) -> std::result::Result<
        homecli_proto::ComputePluginInstallPlanPreparationObservedV1,
        ComputePluginSharingDispatchFailure,
    > {
        let (cmd_tx, pending) = {
            let agents = self.agents.read().await;
            let Some(agent) = agents.get(agent_id) else {
                return Err(preparation_failure(
                    "agent_offline",
                    "COMPUTE_PLUGIN_INSTALL_PLAN_PREPARATION_AGENT_OFFLINE",
                ));
            };
            if &agent.process_session != expected_process_session {
                return Err(preparation_failure(
                    "dispatch_failed",
                    "COMPUTE_PLUGIN_INSTALL_PLAN_PREPARATION_SESSION_REPLACED",
                ));
            }
            if agent.proto_version
                < homecli_proto::COMPUTE_PLUGIN_INSTALL_PLAN_PREPARATION_PROTO_VERSION
                || !agent.capabilities.iter().any(|capability| {
                    capability == homecli_proto::CAP_COMPUTE_PLUGIN_INSTALL_PLAN_PREPARATION_V1
                })
            {
                return Err(preparation_failure(
                    "capability_missing",
                    "COMPUTE_PLUGIN_INSTALL_PLAN_PREPARATION_CAPABILITY_MISSING",
                ));
            }
            (agent.cmd_tx.clone(), agent.pending.clone())
        };
        let (tx, mut rx) = mpsc::unbounded_channel();
        {
            let mut waiters = pending.lock().await;
            if waiters.contains_key(req_id) {
                return Err(preparation_failure(
                    "dispatch_failed",
                    "COMPUTE_PLUGIN_INSTALL_PLAN_PREPARATION_REQUEST_ALREADY_PENDING",
                ));
            }
            waiters.insert(req_id.to_string(), tx);
        }
        if cmd_tx
            .send(
                homecli_proto::ServerToAgent::PrepareComputePluginInstallPlanV1 {
                    req_id: req_id.to_string(),
                    request,
                },
            )
            .is_err()
        {
            pending.lock().await.remove(req_id);
            return Err(preparation_failure(
                "writer_closed",
                "COMPUTE_PLUGIN_INSTALL_PLAN_PREPARATION_WRITER_CLOSED",
            ));
        }
        let received = tokio::time::timeout(super::ACK_TIMEOUT, rx.recv()).await;
        pending.lock().await.remove(req_id);
        match received {
            Ok(Some(AgentToServer::ComputePluginInstallPlanPreparationObservedV1 {
                req_id: observed_req_id,
                observed,
            })) if observed_req_id == req_id => Ok(observed),
            Ok(Some(_)) => Err(preparation_failure(
                "dispatch_failed",
                "COMPUTE_PLUGIN_INSTALL_PLAN_PREPARATION_ACK_TYPE_INVALID",
            )),
            Ok(None) => Err(preparation_failure(
                "writer_closed",
                "COMPUTE_PLUGIN_INSTALL_PLAN_PREPARATION_ACK_CHANNEL_CLOSED",
            )),
            Err(_) => Err(preparation_failure(
                "ack_timeout",
                "COMPUTE_PLUGIN_INSTALL_PLAN_PREPARATION_ACK_TIMEOUT",
            )),
        }
    }
}

pub(super) async fn dispatch_durable_install_plan_preparation(
    state: &AppState,
    intent: &NodeComputePluginInstallPlanPreparationDispatchIntent,
    expected_process_session: &AgentProcessSessionKey,
) {
    let policy_revision = match u64::try_from(intent.policy_revision) {
        Ok(value) => value,
        Err(_) => {
            record_failure(
                state,
                intent,
                "COMPUTE_PLUGIN_INSTALL_PLAN_PREPARATION_REVISION_INVALID",
            );
            return;
        }
    };
    let authorization_revision = match u64::try_from(intent.authorization.revision) {
        Ok(value) => value,
        Err(_) => {
            record_failure(
                state,
                intent,
                "COMPUTE_PLUGIN_INSTALL_PLAN_PREPARATION_AUTH_REVISION_INVALID",
            );
            return;
        }
    };
    let request = homecli_proto::ComputePluginInstallPlanPreparationRequestV1 {
        schema: homecli_proto::COMPUTE_PLUGIN_INSTALL_PLAN_PREPARATION_REQUEST_V1_SCHEMA
            .to_string(),
        preparation_id: intent.preparation_id.clone(),
        node_id: intent.node_id.clone(),
        owner_user_id: intent.owner_user_id.clone(),
        installation_identity_digest: intent.installation_identity_digest.clone(),
        policy_revision,
        policy_digest: intent.policy_digest.clone(),
        policy_snapshot_digest: intent.policy_snapshot_digest.clone(),
        authorization: homecli_proto::ComputePluginSharingAuthorizationBindingV1 {
            authorization_ref: intent.authorization.authorization_ref.clone(),
            revision: authorization_revision,
            digest: intent.authorization.digest.clone(),
        },
    };
    match state
        .agent_manager
        .dispatch_compute_plugin_install_plan_preparation(
            &intent.node_id,
            &intent.delivery_id,
            expected_process_session,
            request,
        )
        .await
    {
        Ok(observed) => record_observed(state, intent, expected_process_session, observed).await,
        Err(dispatch_failure) => {
            if let Err(error) = state
                .store
                .record_node_compute_plugin_install_plan_preparation_delivery(
                    intent,
                    dispatch_failure.event_kind,
                    Some(dispatch_failure.detail_code),
                )
            {
                tracing::warn!(node_id = %intent.node_id, error = %error,
                    "failed to persist InstallPlan preparation dispatch failure");
            }
        }
    }
}

async fn record_observed(
    state: &AppState,
    intent: &NodeComputePluginInstallPlanPreparationDispatchIntent,
    expected_process_session: &AgentProcessSessionKey,
    observed: homecli_proto::ComputePluginInstallPlanPreparationObservedV1,
) {
    if let Err(code) = validate_observed(intent, &observed) {
        record_failure(state, intent, code);
        return;
    }
    let observed_json = match serde_json::to_value(&observed) {
        Ok(value) => value,
        Err(error) => {
            tracing::warn!(node_id = %intent.node_id, error = %error,
                "failed to serialize InstallPlan preparation observation");
            return;
        }
    };
    let context_json = observed
        .context
        .as_ref()
        .map(serde_json::to_value)
        .transpose();
    let context_json = match context_json {
        Ok(value) => value,
        Err(error) => {
            tracing::warn!(node_id = %intent.node_id, error = %error,
                "failed to serialize InstallPlan preparation context");
            return;
        }
    };
    let planning_request = if observed.accepted {
        let source_observation_digest = match crate::compute_plugin_sharing_directive::
            compute_plugin_install_plan_preparation_observed_json_and_digest(&observed)
        {
            Ok((_, digest)) => digest,
            Err(error) => {
                tracing::warn!(node_id = %intent.node_id, error = %error,
                    "failed to derive exact InstallPlan preparation observation digest");
                return;
            }
        };
        let policy_revision = match u64::try_from(intent.policy_revision) {
            Ok(value) => value,
            Err(_) => return,
        };
        let authorization_revision = match u64::try_from(intent.authorization.revision) {
            Ok(value) => value,
            Err(_) => return,
        };
        Some(
            homecli_proto::ComputePluginInstallPlanPlanningSnapshotRequestV2 {
                schema:
                    homecli_proto::COMPUTE_PLUGIN_INSTALL_PLAN_PLANNING_SNAPSHOT_REQUEST_V2_SCHEMA
                        .to_string(),
                preparation_id: intent.preparation_id.clone(),
                cloud_session_id: expected_process_session.session_id().to_string(),
                source_preparation_delivery_id: intent.delivery_id.clone(),
                source_preparation_observation_digest: source_observation_digest,
                node_id: intent.node_id.clone(),
                owner_user_id: intent.owner_user_id.clone(),
                installation_identity_digest: intent.installation_identity_digest.clone(),
                policy_revision,
                policy_digest: intent.policy_digest.clone(),
                policy_snapshot_digest: intent.policy_snapshot_digest.clone(),
                authorization: homecli_proto::ComputePluginSharingAuthorizationBindingV1 {
                    authorization_ref: intent.authorization.authorization_ref.clone(),
                    revision: authorization_revision,
                    digest: intent.authorization.digest.clone(),
                },
            },
        )
    } else {
        None
    };

    let commit = state
        .agent_manager
        .with_current_process_session(expected_process_session, |_| {
            state
                .store
                .record_node_compute_plugin_install_plan_preparation_delivery(
                    intent,
                    "dispatched",
                    None,
                )?;
            state
                .store
                .record_node_compute_plugin_install_plan_preparation_observation(
                    intent,
                    observed.accepted,
                    observed.replayed,
                    observed.context_ready,
                    context_json.as_ref(),
                    &observed.bootstrap_instance_id,
                    &observed_json,
                )?;
            match planning_request {
                Some(request) => state
                    .store
                    .prepare_node_compute_plugin_install_plan_planning_delivery_v2(request)
                    .map(Some),
                None => Ok(None),
            }
        })
        .await;
    let planning = match commit {
        None => {
            record_failure(
                state,
                intent,
                "COMPUTE_PLUGIN_INSTALL_PLAN_PREPARATION_SESSION_REPLACED_BEFORE_ACK_COMMIT",
            );
            return;
        }
        Some(Err(error)) => {
            tracing::warn!(node_id = %intent.node_id, error = %error,
                "failed to persist process-session-fenced preparation ACK closure");
            return;
        }
        Some(Ok(planning)) => planning,
    };
    if let Some(planning) = planning {
        install_plan_planning_snapshot::dispatch_durable_install_plan_planning_snapshot_v2(
            state,
            &planning,
            expected_process_session,
        )
        .await;
    }
}

fn validate_observed(
    intent: &NodeComputePluginInstallPlanPreparationDispatchIntent,
    observed: &homecli_proto::ComputePluginInstallPlanPreparationObservedV1,
) -> std::result::Result<(), &'static str> {
    if observed.schema != homecli_proto::COMPUTE_PLUGIN_INSTALL_PLAN_PREPARATION_OBSERVED_V1_SCHEMA
        || observed.preparation_id != intent.preparation_id
        || observed.node_id != intent.node_id
        || observed.owner_user_id != intent.owner_user_id
        || observed
            .installation_identity_digest
            .as_deref()
            .is_some_and(|digest| digest != intent.installation_identity_digest.as_str())
        || observed.bootstrap_instance_id.trim().is_empty()
        || observed.bootstrap_instance_id.trim() != observed.bootstrap_instance_id
        || observed.bootstrap_instance_id.len() > 256
        || observed.bootstrap_instance_id.chars().any(char::is_control)
        || observed.phase != "blocked"
        || observed.blocked_reasons.is_empty()
        || observed.blocked_reasons.len() > 64
        || observed.blocked_reasons.iter().any(|reason| {
            reason.is_empty()
                || reason.trim() != reason
                || reason.len() > 256
                || reason.chars().any(char::is_control)
        })
        || observed.error_code.as_ref().is_some_and(|code| {
            code.is_empty()
                || code.trim() != code
                || code.len() > 256
                || code.chars().any(char::is_control)
        })
    {
        return Err("COMPUTE_PLUGIN_INSTALL_PLAN_PREPARATION_OBSERVED_IDENTITY_INVALID");
    }
    if observed.compute_plugin_root_lock_acquired
        || observed.trusted_time_authority_configured
        || observed.rollback_anchor_witness_configured
        || observed.root_pinned
        || observed.authority_opened
        || observed.process_fence_acquired
        || observed.new_work_admission_enabled
        || observed.downloads_allowed
        || observed.side_effects_started
    {
        return Err("COMPUTE_PLUGIN_INSTALL_PLAN_PREPARATION_SIDE_EFFECTS_FORBIDDEN");
    }
    if observed.context_ready || observed.context.is_some() {
        return Err("COMPUTE_PLUGIN_INSTALL_PLAN_PREPARATION_CONTEXT_UNEXPECTED");
    }
    if observed.accepted {
        let Ok(revision) = u64::try_from(intent.policy_revision) else {
            return Err("COMPUTE_PLUGIN_INSTALL_PLAN_PREPARATION_OBSERVED_FACTS_MISMATCH");
        };
        let Ok(authorization_revision) = u64::try_from(intent.authorization.revision) else {
            return Err("COMPUTE_PLUGIN_INSTALL_PLAN_PREPARATION_OBSERVED_FACTS_MISMATCH");
        };
        let expected_authorization = homecli_proto::ComputePluginSharingAuthorizationBindingV1 {
            authorization_ref: intent.authorization.authorization_ref.clone(),
            revision: authorization_revision,
            digest: intent.authorization.digest.clone(),
        };
        if observed.installation_identity_digest.as_deref()
            != Some(intent.installation_identity_digest.as_str())
            || observed.observed_policy_revision != Some(revision)
            || observed.observed_policy_digest.as_deref() != Some(intent.policy_digest.as_str())
            || observed.observed_policy_snapshot_digest.as_deref()
                != Some(intent.policy_snapshot_digest.as_str())
            || observed.observed_authorization.as_ref() != Some(&expected_authorization)
            || observed.error_code.is_some()
        {
            return Err("COMPUTE_PLUGIN_INSTALL_PLAN_PREPARATION_OBSERVED_FACTS_MISMATCH");
        }
    } else if observed.error_code.is_none() {
        return Err("COMPUTE_PLUGIN_INSTALL_PLAN_PREPARATION_REJECTION_CODE_MISSING");
    }
    Ok(())
}

fn record_failure(
    state: &AppState,
    intent: &NodeComputePluginInstallPlanPreparationDispatchIntent,
    detail_code: &str,
) {
    if let Err(error) = state
        .store
        .record_node_compute_plugin_install_plan_preparation_delivery(
            intent,
            "dispatch_failed",
            Some(detail_code),
        )
    {
        tracing::warn!(node_id = %intent.node_id, error = %error,
            "failed to persist InstallPlan preparation validation failure");
    }
}

fn preparation_failure(
    event_kind: &'static str,
    detail_code: &'static str,
) -> ComputePluginSharingDispatchFailure {
    failure(event_kind, detail_code)
}
