use std::time::Instant;

use anyhow::{bail, Result};
use chrono::{DateTime, Utc};

use super::{
    plan_application::{AuthenticatedPlanApplicationBarrier, ComputePluginPlanApplicationResult},
    ComputePluginAuthorityInstanceBinding, ComputePluginFetchProcessFence,
    ComputePluginLocalAuthority,
};
use crate::node_agent_compute_plugin_host::{
    identity::ComputePluginReleaseRef,
    install_plan::ComputePluginGrantBinding,
    manifest_validation::is_sha256,
    plugin_manifest::SignedComputePluginManifest,
    signed_artifact_verification::jcs_sha256_hex,
    trusted_time::ComputePluginTrustedTimeObservation,
    work_admission_contract::{
        RevalidatedInstalledWorkAdmission, ValidatedInstalledWorkAdmissionStorePermit,
    },
};

mod binding;
mod current;
mod head;
mod insert;
mod installed;
mod readback;
mod recovery;
mod source;
mod validation;
mod write;

pub(in crate::node_agent_compute_plugin_host) use recovery::ComputePluginWorkAdmissionRecoveryAuthoritySession;

pub(in crate::node_agent_compute_plugin_host) struct ComputePluginPostRevalidationWorkAdmissionAuthoritySession<
    'authority,
> {
    authority: &'authority ComputePluginLocalAuthority,
    process_fence: &'authority ComputePluginFetchProcessFence,
    trusted_observation: &'authority ComputePluginTrustedTimeObservation,
    trusted_now: DateTime<Utc>,
    observed_at: Instant,
    clock_epoch_digest: String,
    plan_application_barrier: AuthenticatedPlanApplicationBarrier,
    plan_application_plan_id: String,
    plan_application_plan_digest: String,
    plan_application_receipt_digest: String,
    plan_application_inventory_revision: i64,
    plan_application_inventory_digest: String,
}

#[derive(Debug, PartialEq, Eq)]
pub(in crate::node_agent_compute_plugin_host) struct ComputePluginInstalledWorkAdmissionAuthorityFacts
{
    signed_manifest: SignedComputePluginManifest,
    grant: ComputePluginGrantBinding,
    selected_host_api_revision: u32,
    action: String,
    plugin_id: String,
    slot_ref: String,
    release: ComputePluginReleaseRef,
    install_receipt_id: String,
    install_receipt_digest: String,
    promotion_receipt_id: String,
    promotion_receipt_digest: String,
    signed_manifest_envelope_digest: String,
    plan_id: String,
    plan_digest: String,
    signed_plan_envelope_digest: String,
    signed_manifest_set_digest: String,
    application_request_digest: String,
    application_receipt_digest: String,
    admission_bindings_digest: String,
    application_inventory_revision: i64,
    policy_revision: i64,
    sharing_authorization_ref: String,
    sharing_authorization_revision: i64,
    sharing_authorization_digest: String,
    policy_binding_receipt_digest: String,
    policy_revocation_receipt_digest: String,
    node_profile_digest: String,
    manifest_catalog_revision: i64,
    manifest_catalog_digest: String,
    manifest_catalog_binding_receipt_digest: String,
    keyring_bundle_revision: i64,
    publisher_keyring_revision: i64,
    publisher_keyring_digest: String,
    control_keyring_revision: i64,
    control_keyring_digest: String,
    install_generation: i64,
    activation_generation: i64,
    runtime_generation: i64,
    work_admission_generation_before: i64,
    work_admission_generation_after: i64,
    previous_work_admission_id: Option<String>,
    previous_work_admission_receipt_digest: Option<String>,
    desired_presence: String,
    desired_activation: String,
    slot_phase: String,
    admission: String,
    runtime_phase: String,
    candidate_slot_present: bool,
    runtime_slot_present: bool,
    runtime_runner_digest_present: bool,
    health_present: bool,
    active_attempts: i64,
    authority_state_revision_before: i64,
    authority_state_revision_after: i64,
    inventory_revision_before: i64,
    inventory_revision_after: i64,
    inventory_digest_before: String,
    inventory_digest_after: String,
    authority_epoch_before: i64,
    authority_epoch_after: i64,
    process_owner_epoch: i64,
    trusted_time_high_water_ms_before: i64,
    authority_updated_at_ms_before: i64,
    admitted_at_ms: i64,
}

impl ComputePluginLocalAuthority {
    pub(in crate::node_agent_compute_plugin_host) fn bind_installed_work_admission_authority_session<
        'authority,
    >(
        &'authority self,
        process_fence: &'authority ComputePluginFetchProcessFence,
        observation: &'authority ComputePluginTrustedTimeObservation,
        plan_application: &ComputePluginPlanApplicationResult,
    ) -> Result<ComputePluginPostRevalidationWorkAdmissionAuthoritySession<'authority>> {
        observation.ensure_live(Instant::now())?;
        process_fence.ensure_process_owner_current()?;
        let trusted_now = observation.trusted_now().clone();
        let observed_at = observation.observed_at();
        let barrier = plan_application
            .authenticated_work_admission_barrier()
            .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_WORK_ADMISSION_PLAN_BARRIER_MISSING"))?;
        let plan_receipt = plan_application.receipt();
        let application_receipt_digest = jcs_sha256_hex(plan_receipt)?;
        if !self
            .instance_binding()
            .matches(process_fence.authority_instance_binding())
            || !self
                .instance_binding()
                .matches(barrier.authority_instance_binding())
            || !is_sha256(observation.installation_id_digest())
            || observation.installation_id_digest() != process_fence.installation_id_digest()
            || observation.installation_id_digest() != barrier.installation_id_digest()
            || !is_sha256(observation.clock_epoch_digest())
            || observation.clock_epoch_digest() != process_fence.clock_epoch_digest()
            || observation.clock_epoch_digest() != barrier.clock_epoch_digest()
            || process_fence.process_owner_epoch() <= 0
            || process_fence.process_owner_epoch() != barrier.process_owner_epoch()
            || process_fence.acquired_at_ms() < 0
            || observed_at <= process_fence.acquired_observed_at()
            || observed_at <= barrier.commit_returned_at()
            || trusted_now.timestamp_millis() < process_fence.acquired_at_ms()
            || trusted_now <= barrier.trusted_now().clone()
            || application_receipt_digest != barrier.application_receipt_digest()
        {
            bail!("COMPUTE_PLUGIN_WORK_ADMISSION_AUTHORITY_SESSION_INVALID");
        }
        Ok(ComputePluginPostRevalidationWorkAdmissionAuthoritySession {
            authority: self,
            process_fence,
            trusted_observation: observation,
            trusted_now,
            observed_at,
            clock_epoch_digest: observation.clock_epoch_digest().to_string(),
            plan_application_barrier: barrier.clone(),
            plan_application_plan_id: plan_receipt.plan_id().to_string(),
            plan_application_plan_digest: plan_receipt.plan_digest().to_string(),
            plan_application_receipt_digest: application_receipt_digest,
            plan_application_inventory_revision: plan_receipt.inventory_after_revision(),
            plan_application_inventory_digest: plan_receipt.inventory_after_digest().to_string(),
        })
    }
}

impl ComputePluginPostRevalidationWorkAdmissionAuthoritySession<'_> {
    pub(in crate::node_agent_compute_plugin_host) fn authority_instance_binding(
        &self,
    ) -> &ComputePluginAuthorityInstanceBinding {
        self.process_fence.authority_instance_binding()
    }

    pub(in crate::node_agent_compute_plugin_host) fn installation_id_digest(&self) -> &str {
        self.process_fence.installation_id_digest()
    }

    pub(in crate::node_agent_compute_plugin_host) fn clock_epoch_digest(&self) -> &str {
        &self.clock_epoch_digest
    }

    pub(in crate::node_agent_compute_plugin_host) fn trusted_now_ms(&self) -> i64 {
        self.trusted_now.timestamp_millis()
    }

    pub(in crate::node_agent_compute_plugin_host) fn was_observed_strictly_after(
        &self,
        barrier: Instant,
    ) -> bool {
        self.observed_at > barrier
    }

    pub(in crate::node_agent_compute_plugin_host) fn plan_application_matches_observation(
        &self,
        observation: &ComputePluginTrustedTimeObservation,
    ) -> bool {
        self.plan_application_barrier
            .matches_observation(observation)
    }

    fn ensure_trusted_time_live(&self) -> Result<()> {
        self.trusted_observation.ensure_live(Instant::now())
    }

    pub(in crate::node_agent_compute_plugin_host) fn read_installed_work_admission_binding(
        &self,
        revalidated: &RevalidatedInstalledWorkAdmission<'_>,
    ) -> Result<ComputePluginInstalledWorkAdmissionAuthorityFacts> {
        self.process_fence.ensure_process_owner_current()?;
        self.authority.with_deferred(|transaction| {
            binding::read_installed_work_admission_binding(transaction, self, revalidated)
        })
    }

    pub(in crate::node_agent_compute_plugin_host) fn persist_installed_work_admission(
        &self,
        permit: ValidatedInstalledWorkAdmissionStorePermit<'_, '_>,
    ) -> Result<()> {
        self.authority.with_immediate(|transaction| {
            write::persist_installed_work_admission(transaction, self, permit)
        })
    }
}

macro_rules! fact_getters {
    (str: $($string:ident),*; i64: $($number:ident),*; bool: $($flag:ident),* $(;)?) => {
        $(pub(in crate::node_agent_compute_plugin_host) fn $string(&self) -> &str {
            &self.$string
        })*
        $(pub(in crate::node_agent_compute_plugin_host) fn $number(&self) -> i64 {
            self.$number
        })*
        $(pub(in crate::node_agent_compute_plugin_host) fn $flag(&self) -> bool {
            self.$flag
        })*
    };
}

impl ComputePluginInstalledWorkAdmissionAuthorityFacts {
    pub(in crate::node_agent_compute_plugin_host) fn signed_manifest(
        &self,
    ) -> &SignedComputePluginManifest {
        &self.signed_manifest
    }

    pub(in crate::node_agent_compute_plugin_host) fn grant(&self) -> &ComputePluginGrantBinding {
        &self.grant
    }

    pub(in crate::node_agent_compute_plugin_host) fn selected_host_api_revision(&self) -> u32 {
        self.selected_host_api_revision
    }

    pub(in crate::node_agent_compute_plugin_host) fn release(&self) -> &ComputePluginReleaseRef {
        &self.release
    }

    pub(in crate::node_agent_compute_plugin_host) fn previous_work_admission_id(
        &self,
    ) -> Option<&str> {
        self.previous_work_admission_id.as_deref()
    }

    pub(in crate::node_agent_compute_plugin_host) fn previous_work_admission_receipt_digest(
        &self,
    ) -> Option<&str> {
        self.previous_work_admission_receipt_digest.as_deref()
    }

    fact_getters! {
        str: action, plugin_id, slot_ref, install_receipt_id, install_receipt_digest,
            promotion_receipt_id, promotion_receipt_digest, signed_manifest_envelope_digest,
            plan_id, plan_digest, signed_plan_envelope_digest, signed_manifest_set_digest,
            application_request_digest, application_receipt_digest, admission_bindings_digest,
            sharing_authorization_ref, sharing_authorization_digest,
            policy_binding_receipt_digest, policy_revocation_receipt_digest,
            node_profile_digest, manifest_catalog_digest,
            manifest_catalog_binding_receipt_digest, publisher_keyring_digest,
            control_keyring_digest, desired_presence, desired_activation, slot_phase, admission,
            runtime_phase, inventory_digest_before, inventory_digest_after;
        i64: application_inventory_revision, policy_revision,
            sharing_authorization_revision, manifest_catalog_revision, keyring_bundle_revision,
            publisher_keyring_revision, control_keyring_revision, install_generation,
            activation_generation, runtime_generation, work_admission_generation_before,
            work_admission_generation_after, active_attempts, authority_state_revision_before,
            authority_state_revision_after, inventory_revision_before, inventory_revision_after,
            authority_epoch_before, authority_epoch_after, process_owner_epoch,
            trusted_time_high_water_ms_before, authority_updated_at_ms_before, admitted_at_ms;
        bool: candidate_slot_present, runtime_slot_present, runtime_runner_digest_present,
            health_present;
    }
}
