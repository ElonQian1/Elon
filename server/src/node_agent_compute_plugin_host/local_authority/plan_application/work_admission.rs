use std::{fmt, time::Instant};

use anyhow::{bail, Result};
use chrono::{DateTime, Utc};

use super::{
    prepare_application_request, read_authority_plan_application_state,
    ComputePluginPlanApplicationDisposition, ComputePluginPlanApplicationResult,
};
use crate::node_agent_compute_plugin_host::{
    install_plan::{SignedComputePluginInstallPlan, PLAN_ACTION_REAUTHORIZE_EXISTING},
    install_plan_admission::admit_install_plan,
    keyring::ComputePluginBootstrapRootKeyResolver,
    local_authority::{
        keyring_snapshot::{
            load_snapshot_for_state, read_authority_keyring_state, KeyringSnapshotValidation,
        },
        plan_application_persistence::{persist_plan_application, replay_plan_application},
        plan_application_projection::project_plan_application,
        ComputePluginAuthorityInstanceBinding, ComputePluginFetchProcessFence,
        ComputePluginLocalAuthority,
    },
    plugin_manifest::SignedComputePluginManifest,
    signed_artifact_verification::jcs_sha256_hex,
    trusted_time::ComputePluginTrustedTimeObservation,
};

#[derive(Clone)]
pub(in crate::node_agent_compute_plugin_host::local_authority) struct AuthenticatedPlanApplicationBarrier
{
    authority_instance_binding: ComputePluginAuthorityInstanceBinding,
    process_owner_epoch: i64,
    trusted_now: DateTime<Utc>,
    observed_at: Instant,
    installation_id_digest: String,
    clock_epoch_digest: String,
    time_authority_id: String,
    attestation_digest: String,
    attestation_sequence: i64,
    signing_key_fingerprint: String,
    commit_returned_at: Instant,
    application_receipt_digest: String,
}

impl ComputePluginLocalAuthority {
    /// Applies a plan under an authenticated observation. Only a non-empty plan containing solely
    /// `reauthorize_existing` items can receive the private work-admission barrier; other actions
    /// retain normal PlanApply behavior but cannot authorize work admission. Historical replay
    /// results never receive the barrier because old rows do not persist attestation provenance.
    pub(in crate::node_agent_compute_plugin_host) fn apply_install_plan(
        &self,
        process_fence: &ComputePluginFetchProcessFence,
        observation: &ComputePluginTrustedTimeObservation,
        signed_plan: &SignedComputePluginInstallPlan,
        signed_manifests: &[SignedComputePluginManifest],
        roots: &dyn ComputePluginBootstrapRootKeyResolver,
    ) -> Result<ComputePluginPlanApplicationResult> {
        validate_authenticated_fence(self, process_fence, observation)?;
        let reauthorization_only = !signed_plan.plan.items.is_empty()
            && signed_plan
                .plan
                .items
                .iter()
                .all(|item| item.action == PLAN_ACTION_REAUTHORIZE_EXISTING);
        let trusted_now = observation.trusted_now().clone();
        let request = prepare_application_request(signed_plan, signed_manifests)?;
        let mut result = self.with_immediate(|transaction| {
            if let Some(replayed) = replay_plan_application(
                transaction,
                &signed_plan.plan.plan_id,
                &signed_plan.plan_digest,
                &request.application_request_digest,
            )? {
                return Ok(replayed);
            }
            let authority = read_authority_plan_application_state(transaction, &trusted_now)?;
            let keyring_state = read_authority_keyring_state(transaction)?;
            if keyring_state.state_revision != authority.state_revision
                || keyring_state.authority_epoch != authority.authority_epoch
            {
                bail!("COMPUTE_PLUGIN_PLAN_AUTHORITY_FENCE_CHANGED");
            }
            let keyring = load_snapshot_for_state(
                transaction,
                &keyring_state,
                KeyringSnapshotValidation::Current(trusted_now.clone()),
                roots,
            )?;
            if keyring.bundle_revision() != authority.keyring_bundle_revision
                || keyring.publisher_binding() != &authority.publisher_keyring
                || keyring.control_binding() != &authority.control_keyring
            {
                bail!("COMPUTE_PLUGIN_PLAN_KEYRING_BINDING_CHANGED");
            }
            let admitted = admit_install_plan(
                signed_plan,
                &request.signed_manifests,
                &authority.inventory,
                &authority.live(),
                trusted_now.clone(),
                &keyring,
                &keyring,
            )?;
            let projected = project_plan_application(
                transaction,
                &authority,
                &admitted,
                trusted_now.timestamp_millis(),
            )?;
            persist_plan_application(
                transaction,
                &authority,
                &keyring,
                &request,
                &admitted,
                projected,
                trusted_now.timestamp_millis(),
            )
        })?;
        let commit_returned_at = Instant::now();
        validate_authenticated_fence(self, process_fence, observation)?;

        if reauthorization_only
            && result.disposition == ComputePluginPlanApplicationDisposition::Applied
            && result.receipt.plan_id == signed_plan.plan.plan_id
            && result.receipt.plan_digest == signed_plan.plan_digest
            && result.receipt.application_request_digest == request.application_request_digest
            && result.receipt.applied_at_ms == observation.trusted_now().timestamp_millis()
        {
            let application_receipt_digest = jcs_sha256_hex(&result.receipt)?;
            result.authenticated_work_admission_barrier =
                Some(AuthenticatedPlanApplicationBarrier {
                    authority_instance_binding: self.instance_binding().clone(),
                    process_owner_epoch: process_fence.process_owner_epoch(),
                    trusted_now: observation.trusted_now().clone(),
                    observed_at: observation.observed_at(),
                    installation_id_digest: observation.installation_id_digest().to_string(),
                    clock_epoch_digest: observation.clock_epoch_digest().to_string(),
                    time_authority_id: observation.time_authority_id().to_string(),
                    attestation_digest: observation.attestation_digest().to_string(),
                    attestation_sequence: observation.attestation_sequence(),
                    signing_key_fingerprint: observation.signing_key_fingerprint().to_string(),
                    commit_returned_at,
                    application_receipt_digest,
                });
        }
        Ok(result)
    }
}

impl ComputePluginPlanApplicationResult {
    pub(in crate::node_agent_compute_plugin_host::local_authority) fn authenticated_work_admission_barrier(
        &self,
    ) -> Option<&AuthenticatedPlanApplicationBarrier> {
        self.authenticated_work_admission_barrier.as_ref()
    }
}

impl AuthenticatedPlanApplicationBarrier {
    pub(in crate::node_agent_compute_plugin_host::local_authority) fn authority_instance_binding(
        &self,
    ) -> &ComputePluginAuthorityInstanceBinding {
        &self.authority_instance_binding
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority) fn process_owner_epoch(
        &self,
    ) -> i64 {
        self.process_owner_epoch
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority) fn trusted_now(
        &self,
    ) -> &DateTime<Utc> {
        &self.trusted_now
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority) fn installation_id_digest(
        &self,
    ) -> &str {
        &self.installation_id_digest
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority) fn clock_epoch_digest(
        &self,
    ) -> &str {
        &self.clock_epoch_digest
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority) fn commit_returned_at(
        &self,
    ) -> Instant {
        self.commit_returned_at
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority) fn application_receipt_digest(
        &self,
    ) -> &str {
        &self.application_receipt_digest
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority) fn matches_observation(
        &self,
        observation: &ComputePluginTrustedTimeObservation,
    ) -> bool {
        &self.trusted_now == observation.trusted_now()
            && self.observed_at == observation.observed_at()
            && self.installation_id_digest == observation.installation_id_digest()
            && self.clock_epoch_digest == observation.clock_epoch_digest()
            && self.time_authority_id == observation.time_authority_id()
            && self.attestation_digest == observation.attestation_digest()
            && self.attestation_sequence == observation.attestation_sequence()
            && self.signing_key_fingerprint == observation.signing_key_fingerprint()
    }
}

fn validate_authenticated_fence(
    authority: &ComputePluginLocalAuthority,
    process_fence: &ComputePluginFetchProcessFence,
    observation: &ComputePluginTrustedTimeObservation,
) -> Result<()> {
    observation.ensure_live(Instant::now())?;
    process_fence.ensure_process_owner_current()?;
    if !authority
        .instance_binding()
        .matches(process_fence.authority_instance_binding())
        || observation.installation_id_digest() != process_fence.installation_id_digest()
        || observation.clock_epoch_digest() != process_fence.clock_epoch_digest()
        || process_fence.process_owner_epoch() <= 0
        || observation.observed_at() <= process_fence.acquired_observed_at()
        || observation.trusted_now().timestamp_millis() < process_fence.acquired_at_ms()
    {
        bail!("COMPUTE_PLUGIN_PLAN_APPLICATION_AUTHENTICATION_INVALID");
    }
    Ok(())
}

impl fmt::Debug for AuthenticatedPlanApplicationBarrier {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AuthenticatedPlanApplicationBarrier")
            .field("trusted_now", &self.trusted_now)
            .field("observed_at", &"<monotonic>")
            .field("commit_returned_at", &"<monotonic>")
            .field(
                "application_receipt_digest",
                &self.application_receipt_digest,
            )
            .finish_non_exhaustive()
    }
}
