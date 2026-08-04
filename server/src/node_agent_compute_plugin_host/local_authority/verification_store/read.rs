use super::super::{
    keyring_snapshot::{
        load_snapshot_for_state, read_authority_keyring_state, KeyringSnapshotValidation,
    },
    plan_application::read_authority_plan_application_state,
    plan_application_persistence::replay_plan_application,
    ComputePluginFetchProcessFence,
};
use super::{
    ComputePluginCandidateArtifactAuthorityFacts, ComputePluginCandidateVerificationAuthorityFacts,
};
use crate::node_agent_compute_plugin_host::{
    identity::ComputePluginReleaseRef,
    install_plan::ComputePluginPlannedDownload,
    install_plan_admission::{
        reverify_admitted_artifacts, validate_inventory, validate_live_binding,
        AdmittedComputePluginInstallPlan,
    },
    install_plan_admission_validation::is_identifier,
    keyring::ComputePluginBootstrapRootKeyResolver,
    lifecycle::SLOT_DOWNLOADING,
    manifest_validation::is_sha256,
    plugin_manifest::SignedComputePluginManifest,
    signed_artifact_verification::jcs_sha256_hex,
};
use anyhow::{bail, Context, Result};
use chrono::{DateTime, Utc};
use rusqlite::{params, OptionalExtension, Transaction};
const MAX_CANDIDATE_ARTIFACTS: usize = 4_096;
struct StoredVerificationApplication {
    plan_digest: String,
    application_request_digest: String,
    signed_manifests: Vec<SignedComputePluginManifest>,
}
struct CandidateRow {
    token: String,
    token_digest: String,
    plugin_id: String,
    slot_ref: String,
    generation: i64,
    release: ComputePluginReleaseRef,
    permission_grant_digest: String,
    owner_plan_id: String,
    owner_plan_digest: String,
    application_inventory_revision: i64,
    state: String,
    created_at_ms: i64,
}
pub(super) fn read_fresh_candidate_verification_authority(
    transaction: &Transaction<'_>,
    process_fence: &ComputePluginFetchProcessFence,
    trusted_now: DateTime<Utc>,
    roots: &dyn ComputePluginBootstrapRootKeyResolver,
    plan_id: &str,
    plan_digest: &str,
    candidate_token: &str,
) -> Result<ComputePluginCandidateVerificationAuthorityFacts> {
    if !is_identifier(plan_id) || !is_sha256(plan_digest) || !is_identifier(candidate_token) {
        bail!("COMPUTE_PLUGIN_VERIFICATION_IDENTITY_INVALID");
    }
    let authority = read_authority_plan_application_state(transaction, &trusted_now)?;
    validate_process_fence(&authority, process_fence, &trusted_now)?;
    let keyring_state = read_authority_keyring_state(transaction)?;
    if keyring_state.state_revision != authority.state_revision
        || keyring_state.authority_epoch != authority.authority_epoch
    {
        bail!("COMPUTE_PLUGIN_VERIFICATION_KEYRING_FENCE_CHANGED");
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
        bail!("COMPUTE_PLUGIN_VERIFICATION_KEYRING_BINDING_CHANGED");
    }

    let stored = read_stored_application(transaction, plan_id)?;
    if stored.plan_digest != plan_digest {
        bail!("COMPUTE_PLUGIN_VERIFICATION_PLAN_BINDING_CHANGED");
    }
    let replayed = replay_plan_application(
        transaction,
        plan_id,
        plan_digest,
        &stored.application_request_digest,
    )?
    .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_VERIFICATION_APPLICATION_MISSING"))?;
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
        bail!("COMPUTE_PLUGIN_VERIFICATION_APPLICATION_KEYRING_STALE");
    }
    let live = authority.live();
    validate_live_binding(replayed.execution_plan().plan(), &live)?;
    validate_inventory(&authority.inventory, trusted_now.clone())?;
    if !live.sharing_enabled || !replayed.execution_plan().plan().sharing_enabled {
        bail!("COMPUTE_PLUGIN_VERIFICATION_SHARING_DISABLED");
    }
    let expected_application_revision = replayed
        .execution_plan()
        .plan()
        .expected_inventory_revision
        .checked_add(1)
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_VERIFICATION_REVISION_EXHAUSTED"))?;
    if receipt.inventory_after_revision != expected_application_revision {
        bail!("COMPUTE_PLUGIN_VERIFICATION_APPLICATION_REVISION_CHANGED");
    }

    let candidate = read_candidate(transaction, plan_id, plan_digest, candidate_token)?;
    validate_candidate(
        &candidate,
        replayed.execution_plan(),
        &authority.inventory,
        expected_application_revision,
        trusted_now.timestamp_millis(),
    )?;
    let artifacts = read_candidate_artifacts(
        transaction,
        replayed.execution_plan(),
        &candidate,
        trusted_now.timestamp_millis(),
    )?;
    require_no_prepared_fetch(transaction, &candidate.token)?;
    let next_verification_generation =
        read_next_verification_generation(transaction, &candidate.token)?;
    let artifact_bytes = artifacts.iter().try_fold(0_i64, |total, artifact| {
        total.checked_add(artifact.planned_download.size_bytes)
    });
    let artifact_bytes = artifact_bytes
        .filter(|total| *total > 0)
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_VERIFICATION_ARTIFACT_BYTES_INVALID"))?;
    let mut facts = ComputePluginCandidateVerificationAuthorityFacts {
        inventory: authority.inventory.clone(),
        live,
        trusted_now,
        observed_trusted_time_high_water_ms: authority.trusted_time_high_water_ms,
        installation_id_digest: authority.installation_id_digest,
        applied_plan_id: plan_id.to_string(),
        applied_plan_digest: plan_digest.to_string(),
        application_inventory_revision: receipt.inventory_after_revision,
        execution_inventory_revision: authority.inventory.inventory_revision,
        authority_state_revision: authority.state_revision,
        inventory_digest: authority.inventory_digest,
        authority_epoch: authority.authority_epoch,
        process_owner_epoch: authority.process_owner_epoch,
        candidate_token_digest: candidate.token_digest,
        candidate_generation: candidate.generation,
        candidate_owner_plan_id: candidate.owner_plan_id,
        candidate_owner_plan_digest: candidate.owner_plan_digest,
        candidate_application_inventory_revision: candidate.application_inventory_revision,
        candidate_state: candidate.state,
        candidate_plugin_id: candidate.plugin_id,
        candidate_slot_ref: candidate.slot_ref,
        candidate_release: candidate.release,
        candidate_permission_grant_digest: candidate.permission_grant_digest,
        next_verification_generation,
        artifact_bytes,
        expected_artifact_set_digest: String::new(),
        artifacts,
    };
    facts.expected_artifact_set_digest = facts.recompute_expected_artifact_set_digest()?;
    Ok(facts)
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
        bail!("COMPUTE_PLUGIN_VERIFICATION_PROCESS_FENCE_CHANGED");
    }
    Ok(())
}

fn read_stored_application(
    transaction: &Transaction<'_>,
    plan_id: &str,
) -> Result<StoredVerificationApplication> {
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
        .context("COMPUTE_PLUGIN_VERIFICATION_APPLICATION_READ")?
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_VERIFICATION_APPLICATION_MISSING"))?;
    Ok(StoredVerificationApplication {
        plan_digest: row.0,
        application_request_digest: row.1,
        signed_manifests: serde_json::from_str(&row.2)
            .context("COMPUTE_PLUGIN_VERIFICATION_MANIFESTS_JSON")?,
    })
}

fn read_candidate(
    transaction: &Transaction<'_>,
    plan_id: &str,
    plan_digest: &str,
    candidate_token: &str,
) -> Result<CandidateRow> {
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
        Option<i64>,
    );
    let row: Row = transaction
        .query_row(
            r#"SELECT candidate_token, plugin_id, slot_ref, candidate_generation,
                release_json, permission_grant_digest, owner_plan_id, owner_plan_digest,
                application_inventory_revision, state, created_at_ms, closed_at_ms
            FROM candidate_owners
            WHERE candidate_token = ?1 AND owner_plan_id = ?2 AND owner_plan_digest = ?3"#,
            params![candidate_token, plan_id, plan_digest],
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
                ))
            },
        )
        .optional()
        .context("COMPUTE_PLUGIN_VERIFICATION_CANDIDATE_READ")?
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_VERIFICATION_CANDIDATE_MISSING"))?;
    if row.11.is_some() {
        bail!("COMPUTE_PLUGIN_VERIFICATION_CANDIDATE_CLOSED");
    }
    Ok(CandidateRow {
        token_digest: jcs_sha256_hex(&row.0)?,
        token: row.0,
        plugin_id: row.1,
        slot_ref: row.2,
        generation: row.3,
        release: serde_json::from_str(&row.4)
            .context("COMPUTE_PLUGIN_VERIFICATION_RELEASE_JSON")?,
        permission_grant_digest: row.5,
        owner_plan_id: row.6,
        owner_plan_digest: row.7,
        application_inventory_revision: row.8,
        state: row.9,
        created_at_ms: row.10,
    })
}

fn validate_candidate(
    candidate: &CandidateRow,
    admitted: &AdmittedComputePluginInstallPlan,
    inventory: &crate::node_agent_compute_plugin_host::lifecycle::ComputePluginInventorySnapshot,
    expected_application_revision: i64,
    trusted_now_ms: i64,
) -> Result<()> {
    let manifest_matches = admitted
        .manifests()
        .iter()
        .filter(|manifest| manifest.release == candidate.release)
        .count();
    let inventory_matches = inventory.plugins.iter().filter(|record| {
        record.plugin_id == candidate.plugin_id
            && record.candidate_slot_ref.as_deref() == Some(candidate.slot_ref.as_str())
            && record.slots.iter().any(|slot| {
                slot.slot_ref == candidate.slot_ref
                    && slot.release == candidate.release
                    && slot.phase == SLOT_DOWNLOADING
            })
            && candidate.generation > record.install_generation
    });
    if !is_identifier(&candidate.token)
        || !is_sha256(&candidate.token_digest)
        || !is_identifier(&candidate.plugin_id)
        || !is_identifier(&candidate.slot_ref)
        || candidate.slot_ref != format!("candidate_{}", candidate.token_digest)
        || candidate.release.plugin_id != candidate.plugin_id
        || candidate.generation <= 0
        || !is_sha256(&candidate.permission_grant_digest)
        || candidate.application_inventory_revision != expected_application_revision
        || candidate.state != "owned"
        || candidate.created_at_ms < 0
        || candidate.created_at_ms > trusted_now_ms
        || manifest_matches != 1
        || inventory_matches.count() != 1
    {
        bail!("COMPUTE_PLUGIN_VERIFICATION_CANDIDATE_CHANGED");
    }
    Ok(())
}

fn read_candidate_artifacts(
    transaction: &Transaction<'_>,
    admitted: &AdmittedComputePluginInstallPlan,
    candidate: &CandidateRow,
    trusted_now_ms: i64,
) -> Result<Vec<ComputePluginCandidateArtifactAuthorityFacts>> {
    let mut statement = transaction
        .prepare(
            r#"SELECT ordinal, item_index, artifact_kind, artifact_id, artifact_digest,
                source_ref, cache_class, part_relative_path, size_bytes, committed_offset,
                cursor_generation, state, created_at_ms, updated_at_ms
            FROM planned_downloads
            WHERE candidate_token = ?1 AND plan_id = ?2 AND plan_digest = ?3
            ORDER BY ordinal"#,
        )
        .context("COMPUTE_PLUGIN_VERIFICATION_ARTIFACT_PREPARE")?;
    let rows = statement
        .query_map(
            params![
                &candidate.token,
                &candidate.owner_plan_id,
                &candidate.owner_plan_digest,
            ],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, String>(7)?,
                    row.get::<_, i64>(8)?,
                    row.get::<_, i64>(9)?,
                    row.get::<_, i64>(10)?,
                    row.get::<_, String>(11)?,
                    row.get::<_, i64>(12)?,
                    row.get::<_, i64>(13)?,
                ))
            },
        )
        .context("COMPUTE_PLUGIN_VERIFICATION_ARTIFACT_QUERY")?;
    let mut artifacts = Vec::new();
    for row in rows {
        if artifacts.len() == MAX_CANDIDATE_ARTIFACTS {
            bail!("COMPUTE_PLUGIN_VERIFICATION_ARTIFACT_LIMIT");
        }
        let row = row.context("COMPUTE_PLUGIN_VERIFICATION_ARTIFACT_ROW")?;
        let ordinal = usize::try_from(row.0).context("COMPUTE_PLUGIN_VERIFICATION_ORDINAL")?;
        let item_index =
            usize::try_from(row.1).context("COMPUTE_PLUGIN_VERIFICATION_ITEM_INDEX")?;
        artifacts.push(ComputePluginCandidateArtifactAuthorityFacts {
            ordinal,
            item_index,
            planned_download: ComputePluginPlannedDownload {
                artifact_kind: row.2,
                artifact_id: row.3,
                digest: row.4,
                source_ref: row.5,
                cache_class: row.6,
                size_bytes: row.8,
            },
            part_relative_path: row.7,
            committed_offset: row.9,
            cursor_generation: row.10,
            download_state: row.11,
            created_at_ms: row.12,
            updated_at_ms: row.13,
        });
    }
    validate_artifacts(admitted, candidate, trusted_now_ms, &artifacts)?;
    Ok(artifacts)
}

fn validate_artifacts(
    admitted: &AdmittedComputePluginInstallPlan,
    candidate: &CandidateRow,
    trusted_now_ms: i64,
    artifacts: &[ComputePluginCandidateArtifactAuthorityFacts],
) -> Result<()> {
    let expected = admitted
        .downloads()
        .iter()
        .filter(|download| download.release == candidate.release)
        .collect::<Vec<_>>();
    if artifacts.is_empty()
        || artifacts.len() > MAX_CANDIDATE_ARTIFACTS
        || artifacts.len() != expected.len()
    {
        bail!("COMPUTE_PLUGIN_VERIFICATION_ARTIFACT_CLOSURE_CHANGED");
    }
    let item_index = artifacts[0].item_index;
    let item = admitted
        .plan()
        .items
        .get(item_index)
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_VERIFICATION_ITEM_MISSING"))?;
    if item.target_release.as_ref() != Some(&candidate.release)
        || item.grant.as_ref().map(|grant| grant.grant_digest.as_str())
            != Some(candidate.permission_grant_digest.as_str())
        || artifacts
            .iter()
            .any(|artifact| artifact.item_index != item_index)
    {
        bail!("COMPUTE_PLUGIN_VERIFICATION_ITEM_BINDING_CHANGED");
    }
    for (position, (artifact, expected)) in artifacts.iter().zip(expected).enumerate() {
        let expected_path = format!(
            "compute-plugin/candidates/{}/downloads/{:04}-{}.part",
            candidate.token_digest, artifact.ordinal, artifact.planned_download.digest
        );
        if artifact.ordinal != expected.ordinal
            || artifact.item_index != expected.item_index
            || artifact.planned_download != expected.download
            || artifact.part_relative_path != expected_path
            || artifact.planned_download.size_bytes <= 0
            || !is_sha256(&artifact.planned_download.digest)
            || artifact.committed_offset != artifact.planned_download.size_bytes
            || artifact.cursor_generation <= 0
            || artifact.download_state != "complete"
            || artifact.created_at_ms < candidate.created_at_ms
            || artifact.updated_at_ms < artifact.created_at_ms
            || artifact.updated_at_ms > trusted_now_ms
            || (position > 0 && artifacts[position - 1].ordinal >= artifact.ordinal)
        {
            bail!("COMPUTE_PLUGIN_VERIFICATION_ARTIFACT_CHANGED");
        }
    }
    Ok(())
}

fn require_no_prepared_fetch(transaction: &Transaction<'_>, candidate_token: &str) -> Result<()> {
    let count = transaction
        .query_row(
            r#"SELECT COUNT(*) FROM fetch_claims
            WHERE candidate_token = ?1 AND state = 'prepared'"#,
            [candidate_token],
            |row| row.get::<_, i64>(0),
        )
        .context("COMPUTE_PLUGIN_VERIFICATION_PREPARED_FETCH_CHECK")?;
    if count != 0 {
        bail!("COMPUTE_PLUGIN_VERIFICATION_FETCH_STILL_PREPARED");
    }
    Ok(())
}

fn read_next_verification_generation(
    transaction: &Transaction<'_>,
    candidate_token: &str,
) -> Result<i64> {
    let (last_generation, open_count) = transaction
        .query_row(
            r#"SELECT COALESCE(MAX(verification_generation), 0),
                COALESCE(SUM(CASE WHEN state IN ('prepared', 'verified') THEN 1 ELSE 0 END), 0)
            FROM candidate_verification_runs WHERE candidate_token = ?1"#,
            [candidate_token],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
        )
        .context("COMPUTE_PLUGIN_VERIFICATION_GENERATION_READ")?;
    if open_count != 0 || last_generation < 0 {
        bail!("COMPUTE_PLUGIN_VERIFICATION_ALREADY_OPEN");
    }
    last_generation
        .checked_add(1)
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_VERIFICATION_GENERATION_EXHAUSTED"))
}
