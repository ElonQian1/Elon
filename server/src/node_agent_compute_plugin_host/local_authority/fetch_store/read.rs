use anyhow::{bail, Context, Result};
use chrono::{DateTime, Utc};
use rusqlite::{params, OptionalExtension, Transaction};

use super::{ComputePluginFetchAuthorityFacts, ComputePluginPreparedFetchClaimFacts};
use crate::node_agent_compute_plugin_host::{
    identity::ComputePluginReleaseRef,
    install_plan::ComputePluginPlannedDownload,
    install_plan_admission::{reverify_admitted_artifacts, AdmittedComputePluginInstallPlan},
    install_plan_admission_validation::is_identifier,
    keyring::ComputePluginBootstrapRootKeyResolver,
    manifest_validation::is_sha256,
    plugin_manifest::SignedComputePluginManifest,
    signed_artifact_verification::jcs_sha256_hex,
};

use super::super::{
    keyring_snapshot::{
        load_snapshot_for_state, read_authority_keyring_state, KeyringSnapshotValidation,
    },
    plan_application::read_authority_plan_application_state,
    plan_application_persistence::replay_plan_application,
    ComputePluginFetchProcessFence,
};

struct StoredFetchApplication {
    plan_digest: String,
    application_request_digest: String,
    signed_manifests: Vec<SignedComputePluginManifest>,
}

struct CandidateDownloadRow {
    candidate_token: String,
    candidate_token_digest: String,
    plugin_id: String,
    slot_ref: String,
    candidate_generation: i64,
    release: ComputePluginReleaseRef,
    permission_grant_digest: String,
    owner_plan_id: String,
    owner_plan_digest: String,
    application_inventory_revision: i64,
    candidate_state: String,
    item_index: usize,
    planned_download: ComputePluginPlannedDownload,
    part_relative_path: String,
    committed_offset: i64,
    cursor_generation: i64,
    download_state: String,
    created_at_ms: i64,
    updated_at_ms: i64,
}

pub(super) fn read_fresh_segment_authority(
    transaction: &Transaction<'_>,
    process_fence: &ComputePluginFetchProcessFence,
    trusted_now: DateTime<Utc>,
    roots: &dyn ComputePluginBootstrapRootKeyResolver,
    plan_id: &str,
    plan_digest: &str,
    ordinal: usize,
) -> Result<ComputePluginFetchAuthorityFacts> {
    if !is_identifier(plan_id) || !is_sha256(plan_digest) {
        bail!("COMPUTE_PLUGIN_FETCH_PLAN_IDENTITY_INVALID");
    }
    let ordinal_i64 = i64::try_from(ordinal).context("COMPUTE_PLUGIN_FETCH_ORDINAL_RANGE")?;
    let authority = read_authority_plan_application_state(transaction, &trusted_now)?;
    validate_process_fence(&authority, process_fence, &trusted_now)?;
    let keyring_state = read_authority_keyring_state(transaction)?;
    if keyring_state.state_revision != authority.state_revision
        || keyring_state.authority_epoch != authority.authority_epoch
    {
        bail!("COMPUTE_PLUGIN_FETCH_KEYRING_FENCE_CHANGED");
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
        bail!("COMPUTE_PLUGIN_FETCH_KEYRING_BINDING_CHANGED");
    }

    let stored = read_stored_fetch_application(transaction, plan_id)?;
    if stored.plan_digest != plan_digest {
        bail!("COMPUTE_PLUGIN_FETCH_PLAN_BINDING_CHANGED");
    }
    let replayed = replay_plan_application(
        transaction,
        plan_id,
        plan_digest,
        &stored.application_request_digest,
    )?
    .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_FETCH_PLAN_APPLICATION_MISSING"))?;
    reverify_admitted_artifacts(
        replayed.execution_plan(),
        &stored.signed_manifests,
        trusted_now.clone(),
        &keyring,
        &keyring,
    )?;
    let receipt = replayed.receipt();
    if receipt.keyring_bundle_revision != keyring.bundle_revision()
        || &receipt.publisher_keyring != keyring.publisher_binding()
        || &receipt.control_keyring != keyring.control_binding()
    {
        bail!("COMPUTE_PLUGIN_FETCH_APPLICATION_KEYRING_STALE");
    }

    let row = read_candidate_download(transaction, plan_id, plan_digest, ordinal_i64)?;
    validate_replayed_download(replayed.execution_plan(), &row, ordinal)?;
    let prepared_claim = read_prepared_claim(
        transaction,
        plan_id,
        plan_digest,
        ordinal_i64,
        &row,
        &trusted_now,
    )?;
    Ok(ComputePluginFetchAuthorityFacts {
        inventory: authority.inventory.clone(),
        live: authority.live(),
        trusted_now,
        observed_trusted_time_high_water_ms: authority.trusted_time_high_water_ms,
        applied_plan_id: plan_id.to_string(),
        applied_plan_digest: plan_digest.to_string(),
        application_inventory_revision: receipt.inventory_after_revision,
        execution_inventory_revision: authority.inventory.inventory_revision,
        authority_state_revision: authority.state_revision,
        inventory_digest: authority.inventory_digest,
        authority_epoch: authority.authority_epoch,
        process_owner_epoch: authority.process_owner_epoch,
        candidate_token_digest: row.candidate_token_digest,
        candidate_generation: row.candidate_generation,
        candidate_owner_plan_id: row.owner_plan_id,
        candidate_owner_plan_digest: row.owner_plan_digest,
        candidate_application_inventory_revision: row.application_inventory_revision,
        candidate_state: row.candidate_state,
        candidate_release: row.release,
        candidate_permission_grant_digest: row.permission_grant_digest,
        slot_ref: row.slot_ref,
        planned_download: row.planned_download,
        part_relative_path: row.part_relative_path,
        committed_offset: row.committed_offset,
        download_cursor_generation: row.cursor_generation,
        download_state: row.download_state,
        download_updated_at_ms: row.updated_at_ms,
        prepared_claim,
    })
}

fn validate_process_fence(
    authority: &super::super::plan_application::AuthorityPlanApplicationState,
    process_fence: &ComputePluginFetchProcessFence,
    trusted_now: &DateTime<Utc>,
) -> Result<()> {
    if process_fence.installation_id_digest() != authority.installation_id_digest
        || process_fence.process_owner_epoch() != authority.process_owner_epoch
        || process_fence.process_owner_epoch() <= 0
        || process_fence.acquired_at_ms() < 0
        || process_fence.acquired_at_ms() > trusted_now.timestamp_millis()
        || process_fence.acquired_at_ms() > authority.trusted_time_high_water_ms
    {
        bail!("COMPUTE_PLUGIN_FETCH_PROCESS_FENCE_CHANGED");
    }
    Ok(())
}

fn read_stored_fetch_application(
    transaction: &Transaction<'_>,
    plan_id: &str,
) -> Result<StoredFetchApplication> {
    let row = transaction
        .query_row(
            r#"SELECT plan_digest, application_request_digest, signed_manifests_json
            FROM plan_applications WHERE plan_id = ?1"#,
            [plan_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        )
        .optional()
        .context("COMPUTE_PLUGIN_FETCH_APPLICATION_READ")?
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_FETCH_PLAN_APPLICATION_MISSING"))?;
    let signed_manifests =
        serde_json::from_str(&row.2).context("COMPUTE_PLUGIN_FETCH_APPLICATION_MANIFESTS_JSON")?;
    Ok(StoredFetchApplication {
        plan_digest: row.0,
        application_request_digest: row.1,
        signed_manifests,
    })
}

fn read_candidate_download(
    transaction: &Transaction<'_>,
    plan_id: &str,
    plan_digest: &str,
    ordinal: i64,
) -> Result<CandidateDownloadRow> {
    type Row = (
        String,
        String,
        String,
        i64,
        String,
        String,
        String,
        String,
        i64,
        String,
        i64,
        String,
        String,
        String,
        String,
        String,
        String,
        i64,
        i64,
        i64,
        String,
        i64,
        i64,
    );
    let row: Row = transaction
        .query_row(
            r#"SELECT candidate.candidate_token, candidate.plugin_id, candidate.slot_ref,
                candidate.candidate_generation, candidate.release_json,
                candidate.permission_grant_digest, candidate.owner_plan_id,
                candidate.owner_plan_digest, candidate.application_inventory_revision,
                candidate.state, download.item_index, download.artifact_kind,
                download.artifact_id, download.artifact_digest, download.source_ref,
                download.cache_class, download.part_relative_path, download.size_bytes,
                download.committed_offset, download.cursor_generation, download.state,
                download.created_at_ms, download.updated_at_ms
            FROM planned_downloads AS download
            JOIN candidate_owners AS candidate
              ON candidate.candidate_token = download.candidate_token
             AND candidate.owner_plan_id = download.plan_id
             AND candidate.owner_plan_digest = download.plan_digest
            WHERE download.plan_id = ?1 AND download.plan_digest = ?2
              AND download.ordinal = ?3"#,
            params![plan_id, plan_digest, ordinal],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                    row.get(7)?,
                    row.get(8)?,
                    row.get(9)?,
                    row.get(10)?,
                    row.get(11)?,
                    row.get(12)?,
                    row.get(13)?,
                    row.get(14)?,
                    row.get(15)?,
                    row.get(16)?,
                    row.get(17)?,
                    row.get(18)?,
                    row.get(19)?,
                    row.get(20)?,
                    row.get(21)?,
                    row.get(22)?,
                ))
            },
        )
        .optional()
        .context("COMPUTE_PLUGIN_FETCH_DOWNLOAD_READ")?
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_FETCH_DOWNLOAD_MISSING"))?;
    let release =
        serde_json::from_str(&row.4).context("COMPUTE_PLUGIN_FETCH_CANDIDATE_RELEASE_JSON")?;
    let item_index = usize::try_from(row.10).context("COMPUTE_PLUGIN_FETCH_ITEM_INDEX")?;
    Ok(CandidateDownloadRow {
        candidate_token_digest: jcs_sha256_hex(&row.0)?,
        candidate_token: row.0,
        plugin_id: row.1,
        slot_ref: row.2,
        candidate_generation: row.3,
        release,
        permission_grant_digest: row.5,
        owner_plan_id: row.6,
        owner_plan_digest: row.7,
        application_inventory_revision: row.8,
        candidate_state: row.9,
        item_index,
        planned_download: ComputePluginPlannedDownload {
            artifact_kind: row.11,
            artifact_id: row.12,
            digest: row.13,
            source_ref: row.14,
            cache_class: row.15,
            size_bytes: row.17,
        },
        part_relative_path: row.16,
        committed_offset: row.18,
        cursor_generation: row.19,
        download_state: row.20,
        created_at_ms: row.21,
        updated_at_ms: row.22,
    })
}

fn validate_replayed_download(
    admitted: &AdmittedComputePluginInstallPlan,
    row: &CandidateDownloadRow,
    ordinal: usize,
) -> Result<()> {
    let download = admitted
        .downloads()
        .get(ordinal)
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_FETCH_ORDINAL_NOT_ADMITTED"))?;
    let handle = admitted
        .manifests()
        .iter()
        .find(|manifest| manifest.item_index == row.item_index);
    if download.ordinal != ordinal
        || download.item_index != row.item_index
        || download.release != row.release
        || download.download != row.planned_download
        || row.plugin_id != row.release.plugin_id
        || handle.is_none_or(|manifest| manifest.release != row.release)
        || !is_identifier(&row.candidate_token)
        || !is_sha256(&row.candidate_token_digest)
        || !is_identifier(&row.slot_ref)
        || row.candidate_generation <= 0
        || !is_sha256(&row.permission_grant_digest)
        || row.created_at_ms < 0
        || row.updated_at_ms < row.created_at_ms
        || row.committed_offset < 0
        || row.committed_offset > row.planned_download.size_bytes
        || row.cursor_generation < 0
    {
        bail!("COMPUTE_PLUGIN_FETCH_REPLAYED_DOWNLOAD_CHANGED");
    }
    Ok(())
}

fn read_prepared_claim(
    transaction: &Transaction<'_>,
    plan_id: &str,
    plan_digest: &str,
    ordinal: i64,
    download: &CandidateDownloadRow,
    trusted_now: &DateTime<Utc>,
) -> Result<Option<ComputePluginPreparedFetchClaimFacts>> {
    type Row = (
        String,
        String,
        String,
        i64,
        String,
        i64,
        i64,
        i64,
        i64,
        i64,
        i64,
        i64,
        i64,
    );
    let row: Option<Row> = transaction
        .query_row(
            r#"SELECT claim_id, plan_id, plan_digest, ordinal, candidate_token,
                authority_epoch, process_owner_epoch, cursor_generation,
                redirect_generation, offset_bytes, length_bytes, end_offset_bytes,
                prepared_at_ms
            FROM fetch_claims
            WHERE plan_id = ?1 AND plan_digest = ?2 AND ordinal = ?3
              AND state = 'prepared'"#,
            params![plan_id, plan_digest, ordinal],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                    row.get(7)?,
                    row.get(8)?,
                    row.get(9)?,
                    row.get(10)?,
                    row.get(11)?,
                    row.get(12)?,
                ))
            },
        )
        .optional()
        .context("COMPUTE_PLUGIN_FETCH_PREPARED_CLAIM_READ")?;
    row.map(|row| {
        let claim_ordinal =
            usize::try_from(row.3).context("COMPUTE_PLUGIN_FETCH_CLAIM_ORDINAL_RANGE")?;
        if row.4 != download.candidate_token
            || row.12 > trusted_now.timestamp_millis()
            || !is_identifier(&row.0)
        {
            bail!("COMPUTE_PLUGIN_FETCH_PREPARED_CLAIM_CORRUPT");
        }
        Ok(ComputePluginPreparedFetchClaimFacts {
            claim_id: row.0,
            plan_id: row.1,
            plan_digest: row.2,
            ordinal: claim_ordinal,
            candidate_token_digest: download.candidate_token_digest.clone(),
            part_relative_path: download.part_relative_path.clone(),
            authority_epoch: row.5,
            process_owner_epoch: row.6,
            cursor_generation: row.7,
            redirect_generation: row.8,
            offset_bytes: row.9,
            length_bytes: row.10,
            end_offset_bytes: row.11,
            prepared_at_ms: row.12,
        })
    })
    .transpose()
}
