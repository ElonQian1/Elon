//! Duplicate and conflict hints against the Git-backed shared memory manifest.

use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, fs, path::Path};

use crate::{
    project_document_governance::{parse_manifest, SECTION_CONFIG_PATH},
    project_document_native_context::ProjectContextMemory,
};

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct NativeContextConflict {
    pub kind: String,
    #[serde(default)]
    pub shared_candidate_id: String,
    #[serde(default)]
    pub overlapping_paths: Vec<String>,
}

pub(crate) fn inspect(
    workspace: &Path,
    candidate: &ProjectContextMemory,
) -> Vec<NativeContextConflict> {
    let path = workspace.join(SECTION_CONFIG_PATH);
    let Some(manifest) = fs::read_to_string(path)
        .ok()
        .and_then(|content| parse_manifest(Some(&content)).ok())
    else {
        return Vec::new();
    };
    manifest
        .context_memories
        .iter()
        .filter_map(|shared| classify(candidate, shared))
        .take(4)
        .collect()
}

pub(crate) fn is_shared_duplicate(conflicts: &[NativeContextConflict]) -> bool {
    conflicts
        .iter()
        .any(|conflict| conflict.kind == "shared_duplicate")
}

fn classify(
    candidate: &ProjectContextMemory,
    shared: &ProjectContextMemory,
) -> Option<NativeContextConflict> {
    let candidate_paths = candidate
        .evidence
        .iter()
        .map(|evidence| evidence.path.as_str())
        .collect::<BTreeSet<_>>();
    let shared_paths = shared
        .evidence
        .iter()
        .map(|evidence| evidence.path.as_str())
        .collect::<BTreeSet<_>>();
    let overlapping_paths = candidate_paths
        .intersection(&shared_paths)
        .map(|path| (*path).to_string())
        .take(4)
        .collect::<Vec<_>>();
    let kind = if same_fact(candidate, shared) {
        "shared_duplicate"
    } else if candidate.candidate_id == shared.candidate_id {
        "shared_replacement"
    } else if !overlapping_paths.is_empty() && topics_overlap(candidate, shared) {
        "potential_semantic_conflict"
    } else {
        return None;
    };
    Some(NativeContextConflict {
        kind: kind.to_string(),
        shared_candidate_id: shared.candidate_id.clone(),
        overlapping_paths,
    })
}

fn same_fact(left: &ProjectContextMemory, right: &ProjectContextMemory) -> bool {
    left.summary.eq_ignore_ascii_case(&right.summary)
        && normalized_topics(left) == normalized_topics(right)
        && left
            .evidence
            .iter()
            .map(|evidence| {
                (
                    evidence.path.as_str(),
                    evidence.locator.as_str(),
                    evidence.evidence_kind.as_str(),
                )
            })
            .eq(right.evidence.iter().map(|evidence| {
                (
                    evidence.path.as_str(),
                    evidence.locator.as_str(),
                    evidence.evidence_kind.as_str(),
                )
            }))
}

fn topics_overlap(left: &ProjectContextMemory, right: &ProjectContextMemory) -> bool {
    let left = normalized_topics(left);
    let right = normalized_topics(right);
    left.iter().any(|topic| right.contains(topic))
}

fn normalized_topics(memory: &ProjectContextMemory) -> BTreeSet<String> {
    memory
        .topics
        .iter()
        .map(|topic| topic.trim().to_ascii_lowercase())
        .collect()
}
