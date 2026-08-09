use anyhow::Result;
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::compute_plugin_sharing_directive::canonical_compute_plugin_ijson_and_sha256;

use super::types::{
    ComputeArtifactAccessBinding, ComputeArtifactAccessEnvelope,
    ComputeAttemptExecutionPlanEnvelope, ComputeAttemptExecutionPlanSealEnvelope,
    ComputeExecutionCapabilityEnvelope, ComputeExecutionResourceGrant,
};
use crate::compute_federation::workload::{ComputeArtifactRef, ComputeWorkloadSpec};

const MAX_EXECUTION_PLAN_JSON_BYTES: usize = 2 * 1024 * 1024;
const CAPABILITY_DIGEST_DOMAIN: &[u8] = b"ELON-COMPUTE-EXECUTION-CAPABILITY-V1";
const ARTIFACT_ACCESS_DIGEST_DOMAIN: &[u8] = b"ELON-COMPUTE-ARTIFACT-ACCESS-V1";
const EXECUTION_PLAN_DIGEST_DOMAIN: &[u8] = b"ELON-COMPUTE-ATTEMPT-EXECUTION-PLAN-V1";
const RESOURCE_GRANT_DIGEST_DOMAIN: &[u8] = b"ELON-COMPUTE-EXECUTION-RESOURCE-GRANT-V1";
const EXECUTION_PLAN_SEAL_DIGEST_DOMAIN: &[u8] = b"ELON-COMPUTE-ATTEMPT-EXECUTION-PLAN-SEAL-V1";
const PLAN_ACCESS_SET_DIGEST_DOMAIN: &[u8] = b"ELON-COMPUTE-PLAN-ACCESS-SET-V1";
const WORKLOAD_SPEC_DIGEST_DOMAIN: &[u8] = b"ELON-COMPUTE-WORKLOAD-SPEC-V1";
const CANONICAL_INPUT_DIGEST_DOMAIN: &[u8] = b"ELON-COMPUTE-CANONICAL-INPUT-V1";

pub(crate) fn canonical_execution_capability_json_and_digest(
    envelope: &ComputeExecutionCapabilityEnvelope,
) -> Result<(String, String)> {
    #[derive(Serialize)]
    struct Projection<'a> {
        schema: &'a str,
        capability_id: &'a str,
        canonicalization: &'a str,
        digest_algorithm: &'a str,
        capability: &'a super::types::ComputeExecutionCapability,
    }
    envelope_json_and_digest(
        CAPABILITY_DIGEST_DOMAIN,
        &Projection {
            schema: &envelope.schema,
            capability_id: &envelope.capability_id,
            canonicalization: &envelope.canonicalization,
            digest_algorithm: &envelope.digest_algorithm,
            capability: &envelope.capability,
        },
        envelope,
    )
}

pub(crate) fn canonical_artifact_access_json_and_digest(
    envelope: &ComputeArtifactAccessEnvelope,
) -> Result<(String, String)> {
    #[derive(Serialize)]
    struct Projection<'a> {
        schema: &'a str,
        access_id: &'a str,
        canonicalization: &'a str,
        digest_algorithm: &'a str,
        access: &'a super::types::ComputeArtifactAccess,
    }
    envelope_json_and_digest(
        ARTIFACT_ACCESS_DIGEST_DOMAIN,
        &Projection {
            schema: &envelope.schema,
            access_id: &envelope.access_id,
            canonicalization: &envelope.canonicalization,
            digest_algorithm: &envelope.digest_algorithm,
            access: &envelope.access,
        },
        envelope,
    )
}

pub(crate) fn canonical_execution_plan_json_and_digest(
    envelope: &ComputeAttemptExecutionPlanEnvelope,
) -> Result<(String, String)> {
    #[derive(Serialize)]
    struct Projection<'a> {
        schema: &'a str,
        plan_id: &'a str,
        canonicalization: &'a str,
        digest_algorithm: &'a str,
        plan: &'a super::types::ComputeAttemptExecutionPlan,
    }
    envelope_json_and_digest(
        EXECUTION_PLAN_DIGEST_DOMAIN,
        &Projection {
            schema: &envelope.schema,
            plan_id: &envelope.plan_id,
            canonicalization: &envelope.canonicalization,
            digest_algorithm: &envelope.digest_algorithm,
            plan: &envelope.plan,
        },
        envelope,
    )
}

pub(crate) fn canonical_resource_grant_json_and_digest(
    grant: &ComputeExecutionResourceGrant,
) -> Result<(String, String)> {
    #[derive(Serialize)]
    struct Projection<'a> {
        schema: &'a str,
        grant_id: &'a str,
        enforcement_kind: &'a str,
        accelerator_count: i64,
        cpu_millicores: i64,
        memory_bytes: i64,
        vram_bytes: i64,
        disk_bytes: i64,
        max_processes: i64,
        max_runtime_seconds: i64,
        max_output_bytes: i64,
        concurrency_units: i64,
        allow_network_egress: bool,
        usage_limits: &'a [crate::compute_federation::attempt::ComputeAttemptUsageLimit],
    }
    let projection = Projection {
        schema: &grant.schema,
        grant_id: &grant.grant_id,
        enforcement_kind: &grant.enforcement_kind,
        accelerator_count: grant.accelerator_count,
        cpu_millicores: grant.cpu_millicores,
        memory_bytes: grant.memory_bytes,
        vram_bytes: grant.vram_bytes,
        disk_bytes: grant.disk_bytes,
        max_processes: grant.max_processes,
        max_runtime_seconds: grant.max_runtime_seconds,
        max_output_bytes: grant.max_output_bytes,
        concurrency_units: grant.concurrency_units,
        allow_network_egress: grant.allow_network_egress,
        usage_limits: &grant.usage_limits,
    };
    envelope_json_and_digest(RESOURCE_GRANT_DIGEST_DOMAIN, &projection, grant)
}

pub(crate) fn canonical_execution_plan_seal_json_and_digest(
    seal: &ComputeAttemptExecutionPlanSealEnvelope,
) -> Result<(String, String)> {
    #[derive(Serialize)]
    struct Projection<'a> {
        schema: &'a str,
        seal_id: &'a str,
        canonicalization: &'a str,
        digest_algorithm: &'a str,
        plan_id: &'a str,
        plan_digest: &'a str,
        capability_digest: &'a str,
        artifact_access_count: i64,
        artifact_access_set_digest: &'a str,
        resource_grant_digest: &'a str,
        sealed_at: &'a str,
    }
    envelope_json_and_digest(
        EXECUTION_PLAN_SEAL_DIGEST_DOMAIN,
        &Projection {
            schema: &seal.schema,
            seal_id: &seal.seal_id,
            canonicalization: &seal.canonicalization,
            digest_algorithm: &seal.digest_algorithm,
            plan_id: &seal.plan_id,
            plan_digest: &seal.plan_digest,
            capability_digest: &seal.capability_digest,
            artifact_access_count: seal.artifact_access_count,
            artifact_access_set_digest: &seal.artifact_access_set_digest,
            resource_grant_digest: &seal.resource_grant_digest,
            sealed_at: &seal.sealed_at,
        },
        seal,
    )
}

pub(crate) fn canonical_plan_access_set_digest(
    accesses: &[ComputeArtifactAccessBinding],
) -> Result<String> {
    domain_digest(PLAN_ACCESS_SET_DIGEST_DOMAIN, &accesses)
}

pub(crate) fn canonical_workload_spec_digest(workload: &ComputeWorkloadSpec) -> Result<String> {
    domain_digest(WORKLOAD_SPEC_DIGEST_DOMAIN, workload)
}

pub(crate) fn canonical_input_digest(artifacts: &[ComputeArtifactRef]) -> Result<String> {
    #[derive(Serialize)]
    struct Input<'a> {
        artifact_id: &'a str,
        digest_algorithm: &'a str,
        digest: &'a str,
        media_type: &'a str,
        size_bytes: i64,
        encryption_profile: &'a Option<String>,
    }
    let inputs = artifacts
        .iter()
        .map(|artifact| Input {
            artifact_id: &artifact.artifact_id,
            digest_algorithm: &artifact.digest_algorithm,
            digest: &artifact.digest,
            media_type: &artifact.media_type,
            size_bytes: artifact.size_bytes,
            encryption_profile: &artifact.encryption_profile,
        })
        .collect::<Vec<_>>();
    domain_digest(CANONICAL_INPUT_DIGEST_DOMAIN, &inputs)
}

fn envelope_json_and_digest<P: Serialize, E: Serialize>(
    domain: &[u8],
    projection: &P,
    envelope: &E,
) -> Result<(String, String)> {
    let digest = domain_digest(domain, projection)?;
    let (json, _) =
        canonical_compute_plugin_ijson_and_sha256(envelope, MAX_EXECUTION_PLAN_JSON_BYTES)?;
    Ok((json, digest))
}

fn domain_digest<T: Serialize>(domain: &[u8], value: &T) -> Result<String> {
    let (json, _) =
        canonical_compute_plugin_ijson_and_sha256(value, MAX_EXECUTION_PLAN_JSON_BYTES)?;
    let mut digest = Sha256::new();
    digest.update(domain);
    digest.update([0]);
    digest.update(json.as_bytes());
    Ok(hex::encode(digest.finalize()))
}
