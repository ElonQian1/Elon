use std::collections::{BTreeMap, HashMap};

use anyhow::{bail, Context, Result};
use chrono::{SecondsFormat, Utc};
use rusqlite::{params, Transaction};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::plan_application::{
    AuthorityPlanApplicationState, ComputePluginCandidateHandle, ComputePluginCandidateReceipt,
    ComputePluginDownloadReceipt, ComputePluginReleasedCandidateReceipt,
};
use crate::node_agent_compute_plugin_host::{
    identity::ComputePluginReleaseRef,
    install_plan::{
        PLAN_ACTION_CANCEL_CANDIDATE, PLAN_ACTION_DISABLE, PLAN_ACTION_INSTALL, PLAN_ACTION_KEEP,
        PLAN_ACTION_REMOVE, PLAN_ACTION_UPGRADE, PLAN_TARGET_ENABLED,
    },
    install_plan_admission::AdmittedComputePluginInstallPlan,
    install_plan_admission_validation::{is_identifier, item_plugin_id},
    lifecycle::{
        can_remove_active_slot, is_valid_slot_transition, local_record_shape_is_valid,
        ComputePluginInventorySnapshot, ComputePluginLocalRecord, ComputePluginRuntimeObservation,
        ComputePluginSlotRecord, ACTIVATION_DISABLED, ACTIVATION_ENABLED, ADMISSION_ALLOWED,
        COMPUTE_PLUGIN_INVENTORY_SCHEMA, DESIRED_PRESENCE_ABSENT, DESIRED_PRESENCE_PRESENT,
        MAX_COMPUTE_PLUGIN_INVENTORY_RECORDS, RUNTIME_STOPPED, SLOT_DOWNLOADING, SLOT_FAILED,
        SLOT_INSTALLED, SLOT_REMOVING, SLOT_STAGED, SLOT_VERIFYING,
    },
    manifest_validation::is_sha256,
    signed_artifact_verification::jcs_sha256_hex,
};

pub(super) const ADMISSION_BINDINGS_SCHEMA: &str = "elon.compute_plugin.plan_admission_bindings.v1";

pub(super) struct ProjectedPlanApplication {
    pub inventory_after: ComputePluginInventorySnapshot,
    pub candidates_to_create: Vec<ProjectedCandidateOwner>,
    pub candidates_to_release: Vec<ProjectedCandidateClosure>,
    pub downloads: Vec<ProjectedDownload>,
    pub admission_bindings: PersistedAdmissionBindings,
}

pub(super) struct ProjectedCandidateOwner {
    pub candidate_token: String,
    pub candidate_token_digest: String,
    pub plugin_id: String,
    pub slot_ref: String,
    pub candidate_generation: i64,
    pub release: ComputePluginReleaseRef,
    pub permission_grant_digest: String,
}

impl ProjectedCandidateOwner {
    pub(super) fn handle(&self) -> ComputePluginCandidateHandle {
        ComputePluginCandidateHandle {
            plugin_id: self.plugin_id.clone(),
            candidate_token: self.candidate_token.clone(),
            candidate_token_digest: self.candidate_token_digest.clone(),
            slot_ref: self.slot_ref.clone(),
            candidate_generation: self.candidate_generation,
        }
    }

    pub(super) fn receipt(&self) -> ComputePluginCandidateReceipt {
        ComputePluginCandidateReceipt {
            plugin_id: self.plugin_id.clone(),
            candidate_token_digest: self.candidate_token_digest.clone(),
            slot_ref: self.slot_ref.clone(),
            candidate_generation: self.candidate_generation,
            release: self.release.clone(),
            permission_grant_digest: self.permission_grant_digest.clone(),
        }
    }
}

pub(super) struct ProjectedCandidateClosure {
    pub candidate_token: String,
    pub candidate_token_digest: String,
    pub plugin_id: String,
    pub slot_ref: String,
    pub candidate_generation: i64,
    pub release: ComputePluginReleaseRef,
    pub owner_plan_id: String,
    pub owner_plan_digest: String,
}

impl ProjectedCandidateClosure {
    pub(super) fn receipt(&self) -> ComputePluginReleasedCandidateReceipt {
        ComputePluginReleasedCandidateReceipt {
            plugin_id: self.plugin_id.clone(),
            candidate_token_digest: self.candidate_token_digest.clone(),
            slot_ref: self.slot_ref.clone(),
            candidate_generation: self.candidate_generation,
            release: self.release.clone(),
        }
    }
}

pub(super) struct ProjectedDownload {
    pub ordinal: i64,
    pub item_index: i64,
    pub candidate_token: String,
    pub candidate_token_digest: String,
    pub artifact_kind: String,
    pub artifact_id: String,
    pub artifact_digest: String,
    pub source_ref: String,
    pub cache_class: String,
    pub part_relative_path: String,
    pub size_bytes: i64,
}

impl ProjectedDownload {
    pub(super) fn receipt(&self) -> ComputePluginDownloadReceipt {
        ComputePluginDownloadReceipt {
            ordinal: self.ordinal,
            item_index: self.item_index,
            candidate_token_digest: self.candidate_token_digest.clone(),
            artifact_kind: self.artifact_kind.clone(),
            artifact_id: self.artifact_id.clone(),
            artifact_digest: self.artifact_digest.clone(),
            source_ref: self.source_ref.clone(),
            cache_class: self.cache_class.clone(),
            size_bytes: self.size_bytes,
            part_relative_path: self.part_relative_path.clone(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(super) struct PersistedAdmissionBindings {
    pub schema: String,
    pub admitted_at_ms: i64,
    pub control_signing_key_fingerprint: String,
    pub manifests: Vec<PersistedManifestAdmissionBinding>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(super) struct PersistedManifestAdmissionBinding {
    pub item_index: i64,
    pub release: ComputePluginReleaseRef,
    pub publisher_id: String,
    pub signing_key_id: String,
    pub signing_key_fingerprint: String,
}

struct ExistingCandidateOwner {
    candidate_token: String,
    candidate_token_digest: String,
    plugin_id: String,
    slot_ref: String,
    candidate_generation: i64,
    release: ComputePluginReleaseRef,
    owner_plan_id: String,
    owner_plan_digest: String,
}

pub(super) fn project_plan_application(
    transaction: &Transaction<'_>,
    authority: &AuthorityPlanApplicationState,
    admitted: &AdmittedComputePluginInstallPlan,
    applied_at_ms: i64,
) -> Result<ProjectedPlanApplication> {
    let plan = admitted.plan();
    let mut owned = read_and_validate_owned_candidates(transaction, authority)?;
    let mut maximum_generations = read_maximum_candidate_generations(transaction)?;
    let mut records = authority
        .inventory
        .plugins
        .iter()
        .cloned()
        .map(|record| (record.plugin_id.clone(), record))
        .collect::<BTreeMap<_, _>>();
    let observed_at = chrono::DateTime::<Utc>::from_timestamp_millis(applied_at_ms)
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_PLAN_APPLICATION_TIME"))?
        .to_rfc3339_opts(SecondsFormat::Millis, true);
    let mut candidates_to_create = Vec::new();
    let mut candidates_to_release = Vec::new();
    let mut candidate_by_item = HashMap::new();

    for (item_index, item) in plan.items.iter().enumerate() {
        let plugin_id = item_plugin_id(item)?.to_string();
        match item.action.as_str() {
            PLAN_ACTION_INSTALL => {
                let target = item.target_release.as_ref().ok_or_else(|| {
                    anyhow::anyhow!("COMPUTE_PLUGIN_PLAN_ADMITTED_INSTALL_TARGET_MISSING")
                })?;
                let grant = item.grant.as_ref().ok_or_else(|| {
                    anyhow::anyhow!("COMPUTE_PLUGIN_PLAN_ADMITTED_INSTALL_GRANT_MISSING")
                })?;
                let candidate = prepare_candidate(
                    &plugin_id,
                    target,
                    &grant.grant_digest,
                    &mut maximum_generations,
                    0,
                )?;
                let record = new_install_record(&candidate, &plan.plan_id, &observed_at);
                if records.insert(plugin_id.clone(), record).is_some() {
                    bail!("COMPUTE_PLUGIN_PLAN_INSTALL_NOT_ABSENT");
                }
                candidate_by_item.insert(item_index, candidates_to_create.len());
                candidates_to_create.push(candidate);
            }
            PLAN_ACTION_UPGRADE => {
                let target = item.target_release.as_ref().ok_or_else(|| {
                    anyhow::anyhow!("COMPUTE_PLUGIN_PLAN_ADMITTED_UPGRADE_TARGET_MISSING")
                })?;
                let grant = item.grant.as_ref().ok_or_else(|| {
                    anyhow::anyhow!("COMPUTE_PLUGIN_PLAN_ADMITTED_UPGRADE_GRANT_MISSING")
                })?;
                let record = records
                    .get_mut(&plugin_id)
                    .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_PLAN_UPGRADE_MISSING"))?;
                let candidate = prepare_candidate(
                    &plugin_id,
                    target,
                    &grant.grant_digest,
                    &mut maximum_generations,
                    record.install_generation,
                )?;
                attach_candidate(record, &candidate, &plan.plan_id, &observed_at)?;
                candidate_by_item.insert(item_index, candidates_to_create.len());
                candidates_to_create.push(candidate);
            }
            PLAN_ACTION_KEEP | PLAN_ACTION_DISABLE => {
                let record = records
                    .get_mut(&plugin_id)
                    .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_PLAN_KEEP_MISSING"))?;
                record.desired_activation = if item.target_activation == PLAN_TARGET_ENABLED {
                    ACTIVATION_ENABLED.to_string()
                } else {
                    ACTIVATION_DISABLED.to_string()
                };
                record.desired_presence = DESIRED_PRESENCE_PRESENT.to_string();
                touch_record(record, &plan.plan_id, &observed_at);
            }
            PLAN_ACTION_REMOVE => {
                let record = records
                    .get_mut(&plugin_id)
                    .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_PLAN_REMOVE_MISSING"))?;
                record.desired_presence = DESIRED_PRESENCE_ABSENT.to_string();
                record.desired_activation = ACTIVATION_DISABLED.to_string();
                if can_remove_active_slot(record) {
                    begin_slot_removal(record, &observed_at)?;
                }
                touch_record(record, &plan.plan_id, &observed_at);
            }
            PLAN_ACTION_CANCEL_CANDIDATE => {
                let existing = owned
                    .remove(&plugin_id)
                    .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_CANDIDATE_OWNER_MISSING"))?;
                let record = records
                    .get_mut(&plugin_id)
                    .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_CANDIDATE_RECORD_MISSING"))?;
                cancel_candidate(record, &existing, &plan.plan_id, &observed_at)?;
                candidates_to_release.push(ProjectedCandidateClosure {
                    candidate_token: existing.candidate_token,
                    candidate_token_digest: existing.candidate_token_digest,
                    plugin_id: existing.plugin_id,
                    slot_ref: existing.slot_ref,
                    candidate_generation: existing.candidate_generation,
                    release: existing.release,
                    owner_plan_id: existing.owner_plan_id,
                    owner_plan_digest: existing.owner_plan_digest,
                });
            }
            _ => bail!("COMPUTE_PLUGIN_PLAN_ACTION_UNSUPPORTED"),
        }
    }
    if records.len() > MAX_COMPUTE_PLUGIN_INVENTORY_RECORDS {
        bail!("COMPUTE_PLUGIN_PROJECTED_INVENTORY_LIMIT");
    }

    let downloads = project_downloads(admitted, &candidates_to_create, &candidate_by_item)?;
    let mut all_owned = owned
        .into_values()
        .map(|owner| (owner.plugin_id.clone(), (owner.slot_ref, owner.release)))
        .collect::<BTreeMap<_, _>>();
    for candidate in &candidates_to_create {
        if all_owned
            .insert(
                candidate.plugin_id.clone(),
                (candidate.slot_ref.clone(), candidate.release.clone()),
            )
            .is_some()
        {
            bail!("COMPUTE_PLUGIN_CANDIDATE_OWNER_DUPLICATE");
        }
    }
    validate_projected_records(&records, &all_owned, plan.sharing_enabled)?;
    let inventory_after = ComputePluginInventorySnapshot {
        schema: COMPUTE_PLUGIN_INVENTORY_SCHEMA.to_string(),
        inventory_revision: authority
            .inventory
            .inventory_revision
            .checked_add(1)
            .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_INVENTORY_REVISION_OVERFLOW"))?,
        desired_policy_revision: plan.desired_policy_revision,
        sharing_enabled: plan.sharing_enabled,
        plugins: records.into_values().collect(),
        observed_at,
    };
    let admission_bindings = PersistedAdmissionBindings {
        schema: ADMISSION_BINDINGS_SCHEMA.to_string(),
        admitted_at_ms: applied_at_ms,
        control_signing_key_fingerprint: admitted.control_signing_key_fingerprint().to_string(),
        manifests: admitted
            .manifests()
            .iter()
            .map(|binding| {
                Ok(PersistedManifestAdmissionBinding {
                    item_index: i64::try_from(binding.item_index)
                        .context("COMPUTE_PLUGIN_PLAN_ITEM_INDEX")?,
                    release: binding.release.clone(),
                    publisher_id: binding.publisher_id.clone(),
                    signing_key_id: binding.signing_key_id.clone(),
                    signing_key_fingerprint: binding.signing_key_fingerprint.clone(),
                })
            })
            .collect::<Result<Vec<_>>>()?,
    };
    Ok(ProjectedPlanApplication {
        inventory_after,
        candidates_to_create,
        candidates_to_release,
        downloads,
        admission_bindings,
    })
}

fn prepare_candidate(
    plugin_id: &str,
    release: &ComputePluginReleaseRef,
    permission_grant_digest: &str,
    maximum_generations: &mut HashMap<String, i64>,
    active_generation: i64,
) -> Result<ProjectedCandidateOwner> {
    let previous = maximum_generations
        .entry(plugin_id.to_string())
        .or_insert(0);
    *previous = (*previous)
        .max(active_generation)
        .checked_add(1)
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_CANDIDATE_GENERATION_OVERFLOW"))?;
    let candidate_token = format!("cpc_{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());
    let candidate_token_digest = jcs_sha256_hex(&candidate_token)?;
    Ok(ProjectedCandidateOwner {
        candidate_token,
        slot_ref: format!("candidate_{candidate_token_digest}"),
        candidate_token_digest,
        plugin_id: plugin_id.to_string(),
        candidate_generation: *previous,
        release: release.clone(),
        permission_grant_digest: permission_grant_digest.to_string(),
    })
}

fn new_install_record(
    candidate: &ProjectedCandidateOwner,
    plan_id: &str,
    observed_at: &str,
) -> ComputePluginLocalRecord {
    ComputePluginLocalRecord {
        plugin_id: candidate.plugin_id.clone(),
        last_plan_id: Some(plan_id.to_string()),
        install_generation: 0,
        activation_generation: 0,
        active_slot_ref: None,
        candidate_slot_ref: Some(candidate.slot_ref.clone()),
        slots: vec![candidate_slot(candidate, observed_at)],
        desired_presence: DESIRED_PRESENCE_PRESENT.to_string(),
        desired_activation: ACTIVATION_ENABLED.to_string(),
        admission: ADMISSION_ALLOWED.to_string(),
        runtime: ComputePluginRuntimeObservation {
            phase: RUNTIME_STOPPED.to_string(),
            runtime_generation: 0,
            slot_ref: None,
            runner_digest: None,
            started_at: None,
            stopped_at: Some(observed_at.to_string()),
        },
        permission_grant_digest: None,
        active_attempts: 0,
        health: None,
        last_error: None,
        state_changed_at: observed_at.to_string(),
    }
}

fn candidate_slot(
    candidate: &ProjectedCandidateOwner,
    observed_at: &str,
) -> ComputePluginSlotRecord {
    ComputePluginSlotRecord {
        slot_ref: candidate.slot_ref.clone(),
        release: candidate.release.clone(),
        phase: SLOT_DOWNLOADING.to_string(),
        phase_changed_at: observed_at.to_string(),
        installed_at: None,
    }
}

fn attach_candidate(
    record: &mut ComputePluginLocalRecord,
    candidate: &ProjectedCandidateOwner,
    plan_id: &str,
    observed_at: &str,
) -> Result<()> {
    if record.candidate_slot_ref.is_some()
        || record
            .slots
            .iter()
            .any(|slot| slot.slot_ref == candidate.slot_ref)
    {
        bail!("COMPUTE_PLUGIN_CANDIDATE_SLOT_BUSY");
    }
    record.candidate_slot_ref = Some(candidate.slot_ref.clone());
    record.slots.push(candidate_slot(candidate, observed_at));
    record
        .slots
        .sort_by(|left, right| left.slot_ref.cmp(&right.slot_ref));
    record.desired_activation = ACTIVATION_ENABLED.to_string();
    record.desired_presence = DESIRED_PRESENCE_PRESENT.to_string();
    touch_record(record, plan_id, observed_at);
    Ok(())
}

fn begin_slot_removal(record: &mut ComputePluginLocalRecord, observed_at: &str) -> Result<()> {
    record.activation_generation = record
        .activation_generation
        .checked_add(1)
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_ACTIVATION_GENERATION_OVERFLOW"))?;
    record.active_slot_ref = None;
    record.health = None;
    record.permission_grant_digest = None;
    record.runtime.slot_ref = None;
    record.runtime.runner_digest = None;
    for slot in &mut record.slots {
        if matches!(slot.phase.as_str(), SLOT_INSTALLED | SLOT_FAILED) {
            if !is_valid_slot_transition(Some(&slot.phase), Some(SLOT_REMOVING)) {
                bail!("COMPUTE_PLUGIN_SLOT_REMOVE_TRANSITION");
            }
            slot.phase = SLOT_REMOVING.to_string();
            slot.phase_changed_at = observed_at.to_string();
        }
    }
    Ok(())
}

fn cancel_candidate(
    record: &mut ComputePluginLocalRecord,
    existing: &ExistingCandidateOwner,
    plan_id: &str,
    observed_at: &str,
) -> Result<()> {
    if record.candidate_slot_ref.as_deref() != Some(existing.slot_ref.as_str()) {
        bail!("COMPUTE_PLUGIN_CANDIDATE_OWNER_SLOT_CHANGED");
    }
    let slot = record
        .slots
        .iter_mut()
        .find(|slot| slot.slot_ref == existing.slot_ref && slot.release == existing.release)
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_CANDIDATE_OWNER_RELEASE_CHANGED"))?;
    if !is_valid_slot_transition(Some(&slot.phase), Some(SLOT_REMOVING)) {
        bail!("COMPUTE_PLUGIN_CANDIDATE_CANCEL_TRANSITION");
    }
    slot.phase = SLOT_REMOVING.to_string();
    slot.phase_changed_at = observed_at.to_string();
    record.candidate_slot_ref = None;
    record.desired_activation = ACTIVATION_DISABLED.to_string();
    record.desired_presence = if record.active_slot_ref.is_some() {
        DESIRED_PRESENCE_PRESENT.to_string()
    } else {
        DESIRED_PRESENCE_ABSENT.to_string()
    };
    touch_record(record, plan_id, observed_at);
    Ok(())
}

fn touch_record(record: &mut ComputePluginLocalRecord, plan_id: &str, observed_at: &str) {
    record.last_plan_id = Some(plan_id.to_string());
    record.state_changed_at = observed_at.to_string();
}

fn project_downloads(
    admitted: &AdmittedComputePluginInstallPlan,
    candidates: &[ProjectedCandidateOwner],
    candidate_by_item: &HashMap<usize, usize>,
) -> Result<Vec<ProjectedDownload>> {
    let mut downloads = Vec::with_capacity(admitted.downloads().len());
    let mut download_counts = vec![0_usize; candidates.len()];
    for (expected_ordinal, admitted_download) in admitted.downloads().iter().enumerate() {
        if admitted_download.ordinal != expected_ordinal {
            bail!("COMPUTE_PLUGIN_DOWNLOAD_ORDINAL_NON_CANONICAL");
        }
        let candidate_index = *candidate_by_item
            .get(&admitted_download.item_index)
            .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_DOWNLOAD_CANDIDATE_MISSING"))?;
        let candidate = &candidates[candidate_index];
        if candidate.release != admitted_download.release {
            bail!("COMPUTE_PLUGIN_DOWNLOAD_RELEASE_CHANGED");
        }
        download_counts[candidate_index] += 1;
        let ordinal =
            i64::try_from(admitted_download.ordinal).context("COMPUTE_PLUGIN_DOWNLOAD_ORDINAL")?;
        downloads.push(ProjectedDownload {
            ordinal,
            item_index: i64::try_from(admitted_download.item_index)
                .context("COMPUTE_PLUGIN_DOWNLOAD_ITEM_INDEX")?,
            candidate_token: candidate.candidate_token.clone(),
            candidate_token_digest: candidate.candidate_token_digest.clone(),
            artifact_kind: admitted_download.download.artifact_kind.clone(),
            artifact_id: admitted_download.download.artifact_id.clone(),
            artifact_digest: admitted_download.download.digest.clone(),
            source_ref: admitted_download.download.source_ref.clone(),
            cache_class: admitted_download.download.cache_class.clone(),
            part_relative_path: format!(
                "compute-plugin/candidates/{}/downloads/{ordinal:04}-{}.part",
                candidate.candidate_token_digest, admitted_download.download.digest
            ),
            size_bytes: admitted_download.download.size_bytes,
        });
    }
    if download_counts.iter().any(|count| *count == 0) {
        bail!("COMPUTE_PLUGIN_CANDIDATE_DOWNLOADS_EMPTY");
    }
    let total = downloads
        .iter()
        .try_fold(0_i64, |sum, download| sum.checked_add(download.size_bytes));
    if total != Some(admitted.plan().total_download_bytes) {
        bail!("COMPUTE_PLUGIN_DOWNLOAD_TOTAL_CHANGED");
    }
    Ok(downloads)
}

fn validate_projected_records(
    records: &BTreeMap<String, ComputePluginLocalRecord>,
    owned: &BTreeMap<String, (String, ComputePluginReleaseRef)>,
    sharing_enabled: bool,
) -> Result<()> {
    for (plugin_id, record) in records {
        if !local_record_shape_is_valid(record) {
            bail!("COMPUTE_PLUGIN_PROJECTED_RECORD_INVALID");
        }
        match (&record.candidate_slot_ref, owned.get(plugin_id)) {
            (None, None) => {}
            (Some(slot_ref), Some((owned_slot, release))) if slot_ref == owned_slot => {
                if !record
                    .slots
                    .iter()
                    .any(|slot| &slot.slot_ref == slot_ref && &slot.release == release)
                {
                    bail!("COMPUTE_PLUGIN_PROJECTED_CANDIDATE_RELEASE_CHANGED");
                }
            }
            _ => bail!("COMPUTE_PLUGIN_PROJECTED_CANDIDATE_OWNER_CHANGED"),
        }
        if !sharing_enabled
            && (record.desired_activation == ACTIVATION_ENABLED
                || record.candidate_slot_ref.is_some())
        {
            bail!("COMPUTE_PLUGIN_SHARING_DISABLED_STATE");
        }
    }
    if owned.len()
        != records
            .values()
            .filter(|record| record.candidate_slot_ref.is_some())
            .count()
    {
        bail!("COMPUTE_PLUGIN_PROJECTED_CANDIDATE_COUNT");
    }
    Ok(())
}

fn read_and_validate_owned_candidates(
    transaction: &Transaction<'_>,
    authority: &AuthorityPlanApplicationState,
) -> Result<BTreeMap<String, ExistingCandidateOwner>> {
    let mut statement = transaction
        .prepare(
            r#"SELECT candidate.candidate_token, candidate.plugin_id, candidate.slot_ref,
                candidate.candidate_generation, candidate.release_json,
                candidate.permission_grant_digest, candidate.owner_plan_id,
                candidate.owner_plan_digest, candidate.application_inventory_revision
            FROM candidate_owners AS candidate
            JOIN plan_application_seals AS seal
              ON seal.plan_id = candidate.owner_plan_id
             AND seal.plan_digest = candidate.owner_plan_digest
            WHERE candidate.state = 'owned'
            ORDER BY candidate.plugin_id"#,
        )
        .context("COMPUTE_PLUGIN_CANDIDATE_OWNERS_PREPARE")?;
    let rows = statement
        .query_map([], |row| {
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
            ))
        })
        .context("COMPUTE_PLUGIN_CANDIDATE_OWNERS_READ")?;
    let mut owners = BTreeMap::new();
    for row in rows {
        let (
            token,
            plugin_id,
            slot_ref,
            generation,
            release_json,
            grant,
            plan_id,
            plan_digest,
            revision,
        ) = row.context("COMPUTE_PLUGIN_CANDIDATE_OWNER_ROW")?;
        let release: ComputePluginReleaseRef =
            serde_json::from_str(&release_json).context("COMPUTE_PLUGIN_CANDIDATE_RELEASE_JSON")?;
        let record = authority
            .inventory
            .plugins
            .iter()
            .find(|record| record.plugin_id == plugin_id)
            .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_CANDIDATE_ORPHAN_OWNER"))?;
        let slot_matches = record.candidate_slot_ref.as_deref() == Some(slot_ref.as_str())
            && record
                .slots
                .iter()
                .any(|slot| slot.slot_ref == slot_ref && slot.release == release);
        if !slot_matches
            || generation <= record.install_generation
            || revision <= 0
            || revision > authority.inventory.inventory_revision
            || !is_sha256(&grant)
            || !is_sha256(&plan_digest)
            || !is_identifier(&token)
            || !is_identifier(&slot_ref)
            || !is_identifier(&plan_id)
        {
            bail!("COMPUTE_PLUGIN_CANDIDATE_OWNER_CORRUPT");
        }
        let (download_count, canceled_downloads) = transaction
            .query_row(
                r#"SELECT COUNT(*), COALESCE(SUM(CASE WHEN state = 'canceled' THEN 1 ELSE 0 END), 0)
                FROM planned_downloads
                WHERE candidate_token = ?1 AND plan_id = ?2 AND plan_digest = ?3"#,
                params![&token, &plan_id, &plan_digest],
                |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
            )
            .context("COMPUTE_PLUGIN_CANDIDATE_DOWNLOADS_READ")?;
        if download_count <= 0 || canceled_downloads != 0 {
            bail!("COMPUTE_PLUGIN_CANDIDATE_DOWNLOADS_MISSING");
        }
        let token_digest = jcs_sha256_hex(&token)?;
        let owner = ExistingCandidateOwner {
            candidate_token: token,
            candidate_token_digest: token_digest,
            plugin_id: plugin_id.clone(),
            slot_ref,
            candidate_generation: generation,
            release,
            owner_plan_id: plan_id,
            owner_plan_digest: plan_digest,
        };
        if owners.insert(plugin_id, owner).is_some() {
            bail!("COMPUTE_PLUGIN_CANDIDATE_OWNER_DUPLICATE");
        }
    }
    let candidate_count = authority
        .inventory
        .plugins
        .iter()
        .filter(|record| record.candidate_slot_ref.is_some())
        .count();
    if owners.len() != candidate_count {
        bail!("COMPUTE_PLUGIN_CANDIDATE_OWNER_INVENTORY_MISMATCH");
    }
    Ok(owners)
}

fn read_maximum_candidate_generations(
    transaction: &Transaction<'_>,
) -> Result<HashMap<String, i64>> {
    let mut statement = transaction
        .prepare(
            "SELECT plugin_id, MAX(candidate_generation) FROM candidate_owners GROUP BY plugin_id",
        )
        .context("COMPUTE_PLUGIN_CANDIDATE_GENERATIONS_PREPARE")?;
    let rows = statement
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })
        .context("COMPUTE_PLUGIN_CANDIDATE_GENERATIONS_READ")?;
    let mut generations = HashMap::new();
    for row in rows {
        let (plugin_id, generation) = row.context("COMPUTE_PLUGIN_CANDIDATE_GENERATION_ROW")?;
        if generation <= 0 || generations.insert(plugin_id, generation).is_some() {
            bail!("COMPUTE_PLUGIN_CANDIDATE_GENERATION_CORRUPT");
        }
    }
    Ok(generations)
}
