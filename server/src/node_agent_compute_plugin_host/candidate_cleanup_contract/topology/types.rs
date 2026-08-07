use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

use crate::node_agent_compute_plugin_host::{
    manifest_validation::is_sha256, signed_artifact_verification::jcs_sha256_hex,
};

pub(in crate::node_agent_compute_plugin_host) const CANDIDATE_CLEANUP_EXPECTED_OBJECT_SCHEMA: &str =
    "elon.compute_plugin.candidate_cleanup_expected_object.v1";
pub(in crate::node_agent_compute_plugin_host) const CANDIDATE_CLEANUP_EXECUTION_PLAN_SCHEMA: &str =
    "elon.compute_plugin.candidate_cleanup_execution_plan.v1";
pub(in crate::node_agent_compute_plugin_host) const HASHED_CANDIDATE_CLEANUP_EXECUTION_PLAN_SCHEMA: &str =
    "elon.compute_plugin.hashed_candidate_cleanup_execution_plan.v1";
pub(in crate::node_agent_compute_plugin_host) const CANDIDATE_CLEANUP_EXECUTION_PLAN_CANONICALIZATION: &str =
    "RFC8785-JCS";
pub(in crate::node_agent_compute_plugin_host) const CANDIDATE_CLEANUP_EXECUTION_PLAN_DIGEST_ALGORITHM: &str =
    "sha256";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(in crate::node_agent_compute_plugin_host) struct ComputePluginCandidateCleanupExpectedObject {
    pub(super) schema: String,
    pub(super) cleanup_id: String,
    pub(super) step_ordinal: i64,
    pub(super) parent_step_ordinal: Option<i64>,
    pub(super) topology_depth: i64,
    pub(super) object_kind: String,
    pub(super) logical_kind: String,
    pub(super) relative_name: String,
    pub(super) relative_path: String,
    pub(super) relative_path_digest: String,
    pub(super) expected_identity_digest: String,
    pub(super) expected_parent_identity_digest: String,
    pub(super) expected_content_digest: Option<String>,
    pub(super) expected_size_bytes: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(in crate::node_agent_compute_plugin_host) struct HashedCandidateCleanupExpectedObject {
    pub(super) object: ComputePluginCandidateCleanupExpectedObject,
    pub(super) object_digest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(in crate::node_agent_compute_plugin_host) struct ComputePluginCandidateCleanupExecutionPlan {
    pub(super) schema: String,
    pub(super) cleanup_id: String,
    pub(super) candidate_token_digest: String,
    pub(super) authorization_receipt_digest: String,
    pub(super) installation_id_digest: String,
    pub(super) root_identity_digest: String,
    pub(super) candidate_parent_anchor_relative_path: String,
    pub(super) candidate_parent_anchor_identity_digest: String,
    pub(super) object_count: i64,
    pub(super) file_count: i64,
    pub(super) directory_count: i64,
    pub(super) expected_file_bytes: i64,
    pub(super) process_owner_epoch: i64,
    pub(super) planned_at_ms: i64,
    pub(super) object_digests: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(in crate::node_agent_compute_plugin_host) struct HashedComputePluginCandidateCleanupExecutionPlan
{
    pub(super) schema: String,
    pub(super) plan: ComputePluginCandidateCleanupExecutionPlan,
    pub(super) objects: Vec<HashedCandidateCleanupExpectedObject>,
    pub(super) canonicalization: String,
    pub(super) digest_algorithm: String,
    pub(super) plan_digest: String,
}

impl HashedComputePluginCandidateCleanupExecutionPlan {
    pub(in crate::node_agent_compute_plugin_host) fn plan(
        &self,
    ) -> &ComputePluginCandidateCleanupExecutionPlan {
        &self.plan
    }
    pub(in crate::node_agent_compute_plugin_host) fn objects(
        &self,
    ) -> &[HashedCandidateCleanupExpectedObject] {
        &self.objects
    }
    pub(in crate::node_agent_compute_plugin_host) fn plan_digest(&self) -> &str {
        &self.plan_digest
    }
}

macro_rules! plan_getter {
    ($name:ident, $field:ident, str) => {
        pub(in crate::node_agent_compute_plugin_host) fn $name(&self) -> &str {
            &self.$field
        }
    };
    ($name:ident, $field:ident, i64) => {
        pub(in crate::node_agent_compute_plugin_host) fn $name(&self) -> i64 {
            self.$field
        }
    };
}

impl ComputePluginCandidateCleanupExecutionPlan {
    plan_getter!(cleanup_id, cleanup_id, str);
    plan_getter!(candidate_token_digest, candidate_token_digest, str);
    plan_getter!(
        authorization_receipt_digest,
        authorization_receipt_digest,
        str
    );
    plan_getter!(installation_id_digest, installation_id_digest, str);
    plan_getter!(root_identity_digest, root_identity_digest, str);
    plan_getter!(
        candidate_parent_anchor_relative_path,
        candidate_parent_anchor_relative_path,
        str
    );
    plan_getter!(
        candidate_parent_anchor_identity_digest,
        candidate_parent_anchor_identity_digest,
        str
    );
    plan_getter!(object_count, object_count, i64);
    plan_getter!(file_count, file_count, i64);
    plan_getter!(directory_count, directory_count, i64);
    plan_getter!(expected_file_bytes, expected_file_bytes, i64);
    plan_getter!(process_owner_epoch, process_owner_epoch, i64);
    plan_getter!(planned_at_ms, planned_at_ms, i64);
    pub(in crate::node_agent_compute_plugin_host) fn object_digests(&self) -> &[String] {
        &self.object_digests
    }
}

impl ComputePluginCandidateCleanupExpectedObject {
    pub(in crate::node_agent_compute_plugin_host) fn cleanup_id(&self) -> &str {
        &self.cleanup_id
    }
    pub(in crate::node_agent_compute_plugin_host) fn step_ordinal(&self) -> i64 {
        self.step_ordinal
    }
    pub(in crate::node_agent_compute_plugin_host) fn parent_step_ordinal(&self) -> Option<i64> {
        self.parent_step_ordinal
    }
    pub(in crate::node_agent_compute_plugin_host) fn topology_depth(&self) -> i64 {
        self.topology_depth
    }
    pub(in crate::node_agent_compute_plugin_host) fn object_kind(&self) -> &str {
        &self.object_kind
    }
    pub(in crate::node_agent_compute_plugin_host) fn logical_kind(&self) -> &str {
        &self.logical_kind
    }
    pub(in crate::node_agent_compute_plugin_host) fn relative_name(&self) -> &str {
        &self.relative_name
    }
    pub(in crate::node_agent_compute_plugin_host) fn relative_path(&self) -> &str {
        &self.relative_path
    }
    pub(in crate::node_agent_compute_plugin_host) fn relative_path_digest(&self) -> &str {
        &self.relative_path_digest
    }
    pub(in crate::node_agent_compute_plugin_host) fn expected_identity_digest(&self) -> &str {
        &self.expected_identity_digest
    }
    pub(in crate::node_agent_compute_plugin_host) fn expected_parent_identity_digest(
        &self,
    ) -> &str {
        &self.expected_parent_identity_digest
    }
    pub(in crate::node_agent_compute_plugin_host) fn expected_content_digest(
        &self,
    ) -> Option<&str> {
        self.expected_content_digest.as_deref()
    }
    pub(in crate::node_agent_compute_plugin_host) fn expected_size_bytes(&self) -> Option<i64> {
        self.expected_size_bytes
    }
}

impl HashedCandidateCleanupExpectedObject {
    pub(in crate::node_agent_compute_plugin_host) fn object(
        &self,
    ) -> &ComputePluginCandidateCleanupExpectedObject {
        &self.object
    }
    pub(in crate::node_agent_compute_plugin_host) fn object_digest(&self) -> &str {
        &self.object_digest
    }
}

pub(super) fn hash_expected_object(
    object: ComputePluginCandidateCleanupExpectedObject,
) -> Result<HashedCandidateCleanupExpectedObject> {
    let object_digest = jcs_sha256_hex(&object)?;
    Ok(HashedCandidateCleanupExpectedObject {
        object,
        object_digest,
    })
}

pub(in crate::node_agent_compute_plugin_host) fn restore_hashed_expected_object(
    object: ComputePluginCandidateCleanupExpectedObject,
    object_digest: String,
) -> Result<HashedCandidateCleanupExpectedObject> {
    if !is_sha256(&object_digest) || jcs_sha256_hex(&object)? != object_digest {
        bail!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_OBJECT_CHANGED");
    }
    Ok(HashedCandidateCleanupExpectedObject {
        object,
        object_digest,
    })
}

pub(super) fn hash_execution_plan(
    plan: ComputePluginCandidateCleanupExecutionPlan,
    objects: Vec<HashedCandidateCleanupExpectedObject>,
) -> Result<HashedComputePluginCandidateCleanupExecutionPlan> {
    let plan_digest = jcs_sha256_hex(&plan)?;
    let hashed = HashedComputePluginCandidateCleanupExecutionPlan {
        schema: HASHED_CANDIDATE_CLEANUP_EXECUTION_PLAN_SCHEMA.to_string(),
        plan,
        objects,
        canonicalization: CANDIDATE_CLEANUP_EXECUTION_PLAN_CANONICALIZATION.to_string(),
        digest_algorithm: CANDIDATE_CLEANUP_EXECUTION_PLAN_DIGEST_ALGORITHM.to_string(),
        plan_digest,
    };
    validate_hashed_execution_plan(&hashed)?;
    Ok(hashed)
}

pub(in crate::node_agent_compute_plugin_host) fn restore_hashed_execution_plan(
    plan: ComputePluginCandidateCleanupExecutionPlan,
    objects: Vec<HashedCandidateCleanupExpectedObject>,
    plan_digest: String,
) -> Result<HashedComputePluginCandidateCleanupExecutionPlan> {
    let hashed = HashedComputePluginCandidateCleanupExecutionPlan {
        schema: HASHED_CANDIDATE_CLEANUP_EXECUTION_PLAN_SCHEMA.to_string(),
        plan,
        objects,
        canonicalization: CANDIDATE_CLEANUP_EXECUTION_PLAN_CANONICALIZATION.to_string(),
        digest_algorithm: CANDIDATE_CLEANUP_EXECUTION_PLAN_DIGEST_ALGORITHM.to_string(),
        plan_digest,
    };
    validate_hashed_execution_plan(&hashed)?;
    Ok(hashed)
}

pub(in crate::node_agent_compute_plugin_host) fn validate_hashed_execution_plan(
    hashed: &HashedComputePluginCandidateCleanupExecutionPlan,
) -> Result<()> {
    let plan = &hashed.plan;
    if hashed.schema != HASHED_CANDIDATE_CLEANUP_EXECUTION_PLAN_SCHEMA
        || plan.schema != CANDIDATE_CLEANUP_EXECUTION_PLAN_SCHEMA
        || hashed.canonicalization != CANDIDATE_CLEANUP_EXECUTION_PLAN_CANONICALIZATION
        || hashed.digest_algorithm != CANDIDATE_CLEANUP_EXECUTION_PLAN_DIGEST_ALGORITHM
        || !is_sha256(&hashed.plan_digest)
        || jcs_sha256_hex(plan)? != hashed.plan_digest
        || usize::try_from(plan.object_count).ok() != Some(hashed.objects.len())
        || plan.object_digests.len() != hashed.objects.len()
        || plan
            .object_digests
            .iter()
            .zip(&hashed.objects)
            .any(|(expected, object)| {
                expected != &object.object_digest
                    || !is_sha256(expected)
                    || jcs_sha256_hex(&object.object).ok().as_deref() != Some(expected.as_str())
            })
    {
        bail!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_PLAN_CHANGED");
    }
    Ok(())
}
