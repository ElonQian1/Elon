use anyhow::{bail, Context, Result};
use rusqlite::{params, Transaction};
use serde::Serialize;

use super::{
    ComputePluginCandidateArtifactAuthorityFacts, ComputePluginCandidateVerificationAuthorityFacts,
};
use crate::node_agent_compute_plugin_host::{
    identity::ComputePluginReleaseRef, install_plan::ComputePluginPlannedDownload,
    install_plan_admission_validation::is_identifier, manifest_validation::is_sha256,
    signed_artifact_verification::jcs_sha256_hex,
};

const DURABLE_CLOSURE_SCHEMA: &str = "elon.compute_plugin.candidate_durable_closure.v1";
const EXPECTED_ARTIFACT_SET_SCHEMA: &str = "elon.compute_plugin.expected_artifact_set.v1";
const MAX_CANDIDATE_ARTIFACTS: usize = 4_096;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct CandidateClosureSnapshot {
    pub durable_closure_digest: String,
    pub expected_artifact_set_digest: String,
    pub artifact_count: usize,
    pub artifact_bytes: i64,
}

#[derive(Serialize)]
struct DurableCandidateClosure {
    schema: &'static str,
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
    candidate_created_at_ms: i64,
    artifacts: Vec<DurableArtifact>,
}

#[derive(Serialize)]
struct DurableArtifact {
    ordinal: usize,
    item_index: usize,
    planned_download: ComputePluginPlannedDownload,
    part_relative_path: String,
    committed_offset: i64,
    cursor_generation: i64,
    download_state: String,
    created_at_ms: i64,
    updated_at_ms: i64,
}

#[derive(Serialize)]
struct ExpectedArtifactSet<'closure> {
    schema: &'static str,
    release: &'closure ComputePluginReleaseRef,
    digest_algorithm: &'static str,
    artifact_count: usize,
    artifact_bytes: i64,
    artifacts: Vec<ExpectedArtifact<'closure>>,
}

#[derive(Serialize)]
struct ExpectedArtifact<'closure> {
    sequence: usize,
    artifact_kind: &'closure str,
    artifact_id: &'closure str,
    digest: &'closure str,
    size_bytes: i64,
}

pub(super) fn candidate_closure_snapshot_from_facts(
    facts: &ComputePluginCandidateVerificationAuthorityFacts,
) -> Result<CandidateClosureSnapshot> {
    snapshot(DurableCandidateClosure {
        schema: DURABLE_CLOSURE_SCHEMA,
        candidate_token_digest: facts.candidate_token_digest.clone(),
        plugin_id: facts.candidate_plugin_id.clone(),
        slot_ref: facts.candidate_slot_ref.clone(),
        candidate_generation: facts.candidate_generation,
        release: facts.candidate_release.clone(),
        permission_grant_digest: facts.candidate_permission_grant_digest.clone(),
        owner_plan_id: facts.candidate_owner_plan_id.clone(),
        owner_plan_digest: facts.candidate_owner_plan_digest.clone(),
        application_inventory_revision: facts.candidate_application_inventory_revision,
        candidate_state: facts.candidate_state.clone(),
        candidate_created_at_ms: facts.candidate_created_at_ms,
        artifacts: facts.artifacts.iter().map(artifact_from_facts).collect(),
    })
}

pub(super) fn read_candidate_closure_snapshot(
    transaction: &Transaction<'_>,
    candidate_token: &str,
) -> Result<CandidateClosureSnapshot> {
    if !is_identifier(candidate_token) {
        bail!("COMPUTE_PLUGIN_VERIFICATION_CLOSURE_TOKEN_INVALID");
    }
    type CandidateRow = (
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
    );
    let candidate: CandidateRow = transaction
        .query_row(
            r#"SELECT plugin_id, slot_ref, candidate_generation, release_json,
                permission_grant_digest, owner_plan_id, owner_plan_digest,
                application_inventory_revision, state, created_at_ms
            FROM candidate_owners WHERE candidate_token = ?1"#,
            [candidate_token],
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
                ))
            },
        )
        .context("COMPUTE_PLUGIN_VERIFICATION_CLOSURE_CANDIDATE_READ")?;
    let release: ComputePluginReleaseRef = serde_json::from_str(&candidate.3)
        .context("COMPUTE_PLUGIN_VERIFICATION_CLOSURE_RELEASE_PARSE")?;
    if serde_json::to_string(&release)? != candidate.3 {
        bail!("COMPUTE_PLUGIN_VERIFICATION_CLOSURE_RELEASE_NONCANONICAL");
    }

    let mut statement = transaction
        .prepare(
            r#"SELECT ordinal, item_index, artifact_kind, artifact_id, artifact_digest,
                size_bytes, source_ref, cache_class, part_relative_path, committed_offset,
                cursor_generation, state, created_at_ms, updated_at_ms
            FROM planned_downloads WHERE candidate_token = ?1 ORDER BY ordinal"#,
        )
        .context("COMPUTE_PLUGIN_VERIFICATION_CLOSURE_ARTIFACT_PREPARE")?;
    let rows = statement
        .query_map(params![candidate_token], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, i64>(5)?,
                row.get::<_, String>(6)?,
                row.get::<_, String>(7)?,
                row.get::<_, String>(8)?,
                row.get::<_, i64>(9)?,
                row.get::<_, i64>(10)?,
                row.get::<_, String>(11)?,
                row.get::<_, i64>(12)?,
                row.get::<_, i64>(13)?,
            ))
        })
        .context("COMPUTE_PLUGIN_VERIFICATION_CLOSURE_ARTIFACT_QUERY")?;
    let mut artifacts = Vec::new();
    for row in rows {
        let row = row.context("COMPUTE_PLUGIN_VERIFICATION_CLOSURE_ARTIFACT_ROW")?;
        artifacts.push(DurableArtifact {
            ordinal: usize::try_from(row.0)
                .context("COMPUTE_PLUGIN_VERIFICATION_CLOSURE_ORDINAL")?,
            item_index: usize::try_from(row.1)
                .context("COMPUTE_PLUGIN_VERIFICATION_CLOSURE_ITEM_INDEX")?,
            planned_download: ComputePluginPlannedDownload {
                artifact_kind: row.2,
                artifact_id: row.3,
                digest: row.4,
                size_bytes: row.5,
                source_ref: row.6,
                cache_class: row.7,
            },
            part_relative_path: row.8,
            committed_offset: row.9,
            cursor_generation: row.10,
            download_state: row.11,
            created_at_ms: row.12,
            updated_at_ms: row.13,
        });
    }
    snapshot(DurableCandidateClosure {
        schema: DURABLE_CLOSURE_SCHEMA,
        candidate_token_digest: jcs_sha256_hex(&candidate_token)?,
        plugin_id: candidate.0,
        slot_ref: candidate.1,
        candidate_generation: candidate.2,
        release,
        permission_grant_digest: candidate.4,
        owner_plan_id: candidate.5,
        owner_plan_digest: candidate.6,
        application_inventory_revision: candidate.7,
        candidate_state: candidate.8,
        candidate_created_at_ms: candidate.9,
        artifacts,
    })
}

fn artifact_from_facts(artifact: &ComputePluginCandidateArtifactAuthorityFacts) -> DurableArtifact {
    DurableArtifact {
        ordinal: artifact.ordinal,
        item_index: artifact.item_index,
        planned_download: artifact.planned_download.clone(),
        part_relative_path: artifact.part_relative_path.clone(),
        committed_offset: artifact.committed_offset,
        cursor_generation: artifact.cursor_generation,
        download_state: artifact.download_state.clone(),
        created_at_ms: artifact.created_at_ms,
        updated_at_ms: artifact.updated_at_ms,
    }
}

fn snapshot(closure: DurableCandidateClosure) -> Result<CandidateClosureSnapshot> {
    if closure.artifacts.is_empty()
        || closure.artifacts.len() > MAX_CANDIDATE_ARTIFACTS
        || !is_sha256(&closure.candidate_token_digest)
        || !is_sha256(&closure.permission_grant_digest)
        || !is_sha256(&closure.owner_plan_digest)
        || closure.candidate_generation <= 0
        || closure.application_inventory_revision <= 0
        || closure.candidate_created_at_ms < 0
        || closure.candidate_state != "owned"
    {
        bail!("COMPUTE_PLUGIN_VERIFICATION_CLOSURE_INVALID");
    }
    let mut artifact_bytes = 0_i64;
    for (position, artifact) in closure.artifacts.iter().enumerate() {
        artifact_bytes = artifact_bytes
            .checked_add(artifact.planned_download.size_bytes)
            .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_VERIFICATION_CLOSURE_BYTES"))?;
        if position > 0 && closure.artifacts[position - 1].ordinal >= artifact.ordinal {
            bail!("COMPUTE_PLUGIN_VERIFICATION_CLOSURE_ORDER_INVALID");
        }
        if !is_sha256(&artifact.planned_download.digest)
            || artifact.planned_download.size_bytes <= 0
            || artifact.committed_offset != artifact.planned_download.size_bytes
            || artifact.cursor_generation <= 0
            || artifact.download_state != "complete"
            || artifact.created_at_ms < closure.candidate_created_at_ms
            || artifact.updated_at_ms < artifact.created_at_ms
        {
            bail!("COMPUTE_PLUGIN_VERIFICATION_CLOSURE_ARTIFACT_INVALID");
        }
    }
    let expected = ExpectedArtifactSet {
        schema: EXPECTED_ARTIFACT_SET_SCHEMA,
        release: &closure.release,
        digest_algorithm: "sha256",
        artifact_count: closure.artifacts.len(),
        artifact_bytes,
        artifacts: closure
            .artifacts
            .iter()
            .enumerate()
            .map(|(sequence, artifact)| ExpectedArtifact {
                sequence,
                artifact_kind: &artifact.planned_download.artifact_kind,
                artifact_id: &artifact.planned_download.artifact_id,
                digest: &artifact.planned_download.digest,
                size_bytes: artifact.planned_download.size_bytes,
            })
            .collect(),
    };
    Ok(CandidateClosureSnapshot {
        durable_closure_digest: jcs_sha256_hex(&closure)?,
        expected_artifact_set_digest: jcs_sha256_hex(&expected)?,
        artifact_count: closure.artifacts.len(),
        artifact_bytes,
    })
}
