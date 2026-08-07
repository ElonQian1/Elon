use std::collections::BTreeMap;

use anyhow::{anyhow, bail, Result};
use serde::Serialize;

use super::types::{
    hash_execution_plan, hash_expected_object, ComputePluginCandidateCleanupExecutionPlan,
    ComputePluginCandidateCleanupExpectedObject, HashedComputePluginCandidateCleanupExecutionPlan,
    CANDIDATE_CLEANUP_EXECUTION_PLAN_SCHEMA, CANDIDATE_CLEANUP_EXPECTED_OBJECT_SCHEMA,
};
use crate::node_agent_compute_plugin_host::{
    manifest_validation::is_sha256, signed_artifact_verification::jcs_sha256_hex,
};

pub(super) const CANDIDATE_PARENT_ANCHOR: &str = "compute-plugin/candidates";

#[derive(Clone)]
pub(in crate::node_agent_compute_plugin_host) struct CandidateCleanupTopologyObjectInput {
    pub(in crate::node_agent_compute_plugin_host) logical_kind: &'static str,
    pub(in crate::node_agent_compute_plugin_host) relative_path: String,
    pub(in crate::node_agent_compute_plugin_host) expected_identity_digest: String,
    pub(in crate::node_agent_compute_plugin_host) expected_parent_identity_digest: String,
    pub(in crate::node_agent_compute_plugin_host) expected_content_digest: Option<String>,
    pub(in crate::node_agent_compute_plugin_host) expected_size_bytes: Option<u64>,
}

#[derive(Clone)]
pub(super) struct CandidateCleanupTopologyPlanInput {
    pub cleanup_id: String,
    pub candidate_token_digest: String,
    pub authorization_receipt_digest: String,
    pub installation_id_digest: String,
    pub root_identity_digest: String,
    pub candidate_parent_anchor_identity_digest: String,
    pub process_owner_epoch: i64,
    pub planned_at_ms: i64,
    pub objects: Vec<CandidateCleanupTopologyObjectInput>,
}

struct PathIndexEntry {
    ordinal: i64,
    identity_digest: String,
    is_directory: bool,
}

#[derive(Serialize)]
struct RelativePathDigestBinding<'path> {
    schema: &'static str,
    relative_path: &'path str,
}

pub(super) fn build_execution_plan(
    input: CandidateCleanupTopologyPlanInput,
) -> Result<HashedComputePluginCandidateCleanupExecutionPlan> {
    validate_plan_input(&input)?;
    let candidate_root = format!("{CANDIDATE_PARENT_ANCHOR}/{}", input.candidate_token_digest);
    let path_ordinals = index_paths(&input.objects)?;
    let root_depth = component_count(&candidate_root)?;
    let mut files = 0_i64;
    let mut directories = 0_i64;
    let mut expected_file_bytes = 0_i64;
    let mut objects = Vec::with_capacity(input.objects.len());

    for (ordinal, source) in input.objects.into_iter().enumerate() {
        let step_ordinal = i64::try_from(ordinal)
            .map_err(|_| anyhow!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_PLAN_TOO_LARGE"))?;
        let path_depth = component_count(&source.relative_path)?;
        if path_depth < root_depth || !is_beneath_or_equal(&source.relative_path, &candidate_root) {
            bail!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_PATH_OUTSIDE_CANDIDATE");
        }
        let is_root = source.relative_path == candidate_root;
        let parent_path = if is_root {
            None
        } else {
            Some(parent_path(&source.relative_path)?)
        };
        let parent_step_ordinal = match parent_path.as_deref() {
            Some(path) => {
                let parent = path_ordinals
                    .get(path)
                    .ok_or_else(|| anyhow!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_PARENT_MISSING"))?;
                if !parent.is_directory
                    || parent.ordinal <= step_ordinal
                    || parent.identity_digest != source.expected_parent_identity_digest
                {
                    bail!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_PARENT_ORDER_INVALID");
                }
                Some(parent.ordinal)
            }
            None => None,
        };
        let (object_kind, expected_size_bytes) = match source.expected_size_bytes {
            Some(size) => {
                files += 1;
                let size = i64::try_from(size)
                    .map_err(|_| anyhow!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_FILE_TOO_LARGE"))?;
                expected_file_bytes = expected_file_bytes
                    .checked_add(size)
                    .ok_or_else(|| anyhow!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_SIZE_OVERFLOW"))?;
                ("file", Some(size))
            }
            None => {
                directories += 1;
                ("directory", None)
            }
        };
        let relative_name = relative_name(&source.relative_path)?;
        let relative_path_digest = jcs_sha256_hex(&RelativePathDigestBinding {
            schema: "elon.compute_plugin.candidate_cleanup_relative_path.v1",
            relative_path: &source.relative_path,
        })?;
        let object = ComputePluginCandidateCleanupExpectedObject {
            schema: CANDIDATE_CLEANUP_EXPECTED_OBJECT_SCHEMA.to_string(),
            cleanup_id: input.cleanup_id.clone(),
            step_ordinal,
            parent_step_ordinal,
            topology_depth: i64::try_from(path_depth - root_depth)
                .map_err(|_| anyhow!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_DEPTH_OVERFLOW"))?,
            object_kind: object_kind.to_string(),
            logical_kind: source.logical_kind.to_string(),
            relative_name,
            relative_path: source.relative_path,
            relative_path_digest,
            expected_identity_digest: source.expected_identity_digest,
            expected_parent_identity_digest: source.expected_parent_identity_digest,
            expected_content_digest: source.expected_content_digest,
            expected_size_bytes,
        };
        validate_expected_object(&object)?;
        objects.push(hash_expected_object(object)?);
    }

    let root = objects
        .last()
        .ok_or_else(|| anyhow!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_ROOT_MISSING"))?
        .object();
    if root.relative_path != candidate_root
        || root.parent_step_ordinal.is_some()
        || root.topology_depth != 0
        || root.object_kind != "directory"
        || root.expected_parent_identity_digest != input.candidate_parent_anchor_identity_digest
    {
        bail!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_ROOT_INVALID");
    }

    let object_count = files + directories;
    let object_digests = objects
        .iter()
        .map(|object| object.object_digest.clone())
        .collect();
    let plan = ComputePluginCandidateCleanupExecutionPlan {
        schema: CANDIDATE_CLEANUP_EXECUTION_PLAN_SCHEMA.to_string(),
        cleanup_id: input.cleanup_id,
        candidate_token_digest: input.candidate_token_digest,
        authorization_receipt_digest: input.authorization_receipt_digest,
        installation_id_digest: input.installation_id_digest,
        root_identity_digest: input.root_identity_digest,
        candidate_parent_anchor_relative_path: CANDIDATE_PARENT_ANCHOR.to_string(),
        candidate_parent_anchor_identity_digest: input.candidate_parent_anchor_identity_digest,
        object_count,
        file_count: files,
        directory_count: directories,
        expected_file_bytes,
        process_owner_epoch: input.process_owner_epoch,
        planned_at_ms: input.planned_at_ms,
        object_digests,
    };
    hash_execution_plan(plan, objects)
}

fn validate_plan_input(input: &CandidateCleanupTopologyPlanInput) -> Result<()> {
    if input.objects.is_empty()
        || input.objects.len() > 32_768
        || input.process_owner_epoch <= 0
        || input.planned_at_ms < 0
        || !is_sha256(&input.candidate_token_digest)
        || !is_sha256(&input.authorization_receipt_digest)
        || !is_sha256(&input.installation_id_digest)
        || !is_sha256(&input.root_identity_digest)
        || !is_sha256(&input.candidate_parent_anchor_identity_digest)
    {
        bail!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_PLAN_INPUT_INVALID");
    }
    Ok(())
}

fn index_paths(
    objects: &[CandidateCleanupTopologyObjectInput],
) -> Result<BTreeMap<String, PathIndexEntry>> {
    let mut paths = BTreeMap::new();
    for (ordinal, object) in objects.iter().enumerate() {
        validate_path(&object.relative_path)?;
        let ordinal = i64::try_from(ordinal)
            .map_err(|_| anyhow!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_PLAN_TOO_LARGE"))?;
        let entry = PathIndexEntry {
            ordinal,
            identity_digest: object.expected_identity_digest.clone(),
            is_directory: object.expected_size_bytes.is_none(),
        };
        if paths.insert(object.relative_path.clone(), entry).is_some() {
            bail!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_PATH_DUPLICATE");
        }
    }
    Ok(paths)
}

fn validate_expected_object(object: &ComputePluginCandidateCleanupExpectedObject) -> Result<()> {
    if object.schema != CANDIDATE_CLEANUP_EXPECTED_OBJECT_SCHEMA
        || !is_sha256(&object.relative_path_digest)
        || !is_sha256(&object.expected_identity_digest)
        || !is_sha256(&object.expected_parent_identity_digest)
        || object
            .expected_content_digest
            .as_deref()
            .is_some_and(|digest| !is_sha256(digest))
        || (object.object_kind == "file") != object.expected_size_bytes.is_some()
        || (object.object_kind == "file") != object.expected_content_digest.is_some()
        || !matches!(object.object_kind.as_str(), "file" | "directory")
    {
        bail!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_OBJECT_INVALID");
    }
    Ok(())
}

fn validate_path(path: &str) -> Result<()> {
    if path.is_empty()
        || path.len() > 4096
        || path.starts_with('/')
        || path.ends_with('/')
        || path.contains('\\')
        || path.contains("//")
        || path
            .split('/')
            .any(|part| part.is_empty() || part == "." || part == ".." || part.len() > 255)
    {
        bail!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_PATH_INVALID");
    }
    Ok(())
}

fn component_count(path: &str) -> Result<usize> {
    validate_path(path)?;
    Ok(path.split('/').count())
}

fn parent_path(path: &str) -> Result<String> {
    validate_path(path)?;
    path.rsplit_once('/')
        .map(|(parent, _)| parent.to_string())
        .ok_or_else(|| anyhow!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_PARENT_MISSING"))
}

fn relative_name(path: &str) -> Result<String> {
    validate_path(path)?;
    path.rsplit('/')
        .next()
        .map(str::to_string)
        .ok_or_else(|| anyhow!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_RELATIVE_NAME_MISSING"))
}

fn is_beneath_or_equal(path: &str, root: &str) -> bool {
    path == root
        || path
            .strip_prefix(root)
            .is_some_and(|suffix| suffix.starts_with('/'))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plan_input() -> CandidateCleanupTopologyPlanInput {
        let candidate = "a".repeat(64);
        let root = format!("compute-plugin/candidates/{candidate}");
        CandidateCleanupTopologyPlanInput {
            cleanup_id: "cca_topology_test".to_string(),
            candidate_token_digest: candidate,
            authorization_receipt_digest: "1".repeat(64),
            installation_id_digest: "2".repeat(64),
            root_identity_digest: "3".repeat(64),
            candidate_parent_anchor_identity_digest: "4".repeat(64),
            process_owner_epoch: 7,
            planned_at_ms: 2_000,
            objects: vec![
                CandidateCleanupTopologyObjectInput {
                    logical_kind: "download_file",
                    relative_path: format!("{root}/downloads/package.part"),
                    expected_identity_digest: "5".repeat(64),
                    expected_parent_identity_digest: "6".repeat(64),
                    expected_content_digest: Some("7".repeat(64)),
                    expected_size_bytes: Some(42),
                },
                CandidateCleanupTopologyObjectInput {
                    logical_kind: "downloads_directory",
                    relative_path: format!("{root}/downloads"),
                    expected_identity_digest: "6".repeat(64),
                    expected_parent_identity_digest: "8".repeat(64),
                    expected_content_digest: None,
                    expected_size_bytes: None,
                },
                CandidateCleanupTopologyObjectInput {
                    logical_kind: "candidate_directory",
                    relative_path: root,
                    expected_identity_digest: "8".repeat(64),
                    expected_parent_identity_digest: "4".repeat(64),
                    expected_content_digest: None,
                    expected_size_bytes: None,
                },
            ],
        }
    }

    #[test]
    fn cleanup_topology_is_deterministic_and_child_first() {
        let first = build_execution_plan(plan_input()).unwrap();
        let second = build_execution_plan(plan_input()).unwrap();

        assert_eq!(first, second);
        assert_eq!(first.plan().object_count(), 3);
        assert_eq!(first.plan().file_count(), 1);
        assert_eq!(first.plan().directory_count(), 2);
        assert_eq!(first.plan().expected_file_bytes(), 42);
        assert_eq!(first.objects()[0].object().parent_step_ordinal(), Some(1));
        assert_eq!(first.objects()[1].object().parent_step_ordinal(), Some(2));
        assert_eq!(first.objects()[2].object().parent_step_ordinal(), None);
    }

    #[test]
    fn cleanup_topology_rejects_parent_before_child() {
        let mut input = plan_input();
        input.objects.swap(0, 1);

        let error = build_execution_plan(input).unwrap_err();
        assert!(error.to_string().contains("PARENT_ORDER_INVALID"));
    }

    #[test]
    fn cleanup_topology_rejects_parent_identity_change() {
        let mut input = plan_input();
        input.objects[0].expected_parent_identity_digest = "9".repeat(64);

        let error = build_execution_plan(input).unwrap_err();
        assert!(error.to_string().contains("PARENT_ORDER_INVALID"));
    }
}
