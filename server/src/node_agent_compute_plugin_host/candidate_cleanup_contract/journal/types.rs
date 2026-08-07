use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

use crate::node_agent_compute_plugin_host::{
    manifest_validation::is_sha256, signed_artifact_verification::jcs_sha256_hex,
};

pub(in crate::node_agent_compute_plugin_host) const CANDIDATE_CLEANUP_STEP_EVENT_SCHEMA: &str =
    "elon.compute_plugin.candidate_cleanup_step_event.v1";
pub(in crate::node_agent_compute_plugin_host) const HASHED_CANDIDATE_CLEANUP_STEP_EVENT_SCHEMA:
    &str = "elon.compute_plugin.hashed_candidate_cleanup_step_event.v1";
pub(in crate::node_agent_compute_plugin_host) const CANDIDATE_CLEANUP_STEP_EVENT_CANONICALIZATION: &str =
    "RFC8785-JCS";
pub(in crate::node_agent_compute_plugin_host) const CANDIDATE_CLEANUP_STEP_EVENT_DIGEST_ALGORITHM: &str =
    "sha256";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(in crate::node_agent_compute_plugin_host) struct ComputePluginCandidateCleanupStepEvent {
    pub(super) schema: String,
    pub(super) cleanup_id: String,
    pub(super) plan_digest: String,
    pub(super) event_sequence: i64,
    pub(super) step_ordinal: i64,
    pub(super) event_kind: String,
    pub(super) object_digest: String,
    pub(super) observed_identity_digest: Option<String>,
    pub(super) observed_parent_identity_digest: String,
    pub(super) namespace_durability_kind: Option<String>,
    pub(super) namespace_durability_evidence_digest: Option<String>,
    pub(super) previous_event_digest: String,
    pub(super) process_owner_epoch: i64,
    pub(super) recorded_at_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(in crate::node_agent_compute_plugin_host) struct HashedComputePluginCandidateCleanupStepEvent {
    pub(super) schema: String,
    pub(super) event: ComputePluginCandidateCleanupStepEvent,
    pub(super) canonicalization: String,
    pub(super) digest_algorithm: String,
    pub(super) event_digest: String,
}

impl HashedComputePluginCandidateCleanupStepEvent {
    pub(in crate::node_agent_compute_plugin_host) fn event(
        &self,
    ) -> &ComputePluginCandidateCleanupStepEvent {
        &self.event
    }

    pub(in crate::node_agent_compute_plugin_host) fn event_digest(&self) -> &str {
        &self.event_digest
    }
}

macro_rules! event_getter {
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

impl ComputePluginCandidateCleanupStepEvent {
    event_getter!(cleanup_id, cleanup_id, str);
    event_getter!(plan_digest, plan_digest, str);
    event_getter!(event_sequence, event_sequence, i64);
    event_getter!(step_ordinal, step_ordinal, i64);
    event_getter!(event_kind, event_kind, str);
    event_getter!(object_digest, object_digest, str);
    event_getter!(
        observed_parent_identity_digest,
        observed_parent_identity_digest,
        str
    );
    event_getter!(previous_event_digest, previous_event_digest, str);
    event_getter!(process_owner_epoch, process_owner_epoch, i64);
    event_getter!(recorded_at_ms, recorded_at_ms, i64);

    pub(in crate::node_agent_compute_plugin_host) fn observed_identity_digest(
        &self,
    ) -> Option<&str> {
        self.observed_identity_digest.as_deref()
    }

    pub(in crate::node_agent_compute_plugin_host) fn namespace_durability_kind(
        &self,
    ) -> Option<&str> {
        self.namespace_durability_kind.as_deref()
    }

    pub(in crate::node_agent_compute_plugin_host) fn namespace_durability_evidence_digest(
        &self,
    ) -> Option<&str> {
        self.namespace_durability_evidence_digest.as_deref()
    }
}

pub(super) fn hash_cleanup_step_event(
    event: ComputePluginCandidateCleanupStepEvent,
) -> Result<HashedComputePluginCandidateCleanupStepEvent> {
    let event_digest = jcs_sha256_hex(&event)?;
    let hashed = HashedComputePluginCandidateCleanupStepEvent {
        schema: HASHED_CANDIDATE_CLEANUP_STEP_EVENT_SCHEMA.to_string(),
        event,
        canonicalization: CANDIDATE_CLEANUP_STEP_EVENT_CANONICALIZATION.to_string(),
        digest_algorithm: CANDIDATE_CLEANUP_STEP_EVENT_DIGEST_ALGORITHM.to_string(),
        event_digest,
    };
    validate_hashed_cleanup_step_event(&hashed)?;
    Ok(hashed)
}

pub(in crate::node_agent_compute_plugin_host) fn restore_hashed_cleanup_step_event(
    event: ComputePluginCandidateCleanupStepEvent,
    event_digest: String,
) -> Result<HashedComputePluginCandidateCleanupStepEvent> {
    let hashed = HashedComputePluginCandidateCleanupStepEvent {
        schema: HASHED_CANDIDATE_CLEANUP_STEP_EVENT_SCHEMA.to_string(),
        event,
        canonicalization: CANDIDATE_CLEANUP_STEP_EVENT_CANONICALIZATION.to_string(),
        digest_algorithm: CANDIDATE_CLEANUP_STEP_EVENT_DIGEST_ALGORITHM.to_string(),
        event_digest,
    };
    validate_hashed_cleanup_step_event(&hashed)?;
    Ok(hashed)
}

pub(in crate::node_agent_compute_plugin_host) fn validate_hashed_cleanup_step_event(
    hashed: &HashedComputePluginCandidateCleanupStepEvent,
) -> Result<()> {
    let event = hashed.event();
    let expected_offset = event_kind_offset(event.event_kind());
    let expected_sequence = event
        .step_ordinal()
        .checked_mul(4)
        .and_then(|value| expected_offset.and_then(|offset| value.checked_add(offset)));
    let durability_fields_present = event.namespace_durability_kind().is_some()
        && event.namespace_durability_evidence_digest().is_some();
    if hashed.schema != HASHED_CANDIDATE_CLEANUP_STEP_EVENT_SCHEMA
        || event.schema != CANDIDATE_CLEANUP_STEP_EVENT_SCHEMA
        || hashed.canonicalization != CANDIDATE_CLEANUP_STEP_EVENT_CANONICALIZATION
        || hashed.digest_algorithm != CANDIDATE_CLEANUP_STEP_EVENT_DIGEST_ALGORITHM
        || event.cleanup_id().is_empty()
        || event.cleanup_id().len() > 256
        || !is_sha256(event.plan_digest())
        || event.event_sequence() <= 0
        || event.event_sequence() > 131_072
        || event.step_ordinal() < 0
        || event.step_ordinal() >= 32_768
        || expected_sequence != Some(event.event_sequence())
        || !is_sha256(event.object_digest())
        || event
            .observed_identity_digest()
            .is_some_and(|digest| !is_sha256(digest))
        || !is_sha256(event.observed_parent_identity_digest())
        || event
            .namespace_durability_evidence_digest()
            .is_some_and(|digest| !is_sha256(digest))
        || event
            .namespace_durability_kind()
            .is_some_and(|kind| kind.is_empty() || kind.len() > 128)
        || (event.event_kind() == "namespace_durable") != durability_fields_present
        || (event.namespace_durability_kind().is_some()
            != event.namespace_durability_evidence_digest().is_some())
        || !is_sha256(event.previous_event_digest())
        || event.process_owner_epoch() <= 0
        || event.recorded_at_ms() < 0
        || !is_sha256(&hashed.event_digest)
        || jcs_sha256_hex(event)? != hashed.event_digest
    {
        bail!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_STEP_EVENT_CHANGED");
    }
    validate_observed_identity_shape(event)
}

fn event_kind_offset(kind: &str) -> Option<i64> {
    match kind {
        "delete_intent" => Some(1),
        "exact_handle_disposition_set" | "absence_recovered_after_intent" => Some(2),
        "parent_namespace_absence_observed" => Some(3),
        "namespace_durable" => Some(4),
        _ => None,
    }
}

fn validate_observed_identity_shape(event: &ComputePluginCandidateCleanupStepEvent) -> Result<()> {
    let requires_identity = matches!(
        event.event_kind(),
        "delete_intent" | "exact_handle_disposition_set"
    );
    if requires_identity != event.observed_identity_digest().is_some() {
        bail!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_STEP_EVENT_IDENTITY_INVALID");
    }
    Ok(())
}
