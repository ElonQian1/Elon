use anyhow::{bail, Context, Result};
use rusqlite::{params, Transaction};

use super::{binding::validate_no_prepared_work, source::read_current_source};
use crate::node_agent_compute_plugin_host::{
    lifecycle::{
        ACTIVATION_ENABLED, ADMISSION_ALLOWED, DESIRED_PRESENCE_PRESENT, RUNTIME_STOPPED,
        SLOT_INSTALLED,
    },
    local_authority::plan_application::read_authority_plan_application_state_at_or_before_observation,
    trusted_time::ComputePluginTrustedTimeObservation,
    work_admission_contract::ComputePluginWorkAdmissionReceiptPair,
};

pub(super) fn validate_current_admission(
    transaction: &Transaction<'_>,
    observation: &ComputePluginTrustedTimeObservation,
    pair: &ComputePluginWorkAdmissionReceiptPair,
) -> Result<()> {
    let source = pair.source().source();
    let plan = source.plan();
    let profile = source.launch_profile();
    let receipt = pair.receipt().receipt();
    let generations = receipt.generations();
    let quiescence = receipt.quiescence();
    let transition = receipt.authority();
    let mut authority = read_authority_plan_application_state_at_or_before_observation(
        transaction,
        observation.trusted_now(),
    )?;
    let updated_at_ms = transaction
        .query_row(
            "SELECT updated_at_ms FROM authority_meta WHERE singleton = 1",
            [],
            |row| row.get::<_, i64>(0),
        )
        .context("COMPUTE_PLUGIN_WORK_ADMISSION_CURRENT_AUTHORITY_TIME")?;

    if authority.installation_id_digest != receipt.installation_id_digest()
        || authority.state_revision != transition.authority_state_revision_after()
        || authority.inventory.inventory_revision != transition.inventory_revision_after()
        || authority.inventory_digest != transition.inventory_digest_after()
        || authority.authority_epoch != transition.authority_epoch_after()
        || authority.process_owner_epoch != transition.process_owner_epoch()
        || authority.trusted_time_high_water_ms != receipt.admitted_at_ms()
        || updated_at_ms != receipt.admitted_at_ms()
        || plan.application_inventory_revision() != transition.inventory_revision_before()
        || plan.policy_revision() != authority.desired_policy_revision
        || !authority.sharing_enabled
        || authority
            .sharing_authorization
            .as_ref()
            .is_none_or(|sharing| {
                sharing.authorization_ref.as_str() != plan.sharing_authorization_ref()
                    || sharing.revision != plan.sharing_authorization_revision()
                    || sharing.digest.as_str() != plan.sharing_authorization_digest()
            })
        || plan.node_profile_digest() != authority.node_profile_digest.as_str()
        || plan.manifest_catalog_revision() != authority.manifest_catalog_revision
        || plan.keyring_bundle_revision() != authority.keyring_bundle_revision
        || plan.publisher_keyring_revision() != authority.publisher_keyring.revision
        || plan.publisher_keyring_digest() != authority.publisher_keyring.digest.as_str()
        || plan.control_keyring_revision() != authority.control_keyring.revision
        || plan.control_keyring_digest() != authority.control_keyring.digest.as_str()
        || profile.target_id() != authority.target_id.as_str()
        || profile.host_api_protocol_id() != authority.host_api_protocol_id.as_str()
        || profile.host_api_revision() != authority.host_api_revision
    {
        bail!("COMPUTE_PLUGIN_WORK_ADMISSION_CURRENT_AUTHORITY_CHANGED");
    }

    let mut records = authority
        .inventory
        .plugins
        .iter()
        .filter(|record| record.plugin_id.as_str() == receipt.plugin_id());
    let record = records
        .next()
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_WORK_ADMISSION_CURRENT_RECORD_MISSING"))?
        .clone();
    if records.next().is_some() {
        bail!("COMPUTE_PLUGIN_WORK_ADMISSION_CURRENT_RECORD_DUPLICATED");
    }
    let active_slot_ref = record.active_slot_ref.as_deref();
    let mut slots = record.slots.iter().filter(|slot| {
        Some(slot.slot_ref.as_str()) == active_slot_ref
            && slot.slot_ref.as_str() == receipt.slot_ref()
            && &slot.release == receipt.release()
            && slot.phase.as_str() == SLOT_INSTALLED
    });
    let slot = slots
        .next()
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_WORK_ADMISSION_CURRENT_SLOT_MISSING"))?;
    if slots.next().is_some()
        || slot.installed_at.is_none()
        || record.last_plan_id.as_deref() != Some(plan.plan_id())
        || record.install_generation != generations.install_generation()
        || record.activation_generation != generations.activation_generation()
        || record.runtime.runtime_generation != generations.runtime_generation()
        || record.desired_presence.as_str() != DESIRED_PRESENCE_PRESENT
        || record.desired_presence.as_str() != quiescence.desired_presence()
        || record.desired_activation.as_str() != ACTIVATION_ENABLED
        || record.desired_activation.as_str() != quiescence.desired_activation()
        || slot.phase.as_str() != quiescence.slot_phase()
        || record.admission.as_str() != ADMISSION_ALLOWED
        || record.admission.as_str() != quiescence.admission()
        || record.candidate_slot_ref.is_some() != quiescence.candidate_slot_present()
        || record.runtime.phase.as_str() != RUNTIME_STOPPED
        || record.runtime.phase.as_str() != quiescence.runtime_phase()
        || record.runtime.slot_ref.is_some() != quiescence.runtime_slot_present()
        || record.runtime.runner_digest.is_some() != quiescence.runtime_runner_digest_present()
        || record.health.is_some() != quiescence.health_present()
        || record.active_attempts != quiescence.active_attempts()
        || record.permission_grant_digest.as_deref() != Some(profile.grant_digest())
    {
        bail!("COMPUTE_PLUGIN_WORK_ADMISSION_CURRENT_INVENTORY_CHANGED");
    }
    validate_current_installed_receipts(transaction, pair)?;
    validate_no_prepared_work(transaction)?;

    authority.state_revision = transition.authority_state_revision_before();
    authority.authority_epoch = transition.authority_epoch_before();
    authority.trusted_time_high_water_ms = transition.trusted_time_high_water_ms_before();
    let current_source = read_current_source(
        transaction,
        &authority,
        transition.authority_updated_at_ms_before(),
        &record,
        receipt.release(),
        receipt.admitted_at_ms(),
    )?;
    let manifest = &current_source.signed_manifest.manifest;
    let runner = manifest
        .package
        .files
        .iter()
        .find(|file| file.relative_path == manifest.entrypoint.relative_path)
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_WORK_ADMISSION_CURRENT_RUNNER_MISSING"))?;
    if current_source.action.as_str() != plan.action()
        || current_source.plan_id.as_str() != plan.plan_id()
        || current_source.plan_digest.as_str() != plan.plan_digest()
        || current_source.signed_plan_envelope_digest.as_str() != plan.signed_plan_envelope_digest()
        || current_source.signed_manifest_set_digest.as_str() != plan.signed_manifest_set_digest()
        || current_source.application_request_digest.as_str() != plan.application_request_digest()
        || current_source.application_receipt_digest.as_str() != plan.application_receipt_digest()
        || current_source.admission_bindings_digest.as_str() != plan.admission_bindings_digest()
        || current_source.application_inventory_revision != plan.application_inventory_revision()
        || current_source.policy_binding_receipt_digest.as_str()
            != plan.policy_binding_receipt_digest()
        || current_source.policy_revocation_receipt_digest.as_str()
            != plan.policy_revocation_receipt_digest()
        || current_source.manifest_catalog_digest.as_str() != plan.manifest_catalog_digest()
        || current_source
            .manifest_catalog_binding_receipt_digest
            .as_str()
            != plan.manifest_catalog_binding_receipt_digest()
        || current_source.signed_manifest_envelope_digest.as_str()
            != profile.signed_manifest_envelope_digest()
        || current_source.grant.grant_ref.as_str() != profile.grant_ref()
        || current_source.grant.grant_digest.as_str() != profile.grant_digest()
        || &current_source.grant.granted_permissions != profile.granted_permissions()
        || &current_source.grant.granted_resources != profile.granted_resources()
        || manifest.plugin_id.as_str() != profile.plugin_id()
        || manifest.plugin_version.as_str() != profile.plugin_version()
        || manifest.publisher_id.as_str() != profile.publisher_id()
        || current_source.signed_manifest.manifest_digest.as_str() != profile.manifest_digest()
        || &manifest.target != profile.target()
        || manifest.task_kinds.as_slice() != profile.task_kinds()
        || manifest.host_api.protocol_id.as_str() != profile.host_api_protocol_id()
        || profile.host_api_revision() < manifest.host_api.minimum_revision
        || profile.host_api_revision() > manifest.host_api.maximum_revision
        || manifest.entrypoint.entrypoint_kind.as_str() != profile.entrypoint_kind()
        || manifest.entrypoint.relative_path.as_str() != profile.entrypoint_relative_path()
        || manifest.entrypoint.arguments.as_slice() != profile.entrypoint_arguments()
        || &manifest.entrypoint.health_check != profile.health_check()
        || runner.relative_path.as_str() != profile.runner_relative_path()
        || runner.digest.as_str() != profile.runner_file_digest()
        || runner.size_bytes != profile.runner_file_size_bytes()
        || runner.executable != profile.runner_file_executable()
    {
        bail!("COMPUTE_PLUGIN_WORK_ADMISSION_CURRENT_SOURCE_CHANGED");
    }
    Ok(())
}

fn validate_current_installed_receipts(
    transaction: &Transaction<'_>,
    pair: &ComputePluginWorkAdmissionReceiptPair,
) -> Result<()> {
    let source = pair.source().source();
    let profile = source.launch_profile();
    let generations = pair.receipt().receipt().generations();
    let release_json = serde_json::to_string(source.release())?;
    let count = transaction
        .query_row(
            r#"SELECT COUNT(*)
            FROM candidate_install_receipts AS installation
            JOIN candidate_promotion_receipts AS promotion
              ON promotion.promotion_id = installation.promotion_id
             AND promotion.install_id = installation.install_id
             AND promotion.candidate_token = installation.candidate_token
             AND promotion.install_receipt_digest = installation.receipt_digest
            JOIN candidate_owners AS owner
              ON owner.candidate_token = installation.candidate_token
            WHERE installation.install_id = ?1 AND installation.receipt_digest = ?2
              AND promotion.promotion_id = ?3 AND promotion.receipt_digest = ?4
              AND installation.installation_id_digest = ?5
              AND promotion.installation_id_digest = ?5
              AND installation.plugin_id = ?6 AND promotion.plugin_id = ?6
              AND installation.slot_ref = ?7 AND promotion.slot_ref = ?7
              AND installation.release_json = ?8 AND promotion.release_json = ?8
              AND installation.install_generation_after = ?9
              AND promotion.install_generation_after = ?9
              AND promotion.activation_generation_after = ?10
              AND installation.signed_manifest_envelope_digest = ?11
              AND promotion.signed_manifest_envelope_digest = ?11
              AND owner.state = 'promoted' AND owner.plugin_id = ?6
              AND owner.slot_ref = ?7 AND owner.release_json = ?8"#,
            params![
                source.install_receipt_id(),
                source.install_receipt_digest(),
                source.promotion_receipt_id(),
                source.promotion_receipt_digest(),
                source.installation_id_digest(),
                source.plugin_id(),
                source.slot_ref(),
                release_json,
                generations.install_generation(),
                generations.activation_generation(),
                profile.signed_manifest_envelope_digest(),
            ],
            |row| row.get::<_, i64>(0),
        )
        .context("COMPUTE_PLUGIN_WORK_ADMISSION_CURRENT_INSTALLED_READ")?;
    if count != 1 {
        bail!("COMPUTE_PLUGIN_WORK_ADMISSION_CURRENT_INSTALLED_CHANGED");
    }
    Ok(())
}
