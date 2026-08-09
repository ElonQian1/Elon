use anyhow::{bail, Context, Result};
use chrono::{DateTime, SecondsFormat, Utc};
use rusqlite::Transaction;

use super::{
    plan_application::{
        ComputePluginCandidateHandle, ComputePluginCandidateReceipt, ComputePluginDownloadReceipt,
        ComputePluginReleasedCandidateReceipt,
    },
    plan_application_projection::PersistedAdmissionBindings,
};
use crate::node_agent_compute_plugin_host::{
    identity::ComputePluginReleaseRef,
    install_plan::{
        SignedComputePluginInstallPlan, PLAN_ACTION_CANCEL_CANDIDATE, PLAN_ACTION_INSTALL,
        PLAN_ACTION_UPGRADE,
    },
    install_plan_admission::{
        AdmittedComputePluginDownload, AdmittedComputePluginInstallPlan,
        AdmittedComputePluginManifestBinding,
    },
    install_plan_admission_validation::is_identifier,
    install_plan_reauthorization::validate_replayed_inventory_intent,
    lifecycle::{ComputePluginInventorySnapshot, SLOT_DOWNLOADING, SLOT_REMOVING},
    manifest_validation::is_sha256,
    signed_artifact_verification::jcs_sha256_hex,
};

pub(super) fn validate_replayed_children(
    signed_plan: &SignedComputePluginInstallPlan,
    inventory_after: &ComputePluginInventorySnapshot,
    new_candidates: &[ComputePluginCandidateReceipt],
    released_candidates: &[ComputePluginReleasedCandidateReceipt],
    downloads: &[ComputePluginDownloadReceipt],
) -> Result<()> {
    let expected_new_count = signed_plan
        .plan
        .items
        .iter()
        .filter(|item| {
            matches!(
                item.action.as_str(),
                PLAN_ACTION_INSTALL | PLAN_ACTION_UPGRADE
            )
        })
        .count();
    let expected_released_count = signed_plan
        .plan
        .items
        .iter()
        .filter(|item| item.action == PLAN_ACTION_CANCEL_CANDIDATE)
        .count();
    if new_candidates.len() != expected_new_count
        || released_candidates.len() != expected_released_count
        || new_candidates
            .windows(2)
            .any(|pair| pair[0].plugin_id >= pair[1].plugin_id)
        || released_candidates
            .windows(2)
            .any(|pair| pair[0].plugin_id >= pair[1].plugin_id)
    {
        bail!("COMPUTE_PLUGIN_PLAN_REPLAY_CANDIDATE_SET");
    }
    for candidate in new_candidates {
        let item = signed_plan
            .plan
            .items
            .iter()
            .find(|item| {
                matches!(
                    item.action.as_str(),
                    PLAN_ACTION_INSTALL | PLAN_ACTION_UPGRADE
                ) && item
                    .target_release
                    .as_ref()
                    .is_some_and(|release| release.plugin_id == candidate.plugin_id)
            })
            .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_PLAN_REPLAY_CANDIDATE_ITEM"))?;
        if item.target_release.as_ref() != Some(&candidate.release)
            || item.grant.as_ref().map(|grant| grant.grant_digest.as_str())
                != Some(candidate.permission_grant_digest.as_str())
            || !candidate_identity_is_valid(
                &candidate.candidate_token_digest,
                &candidate.slot_ref,
                candidate.candidate_generation,
            )
        {
            bail!("COMPUTE_PLUGIN_PLAN_REPLAY_CANDIDATE_BINDING");
        }
        let record = inventory_after
            .plugins
            .iter()
            .find(|record| record.plugin_id == candidate.plugin_id)
            .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_PLAN_REPLAY_CANDIDATE_INVENTORY"))?;
        if record.last_plan_id.as_deref() != Some(signed_plan.plan.plan_id.as_str())
            || record.candidate_slot_ref.as_deref() != Some(candidate.slot_ref.as_str())
            || !record.slots.iter().any(|slot| {
                slot.slot_ref == candidate.slot_ref
                    && slot.release == candidate.release
                    && slot.phase == SLOT_DOWNLOADING
            })
        {
            bail!("COMPUTE_PLUGIN_PLAN_REPLAY_CANDIDATE_INVENTORY_BINDING");
        }
    }
    for candidate in released_candidates {
        let item = signed_plan
            .plan
            .items
            .iter()
            .find(|item| {
                item.action == PLAN_ACTION_CANCEL_CANDIDATE
                    && item
                        .expected_candidate_release
                        .as_ref()
                        .is_some_and(|release| release.plugin_id == candidate.plugin_id)
            })
            .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_PLAN_REPLAY_RELEASED_ITEM"))?;
        if item.expected_candidate_release.as_ref() != Some(&candidate.release)
            || !candidate_identity_is_valid(
                &candidate.candidate_token_digest,
                &candidate.slot_ref,
                candidate.candidate_generation,
            )
        {
            bail!("COMPUTE_PLUGIN_PLAN_REPLAY_RELEASED_BINDING");
        }
        let record = inventory_after
            .plugins
            .iter()
            .find(|record| record.plugin_id == candidate.plugin_id)
            .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_PLAN_REPLAY_RELEASED_INVENTORY"))?;
        if record.last_plan_id.as_deref() != Some(signed_plan.plan.plan_id.as_str())
            || record.candidate_slot_ref.is_some()
            || !record.slots.iter().any(|slot| {
                slot.slot_ref == candidate.slot_ref
                    && slot.release == candidate.release
                    && slot.phase == SLOT_REMOVING
            })
        {
            bail!("COMPUTE_PLUGIN_PLAN_REPLAY_RELEASED_INVENTORY_BINDING");
        }
    }
    validate_replayed_inventory_intent(signed_plan, inventory_after)?;
    let expected_download_count = signed_plan
        .plan
        .items
        .iter()
        .try_fold(0_usize, |count, item| {
            count.checked_add(item.downloads.len())
        })
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_PLAN_REPLAY_DOWNLOAD_COUNT_OVERFLOW"))?;
    if downloads.len() != expected_download_count {
        bail!("COMPUTE_PLUGIN_PLAN_REPLAY_DOWNLOAD_COUNT");
    }
    let mut ordinal = 0_usize;
    for (item_index, item) in signed_plan.plan.items.iter().enumerate() {
        for expected in &item.downloads {
            let actual = &downloads[ordinal];
            let target = item
                .target_release
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_PLAN_REPLAY_DOWNLOAD_TARGET"))?;
            let candidate = new_candidates
                .iter()
                .find(|candidate| candidate.plugin_id == target.plugin_id)
                .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_PLAN_REPLAY_DOWNLOAD_CANDIDATE"))?;
            let ordinal_i64 =
                i64::try_from(ordinal).context("COMPUTE_PLUGIN_PLAN_REPLAY_DOWNLOAD_ORDINAL")?;
            let item_index_i64 = i64::try_from(item_index)
                .context("COMPUTE_PLUGIN_PLAN_REPLAY_DOWNLOAD_ITEM_INDEX")?;
            if actual.ordinal != ordinal_i64
                || actual.item_index != item_index_i64
                || actual.candidate_token_digest != candidate.candidate_token_digest
                || actual.artifact_kind != expected.artifact_kind
                || actual.artifact_id != expected.artifact_id
                || actual.artifact_digest != expected.digest
                || actual.source_ref != expected.source_ref
                || actual.cache_class != expected.cache_class
                || actual.size_bytes != expected.size_bytes
                || actual.part_relative_path
                    != format!(
                        "compute-plugin/candidates/{}/downloads/{ordinal_i64:04}-{}.part",
                        candidate.candidate_token_digest, expected.digest
                    )
            {
                bail!("COMPUTE_PLUGIN_PLAN_REPLAY_DOWNLOAD_BINDING");
            }
            ordinal += 1;
        }
    }
    Ok(())
}

fn candidate_identity_is_valid(token_digest: &str, slot_ref: &str, generation: i64) -> bool {
    is_sha256(token_digest)
        && is_identifier(slot_ref)
        && slot_ref == format!("candidate_{token_digest}")
        && generation > 0
}

pub(super) fn restore_execution_plan(
    signed_plan: SignedComputePluginInstallPlan,
    admission: PersistedAdmissionBindings,
) -> Result<AdmittedComputePluginInstallPlan> {
    let admitted_at = DateTime::<Utc>::from_timestamp_millis(admission.admitted_at_ms)
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_PLAN_REPLAY_ADMITTED_AT"))?
        .to_rfc3339_opts(SecondsFormat::Millis, true);
    let manifests = admission
        .manifests
        .into_iter()
        .map(|binding| {
            Ok(AdmittedComputePluginManifestBinding {
                item_index: usize::try_from(binding.item_index)
                    .context("COMPUTE_PLUGIN_PLAN_REPLAY_RESTORE_ITEM_INDEX")?,
                release: binding.release,
                publisher_id: binding.publisher_id,
                signing_key_id: binding.signing_key_id,
                signing_key_fingerprint: binding.signing_key_fingerprint,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let mut downloads = Vec::new();
    for (item_index, item) in signed_plan.plan.items.iter().enumerate() {
        let Some(release) = item.target_release.as_ref() else {
            continue;
        };
        for download in &item.downloads {
            downloads.push(AdmittedComputePluginDownload {
                ordinal: downloads.len(),
                item_index,
                release: release.clone(),
                download: download.clone(),
            });
        }
    }
    Ok(AdmittedComputePluginInstallPlan::from_sealed_application(
        signed_plan,
        admitted_at,
        manifests,
        downloads,
        admission.control_signing_key_fingerprint,
    ))
}

pub(super) fn read_created_candidates(
    transaction: &Transaction<'_>,
    plan_id: &str,
    plan_digest: &str,
    application_inventory_revision: i64,
    applied_at_ms: i64,
) -> Result<(
    Vec<ComputePluginCandidateReceipt>,
    Vec<ComputePluginCandidateHandle>,
)> {
    let mut statement = transaction
        .prepare(
            r#"SELECT candidate_token, plugin_id, slot_ref, candidate_generation,
            release_json, permission_grant_digest, state, owner_plan_digest,
            application_inventory_revision, created_at_ms
        FROM candidate_owners WHERE owner_plan_id = ?1 ORDER BY plugin_id"#,
        )
        .context("COMPUTE_PLUGIN_PLAN_REPLAY_CANDIDATES_PREPARE")?;
    let rows = statement
        .query_map([plan_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, String>(6)?,
                row.get::<_, String>(7)?,
                row.get::<_, i64>(8)?,
                row.get::<_, i64>(9)?,
            ))
        })
        .context("COMPUTE_PLUGIN_PLAN_REPLAY_CANDIDATES_READ")?;
    let mut receipts = Vec::new();
    let mut handles = Vec::new();
    for row in rows {
        let (
            token,
            plugin_id,
            slot_ref,
            generation,
            release_json,
            grant,
            state,
            owner_plan_digest,
            owner_inventory_revision,
            created_at_ms,
        ) = row.context("COMPUTE_PLUGIN_PLAN_REPLAY_CANDIDATE_ROW")?;
        let token_digest = jcs_sha256_hex(&token)?;
        let release: ComputePluginReleaseRef = serde_json::from_str(&release_json)
            .context("COMPUTE_PLUGIN_PLAN_REPLAY_CANDIDATE_RELEASE")?;
        if !matches!(
            state.as_str(),
            "owned" | "cleanup_pending" | "released" | "promoted" | "cleaned"
        ) || !is_identifier(&token)
            || !candidate_identity_is_valid(&token_digest, &slot_ref, generation)
            || !is_sha256(&grant)
            || owner_plan_digest != plan_digest
            || owner_inventory_revision != application_inventory_revision
            || created_at_ms != applied_at_ms
        {
            bail!("COMPUTE_PLUGIN_PLAN_REPLAY_CANDIDATE_IDENTITY");
        }
        receipts.push(ComputePluginCandidateReceipt {
            plugin_id: plugin_id.clone(),
            candidate_token_digest: token_digest.clone(),
            slot_ref: slot_ref.clone(),
            candidate_generation: generation,
            release,
            permission_grant_digest: grant,
        });
        if state == "owned" {
            handles.push(ComputePluginCandidateHandle {
                plugin_id,
                candidate_token: token,
                candidate_token_digest: token_digest,
                slot_ref,
                candidate_generation: generation,
            });
        }
    }
    Ok((receipts, handles))
}

pub(super) fn read_released_candidates(
    transaction: &Transaction<'_>,
    plan_id: &str,
    plan_digest: &str,
    applied_at_ms: i64,
) -> Result<Vec<ComputePluginReleasedCandidateReceipt>> {
    let mut statement = transaction
        .prepare(
            r#"SELECT candidate_token, plugin_id, slot_ref, candidate_generation, release_json,
            closed_by_plan_digest, closed_at_ms, close_reason
        FROM candidate_owners
        WHERE state = 'released' AND closed_by_plan_id = ?1 ORDER BY plugin_id"#,
        )
        .context("COMPUTE_PLUGIN_PLAN_REPLAY_RELEASED_PREPARE")?;
    let rows = statement
        .query_map([plan_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, i64>(6)?,
                row.get::<_, String>(7)?,
            ))
        })
        .context("COMPUTE_PLUGIN_PLAN_REPLAY_RELEASED_READ")?;
    rows.map(|row| {
        let (
            token,
            plugin_id,
            slot_ref,
            generation,
            release_json,
            closed_by_plan_digest,
            closed_at_ms,
            close_reason,
        ) = row.context("COMPUTE_PLUGIN_PLAN_REPLAY_RELEASED_ROW")?;
        let token_digest = jcs_sha256_hex(&token)?;
        let release: ComputePluginReleaseRef = serde_json::from_str(&release_json)
            .context("COMPUTE_PLUGIN_PLAN_REPLAY_RELEASED_RELEASE")?;
        if !is_identifier(&token)
            || !candidate_identity_is_valid(&token_digest, &slot_ref, generation)
            || closed_by_plan_digest != plan_digest
            || closed_at_ms != applied_at_ms
            || close_reason != "cancel_candidate"
        {
            bail!("COMPUTE_PLUGIN_PLAN_REPLAY_RELEASED_IDENTITY");
        }
        Ok(ComputePluginReleasedCandidateReceipt {
            plugin_id,
            candidate_token_digest: token_digest,
            slot_ref,
            candidate_generation: generation,
            release,
        })
    })
    .collect()
}

pub(super) fn read_downloads(
    transaction: &Transaction<'_>,
    plan_id: &str,
    plan_digest: &str,
    applied_at_ms: i64,
) -> Result<Vec<ComputePluginDownloadReceipt>> {
    let mut statement = transaction
        .prepare(
            r#"SELECT download.ordinal, download.item_index, candidate.candidate_token,
            download.artifact_kind, download.artifact_id, download.artifact_digest,
            download.source_ref, download.cache_class, download.size_bytes,
            download.part_relative_path, download.plan_digest, download.created_at_ms
        FROM planned_downloads AS download
        JOIN candidate_owners AS candidate
          ON candidate.candidate_token = download.candidate_token
        WHERE download.plan_id = ?1 ORDER BY download.ordinal"#,
        )
        .context("COMPUTE_PLUGIN_PLAN_REPLAY_DOWNLOADS_PREPARE")?;
    let rows = statement
        .query_map([plan_id], |row| {
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
                row.get::<_, String>(9)?,
                row.get::<_, String>(10)?,
                row.get::<_, i64>(11)?,
            ))
        })
        .context("COMPUTE_PLUGIN_PLAN_REPLAY_DOWNLOADS_READ")?;
    let downloads = rows
        .map(|row| {
            let (
                ordinal,
                item_index,
                token,
                artifact_kind,
                artifact_id,
                digest,
                source_ref,
                cache_class,
                size_bytes,
                path,
                stored_plan_digest,
                created_at_ms,
            ) = row.context("COMPUTE_PLUGIN_PLAN_REPLAY_DOWNLOAD_ROW")?;
            if stored_plan_digest != plan_digest || created_at_ms != applied_at_ms {
                bail!("COMPUTE_PLUGIN_PLAN_REPLAY_DOWNLOAD_IDENTITY");
            }
            Ok(ComputePluginDownloadReceipt {
                ordinal,
                item_index,
                candidate_token_digest: jcs_sha256_hex(&token)?,
                artifact_kind,
                artifact_id,
                artifact_digest: digest,
                source_ref,
                cache_class,
                size_bytes,
                part_relative_path: path,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    if downloads
        .iter()
        .enumerate()
        .any(|(index, download)| i64::try_from(index).ok() != Some(download.ordinal))
    {
        bail!("COMPUTE_PLUGIN_PLAN_REPLAY_DOWNLOAD_ORDINAL");
    }
    Ok(downloads)
}
