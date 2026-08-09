use anyhow::{bail, Result};

use crate::node_agent_compute_plugin_host::{
    install_plan::PLAN_ACTION_REAUTHORIZE_EXISTING,
    install_plan_admission_validation::is_identifier,
    lifecycle::{
        ACTIVATION_ENABLED, ADMISSION_ALLOWED, DESIRED_PRESENCE_PRESENT, RUNTIME_STOPPED,
        SLOT_INSTALLED,
    },
    manifest_validation::is_sha256,
    signed_artifact_verification::jcs_sha256_hex,
};

use super::{
    ComputePluginWorkAdmissionAuthorityTransition, ComputePluginWorkAdmissionGenerationTransition,
    ComputePluginWorkAdmissionPlanBinding, ComputePluginWorkAdmissionQuiescence,
    ComputePluginWorkAdmissionReceipt, ComputePluginWorkAdmissionReceiptPair,
    ComputePluginWorkAdmissionSource, HashedComputePluginWorkAdmissionReceipt,
    HashedComputePluginWorkAdmissionSource, WorkAdmissionIdBinding, ID_BINDING_SCHEMA,
    PLAN_BINDING_SCHEMA,
};
use crate::node_agent_compute_plugin_host::work_admission_contract::{
    CANONICALIZATION, DIGEST_ALGORITHM, HASHED_RECEIPT_SCHEMA, HASHED_SOURCE_SCHEMA,
    RECEIPT_SCHEMA, SOURCE_SCHEMA,
};

const RELEASE_JSON_MAX_BYTES: usize = 65_536;
const SOURCE_JSON_MAX_BYTES: usize = 1_048_576;
const RECEIPT_JSON_MAX_BYTES: usize = 1_048_576;
const I_JSON_MAX_SAFE_INTEGER: i64 = 9_007_199_254_740_991;

impl ComputePluginWorkAdmissionPlanBinding {
    pub(in crate::node_agent_compute_plugin_host) fn validate(&self) -> Result<()> {
        let digests = [
            self.plan_digest.as_str(),
            self.signed_plan_envelope_digest.as_str(),
            self.signed_manifest_set_digest.as_str(),
            self.application_request_digest.as_str(),
            self.application_receipt_digest.as_str(),
            self.admission_bindings_digest.as_str(),
            self.sharing_authorization_digest.as_str(),
            self.policy_binding_receipt_digest.as_str(),
            self.policy_revocation_receipt_digest.as_str(),
            self.node_profile_digest.as_str(),
            self.manifest_catalog_digest.as_str(),
            self.manifest_catalog_binding_receipt_digest.as_str(),
            self.publisher_keyring_digest.as_str(),
            self.control_keyring_digest.as_str(),
        ];
        if self.schema != PLAN_BINDING_SCHEMA
            || self.action != PLAN_ACTION_REAUTHORIZE_EXISTING
            || !is_identifier(&self.plan_id)
            || !is_identifier(&self.sharing_authorization_ref)
            || self.application_inventory_revision <= 0
            || self.application_inventory_revision > I_JSON_MAX_SAFE_INTEGER
            || self.policy_revision <= 0
            || self.policy_revision > I_JSON_MAX_SAFE_INTEGER
            || self.sharing_authorization_revision <= 0
            || self.sharing_authorization_revision > I_JSON_MAX_SAFE_INTEGER
            || self.sharing_authorization_revision != self.policy_revision
            || self.manifest_catalog_revision <= 0
            || self.manifest_catalog_revision > I_JSON_MAX_SAFE_INTEGER
            || self.keyring_bundle_revision <= 0
            || self.keyring_bundle_revision > I_JSON_MAX_SAFE_INTEGER
            || self.publisher_keyring_revision <= 0
            || self.publisher_keyring_revision > I_JSON_MAX_SAFE_INTEGER
            || self.control_keyring_revision <= 0
            || self.control_keyring_revision > I_JSON_MAX_SAFE_INTEGER
            || digests.iter().any(|value| !is_sha256(value))
        {
            bail!("COMPUTE_PLUGIN_WORK_ADMISSION_PLAN_BINDING_INVALID");
        }
        Ok(())
    }
}

impl ComputePluginWorkAdmissionSource {
    pub(in crate::node_agent_compute_plugin_host) fn validate(&self) -> Result<()> {
        self.plan.validate()?;
        self.launch_profile.validate()?;
        let release_json_len = serde_json::to_string(&self.release)?.len();
        let source_json_len = serde_json::to_string(self)?.len();
        if self.schema != SOURCE_SCHEMA
            || !is_sha256(&self.installation_id_digest)
            || self.plugin_id.trim().is_empty()
            || self.slot_ref.trim().is_empty()
            || !is_identifier(&self.install_receipt_id)
            || !is_identifier(&self.promotion_receipt_id)
            || !is_sha256(&self.install_receipt_digest)
            || !is_sha256(&self.promotion_receipt_digest)
            || self.plugin_id != self.release.plugin_id
            || self.plugin_id != self.launch_profile.plugin_id()
            || self.release.plugin_version != self.launch_profile.plugin_version()
            || self.release.target_id != self.launch_profile.target_id()
            || self.release.manifest_digest != self.launch_profile.manifest_digest()
            || release_json_len == 0
            || release_json_len > RELEASE_JSON_MAX_BYTES
            || source_json_len == 0
            || source_json_len > SOURCE_JSON_MAX_BYTES
        {
            bail!("COMPUTE_PLUGIN_WORK_ADMISSION_SOURCE_INVALID");
        }
        Ok(())
    }
}

impl ComputePluginWorkAdmissionGenerationTransition {
    pub(in crate::node_agent_compute_plugin_host) fn validate(&self) -> Result<()> {
        let previous_group = [
            self.previous_work_admission_id.is_some(),
            self.previous_work_admission_receipt_digest.is_some(),
        ];
        if self.install_generation <= 0
            || self.install_generation > I_JSON_MAX_SAFE_INTEGER
            || self.activation_generation <= 0
            || self.activation_generation > I_JSON_MAX_SAFE_INTEGER
            || self.runtime_generation < 0
            || self.runtime_generation > I_JSON_MAX_SAFE_INTEGER
            || self.work_admission_generation_before < 0
            || self.work_admission_generation_before >= I_JSON_MAX_SAFE_INTEGER
            || self.work_admission_generation_after
                != self
                    .work_admission_generation_before
                    .checked_add(1)
                    .unwrap_or(-1)
            || self.work_admission_generation_after > I_JSON_MAX_SAFE_INTEGER
            || previous_group[0] != previous_group[1]
            || (self.work_admission_generation_before == 0) != !previous_group[0]
            || self
                .previous_work_admission_id
                .as_deref()
                .is_some_and(|value| !is_identifier(value))
            || self
                .previous_work_admission_receipt_digest
                .as_deref()
                .is_some_and(|value| !is_sha256(value))
        {
            bail!("COMPUTE_PLUGIN_WORK_ADMISSION_GENERATION_TRANSITION_INVALID");
        }
        Ok(())
    }
}

impl ComputePluginWorkAdmissionQuiescence {
    pub(in crate::node_agent_compute_plugin_host) fn validate(&self) -> Result<()> {
        if self.desired_presence != DESIRED_PRESENCE_PRESENT
            || self.desired_activation != ACTIVATION_ENABLED
            || self.slot_phase != SLOT_INSTALLED
            || self.admission != ADMISSION_ALLOWED
            || self.runtime_phase != RUNTIME_STOPPED
            || self.candidate_slot_present
            || self.runtime_slot_present
            || self.runtime_runner_digest_present
            || self.health_present
            || self.active_attempts != 0
        {
            bail!("COMPUTE_PLUGIN_WORK_ADMISSION_SOURCE_NOT_QUIESCENT");
        }
        Ok(())
    }
}

impl ComputePluginWorkAdmissionAuthorityTransition {
    pub(in crate::node_agent_compute_plugin_host) fn validate(&self) -> Result<()> {
        if self.authority_state_revision_before <= 0
            || self.authority_state_revision_before >= I_JSON_MAX_SAFE_INTEGER
            || self.authority_state_revision_after
                != self
                    .authority_state_revision_before
                    .checked_add(1)
                    .unwrap_or(-1)
            || self.authority_state_revision_after > I_JSON_MAX_SAFE_INTEGER
            || self.inventory_revision_before <= 0
            || self.inventory_revision_before > I_JSON_MAX_SAFE_INTEGER
            || self.inventory_revision_after != self.inventory_revision_before
            || self.inventory_revision_after > I_JSON_MAX_SAFE_INTEGER
            || self.inventory_digest_before != self.inventory_digest_after
            || !is_sha256(&self.inventory_digest_before)
            || !is_sha256(&self.inventory_digest_after)
            || self.authority_epoch_before <= 0
            || self.authority_epoch_before >= I_JSON_MAX_SAFE_INTEGER
            || self.authority_epoch_after
                != self.authority_epoch_before.checked_add(1).unwrap_or(-1)
            || self.authority_epoch_after > I_JSON_MAX_SAFE_INTEGER
            || self.process_owner_epoch <= 0
            || self.process_owner_epoch > I_JSON_MAX_SAFE_INTEGER
            || self.trusted_time_high_water_ms_before <= 0
            || self.trusted_time_high_water_ms_before >= I_JSON_MAX_SAFE_INTEGER
            || self.authority_updated_at_ms_before <= 0
            || self.authority_updated_at_ms_before >= I_JSON_MAX_SAFE_INTEGER
        {
            bail!("COMPUTE_PLUGIN_WORK_ADMISSION_AUTHORITY_TRANSITION_INVALID");
        }
        Ok(())
    }
}

impl ComputePluginWorkAdmissionReceipt {
    pub(in crate::node_agent_compute_plugin_host) fn validate(&self) -> Result<()> {
        self.generations.validate()?;
        self.quiescence.validate()?;
        self.authority.validate()?;
        let digests = [
            self.installation_id_digest.as_str(),
            self.clock_epoch_digest.as_str(),
            self.install_receipt_digest.as_str(),
            self.promotion_receipt_digest.as_str(),
            self.source_digest.as_str(),
        ];
        let expected_id = format!(
            "cpw_{}",
            jcs_sha256_hex(&WorkAdmissionIdBinding {
                schema: ID_BINDING_SCHEMA,
                installation_id_digest: &self.installation_id_digest,
                clock_epoch_digest: &self.clock_epoch_digest,
                plugin_id: &self.plugin_id,
                slot_ref: &self.slot_ref,
                release: &self.release,
                install_receipt_digest: &self.install_receipt_digest,
                promotion_receipt_digest: &self.promotion_receipt_digest,
                source_digest: &self.source_digest,
                generations: &self.generations,
                quiescence: &self.quiescence,
                authority: &self.authority,
                admitted_at_ms: self.admitted_at_ms,
            })?
        );
        let release_json_len = serde_json::to_string(&self.release)?.len();
        let receipt_json_len = serde_json::to_string(self)?.len();
        if self.schema != RECEIPT_SCHEMA
            || !is_identifier(&self.work_admission_id)
            || self.work_admission_id != expected_id
            || !is_identifier(&self.install_receipt_id)
            || !is_identifier(&self.promotion_receipt_id)
            || self.plugin_id.trim().is_empty()
            || self.slot_ref.trim().is_empty()
            || self.plugin_id != self.release.plugin_id
            || digests.iter().any(|value| !is_sha256(value))
            || self.admitted_at_ms <= self.authority.trusted_time_high_water_ms_before
            || self.admitted_at_ms <= self.authority.authority_updated_at_ms_before
            || self.admitted_at_ms > I_JSON_MAX_SAFE_INTEGER
            || release_json_len == 0
            || release_json_len > RELEASE_JSON_MAX_BYTES
            || receipt_json_len == 0
            || receipt_json_len > RECEIPT_JSON_MAX_BYTES
        {
            bail!("COMPUTE_PLUGIN_WORK_ADMISSION_RECEIPT_INVALID");
        }
        Ok(())
    }
}

impl HashedComputePluginWorkAdmissionSource {
    pub(in crate::node_agent_compute_plugin_host) fn validate(&self) -> Result<()> {
        self.source.validate()?;
        if self.schema != HASHED_SOURCE_SCHEMA
            || self.canonicalization != CANONICALIZATION
            || self.digest_algorithm != DIGEST_ALGORITHM
            || !is_sha256(&self.source_digest)
            || jcs_sha256_hex(&self.source)? != self.source_digest
        {
            bail!("COMPUTE_PLUGIN_HASHED_WORK_ADMISSION_SOURCE_INVALID");
        }
        Ok(())
    }
}

impl HashedComputePluginWorkAdmissionReceipt {
    pub(in crate::node_agent_compute_plugin_host) fn validate(&self) -> Result<()> {
        self.receipt.validate()?;
        if self.schema != HASHED_RECEIPT_SCHEMA
            || self.canonicalization != CANONICALIZATION
            || self.digest_algorithm != DIGEST_ALGORITHM
            || !is_sha256(&self.receipt_digest)
            || jcs_sha256_hex(&self.receipt)? != self.receipt_digest
        {
            bail!("COMPUTE_PLUGIN_HASHED_WORK_ADMISSION_RECEIPT_INVALID");
        }
        Ok(())
    }
}

impl ComputePluginWorkAdmissionReceiptPair {
    pub(in crate::node_agent_compute_plugin_host) fn validate(&self) -> Result<()> {
        self.source.validate()?;
        self.receipt.validate()?;
        let source = self.source.source();
        let receipt = self.receipt.receipt();
        if receipt.source_digest() != self.source.source_digest()
            || receipt.installation_id_digest() != source.installation_id_digest()
            || receipt.plugin_id() != source.plugin_id()
            || receipt.slot_ref() != source.slot_ref()
            || receipt.release() != source.release()
            || receipt.install_receipt_id() != source.install_receipt_id()
            || receipt.install_receipt_digest() != source.install_receipt_digest()
            || receipt.promotion_receipt_id() != source.promotion_receipt_id()
            || receipt.promotion_receipt_digest() != source.promotion_receipt_digest()
            || source.plan().application_inventory_revision()
                != receipt.authority().inventory_revision_before()
        {
            bail!("COMPUTE_PLUGIN_WORK_ADMISSION_RECEIPT_PAIR_CHANGED");
        }
        Ok(())
    }
}
