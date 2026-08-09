use homecli_proto::{
    ComputePluginSharingAuthorizationBindingV1, ComputePluginSharingPolicyObservedV1,
    ComputePluginSharingPolicySnapshotV1, COMPUTE_PLUGIN_SHARING_POLICY_OBSERVED_V1_SCHEMA,
};

use super::{
    AcceptedComputePluginSharingPolicy, ComputePluginBootstrapState, ComputePluginBootstrapStatus,
    DormantComputePluginInitializationPlan,
};

impl ComputePluginBootstrapState {
    pub(super) fn apply_policy_snapshot(
        &mut self,
        snapshot: &ComputePluginSharingPolicySnapshotV1,
        snapshot_digest: String,
        session_node_id: &str,
        session_owner_user_id: &str,
    ) -> ComputePluginSharingPolicyObservedV1 {
        let reject = |state: &Self, code| {
            state.observation(
                session_node_id,
                session_owner_user_id,
                false,
                false,
                Some(code),
            )
        };
        if snapshot.node_id != session_node_id {
            return reject(self, "COMPUTE_PLUGIN_SHARING_POLICY_NODE_BINDING_MISMATCH");
        }
        if snapshot.owner_user_id != session_owner_user_id {
            return reject(self, "COMPUTE_PLUGIN_SHARING_POLICY_OWNER_BINDING_MISMATCH");
        }
        let Some(installation) = self.installation.as_ref() else {
            return reject(
                self,
                "COMPUTE_PLUGIN_SHARING_POLICY_LOCAL_INSTALLATION_INVALID",
            );
        };
        if snapshot.installation_identity_digest != installation.digest() {
            return reject(
                self,
                "COMPUTE_PLUGIN_SHARING_POLICY_INSTALLATION_BINDING_MISMATCH",
            );
        }
        if let Some(current) = &self.desired_policy {
            if snapshot.policy_revision < current.snapshot.policy_revision {
                return reject(self, "COMPUTE_PLUGIN_SHARING_POLICY_REVISION_STALE");
            }
            if snapshot.policy_revision == current.snapshot.policy_revision {
                return if current.snapshot == *snapshot
                    && current.snapshot_digest == snapshot_digest
                {
                    self.observation(session_node_id, session_owner_user_id, true, true, None)
                } else {
                    reject(self, "COMPUTE_PLUGIN_SHARING_POLICY_REVISION_CONFLICT")
                };
            }
        }
        if let Some(error_code) = self.authorization_rollback_error(snapshot.authorization.as_ref())
        {
            return reject(self, error_code);
        }
        if self.configuration_exhausted {
            return reject(
                self,
                "COMPUTE_PLUGIN_SHARING_POLICY_CONFIGURATION_GENERATION_EXHAUSTED",
            );
        }
        let Some(next_configuration_generation) = self.configuration_generation.checked_add(1)
        else {
            self.configuration_exhausted = true;
            return reject(
                self,
                "COMPUTE_PLUGIN_SHARING_POLICY_CONFIGURATION_GENERATION_EXHAUSTED",
            );
        };
        // Every non-replay policy revision, including enabled-to-enabled authorization
        // replacement, revokes work admitted under the previous policy. Disabled snapshots are
        // not the only cancellation boundary.
        let Some(next_cancellation_generation) = self.cancellation_generation.checked_add(1) else {
            return reject(
                self,
                "COMPUTE_PLUGIN_SHARING_POLICY_CANCELLATION_GENERATION_EXHAUSTED",
            );
        };
        let initialization_plan = snapshot
            .authorization
            .as_ref()
            .filter(|_| !self.root_change_requires_restart)
            .map(|authorization| DormantComputePluginInitializationPlan {
                snapshot_digest: snapshot_digest.clone(),
                policy_revision: snapshot.policy_revision,
                policy_digest: snapshot.policy_digest.clone(),
                authorization: authorization.clone(),
                cancellation_generation: next_cancellation_generation,
            });
        if !self.invalidate_local_policy_binding_intents() {
            return reject(
                self,
                "COMPUTE_PLUGIN_SHARING_POLICY_CONFIGURATION_GENERATION_EXHAUSTED",
            );
        }
        if let Some(authorization) = snapshot.authorization.as_ref() {
            if self
                .authorization_high_water
                .as_ref()
                .is_none_or(|current| authorization.revision > current.revision)
            {
                self.authorization_high_water = Some(authorization.clone());
            }
        }
        self.configuration_generation = next_configuration_generation;
        self.cancellation_generation = next_cancellation_generation;
        self.sharing_requested = snapshot.plugin_runtime_requested;
        self.initialization_plan = initialization_plan;
        self.last_install_plan_preparation = None;
        self.last_install_plan_planning_snapshot = None;
        self.desired_policy = Some(AcceptedComputePluginSharingPolicy {
            snapshot: snapshot.clone(),
            snapshot_digest,
        });
        self.observation(session_node_id, session_owner_user_id, true, false, None)
    }

    fn authorization_rollback_error(
        &self,
        next: Option<&ComputePluginSharingAuthorizationBindingV1>,
    ) -> Option<&'static str> {
        let (Some(current), Some(next)) = (self.authorization_high_water.as_ref(), next) else {
            return None;
        };
        if next.revision < current.revision {
            Some("COMPUTE_PLUGIN_SHARING_POLICY_AUTHORIZATION_REVISION_STALE")
        } else if next.revision == current.revision && next != current {
            Some("COMPUTE_PLUGIN_SHARING_POLICY_AUTHORIZATION_REVISION_CONFLICT")
        } else {
            None
        }
    }

    pub(super) fn observation(
        &self,
        node_id: &str,
        owner_user_id: &str,
        accepted: bool,
        replayed: bool,
        error_code: Option<&'static str>,
    ) -> ComputePluginSharingPolicyObservedV1 {
        let status = self.status();
        ComputePluginSharingPolicyObservedV1 {
            schema: COMPUTE_PLUGIN_SHARING_POLICY_OBSERVED_V1_SCHEMA.to_string(),
            node_id: node_id.to_string(),
            owner_user_id: owner_user_id.to_string(),
            installation_identity_digest: self
                .installation
                .as_ref()
                .map(|installation| installation.digest().to_string()),
            accepted,
            replayed,
            observed_policy_revision: status.desired_policy_revision,
            observed_policy_digest: status.desired_policy_digest,
            observed_snapshot_digest: status.desired_snapshot_digest,
            phase: status.phase.to_string(),
            configuration_generation: status.configuration_generation,
            cancellation_generation: status.cancellation_generation,
            side_effects_started: status.side_effects_started,
            blocked_reasons: status
                .blocked_reasons
                .into_iter()
                .map(str::to_string)
                .collect(),
            error_code: error_code.map(str::to_string),
        }
    }

    pub(super) fn poisoned_observation(
        node_id: &str,
        owner_user_id: &str,
    ) -> ComputePluginSharingPolicyObservedV1 {
        let status = ComputePluginBootstrapStatus::poisoned();
        ComputePluginSharingPolicyObservedV1 {
            schema: COMPUTE_PLUGIN_SHARING_POLICY_OBSERVED_V1_SCHEMA.to_string(),
            node_id: node_id.to_string(),
            owner_user_id: owner_user_id.to_string(),
            installation_identity_digest: None,
            accepted: false,
            replayed: false,
            observed_policy_revision: None,
            observed_policy_digest: None,
            observed_snapshot_digest: None,
            phase: status.phase.to_string(),
            configuration_generation: status.configuration_generation,
            cancellation_generation: status.cancellation_generation,
            side_effects_started: status.side_effects_started,
            blocked_reasons: status
                .blocked_reasons
                .into_iter()
                .map(str::to_string)
                .collect(),
            error_code: Some("COMPUTE_PLUGIN_BOOTSTRAP_STATE_POISONED".to_string()),
        }
    }
}
