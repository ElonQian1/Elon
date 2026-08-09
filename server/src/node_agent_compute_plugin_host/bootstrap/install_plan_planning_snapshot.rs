use homecli_proto::{
    ComputePluginInstallPlanPlanningSnapshotObservedV2,
    ComputePluginInstallPlanPlanningSnapshotRequestV2,
    HashedComputePluginInstallPlanPlanningSnapshotV2,
    COMPUTE_PLUGIN_INSTALL_PLAN_PLANNING_SNAPSHOT_OBSERVED_V2_SCHEMA,
    COMPUTE_PLUGIN_INSTALL_PLAN_PLANNING_SNAPSHOT_REQUEST_V2_SCHEMA,
    MAX_COMPUTE_PLUGIN_INSTALL_PLAN_PLANNING_SAFE_INTEGER,
    MAX_COMPUTE_PLUGIN_INSTALL_PLAN_PLANNING_SNAPSHOT_BYTES,
};

use super::{ComputePluginBootstrap, ComputePluginBootstrapState};
use crate::compute_plugin_sharing_directive::canonical_compute_plugin_ijson_and_sha256;

const PHASE_BLOCKED: &str = "blocked";
const MAX_IDENTIFIER_BYTES: usize = 256;

impl ComputePluginBootstrap {
    /// Observes a V2 planning request against dormant Bootstrap state. There is deliberately no
    /// snapshot producer in this implementation, so an accepted request still reports
    /// `snapshot_ready=false` without pinning a root, opening SQLite or reaching execution code.
    pub(crate) fn observe_install_plan_planning_snapshot_v2(
        &self,
        request: &ComputePluginInstallPlanPlanningSnapshotRequestV2,
        session_node_id: &str,
        session_owner_user_id: &str,
    ) -> ComputePluginInstallPlanPlanningSnapshotObservedV2 {
        match self.state.lock() {
            Ok(mut state) => state.observe_install_plan_planning_snapshot(
                request,
                session_node_id,
                session_owner_user_id,
                &self.bootstrap_instance_id,
            ),
            Err(_) => {
                self.invalidate_policy_binding_intents_after_poison();
                poisoned_observation(
                    request,
                    session_node_id,
                    session_owner_user_id,
                    &self.bootstrap_instance_id,
                )
            }
        }
    }
}

impl ComputePluginBootstrapState {
    fn observe_install_plan_planning_snapshot(
        &mut self,
        request: &ComputePluginInstallPlanPlanningSnapshotRequestV2,
        session_node_id: &str,
        session_owner_user_id: &str,
        bootstrap_instance_id: &str,
    ) -> ComputePluginInstallPlanPlanningSnapshotObservedV2 {
        let reject = |state: &Self, code| {
            state.planning_snapshot_observation(
                request,
                session_node_id,
                session_owner_user_id,
                bootstrap_instance_id,
                false,
                false,
                Some(code),
            )
        };
        if request.schema != COMPUTE_PLUGIN_INSTALL_PLAN_PLANNING_SNAPSHOT_REQUEST_V2_SCHEMA {
            return reject(self, "COMPUTE_PLUGIN_PLANNING_SNAPSHOT_SCHEMA_UNSUPPORTED");
        }
        if !request_shape_is_valid(request) {
            return reject(self, "COMPUTE_PLUGIN_PLANNING_SNAPSHOT_REQUEST_INVALID");
        }
        if !safe_generation(self.configuration_generation)
            || !safe_generation(self.cancellation_generation)
        {
            self.last_install_plan_planning_snapshot = None;
            return reject(
                self,
                "COMPUTE_PLUGIN_PLANNING_SNAPSHOT_GENERATION_OUT_OF_RANGE",
            );
        }
        if request.node_id != session_node_id {
            return reject(
                self,
                "COMPUTE_PLUGIN_PLANNING_SNAPSHOT_NODE_BINDING_MISMATCH",
            );
        }
        if request.owner_user_id != session_owner_user_id {
            return reject(
                self,
                "COMPUTE_PLUGIN_PLANNING_SNAPSHOT_OWNER_BINDING_MISMATCH",
            );
        }
        let Some(installation) = self.installation.as_ref() else {
            return reject(
                self,
                "COMPUTE_PLUGIN_PLANNING_SNAPSHOT_LOCAL_INSTALLATION_INVALID",
            );
        };
        if request.installation_identity_digest != installation.digest() {
            return reject(
                self,
                "COMPUTE_PLUGIN_PLANNING_SNAPSHOT_INSTALLATION_BINDING_MISMATCH",
            );
        }
        let Some(desired) = self.desired_policy.as_ref() else {
            return reject(self, "COMPUTE_PLUGIN_PLANNING_SNAPSHOT_POLICY_UNAVAILABLE");
        };
        if !self.sharing_requested || !desired.snapshot.plugin_runtime_requested {
            return reject(self, "COMPUTE_PLUGIN_PLANNING_SNAPSHOT_SHARING_DISABLED");
        }
        if request.policy_revision != desired.snapshot.policy_revision
            || request.policy_digest != desired.snapshot.policy_digest
            || request.policy_snapshot_digest != desired.snapshot_digest
        {
            return reject(
                self,
                "COMPUTE_PLUGIN_PLANNING_SNAPSHOT_POLICY_BINDING_MISMATCH",
            );
        }
        if desired.snapshot.authorization.as_ref() != Some(&request.authorization)
            || request.authorization.revision != request.policy_revision
            || request.authorization.digest != request.policy_digest
        {
            return reject(
                self,
                "COMPUTE_PLUGIN_PLANNING_SNAPSHOT_AUTHORIZATION_BINDING_MISMATCH",
            );
        }
        let Some(source) = self.last_install_plan_preparation.as_ref() else {
            return reject(
                self,
                "COMPUTE_PLUGIN_PLANNING_SNAPSHOT_SOURCE_PREPARATION_UNAVAILABLE",
            );
        };
        if !bounded_identifier(&source.planning_context_id) {
            return reject(
                self,
                "COMPUTE_PLUGIN_PLANNING_SNAPSHOT_SOURCE_CONTEXT_INVALID",
            );
        }
        if !source_preparation_matches(request, &source.request) {
            return reject(
                self,
                "COMPUTE_PLUGIN_PLANNING_SNAPSHOT_SOURCE_PREPARATION_MISMATCH",
            );
        }
        if request.source_preparation_delivery_id != source.delivery_id {
            return reject(
                self,
                "COMPUTE_PLUGIN_PLANNING_SNAPSHOT_SOURCE_DELIVERY_MISMATCH",
            );
        }
        if request.source_preparation_observation_digest != source.observation_digest {
            return reject(
                self,
                "COMPUTE_PLUGIN_PLANNING_SNAPSHOT_SOURCE_OBSERVATION_DIGEST_MISMATCH",
            );
        }
        if source.bootstrap_instance_id != bootstrap_instance_id
            || source.configuration_generation != self.configuration_generation
            || source.cancellation_generation != self.cancellation_generation
        {
            return reject(
                self,
                "COMPUTE_PLUGIN_PLANNING_SNAPSHOT_SOURCE_WITNESS_STALE",
            );
        }
        if self
            .last_install_plan_planning_snapshot
            .as_ref()
            .is_some_and(|current| !same_policy_binding(current, request))
        {
            self.last_install_plan_planning_snapshot = None;
        }
        let replayed = match self.last_install_plan_planning_snapshot.as_ref() {
            Some(current) if current == request => true,
            Some(current)
                if current.cloud_session_id == request.cloud_session_id
                    && current.source_preparation_delivery_id
                        == request.source_preparation_delivery_id =>
            {
                return reject(self, "COMPUTE_PLUGIN_PLANNING_SNAPSHOT_DELIVERY_CONFLICT")
            }
            Some(_) => return reject(self, "COMPUTE_PLUGIN_PLANNING_SNAPSHOT_ALREADY_BOUND"),
            None => false,
        };
        if !replayed {
            self.last_install_plan_planning_snapshot = Some(request.clone());
        }
        self.planning_snapshot_observation(
            request,
            session_node_id,
            session_owner_user_id,
            bootstrap_instance_id,
            true,
            replayed,
            None,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn planning_snapshot_observation(
        &self,
        request: &ComputePluginInstallPlanPlanningSnapshotRequestV2,
        session_node_id: &str,
        session_owner_user_id: &str,
        bootstrap_instance_id: &str,
        accepted: bool,
        replayed: bool,
        error_code: Option<&'static str>,
    ) -> ComputePluginInstallPlanPlanningSnapshotObservedV2 {
        let desired = self.desired_policy.as_ref().filter(|current| {
            safe_positive_revision(current.snapshot.policy_revision)
                && current
                    .snapshot
                    .authorization
                    .as_ref()
                    .is_none_or(|authorization| safe_positive_revision(authorization.revision))
        });
        ComputePluginInstallPlanPlanningSnapshotObservedV2 {
            schema: COMPUTE_PLUGIN_INSTALL_PLAN_PLANNING_SNAPSHOT_OBSERVED_V2_SCHEMA.to_string(),
            preparation_id: request.preparation_id.clone(),
            cloud_session_id: request.cloud_session_id.clone(),
            source_preparation_delivery_id: request.source_preparation_delivery_id.clone(),
            source_preparation_observation_digest: request
                .source_preparation_observation_digest
                .clone(),
            node_id: session_node_id.to_string(),
            owner_user_id: session_owner_user_id.to_string(),
            installation_identity_digest: self
                .installation
                .as_ref()
                .map(|installation| installation.digest().to_string()),
            accepted,
            replayed,
            snapshot_ready: false,
            snapshot: None,
            observed_policy_revision: desired.map(|current| current.snapshot.policy_revision),
            observed_policy_digest: desired.map(|current| current.snapshot.policy_digest.clone()),
            observed_policy_snapshot_digest: desired.map(|current| current.snapshot_digest.clone()),
            observed_authorization: desired
                .and_then(|current| current.snapshot.authorization.clone()),
            bootstrap_instance_id: bootstrap_instance_id.to_string(),
            phase: PHASE_BLOCKED.to_string(),
            configuration_generation: wire_generation(self.configuration_generation),
            cancellation_generation: wire_generation(self.cancellation_generation),
            local_confirmation_available: false,
            compute_plugin_root_lock_acquired: false,
            trusted_time_authority_configured: false,
            rollback_anchor_witness_configured: false,
            root_pinned: false,
            authority_opened: false,
            process_fence_acquired: false,
            plan_apply_allowed: false,
            new_work_admission_enabled: false,
            downloads_allowed: false,
            sidecar_launch_allowed: false,
            side_effects_started: false,
            blocked_reasons: planning_snapshot_blocked_reasons(),
            error_code: error_code.map(str::to_string),
        }
    }
}

/// The only admissible shape/digest/binding gate for a future local snapshot producer. No caller
/// exists while Bootstrap lacks a coherent read-only authority projection.
#[allow(dead_code)]
fn validate_ready_snapshot_candidate(
    state: &ComputePluginBootstrapState,
    request: &ComputePluginInstallPlanPlanningSnapshotRequestV2,
    bootstrap_instance_id: &str,
    hashed: &HashedComputePluginInstallPlanPlanningSnapshotV2,
) -> Result<(), &'static str> {
    hashed.validate_ready_shape_v2()?;
    let snapshot = &hashed.snapshot;
    let source = state
        .last_install_plan_preparation
        .as_ref()
        .ok_or("COMPUTE_PLUGIN_PLANNING_SNAPSHOT_READY_SOURCE_UNAVAILABLE")?;
    if !bounded_identifier(&source.planning_context_id) {
        return Err("COMPUTE_PLUGIN_PLANNING_SNAPSHOT_READY_SOURCE_CONTEXT_INVALID");
    }
    if snapshot.preparation_id != request.preparation_id
        || snapshot.cloud_session_id != request.cloud_session_id
        || snapshot.source_preparation_delivery_id != request.source_preparation_delivery_id
        || snapshot.source_preparation_observation_digest
            != request.source_preparation_observation_digest
        || snapshot.node_id != request.node_id
        || snapshot.owner_user_id != request.owner_user_id
        || snapshot.installation_identity_digest != request.installation_identity_digest
        || snapshot.policy_revision != request.policy_revision
        || snapshot.policy_digest != request.policy_digest
        || snapshot.policy_snapshot_digest != request.policy_snapshot_digest
        || snapshot.authorization != request.authorization
        || snapshot.bootstrap_instance_id != bootstrap_instance_id
        || snapshot.configuration_generation != state.configuration_generation
        || snapshot.cancellation_generation != state.cancellation_generation
        || !source_preparation_matches(request, &source.request)
        || source.delivery_id != request.source_preparation_delivery_id
        || source.observation_digest != request.source_preparation_observation_digest
        || source.bootstrap_instance_id != bootstrap_instance_id
        || source.configuration_generation != state.configuration_generation
        || source.cancellation_generation != state.cancellation_generation
    {
        return Err("COMPUTE_PLUGIN_PLANNING_SNAPSHOT_READY_BINDING_MISMATCH");
    }
    let (_, digest) = canonical_compute_plugin_ijson_and_sha256(
        snapshot,
        MAX_COMPUTE_PLUGIN_INSTALL_PLAN_PLANNING_SNAPSHOT_BYTES,
    )
    .map_err(|_| "COMPUTE_PLUGIN_PLANNING_SNAPSHOT_READY_CANONICALIZATION_INVALID")?;
    if digest != hashed.snapshot_digest {
        return Err("COMPUTE_PLUGIN_PLANNING_SNAPSHOT_READY_DIGEST_MISMATCH");
    }
    Ok(())
}

fn poisoned_observation(
    request: &ComputePluginInstallPlanPlanningSnapshotRequestV2,
    session_node_id: &str,
    session_owner_user_id: &str,
    bootstrap_instance_id: &str,
) -> ComputePluginInstallPlanPlanningSnapshotObservedV2 {
    ComputePluginInstallPlanPlanningSnapshotObservedV2 {
        schema: COMPUTE_PLUGIN_INSTALL_PLAN_PLANNING_SNAPSHOT_OBSERVED_V2_SCHEMA.to_string(),
        preparation_id: request.preparation_id.clone(),
        cloud_session_id: request.cloud_session_id.clone(),
        source_preparation_delivery_id: request.source_preparation_delivery_id.clone(),
        source_preparation_observation_digest: request
            .source_preparation_observation_digest
            .clone(),
        node_id: session_node_id.to_string(),
        owner_user_id: session_owner_user_id.to_string(),
        installation_identity_digest: None,
        accepted: false,
        replayed: false,
        snapshot_ready: false,
        snapshot: None,
        observed_policy_revision: None,
        observed_policy_digest: None,
        observed_policy_snapshot_digest: None,
        observed_authorization: None,
        bootstrap_instance_id: bootstrap_instance_id.to_string(),
        phase: PHASE_BLOCKED.to_string(),
        configuration_generation: 0,
        cancellation_generation: 0,
        local_confirmation_available: false,
        compute_plugin_root_lock_acquired: false,
        trusted_time_authority_configured: false,
        rollback_anchor_witness_configured: false,
        root_pinned: false,
        authority_opened: false,
        process_fence_acquired: false,
        plan_apply_allowed: false,
        new_work_admission_enabled: false,
        downloads_allowed: false,
        sidecar_launch_allowed: false,
        side_effects_started: false,
        blocked_reasons: vec!["bootstrap_state_poisoned".to_string()],
        error_code: Some("COMPUTE_PLUGIN_BOOTSTRAP_STATE_POISONED".to_string()),
    }
}

fn planning_snapshot_blocked_reasons() -> Vec<String> {
    [
        "planning_snapshot_producer_unavailable",
        "compute_plugin_root_lock_unavailable",
        "authenticated_trusted_time_unavailable",
        "production_rollback_anchor_witness_unavailable",
        "compute_plugin_authority_policy_binding_receipt_unavailable",
        "compute_plugin_policy_capability_revocation_receipt_unavailable",
        "compute_plugin_inventory_snapshot_unavailable",
        "compute_plugin_node_profile_binding_unavailable",
        "compute_plugin_manifest_catalog_binding_unavailable",
        "compute_plugin_publisher_keyring_binding_unavailable",
        "compute_plugin_control_keyring_binding_unavailable",
        "compute_plugin_installed_receipts_unavailable",
        "compute_plugin_work_admission_receipts_unavailable",
    ]
    .into_iter()
    .map(str::to_string)
    .collect()
}

fn request_shape_is_valid(request: &ComputePluginInstallPlanPlanningSnapshotRequestV2) -> bool {
    bounded_identifier(&request.preparation_id)
        && bounded_identifier(&request.cloud_session_id)
        && bounded_identifier(&request.source_preparation_delivery_id)
        && bounded_identifier(&request.node_id)
        && bounded_identifier(&request.owner_user_id)
        && bounded_identifier(&request.authorization.authorization_ref)
        && is_sha256(&request.source_preparation_observation_digest)
        && is_sha256(&request.installation_identity_digest)
        && is_sha256(&request.policy_digest)
        && is_sha256(&request.policy_snapshot_digest)
        && is_sha256(&request.authorization.digest)
        && safe_positive_revision(request.policy_revision)
        && safe_positive_revision(request.authorization.revision)
}

fn source_preparation_matches(
    request: &ComputePluginInstallPlanPlanningSnapshotRequestV2,
    source: &homecli_proto::ComputePluginInstallPlanPreparationRequestV1,
) -> bool {
    request.preparation_id == source.preparation_id
        && request.node_id == source.node_id
        && request.owner_user_id == source.owner_user_id
        && request.installation_identity_digest == source.installation_identity_digest
        && request.policy_revision == source.policy_revision
        && request.policy_digest == source.policy_digest
        && request.policy_snapshot_digest == source.policy_snapshot_digest
        && request.authorization == source.authorization
}

fn same_policy_binding(
    current: &ComputePluginInstallPlanPlanningSnapshotRequestV2,
    next: &ComputePluginInstallPlanPlanningSnapshotRequestV2,
) -> bool {
    current.preparation_id == next.preparation_id
        && current.node_id == next.node_id
        && current.owner_user_id == next.owner_user_id
        && current.installation_identity_digest == next.installation_identity_digest
        && current.policy_revision == next.policy_revision
        && current.policy_digest == next.policy_digest
        && current.policy_snapshot_digest == next.policy_snapshot_digest
        && current.authorization == next.authorization
}

fn bounded_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_IDENTIFIER_BYTES
        && value.trim() == value
        && !value.chars().any(char::is_control)
}

fn safe_positive_revision(value: u64) -> bool {
    (1..=MAX_COMPUTE_PLUGIN_INSTALL_PLAN_PLANNING_SAFE_INTEGER).contains(&value)
}

fn safe_generation(value: u64) -> bool {
    value <= MAX_COMPUTE_PLUGIN_INSTALL_PLAN_PLANNING_SAFE_INTEGER
}

fn wire_generation(value: u64) -> u64 {
    if safe_generation(value) {
        value
    } else {
        0
    }
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}
