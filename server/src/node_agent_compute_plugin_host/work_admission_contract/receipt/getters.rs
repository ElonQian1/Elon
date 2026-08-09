use crate::node_agent_compute_plugin_host::identity::ComputePluginReleaseRef;

use super::{
    ComputePluginWorkAdmissionAuthorityTransition, ComputePluginWorkAdmissionGenerationTransition,
    ComputePluginWorkAdmissionPlanBinding, ComputePluginWorkAdmissionQuiescence,
    ComputePluginWorkAdmissionReceipt, ComputePluginWorkAdmissionReceiptPair,
    ComputePluginWorkAdmissionSource, HashedComputePluginWorkAdmissionReceipt,
    HashedComputePluginWorkAdmissionSource,
};
use crate::node_agent_compute_plugin_host::work_admission_contract::ComputePluginWorkAdmissionLaunchProfile;

macro_rules! string_getters {
    ($ty:ty; $($name:ident),* $(,)?) => {
        impl $ty {$(
            pub(in crate::node_agent_compute_plugin_host) fn $name(&self) -> &str {
                &self.$name
            }
        )*}
    };
}

macro_rules! number_getters {
    ($ty:ty; $($name:ident),* $(,)?) => {
        impl $ty {$(
            pub(in crate::node_agent_compute_plugin_host) fn $name(&self) -> i64 {
                self.$name
            }
        )*}
    };
}

string_getters! {
    ComputePluginWorkAdmissionPlanBinding;
    action, plan_id, plan_digest, signed_plan_envelope_digest, signed_manifest_set_digest,
    application_request_digest, application_receipt_digest,
    admission_bindings_digest, sharing_authorization_ref, sharing_authorization_digest,
    policy_binding_receipt_digest, policy_revocation_receipt_digest, node_profile_digest,
    manifest_catalog_digest, manifest_catalog_binding_receipt_digest,
    publisher_keyring_digest, control_keyring_digest,
}

number_getters! {
    ComputePluginWorkAdmissionPlanBinding;
    application_inventory_revision, policy_revision, sharing_authorization_revision,
    manifest_catalog_revision, keyring_bundle_revision, publisher_keyring_revision,
    control_keyring_revision,
}

string_getters! {
    ComputePluginWorkAdmissionSource;
    installation_id_digest, plugin_id, slot_ref, install_receipt_id, install_receipt_digest,
    promotion_receipt_id, promotion_receipt_digest,
}

impl ComputePluginWorkAdmissionSource {
    pub(in crate::node_agent_compute_plugin_host) fn release(&self) -> &ComputePluginReleaseRef {
        &self.release
    }

    pub(in crate::node_agent_compute_plugin_host) fn plan(
        &self,
    ) -> &ComputePluginWorkAdmissionPlanBinding {
        &self.plan
    }

    pub(in crate::node_agent_compute_plugin_host) fn launch_profile(
        &self,
    ) -> &ComputePluginWorkAdmissionLaunchProfile {
        &self.launch_profile
    }
}

impl HashedComputePluginWorkAdmissionSource {
    pub(in crate::node_agent_compute_plugin_host) fn source(
        &self,
    ) -> &ComputePluginWorkAdmissionSource {
        &self.source
    }

    pub(in crate::node_agent_compute_plugin_host) fn source_digest(&self) -> &str {
        &self.source_digest
    }
}

number_getters! {
    ComputePluginWorkAdmissionGenerationTransition;
    install_generation, activation_generation, runtime_generation,
    work_admission_generation_before, work_admission_generation_after,
}

impl ComputePluginWorkAdmissionGenerationTransition {
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
}

string_getters! {
    ComputePluginWorkAdmissionQuiescence;
    desired_presence, desired_activation, slot_phase, admission, runtime_phase,
}

impl ComputePluginWorkAdmissionQuiescence {
    pub(in crate::node_agent_compute_plugin_host) fn candidate_slot_present(&self) -> bool {
        self.candidate_slot_present
    }

    pub(in crate::node_agent_compute_plugin_host) fn runtime_slot_present(&self) -> bool {
        self.runtime_slot_present
    }

    pub(in crate::node_agent_compute_plugin_host) fn runtime_runner_digest_present(&self) -> bool {
        self.runtime_runner_digest_present
    }

    pub(in crate::node_agent_compute_plugin_host) fn health_present(&self) -> bool {
        self.health_present
    }

    pub(in crate::node_agent_compute_plugin_host) fn active_attempts(&self) -> i64 {
        self.active_attempts
    }
}

number_getters! {
    ComputePluginWorkAdmissionAuthorityTransition;
    authority_state_revision_before, authority_state_revision_after,
    inventory_revision_before, inventory_revision_after, authority_epoch_before,
    authority_epoch_after, process_owner_epoch, trusted_time_high_water_ms_before,
    authority_updated_at_ms_before,
}

string_getters! {
    ComputePluginWorkAdmissionAuthorityTransition;
    inventory_digest_before, inventory_digest_after,
}

string_getters! {
    ComputePluginWorkAdmissionReceipt;
    work_admission_id, installation_id_digest, clock_epoch_digest, plugin_id, slot_ref,
    install_receipt_id, install_receipt_digest, promotion_receipt_id,
    promotion_receipt_digest, source_digest,
}

impl ComputePluginWorkAdmissionReceipt {
    pub(in crate::node_agent_compute_plugin_host) fn release(&self) -> &ComputePluginReleaseRef {
        &self.release
    }

    pub(in crate::node_agent_compute_plugin_host) fn generations(
        &self,
    ) -> &ComputePluginWorkAdmissionGenerationTransition {
        &self.generations
    }

    pub(in crate::node_agent_compute_plugin_host) fn quiescence(
        &self,
    ) -> &ComputePluginWorkAdmissionQuiescence {
        &self.quiescence
    }

    pub(in crate::node_agent_compute_plugin_host) fn authority(
        &self,
    ) -> &ComputePluginWorkAdmissionAuthorityTransition {
        &self.authority
    }

    pub(in crate::node_agent_compute_plugin_host) fn admitted_at_ms(&self) -> i64 {
        self.admitted_at_ms
    }
}

impl HashedComputePluginWorkAdmissionReceipt {
    pub(in crate::node_agent_compute_plugin_host) fn receipt(
        &self,
    ) -> &ComputePluginWorkAdmissionReceipt {
        &self.receipt
    }

    pub(in crate::node_agent_compute_plugin_host) fn receipt_digest(&self) -> &str {
        &self.receipt_digest
    }
}

impl ComputePluginWorkAdmissionReceiptPair {
    pub(in crate::node_agent_compute_plugin_host) fn source(
        &self,
    ) -> &HashedComputePluginWorkAdmissionSource {
        &self.source
    }

    pub(in crate::node_agent_compute_plugin_host) fn receipt(
        &self,
    ) -> &HashedComputePluginWorkAdmissionReceipt {
        &self.receipt
    }
}
