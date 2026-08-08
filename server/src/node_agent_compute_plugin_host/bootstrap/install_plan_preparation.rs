use homecli_proto::{
    ComputePluginInstallPlanPreparationObservedV1, ComputePluginInstallPlanPreparationRequestV1,
    COMPUTE_PLUGIN_INSTALL_PLAN_PREPARATION_OBSERVED_V1_SCHEMA,
    COMPUTE_PLUGIN_INSTALL_PLAN_PREPARATION_REQUEST_V1_SCHEMA,
};

use super::{ComputePluginBootstrap, ComputePluginBootstrapState};

const PHASE_BLOCKED: &str = "blocked";
const MAX_IDENTIFIER_BYTES: usize = 256;
const MAX_IJSON_SAFE_INTEGER: u64 = 9_007_199_254_740_991;

impl ComputePluginBootstrap {
    /// Observes one cloud preparation request against the currently accepted dormant policy.
    /// Even an accepted observation returns no preparation context and cannot initialize local
    /// authority, pin a root, fetch bytes, admit work, or launch the plugin Host.
    pub(crate) fn observe_install_plan_preparation_v1(
        &self,
        request: &ComputePluginInstallPlanPreparationRequestV1,
        session_node_id: &str,
        session_owner_user_id: &str,
    ) -> ComputePluginInstallPlanPreparationObservedV1 {
        match self.state.lock() {
            Ok(mut state) => state.observe_install_plan_preparation(
                request,
                session_node_id,
                session_owner_user_id,
                &self.bootstrap_instance_id,
            ),
            Err(_) => poisoned_observation(
                request,
                session_node_id,
                session_owner_user_id,
                &self.bootstrap_instance_id,
            ),
        }
    }
}

impl ComputePluginBootstrapState {
    fn observe_install_plan_preparation(
        &mut self,
        request: &ComputePluginInstallPlanPreparationRequestV1,
        session_node_id: &str,
        session_owner_user_id: &str,
        bootstrap_instance_id: &str,
    ) -> ComputePluginInstallPlanPreparationObservedV1 {
        let reject = |state: &Self, code| {
            state.preparation_observation(
                request,
                session_node_id,
                session_owner_user_id,
                bootstrap_instance_id,
                false,
                false,
                Some(code),
            )
        };
        if request.schema != COMPUTE_PLUGIN_INSTALL_PLAN_PREPARATION_REQUEST_V1_SCHEMA {
            return reject(
                self,
                "COMPUTE_PLUGIN_INSTALL_PLAN_PREPARATION_SCHEMA_UNSUPPORTED",
            );
        }
        if !bounded_identifier(&request.preparation_id) {
            return reject(self, "COMPUTE_PLUGIN_INSTALL_PLAN_PREPARATION_ID_INVALID");
        }
        if request.node_id != session_node_id {
            return reject(
                self,
                "COMPUTE_PLUGIN_INSTALL_PLAN_PREPARATION_NODE_BINDING_MISMATCH",
            );
        }
        if request.owner_user_id != session_owner_user_id {
            return reject(
                self,
                "COMPUTE_PLUGIN_INSTALL_PLAN_PREPARATION_OWNER_BINDING_MISMATCH",
            );
        }
        let Some(installation) = self.installation.as_ref() else {
            return reject(
                self,
                "COMPUTE_PLUGIN_INSTALL_PLAN_PREPARATION_LOCAL_INSTALLATION_INVALID",
            );
        };
        if request.installation_identity_digest != installation.digest() {
            return reject(
                self,
                "COMPUTE_PLUGIN_INSTALL_PLAN_PREPARATION_INSTALLATION_BINDING_MISMATCH",
            );
        }
        if !safe_positive_revision(request.policy_revision)
            || !is_sha256(&request.policy_digest)
            || !is_sha256(&request.policy_snapshot_digest)
            || !bounded_identifier(&request.authorization.authorization_ref)
            || !safe_positive_revision(request.authorization.revision)
            || !is_sha256(&request.authorization.digest)
        {
            return reject(
                self,
                "COMPUTE_PLUGIN_INSTALL_PLAN_PREPARATION_BINDING_INVALID",
            );
        }
        let Some(desired) = self.desired_policy.as_ref() else {
            return reject(
                self,
                "COMPUTE_PLUGIN_INSTALL_PLAN_PREPARATION_POLICY_UNAVAILABLE",
            );
        };
        if !self.sharing_requested || !desired.snapshot.plugin_runtime_requested {
            return reject(
                self,
                "COMPUTE_PLUGIN_INSTALL_PLAN_PREPARATION_SHARING_DISABLED",
            );
        }
        if request.policy_revision != desired.snapshot.policy_revision
            || request.policy_digest != desired.snapshot.policy_digest
            || request.policy_snapshot_digest != desired.snapshot_digest
        {
            return reject(
                self,
                "COMPUTE_PLUGIN_INSTALL_PLAN_PREPARATION_POLICY_BINDING_MISMATCH",
            );
        }
        if desired.snapshot.authorization.as_ref() != Some(&request.authorization) {
            return reject(
                self,
                "COMPUTE_PLUGIN_INSTALL_PLAN_PREPARATION_AUTHORIZATION_BINDING_MISMATCH",
            );
        }
        if request.authorization.revision != request.policy_revision
            || request.authorization.digest != request.policy_digest
        {
            return reject(
                self,
                "COMPUTE_PLUGIN_INSTALL_PLAN_PREPARATION_AUTHORIZATION_POLICY_MISMATCH",
            );
        }
        if self
            .last_install_plan_preparation
            .as_ref()
            .is_some_and(|current| !same_policy_binding(current, request))
        {
            self.last_install_plan_preparation = None;
        }
        let replayed = match self.last_install_plan_preparation.as_ref() {
            Some(current) if current == request => true,
            Some(current) if current.preparation_id == request.preparation_id => {
                return reject(self, "COMPUTE_PLUGIN_INSTALL_PLAN_PREPARATION_ID_CONFLICT")
            }
            Some(_) => {
                return reject(
                    self,
                    "COMPUTE_PLUGIN_INSTALL_PLAN_PREPARATION_ALREADY_BOUND",
                )
            }
            None => false,
        };
        if !replayed {
            self.last_install_plan_preparation = Some(request.clone());
        }
        self.preparation_observation(
            request,
            session_node_id,
            session_owner_user_id,
            bootstrap_instance_id,
            true,
            replayed,
            None,
        )
    }

    fn preparation_observation(
        &self,
        request: &ComputePluginInstallPlanPreparationRequestV1,
        session_node_id: &str,
        session_owner_user_id: &str,
        bootstrap_instance_id: &str,
        accepted: bool,
        replayed: bool,
        error_code: Option<&'static str>,
    ) -> ComputePluginInstallPlanPreparationObservedV1 {
        let desired = self.desired_policy.as_ref();
        ComputePluginInstallPlanPreparationObservedV1 {
            schema: COMPUTE_PLUGIN_INSTALL_PLAN_PREPARATION_OBSERVED_V1_SCHEMA.to_string(),
            preparation_id: request.preparation_id.clone(),
            node_id: session_node_id.to_string(),
            owner_user_id: session_owner_user_id.to_string(),
            installation_identity_digest: self
                .installation
                .as_ref()
                .map(|installation| installation.digest().to_string()),
            accepted,
            replayed,
            context_ready: false,
            context: None,
            observed_policy_revision: desired.map(|current| current.snapshot.policy_revision),
            observed_policy_digest: desired.map(|current| current.snapshot.policy_digest.clone()),
            observed_policy_snapshot_digest: desired.map(|current| current.snapshot_digest.clone()),
            observed_authorization: desired
                .and_then(|current| current.snapshot.authorization.clone()),
            bootstrap_instance_id: bootstrap_instance_id.to_string(),
            phase: PHASE_BLOCKED.to_string(),
            configuration_generation: self.configuration_generation,
            cancellation_generation: self.cancellation_generation,
            compute_plugin_root_lock_acquired: false,
            trusted_time_authority_configured: false,
            rollback_anchor_witness_configured: false,
            root_pinned: false,
            authority_opened: false,
            process_fence_acquired: false,
            new_work_admission_enabled: false,
            downloads_allowed: false,
            side_effects_started: false,
            blocked_reasons: preparation_blocked_reasons(),
            error_code: error_code.map(str::to_string),
        }
    }
}

fn poisoned_observation(
    request: &ComputePluginInstallPlanPreparationRequestV1,
    session_node_id: &str,
    session_owner_user_id: &str,
    bootstrap_instance_id: &str,
) -> ComputePluginInstallPlanPreparationObservedV1 {
    ComputePluginInstallPlanPreparationObservedV1 {
        schema: COMPUTE_PLUGIN_INSTALL_PLAN_PREPARATION_OBSERVED_V1_SCHEMA.to_string(),
        preparation_id: request.preparation_id.clone(),
        node_id: session_node_id.to_string(),
        owner_user_id: session_owner_user_id.to_string(),
        installation_identity_digest: None,
        accepted: false,
        replayed: false,
        context_ready: false,
        context: None,
        observed_policy_revision: None,
        observed_policy_digest: None,
        observed_policy_snapshot_digest: None,
        observed_authorization: None,
        bootstrap_instance_id: bootstrap_instance_id.to_string(),
        phase: PHASE_BLOCKED.to_string(),
        configuration_generation: 0,
        cancellation_generation: 0,
        compute_plugin_root_lock_acquired: false,
        trusted_time_authority_configured: false,
        rollback_anchor_witness_configured: false,
        root_pinned: false,
        authority_opened: false,
        process_fence_acquired: false,
        new_work_admission_enabled: false,
        downloads_allowed: false,
        side_effects_started: false,
        blocked_reasons: vec!["bootstrap_state_poisoned".to_string()],
        error_code: Some("COMPUTE_PLUGIN_BOOTSTRAP_STATE_POISONED".to_string()),
    }
}

fn preparation_blocked_reasons() -> Vec<String> {
    [
        "compute_plugin_root_lock_unavailable",
        "authenticated_trusted_time_unavailable",
        "production_rollback_anchor_witness_unavailable",
        "compute_plugin_authority_policy_binding_unavailable",
        "compute_plugin_inventory_snapshot_unavailable",
        "compute_plugin_node_profile_binding_unavailable",
        "compute_plugin_manifest_catalog_binding_unavailable",
        "compute_plugin_publisher_keyring_binding_unavailable",
        "compute_plugin_control_keyring_binding_unavailable",
    ]
    .into_iter()
    .map(str::to_string)
    .collect()
}

fn bounded_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_IDENTIFIER_BYTES
        && value.trim() == value
        && !value.chars().any(char::is_control)
}

fn safe_positive_revision(value: u64) -> bool {
    value > 0 && value <= MAX_IJSON_SAFE_INTEGER
}

fn same_policy_binding(
    current: &ComputePluginInstallPlanPreparationRequestV1,
    next: &ComputePluginInstallPlanPreparationRequestV1,
) -> bool {
    current.node_id == next.node_id
        && current.owner_user_id == next.owner_user_id
        && current.installation_identity_digest == next.installation_identity_digest
        && current.policy_revision == next.policy_revision
        && current.policy_digest == next.policy_digest
        && current.policy_snapshot_digest == next.policy_snapshot_digest
        && current.authorization == next.authorization
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}
