use anyhow::{bail, Result};

use super::ComputePluginPostRevalidationWorkAdmissionAuthoritySession;
use crate::node_agent_compute_plugin_host::work_admission_contract::ValidatedInstalledWorkAdmissionStorePermit;

pub(super) fn validate_permit(
    session: &ComputePluginPostRevalidationWorkAdmissionAuthoritySession<'_>,
    permit: &ValidatedInstalledWorkAdmissionStorePermit<'_, '_>,
) -> Result<()> {
    let facts = permit.facts();
    let pair = permit.receipts();
    pair.validate()?;
    let source = pair.source().source();
    let plan = source.plan();
    let profile = source.launch_profile();
    let receipt = pair.receipt().receipt();
    let generations = receipt.generations();
    let quiescence = receipt.quiescence();
    let authority = receipt.authority();
    let installed = permit.revalidated().installed().receipts();
    let install = installed.install();
    let promotion = installed.promotion();

    if receipt.installation_id_digest() != session.installation_id_digest()
        || receipt.clock_epoch_digest() != session.clock_epoch_digest()
        || receipt.admitted_at_ms() != session.trusted_now_ms()
        || source.installation_id_digest() != session.installation_id_digest()
        || source.plugin_id() != facts.plugin_id()
        || source.slot_ref() != facts.slot_ref()
        || source.release() != facts.release()
        || source.install_receipt_id() != facts.install_receipt_id()
        || source.install_receipt_digest() != facts.install_receipt_digest()
        || source.promotion_receipt_id() != facts.promotion_receipt_id()
        || source.promotion_receipt_digest() != facts.promotion_receipt_digest()
        || source.install_receipt_id() != install.receipt().install_receipt_id()
        || source.install_receipt_digest() != install.receipt_digest()
        || source.promotion_receipt_id() != promotion.receipt().promotion_receipt_id()
        || source.promotion_receipt_digest() != promotion.receipt_digest()
        || plan.action() != facts.action()
        || plan.plan_id() != facts.plan_id()
        || plan.plan_digest() != facts.plan_digest()
        || plan.signed_plan_envelope_digest() != facts.signed_plan_envelope_digest()
        || plan.signed_manifest_set_digest() != facts.signed_manifest_set_digest()
        || plan.application_request_digest() != facts.application_request_digest()
        || plan.application_receipt_digest() != facts.application_receipt_digest()
        || plan.admission_bindings_digest() != facts.admission_bindings_digest()
        || plan.application_inventory_revision() != facts.application_inventory_revision()
        || plan.policy_revision() != facts.policy_revision()
        || plan.sharing_authorization_ref() != facts.sharing_authorization_ref()
        || plan.sharing_authorization_revision() != facts.sharing_authorization_revision()
        || plan.sharing_authorization_digest() != facts.sharing_authorization_digest()
        || plan.policy_binding_receipt_digest() != facts.policy_binding_receipt_digest()
        || plan.policy_revocation_receipt_digest() != facts.policy_revocation_receipt_digest()
        || plan.node_profile_digest() != facts.node_profile_digest()
        || plan.manifest_catalog_revision() != facts.manifest_catalog_revision()
        || plan.manifest_catalog_digest() != facts.manifest_catalog_digest()
        || plan.manifest_catalog_binding_receipt_digest()
            != facts.manifest_catalog_binding_receipt_digest()
        || plan.keyring_bundle_revision() != facts.keyring_bundle_revision()
        || plan.publisher_keyring_revision() != facts.publisher_keyring_revision()
        || plan.publisher_keyring_digest() != facts.publisher_keyring_digest()
        || plan.control_keyring_revision() != facts.control_keyring_revision()
        || plan.control_keyring_digest() != facts.control_keyring_digest()
        || profile.plugin_id() != facts.signed_manifest().manifest.plugin_id
        || profile.manifest_digest() != facts.signed_manifest().manifest_digest
        || profile.signed_manifest_envelope_digest() != facts.signed_manifest_envelope_digest()
        || profile.host_api_revision() != facts.selected_host_api_revision()
        || profile.grant_ref() != facts.grant().grant_ref
        || profile.grant_digest() != facts.grant().grant_digest
        || receipt.plugin_id() != facts.plugin_id()
        || receipt.slot_ref() != facts.slot_ref()
        || receipt.release() != facts.release()
        || receipt.install_receipt_id() != facts.install_receipt_id()
        || receipt.install_receipt_digest() != facts.install_receipt_digest()
        || receipt.promotion_receipt_id() != facts.promotion_receipt_id()
        || receipt.promotion_receipt_digest() != facts.promotion_receipt_digest()
        || receipt.source_digest() != pair.source().source_digest()
        || generations.install_generation() != facts.install_generation()
        || generations.activation_generation() != facts.activation_generation()
        || generations.runtime_generation() != facts.runtime_generation()
        || generations.work_admission_generation_before()
            != facts.work_admission_generation_before()
        || generations.work_admission_generation_after() != facts.work_admission_generation_after()
        || generations.previous_work_admission_id() != facts.previous_work_admission_id()
        || generations.previous_work_admission_receipt_digest()
            != facts.previous_work_admission_receipt_digest()
        || quiescence.desired_presence() != facts.desired_presence()
        || quiescence.desired_activation() != facts.desired_activation()
        || quiescence.slot_phase() != facts.slot_phase()
        || quiescence.admission() != facts.admission()
        || quiescence.runtime_phase() != facts.runtime_phase()
        || quiescence.candidate_slot_present() != facts.candidate_slot_present()
        || quiescence.runtime_slot_present() != facts.runtime_slot_present()
        || quiescence.runtime_runner_digest_present() != facts.runtime_runner_digest_present()
        || quiescence.health_present() != facts.health_present()
        || quiescence.active_attempts() != facts.active_attempts()
        || authority.authority_state_revision_before() != facts.authority_state_revision_before()
        || authority.authority_state_revision_after() != facts.authority_state_revision_after()
        || authority.inventory_revision_before() != facts.inventory_revision_before()
        || authority.inventory_revision_after() != facts.inventory_revision_after()
        || authority.inventory_digest_before() != facts.inventory_digest_before()
        || authority.inventory_digest_after() != facts.inventory_digest_after()
        || authority.authority_epoch_before() != facts.authority_epoch_before()
        || authority.authority_epoch_after() != facts.authority_epoch_after()
        || authority.process_owner_epoch() != facts.process_owner_epoch()
        || authority.trusted_time_high_water_ms_before()
            != facts.trusted_time_high_water_ms_before()
        || authority.authority_updated_at_ms_before() != facts.authority_updated_at_ms_before()
        || receipt.admitted_at_ms() != facts.admitted_at_ms()
    {
        bail!("COMPUTE_PLUGIN_WORK_ADMISSION_STORE_PERMIT_CHANGED");
    }
    Ok(())
}
