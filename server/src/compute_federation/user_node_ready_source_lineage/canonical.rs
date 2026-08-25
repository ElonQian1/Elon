use anyhow::{bail, ensure, Result};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::compute_plugin_sharing_directive::canonical_compute_plugin_ijson_and_sha256;

use super::{
    source_inputs::UntrustedComputeUserNodeHostRuntimeObservationDraftV1,
    types::{
        UntrustedComputeUserNodeHostRuntimeObservationV1,
        UntrustedComputeUserNodeReadySourceLineageEnvelopeV1,
        COMPUTE_USER_NODE_READY_SOURCE_LINEAGE_DIGEST_DOMAIN,
        COMPUTE_USER_NODE_READY_SOURCE_LINEAGE_MAX_JSON_BYTES,
        UNTRUSTED_COMPUTE_USER_NODE_HOST_RUNTIME_OBSERVATION_DIGEST_DOMAIN,
        UNTRUSTED_COMPUTE_USER_NODE_HOST_RUNTIME_OBSERVATION_SCHEMA,
    },
    validation::{
        validate_compute_user_node_ready_source_lineage,
        validate_untrusted_compute_user_node_host_runtime_observation,
    },
};

pub(crate) fn canonical_compute_user_node_ready_source_lineage_json_and_digest(
    envelope: &UntrustedComputeUserNodeReadySourceLineageEnvelopeV1,
) -> Result<(String, String)> {
    let value = serde_json::to_value(envelope)?;
    let mut projection = value
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("user-node Ready source lineage must be an object"))?
        .clone();
    if projection
        .insert(
            "lineage_digest".to_string(),
            serde_json::Value::String(String::new()),
        )
        .is_none()
    {
        bail!("user-node Ready source lineage lacks lineage_digest");
    }
    let digest = domain_digest(
        COMPUTE_USER_NODE_READY_SOURCE_LINEAGE_DIGEST_DOMAIN,
        &projection,
    )?;
    Ok((canonical_json(envelope)?, digest))
}

pub(crate) fn compute_user_node_ready_source_lineage_from_json(
    value: &str,
) -> Result<UntrustedComputeUserNodeReadySourceLineageEnvelopeV1> {
    ensure!(
        value.len() <= COMPUTE_USER_NODE_READY_SOURCE_LINEAGE_MAX_JSON_BYTES,
        "user-node Ready source lineage exceeds its byte limit"
    );
    let envelope =
        serde_json::from_str::<UntrustedComputeUserNodeReadySourceLineageEnvelopeV1>(value)?;
    validate_compute_user_node_ready_source_lineage(&envelope)?;
    ensure!(
        canonical_json(&envelope)? == value,
        "user-node Ready source lineage JSON is not canonical"
    );
    Ok(envelope)
}

pub(crate) fn project_untrusted_compute_user_node_host_runtime_observation(
    draft: UntrustedComputeUserNodeHostRuntimeObservationDraftV1,
) -> Result<UntrustedComputeUserNodeHostRuntimeObservationV1> {
    let mut observation = UntrustedComputeUserNodeHostRuntimeObservationV1 {
        schema: UNTRUSTED_COMPUTE_USER_NODE_HOST_RUNTIME_OBSERVATION_SCHEMA.to_string(),
        observation_digest: String::new(),
        executor_id: draft.executor_id,
        runner_id: draft.runner_id,
        runner_digest: draft.runner_digest,
        runtime_digest: draft.runtime_digest,
        host_enforcement_ref: draft.host_enforcement_ref,
        host_enforcement_digest: draft.host_enforcement_digest,
        resource_profile_digest: draft.resource_profile_digest,
        task_kinds: draft.task_kinds,
        model_bindings: draft.model_bindings,
        supported_precisions: draft.supported_precisions,
        resources: draft.resources,
        technical_concurrency_limit: draft.technical_concurrency_limit,
        observed_at: draft.observed_at,
        expires_at: draft.expires_at,
    };
    observation.observation_digest =
        canonical_untrusted_host_runtime_observation_digest(&observation)?;
    validate_untrusted_compute_user_node_host_runtime_observation(&observation)?;
    Ok(observation)
}

pub(super) fn canonical_untrusted_host_runtime_observation_digest(
    observation: &UntrustedComputeUserNodeHostRuntimeObservationV1,
) -> Result<String> {
    let value = serde_json::to_value(observation)?;
    let mut projection = value
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("untrusted Host runtime observation must be an object"))?
        .clone();
    if projection
        .insert(
            "observation_digest".to_string(),
            serde_json::Value::String(String::new()),
        )
        .is_none()
    {
        bail!("untrusted Host runtime observation lacks observation_digest");
    }
    domain_digest(
        UNTRUSTED_COMPUTE_USER_NODE_HOST_RUNTIME_OBSERVATION_DIGEST_DOMAIN,
        &projection,
    )
}

fn canonical_json<T: Serialize + ?Sized>(value: &T) -> Result<String> {
    canonical_compute_plugin_ijson_and_sha256(
        value,
        COMPUTE_USER_NODE_READY_SOURCE_LINEAGE_MAX_JSON_BYTES,
    )
    .map(|(json, _)| json)
}

fn domain_digest<T: Serialize + ?Sized>(domain: &str, value: &T) -> Result<String> {
    let json = canonical_json(value)?;
    let mut digest = Sha256::new();
    digest.update(domain.as_bytes());
    digest.update([0]);
    digest.update(json.as_bytes());
    Ok(hex::encode(digest.finalize()))
}
