use anyhow::{bail, Context, Result};
use rusqlite::{named_params, params, Transaction};

use super::{
    binding::read_candidate_cleanup_binding,
    types::{
        ComputePluginCandidateCleanupAuthorizationReceipt,
        HashedComputePluginCandidateCleanupAuthorizationReceipt,
        CANDIDATE_CLEANUP_AUTHORIZATION_RECEIPT_CANONICALIZATION,
        CANDIDATE_CLEANUP_AUTHORIZATION_RECEIPT_DIGEST_ALGORITHM,
        CANDIDATE_CLEANUP_AUTHORIZATION_RECEIPT_SCHEMA,
        HASHED_CANDIDATE_CLEANUP_AUTHORIZATION_RECEIPT_SCHEMA,
    },
    ComputePluginCandidateCleanupAuthoritySession,
};
use crate::node_agent_compute_plugin_host::{
    candidate_cleanup_contract::ValidatedCandidateCleanupAuthorizationPermit,
    lifecycle::SLOT_FAILED,
    local_authority::{
        keyring_snapshot::{advance_trusted_time, read_authority_keyring_state},
        plan_application::{read_authority_plan_application_state, AuthorityPlanApplicationState},
    },
    signed_artifact_verification::jcs_sha256_hex,
};

pub(super) fn persist_candidate_cleanup_authorization(
    transaction: &Transaction<'_>,
    session: &ComputePluginCandidateCleanupAuthoritySession<'_>,
    permit: ValidatedCandidateCleanupAuthorizationPermit<'_, '_>,
) -> Result<HashedComputePluginCandidateCleanupAuthorizationReceipt> {
    let current = read_candidate_cleanup_binding(transaction, session, permit.quarantined())?;
    if &current != permit.facts() {
        bail!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_AUTHORITY_CHANGED");
    }
    let authority = read_authority_plan_application_state(transaction, &session.trusted_now)?;
    let time_state = read_authority_keyring_state(transaction)?;
    if time_state.state_revision != current.authority_state_revision_before()
        || time_state.authority_epoch != current.authority_epoch_before()
        || time_state.trusted_time_high_water_ms
            != Some(current.trusted_time_high_water_ms_before())
        || time_state.clock_status != "trusted"
    {
        bail!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_TIME_CHANGED");
    }
    advance_trusted_time(transaction, &time_state, current.authorized_at_ms())?;
    update_cleanup_authority_meta(transaction, &authority, &current)?;

    let receipt = ComputePluginCandidateCleanupAuthorizationReceipt {
        schema: CANDIDATE_CLEANUP_AUTHORIZATION_RECEIPT_SCHEMA.to_string(),
        cleanup_id: permit.cleanup_id().to_string(),
        candidate_token_digest: current.candidate_token_digest().to_string(),
        quarantine_id: current.quarantine_id().to_string(),
        quarantine_receipt_digest: current.quarantine_receipt_digest().to_string(),
        staging_id: current.staging_id().to_string(),
        staging_run_digest: current.staging_run_digest().to_string(),
        authority_state_revision_before: current.authority_state_revision_before(),
        authority_state_revision_after: current.authority_state_revision_after(),
        inventory_revision: current.inventory_revision(),
        inventory_digest: current.inventory_digest().to_string(),
        authority_epoch_before: current.authority_epoch_before(),
        authority_epoch_after: current.authority_epoch_after(),
        process_owner_epoch: current.process_owner_epoch(),
        trusted_time_high_water_ms_before: current.trusted_time_high_water_ms_before(),
        authorized_at_ms: current.authorized_at_ms(),
        slot_phase_before: SLOT_FAILED.to_string(),
    };
    let hashed = HashedComputePluginCandidateCleanupAuthorizationReceipt {
        schema: HASHED_CANDIDATE_CLEANUP_AUTHORIZATION_RECEIPT_SCHEMA.to_string(),
        receipt_digest: jcs_sha256_hex(&receipt)?,
        receipt,
        canonicalization: CANDIDATE_CLEANUP_AUTHORIZATION_RECEIPT_CANONICALIZATION.to_string(),
        digest_algorithm: CANDIDATE_CLEANUP_AUTHORIZATION_RECEIPT_DIGEST_ALGORITHM.to_string(),
    };
    insert_authorization(transaction, &permit, &hashed)?;
    mark_owner_cleanup_pending(transaction, &permit)?;
    validate_readback(transaction, &permit, &hashed)?;
    Ok(hashed)
}

fn update_cleanup_authority_meta(
    transaction: &Transaction<'_>,
    authority: &AuthorityPlanApplicationState,
    facts: &super::ComputePluginCandidateCleanupAuthorityFacts,
) -> Result<()> {
    let authorization_ref = authority
        .sharing_authorization
        .as_ref()
        .map(|binding| binding.authorization_ref.as_str());
    let authorization_revision = authority
        .sharing_authorization
        .as_ref()
        .map(|binding| binding.revision);
    let authorization_digest = authority
        .sharing_authorization
        .as_ref()
        .map(|binding| binding.digest.as_str());
    let updated = transaction
        .execute(
            r#"UPDATE authority_meta SET
                state_revision = :new_state_revision,
                authority_epoch = :new_authority_epoch
            WHERE singleton = 1
              AND installation_id_digest = :installation_id_digest
              AND state_revision = :old_state_revision
              AND inventory_revision = :inventory_revision
              AND inventory_digest = :inventory_digest
              AND inventory_json = :inventory_json
              AND desired_policy_revision = :desired_policy_revision
              AND sharing_enabled = :sharing_enabled
              AND sharing_authorization_ref IS :authorization_ref
              AND sharing_authorization_revision IS :authorization_revision
              AND sharing_authorization_digest IS :authorization_digest
              AND node_profile_digest = :node_profile_digest
              AND manifest_catalog_revision = :manifest_catalog_revision
              AND target_id = :target_id
              AND host_api_protocol_id = :host_api_protocol_id
              AND host_api_revision = :host_api_revision
              AND active_bundle_revision = :bundle_revision
              AND publisher_keyring_revision = :publisher_revision
              AND publisher_keyring_digest = :publisher_digest
              AND control_keyring_revision = :control_revision
              AND control_keyring_digest = :control_digest
              AND authority_epoch = :old_authority_epoch
              AND process_owner_epoch = :process_owner_epoch
              AND trusted_time_high_water_ms = :authorized_at
              AND clock_status = 'trusted' AND updated_at_ms = :authorized_at"#,
            named_params! {
                ":new_state_revision": facts.authority_state_revision_after(),
                ":new_authority_epoch": facts.authority_epoch_after(),
                ":installation_id_digest": &authority.installation_id_digest,
                ":old_state_revision": authority.state_revision,
                ":inventory_revision": authority.inventory.inventory_revision,
                ":inventory_digest": &authority.inventory_digest,
                ":inventory_json": &authority.inventory_json,
                ":desired_policy_revision": authority.desired_policy_revision,
                ":sharing_enabled": if authority.sharing_enabled { 1_i64 } else { 0_i64 },
                ":authorization_ref": authorization_ref,
                ":authorization_revision": authorization_revision,
                ":authorization_digest": authorization_digest,
                ":node_profile_digest": &authority.node_profile_digest,
                ":manifest_catalog_revision": authority.manifest_catalog_revision,
                ":target_id": &authority.target_id,
                ":host_api_protocol_id": &authority.host_api_protocol_id,
                ":host_api_revision": i64::from(authority.host_api_revision),
                ":bundle_revision": authority.keyring_bundle_revision,
                ":publisher_revision": authority.publisher_keyring.revision,
                ":publisher_digest": &authority.publisher_keyring.digest,
                ":control_revision": authority.control_keyring.revision,
                ":control_digest": &authority.control_keyring.digest,
                ":old_authority_epoch": authority.authority_epoch,
                ":process_owner_epoch": authority.process_owner_epoch,
                ":authorized_at": facts.authorized_at_ms(),
            },
        )
        .context("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_AUTHORITY_UPDATE")?;
    if updated != 1 {
        bail!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_AUTHORITY_CAS");
    }
    Ok(())
}

fn insert_authorization(
    transaction: &Transaction<'_>,
    permit: &ValidatedCandidateCleanupAuthorizationPermit<'_, '_>,
    hashed: &HashedComputePluginCandidateCleanupAuthorizationReceipt,
) -> Result<()> {
    let candidate_token = permit
        .quarantined()
        .staged()
        .recovery_key()
        .candidate_token();
    let receipt = hashed.receipt();
    let receipt_json = serde_json::to_string(receipt)
        .context("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_AUTHORIZATION_SERIALIZE")?;
    transaction
        .execute(
            r#"INSERT INTO candidate_cleanup_authorizations (
                cleanup_id, candidate_token, candidate_token_digest,
                quarantine_id, quarantine_receipt_digest, staging_id, staging_run_digest,
                authority_state_revision_before, authority_state_revision_after,
                inventory_revision, inventory_digest,
                authority_epoch_before, authority_epoch_after, process_owner_epoch,
                trusted_time_high_water_ms_before, authorized_at_ms, slot_phase_before,
                receipt_json, receipt_digest
            ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10,
                ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19
            )"#,
            params![
                receipt.cleanup_id(),
                candidate_token,
                receipt.candidate_token_digest(),
                receipt.quarantine_id(),
                receipt.quarantine_receipt_digest(),
                receipt.staging_id(),
                receipt.staging_run_digest(),
                receipt.authority_state_revision_before(),
                receipt.authority_state_revision_after(),
                receipt.inventory_revision(),
                receipt.inventory_digest(),
                receipt.authority_epoch_before(),
                receipt.authority_epoch_after(),
                receipt.process_owner_epoch(),
                receipt.trusted_time_high_water_ms_before(),
                receipt.authorized_at_ms(),
                receipt.slot_phase_before(),
                receipt_json,
                hashed.receipt_digest(),
            ],
        )
        .context("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_AUTHORIZATION_INSERT")?;
    Ok(())
}

fn mark_owner_cleanup_pending(
    transaction: &Transaction<'_>,
    permit: &ValidatedCandidateCleanupAuthorizationPermit<'_, '_>,
) -> Result<()> {
    let candidate_token = permit
        .quarantined()
        .staged()
        .recovery_key()
        .candidate_token();
    let updated = transaction
        .execute(
            r#"UPDATE candidate_owners SET state = 'cleanup_pending'
               WHERE candidate_token = ?1 AND state = 'owned'
                 AND closed_at_ms IS NULL AND closed_by_plan_id IS NULL
                 AND closed_by_plan_digest IS NULL AND close_reason IS NULL"#,
            params![candidate_token],
        )
        .context("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_OWNER_UPDATE")?;
    if updated != 1 {
        bail!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_OWNER_CAS");
    }
    Ok(())
}

fn validate_readback(
    transaction: &Transaction<'_>,
    permit: &ValidatedCandidateCleanupAuthorizationPermit<'_, '_>,
    hashed: &HashedComputePluginCandidateCleanupAuthorizationReceipt,
) -> Result<()> {
    let receipt = hashed.receipt();
    let candidate_token = permit
        .quarantined()
        .staged()
        .recovery_key()
        .candidate_token();
    let row_count = transaction
        .query_row(
            r#"SELECT COUNT(*) FROM candidate_cleanup_authorizations
               WHERE cleanup_id = ?1 AND candidate_token = ?2
                 AND receipt_digest = ?3 AND authorized_at_ms = ?4"#,
            params![
                receipt.cleanup_id(),
                candidate_token,
                hashed.receipt_digest(),
                receipt.authorized_at_ms(),
            ],
            |row| row.get::<_, i64>(0),
        )
        .context("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_AUTHORIZATION_READBACK")?;
    let owner_count = transaction
        .query_row(
            "SELECT COUNT(*) FROM candidate_owners WHERE candidate_token = ?1 AND state = 'cleanup_pending'",
            params![candidate_token],
            |row| row.get::<_, i64>(0),
        )
        .context("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_OWNER_READBACK")?;
    let meta_count = transaction
        .query_row(
            r#"SELECT COUNT(*) FROM authority_meta WHERE singleton = 1
               AND state_revision = ?1 AND inventory_revision = ?2
               AND inventory_digest = ?3 AND authority_epoch = ?4
               AND process_owner_epoch = ?5 AND trusted_time_high_water_ms = ?6
               AND updated_at_ms = ?6 AND clock_status = 'trusted'"#,
            params![
                receipt.authority_state_revision_after(),
                receipt.inventory_revision(),
                receipt.inventory_digest(),
                receipt.authority_epoch_after(),
                receipt.process_owner_epoch(),
                receipt.authorized_at_ms(),
            ],
            |row| row.get::<_, i64>(0),
        )
        .context("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_META_READBACK")?;
    if row_count != 1 || owner_count != 1 || meta_count != 1 {
        bail!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_AUTHORIZATION_READBACK_CHANGED");
    }
    Ok(())
}
