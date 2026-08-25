use anyhow::{ensure, Result};

use super::{
    canonical::canonical_compute_user_node_ready_source_lineage_json_and_digest,
    source_inputs::ComputeUserNodeReadySourceLineageSources, types::*,
    validation::validate_compute_user_node_ready_source_lineage,
};

pub(crate) fn build_compute_user_node_ready_source_lineage(
    sources: ComputeUserNodeReadySourceLineageSources,
) -> Result<ProjectedComputeUserNodeReadySourceLineageV1> {
    let lineage = ComputeUserNodeReadySourceLineageV1 {
        projection_status: COMPUTE_USER_NODE_READY_SOURCE_PROJECTION_STATUS.to_string(),
        work_admission: sources.work_admission,
        ready_health: sources.ready_health,
        host_runtime_observation: sources.host_runtime_observation,
        authority_gaps: ComputeUserNodeReadySourceAuthorityGapsV1 {
            node_local_authority_currentness: COMPUTE_USER_NODE_READY_SOURCE_AUTHORITY_MISSING
                .to_string(),
            runtime_transition_authority: COMPUTE_USER_NODE_READY_SOURCE_AUTHORITY_MISSING
                .to_string(),
            host_runtime_authority: COMPUTE_USER_NODE_READY_SOURCE_AUTHORITY_MISSING.to_string(),
            v15_authenticated_session: COMPUTE_USER_NODE_READY_SOURCE_AUTHORITY_MISSING.to_string(),
        },
        effects: inert_effects(),
    };
    let mut envelope = UntrustedComputeUserNodeReadySourceLineageEnvelopeV1 {
        schema: COMPUTE_USER_NODE_READY_SOURCE_LINEAGE_SCHEMA.to_string(),
        lineage_kind: COMPUTE_USER_NODE_READY_SOURCE_LINEAGE_KIND.to_string(),
        lineage_digest: String::new(),
        canonicalization: COMPUTE_USER_NODE_READY_SOURCE_LINEAGE_CANONICALIZATION.to_string(),
        digest_algorithm: COMPUTE_USER_NODE_READY_SOURCE_LINEAGE_DIGEST_ALGORITHM.to_string(),
        lineage,
    };
    envelope.lineage_digest =
        canonical_compute_user_node_ready_source_lineage_json_and_digest(&envelope)?.1;
    validate_compute_user_node_ready_source_lineage(&envelope)?;
    Ok(ProjectedComputeUserNodeReadySourceLineageV1 { envelope })
}

pub(crate) fn validate_compute_user_node_ready_source_lineage_against_sources(
    projected: &ProjectedComputeUserNodeReadySourceLineageV1,
    sources: &ComputeUserNodeReadySourceLineageSources,
) -> Result<()> {
    validate_compute_user_node_ready_source_lineage(projected.envelope())?;
    let lineage = projected.lineage();
    ensure!(
        lineage.work_admission == sources.work_admission
            && lineage.ready_health == sources.ready_health
            && lineage.host_runtime_observation == sources.host_runtime_observation,
        "user-node Ready source lineage differs from its retained source projection"
    );
    Ok(())
}

fn inert_effects() -> ComputeUserNodeReadySourceLineageEffectsV1 {
    ComputeUserNodeReadySourceLineageEffectsV1 {
        projection_effect: COMPUTE_USER_NODE_READY_SOURCE_PROJECTION_EFFECT.to_string(),
        readiness_effect: COMPUTE_USER_NODE_READY_SOURCE_NO_EFFECT.to_string(),
        provider_effect: COMPUTE_USER_NODE_READY_SOURCE_NO_EFFECT.to_string(),
        route_effect: COMPUTE_USER_NODE_READY_SOURCE_NO_EFFECT.to_string(),
        offer_effect: COMPUTE_USER_NODE_READY_SOURCE_NO_EFFECT.to_string(),
        capacity_effect: COMPUTE_USER_NODE_READY_SOURCE_NO_EFFECT.to_string(),
        execution_effect: COMPUTE_USER_NODE_READY_SOURCE_NO_EFFECT.to_string(),
        lease_effect: COMPUTE_USER_NODE_READY_SOURCE_NO_EFFECT.to_string(),
        settlement_effect: COMPUTE_USER_NODE_READY_SOURCE_NO_EFFECT.to_string(),
        money_effect: COMPUTE_USER_NODE_READY_SOURCE_NO_EFFECT.to_string(),
    }
}
