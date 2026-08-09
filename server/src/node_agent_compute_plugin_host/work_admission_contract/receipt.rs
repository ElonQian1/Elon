use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::node_agent_compute_plugin_host::{
    candidate_promotion_contract::DurableInstalledPluginSlot,
    identity::ComputePluginReleaseRef,
    local_authority::{
        ComputePluginInstalledWorkAdmissionAuthorityFacts,
        ComputePluginPostRevalidationWorkAdmissionAuthoritySession,
    },
    signed_artifact_verification::jcs_sha256_hex,
};

use super::{
    ComputePluginWorkAdmissionLaunchProfile, CANONICALIZATION, DIGEST_ALGORITHM,
    HASHED_RECEIPT_SCHEMA, HASHED_SOURCE_SCHEMA, RECEIPT_SCHEMA, SOURCE_SCHEMA,
};

mod getters;
mod validation;

const PLAN_BINDING_SCHEMA: &str = "elon.compute_plugin.work_admission_plan_binding.v1";
const ID_BINDING_SCHEMA: &str = "elon.compute_plugin.work_admission_id_binding.v1";

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(in crate::node_agent_compute_plugin_host) struct ComputePluginWorkAdmissionPlanBinding {
    schema: String,
    action: String,
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
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(in crate::node_agent_compute_plugin_host) struct ComputePluginWorkAdmissionSource {
    schema: String,
    installation_id_digest: String,
    plugin_id: String,
    slot_ref: String,
    release: ComputePluginReleaseRef,
    install_receipt_id: String,
    install_receipt_digest: String,
    promotion_receipt_id: String,
    promotion_receipt_digest: String,
    plan: ComputePluginWorkAdmissionPlanBinding,
    launch_profile: ComputePluginWorkAdmissionLaunchProfile,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(in crate::node_agent_compute_plugin_host) struct HashedComputePluginWorkAdmissionSource {
    schema: String,
    source: ComputePluginWorkAdmissionSource,
    canonicalization: String,
    digest_algorithm: String,
    source_digest: String,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(in crate::node_agent_compute_plugin_host) struct ComputePluginWorkAdmissionGenerationTransition
{
    install_generation: i64,
    activation_generation: i64,
    runtime_generation: i64,
    work_admission_generation_before: i64,
    work_admission_generation_after: i64,
    previous_work_admission_id: Option<String>,
    previous_work_admission_receipt_digest: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(in crate::node_agent_compute_plugin_host) struct ComputePluginWorkAdmissionQuiescence {
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
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(in crate::node_agent_compute_plugin_host) struct ComputePluginWorkAdmissionAuthorityTransition {
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
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(in crate::node_agent_compute_plugin_host) struct ComputePluginWorkAdmissionReceipt {
    schema: String,
    work_admission_id: String,
    installation_id_digest: String,
    clock_epoch_digest: String,
    plugin_id: String,
    slot_ref: String,
    release: ComputePluginReleaseRef,
    install_receipt_id: String,
    install_receipt_digest: String,
    promotion_receipt_id: String,
    promotion_receipt_digest: String,
    source_digest: String,
    generations: ComputePluginWorkAdmissionGenerationTransition,
    quiescence: ComputePluginWorkAdmissionQuiescence,
    authority: ComputePluginWorkAdmissionAuthorityTransition,
    admitted_at_ms: i64,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(in crate::node_agent_compute_plugin_host) struct HashedComputePluginWorkAdmissionReceipt {
    schema: String,
    receipt: ComputePluginWorkAdmissionReceipt,
    canonicalization: String,
    digest_algorithm: String,
    receipt_digest: String,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(in crate::node_agent_compute_plugin_host) struct ComputePluginWorkAdmissionReceiptPair {
    source: HashedComputePluginWorkAdmissionSource,
    receipt: HashedComputePluginWorkAdmissionReceipt,
}

#[derive(Serialize)]
struct WorkAdmissionIdBinding<'a> {
    schema: &'static str,
    installation_id_digest: &'a str,
    clock_epoch_digest: &'a str,
    plugin_id: &'a str,
    slot_ref: &'a str,
    release: &'a ComputePluginReleaseRef,
    install_receipt_digest: &'a str,
    promotion_receipt_digest: &'a str,
    source_digest: &'a str,
    generations: &'a ComputePluginWorkAdmissionGenerationTransition,
    quiescence: &'a ComputePluginWorkAdmissionQuiescence,
    authority: &'a ComputePluginWorkAdmissionAuthorityTransition,
    admitted_at_ms: i64,
}

pub(super) fn build_work_admission_receipts(
    session: &ComputePluginPostRevalidationWorkAdmissionAuthoritySession<'_>,
    facts: &ComputePluginInstalledWorkAdmissionAuthorityFacts,
    installed: &DurableInstalledPluginSlot<'_>,
    profile: ComputePluginWorkAdmissionLaunchProfile,
) -> Result<ComputePluginWorkAdmissionReceiptPair> {
    let install = installed.receipts().install();
    let promotion = installed.receipts().promotion();
    let plan = ComputePluginWorkAdmissionPlanBinding::from_facts(facts);
    let source = ComputePluginWorkAdmissionSource {
        schema: SOURCE_SCHEMA.to_string(),
        installation_id_digest: session.installation_id_digest().to_string(),
        plugin_id: facts.plugin_id().to_string(),
        slot_ref: facts.slot_ref().to_string(),
        release: facts.release().clone(),
        install_receipt_id: install.receipt().install_receipt_id().to_string(),
        install_receipt_digest: install.receipt_digest().to_string(),
        promotion_receipt_id: promotion.receipt().promotion_receipt_id().to_string(),
        promotion_receipt_digest: promotion.receipt_digest().to_string(),
        plan,
        launch_profile: profile,
    };
    let source = HashedComputePluginWorkAdmissionSource::from_authority_source(source)?;
    let generations = ComputePluginWorkAdmissionGenerationTransition::from_facts(facts);
    let quiescence = ComputePluginWorkAdmissionQuiescence::from_facts(facts);
    let authority = ComputePluginWorkAdmissionAuthorityTransition::from_facts(facts);
    let id_binding = WorkAdmissionIdBinding {
        schema: ID_BINDING_SCHEMA,
        installation_id_digest: session.installation_id_digest(),
        clock_epoch_digest: session.clock_epoch_digest(),
        plugin_id: facts.plugin_id(),
        slot_ref: facts.slot_ref(),
        release: facts.release(),
        install_receipt_digest: install.receipt_digest(),
        promotion_receipt_digest: promotion.receipt_digest(),
        source_digest: source.source_digest(),
        generations: &generations,
        quiescence: &quiescence,
        authority: &authority,
        admitted_at_ms: facts.admitted_at_ms(),
    };
    let receipt = ComputePluginWorkAdmissionReceipt {
        schema: RECEIPT_SCHEMA.to_string(),
        work_admission_id: format!("cpw_{}", jcs_sha256_hex(&id_binding)?),
        installation_id_digest: session.installation_id_digest().to_string(),
        clock_epoch_digest: session.clock_epoch_digest().to_string(),
        plugin_id: facts.plugin_id().to_string(),
        slot_ref: facts.slot_ref().to_string(),
        release: facts.release().clone(),
        install_receipt_id: install.receipt().install_receipt_id().to_string(),
        install_receipt_digest: install.receipt_digest().to_string(),
        promotion_receipt_id: promotion.receipt().promotion_receipt_id().to_string(),
        promotion_receipt_digest: promotion.receipt_digest().to_string(),
        source_digest: source.source_digest().to_string(),
        generations,
        quiescence,
        authority,
        admitted_at_ms: facts.admitted_at_ms(),
    };
    let receipt = HashedComputePluginWorkAdmissionReceipt::from_store_receipt(receipt)?;
    ComputePluginWorkAdmissionReceiptPair::new(source, receipt)
}

impl ComputePluginWorkAdmissionPlanBinding {
    fn from_facts(facts: &ComputePluginInstalledWorkAdmissionAuthorityFacts) -> Self {
        Self {
            schema: PLAN_BINDING_SCHEMA.to_string(),
            action: facts.action().to_string(),
            plan_id: facts.plan_id().to_string(),
            plan_digest: facts.plan_digest().to_string(),
            signed_plan_envelope_digest: facts.signed_plan_envelope_digest().to_string(),
            signed_manifest_set_digest: facts.signed_manifest_set_digest().to_string(),
            application_request_digest: facts.application_request_digest().to_string(),
            application_receipt_digest: facts.application_receipt_digest().to_string(),
            admission_bindings_digest: facts.admission_bindings_digest().to_string(),
            application_inventory_revision: facts.application_inventory_revision(),
            policy_revision: facts.policy_revision(),
            sharing_authorization_ref: facts.sharing_authorization_ref().to_string(),
            sharing_authorization_revision: facts.sharing_authorization_revision(),
            sharing_authorization_digest: facts.sharing_authorization_digest().to_string(),
            policy_binding_receipt_digest: facts.policy_binding_receipt_digest().to_string(),
            policy_revocation_receipt_digest: facts.policy_revocation_receipt_digest().to_string(),
            node_profile_digest: facts.node_profile_digest().to_string(),
            manifest_catalog_revision: facts.manifest_catalog_revision(),
            manifest_catalog_digest: facts.manifest_catalog_digest().to_string(),
            manifest_catalog_binding_receipt_digest: facts
                .manifest_catalog_binding_receipt_digest()
                .to_string(),
            keyring_bundle_revision: facts.keyring_bundle_revision(),
            publisher_keyring_revision: facts.publisher_keyring_revision(),
            publisher_keyring_digest: facts.publisher_keyring_digest().to_string(),
            control_keyring_revision: facts.control_keyring_revision(),
            control_keyring_digest: facts.control_keyring_digest().to_string(),
        }
    }
}

impl ComputePluginWorkAdmissionGenerationTransition {
    fn from_facts(facts: &ComputePluginInstalledWorkAdmissionAuthorityFacts) -> Self {
        Self {
            install_generation: facts.install_generation(),
            activation_generation: facts.activation_generation(),
            runtime_generation: facts.runtime_generation(),
            work_admission_generation_before: facts.work_admission_generation_before(),
            work_admission_generation_after: facts.work_admission_generation_after(),
            previous_work_admission_id: facts.previous_work_admission_id().map(str::to_string),
            previous_work_admission_receipt_digest: facts
                .previous_work_admission_receipt_digest()
                .map(str::to_string),
        }
    }
}

impl ComputePluginWorkAdmissionQuiescence {
    fn from_facts(facts: &ComputePluginInstalledWorkAdmissionAuthorityFacts) -> Self {
        Self {
            desired_presence: facts.desired_presence().to_string(),
            desired_activation: facts.desired_activation().to_string(),
            slot_phase: facts.slot_phase().to_string(),
            admission: facts.admission().to_string(),
            runtime_phase: facts.runtime_phase().to_string(),
            candidate_slot_present: facts.candidate_slot_present(),
            runtime_slot_present: facts.runtime_slot_present(),
            runtime_runner_digest_present: facts.runtime_runner_digest_present(),
            health_present: facts.health_present(),
            active_attempts: facts.active_attempts(),
        }
    }
}

impl ComputePluginWorkAdmissionAuthorityTransition {
    fn from_facts(facts: &ComputePluginInstalledWorkAdmissionAuthorityFacts) -> Self {
        Self {
            authority_state_revision_before: facts.authority_state_revision_before(),
            authority_state_revision_after: facts.authority_state_revision_after(),
            inventory_revision_before: facts.inventory_revision_before(),
            inventory_revision_after: facts.inventory_revision_after(),
            inventory_digest_before: facts.inventory_digest_before().to_string(),
            inventory_digest_after: facts.inventory_digest_after().to_string(),
            authority_epoch_before: facts.authority_epoch_before(),
            authority_epoch_after: facts.authority_epoch_after(),
            process_owner_epoch: facts.process_owner_epoch(),
            trusted_time_high_water_ms_before: facts.trusted_time_high_water_ms_before(),
            authority_updated_at_ms_before: facts.authority_updated_at_ms_before(),
        }
    }
}

impl HashedComputePluginWorkAdmissionSource {
    fn from_authority_source(source: ComputePluginWorkAdmissionSource) -> Result<Self> {
        source.validate()?;
        let source_digest = jcs_sha256_hex(&source)?;
        Self::from_store_readback(source, source_digest)
    }

    pub(in crate::node_agent_compute_plugin_host) fn from_store_readback(
        source: ComputePluginWorkAdmissionSource,
        source_digest: String,
    ) -> Result<Self> {
        let value = Self {
            schema: HASHED_SOURCE_SCHEMA.to_string(),
            source,
            canonicalization: CANONICALIZATION.to_string(),
            digest_algorithm: DIGEST_ALGORITHM.to_string(),
            source_digest,
        };
        value.validate()?;
        Ok(value)
    }
}

impl HashedComputePluginWorkAdmissionReceipt {
    fn from_store_receipt(receipt: ComputePluginWorkAdmissionReceipt) -> Result<Self> {
        receipt.validate()?;
        let receipt_digest = jcs_sha256_hex(&receipt)?;
        Self::from_store_readback(receipt, receipt_digest)
    }

    pub(in crate::node_agent_compute_plugin_host) fn from_store_readback(
        receipt: ComputePluginWorkAdmissionReceipt,
        receipt_digest: String,
    ) -> Result<Self> {
        let value = Self {
            schema: HASHED_RECEIPT_SCHEMA.to_string(),
            receipt,
            canonicalization: CANONICALIZATION.to_string(),
            digest_algorithm: DIGEST_ALGORITHM.to_string(),
            receipt_digest,
        };
        value.validate()?;
        Ok(value)
    }
}

impl ComputePluginWorkAdmissionReceiptPair {
    pub(in crate::node_agent_compute_plugin_host) fn new(
        source: HashedComputePluginWorkAdmissionSource,
        receipt: HashedComputePluginWorkAdmissionReceipt,
    ) -> Result<Self> {
        let value = Self { source, receipt };
        value.validate()?;
        Ok(value)
    }
}
