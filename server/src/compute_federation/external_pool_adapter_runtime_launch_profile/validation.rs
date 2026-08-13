use anyhow::{bail, Result};
use chrono::{DateTime, SecondsFormat};

use crate::compute_federation::provider::{
    PROVIDER_KIND_EXTERNAL_POOL, PROVIDER_STATUS_REGISTERING,
};

use super::*;

const MAX_SAFE_INTEGER: u64 = 9_007_199_254_740_991;

pub(crate) fn validate_runtime_launch_profile_receipt(
    receipt: &ExternalPoolAdapterRuntimeLaunchProfileReceipt,
) -> Result<()> {
    metadata(
        &receipt.schema,
        RUNTIME_LAUNCH_PROFILE_SCHEMA,
        &receipt.profile_id,
        &receipt.profile_digest,
        &receipt.profile_material_digest,
        &receipt.canonicalization,
        &receipt.digest_algorithm,
    )?;
    let p = &receipt.profile;
    identifiers([
        &p.candidate_id,
        &p.delegation_id,
        &p.provider_binding_id,
        &p.registry_release_id,
        &p.installation_receipt_id,
        &p.route_adapter_projection_id,
        &p.provider_id,
        &p.provider_owner_account_id,
        &p.logical_adapter_id,
        &p.release_version,
        &p.service_actor_id,
        &p.recorded_by_actor_user_id,
        &p.idempotency_scope,
        &p.idempotency_key,
    ])?;
    digests([
        &p.candidate_digest,
        &p.delegation_digest,
        &p.provider_binding_digest,
        &p.registry_release_digest,
        &p.installation_receipt_digest,
        &p.installation_content_digest,
        &p.provider_digest,
        &p.implementation_digest,
        &p.capability_set_digest,
        &p.credential_verifier_digest,
        &p.credential_locator_commitment,
        &p.entrypoint_path_digest,
        &p.entrypoint_sha256,
        &p.entry_inventory_digest,
        &p.launch_policy_digest,
    ])?;
    optional_identifier_and_digest(&p.predecessor_profile_id, &p.predecessor_profile_digest)?;
    validate_relative_path(&p.entrypoint_relative_path)?;
    if runtime_launch_entrypoint_path_digest(&p.entrypoint_relative_path)
        != p.entrypoint_path_digest
    {
        bail!("runtime launch entrypoint path digest is not exact");
    }
    if p.provider_status != PROVIDER_STATUS_REGISTERING
        || p.profile_status != RUNTIME_LAUNCH_PROFILE_STATUS
        || p.profile_effect != RUNTIME_LAUNCH_PROFILE_EFFECT
        || p.confirmation != RUNTIME_LAUNCH_PROFILE_CONFIRMATION
        || p.credential_ref_scheme != "vault_ref"
        || !actor(&p.recorded_by_actor_kind)
        || !opaque_digest(&p.adapter_config_digest)
        || !positive(p.provider_policy_revision)
        || !positive(p.adapter_config_revision)
        || !safe_positive(p.entrypoint_size_bytes)
        || !safe_positive(p.installed_file_count)
        || !safe_positive(p.installed_total_bytes)
        || !safe_positive(p.sequence)
        || !paired(&p.predecessor_profile_id, &p.predecessor_profile_digest)
        || !no_effects([
            &p.runtime_effect,
            &p.adapter_effect,
            &p.provider_effect,
            &p.credential_effect,
            &p.route_effect,
            &p.execution_effect,
            &p.usage_effect,
            &p.market_effect,
            &p.settlement_effect,
        ])
    {
        bail!("runtime launch profile material is not exact");
    }
    canonical_nanos(&p.recorded_at)?;
    validate_policy(&p.launch_policy)?;
    if runtime_launch_policy_digest(&p.launch_policy)? != p.launch_policy_digest {
        bail!("runtime launch profile policy digest is not exact");
    }
    exact_digests(
        runtime_launch_profile_material_digest(p)?,
        &receipt.profile_material_digest,
        canonical_runtime_launch_profile_json_and_digest(receipt)?.1,
        &receipt.profile_digest,
    )
}

pub(crate) fn validate_runtime_launch_profile_revocation_receipt(
    receipt: &ExternalPoolAdapterRuntimeLaunchProfileRevocationReceipt,
) -> Result<()> {
    metadata(
        &receipt.schema,
        RUNTIME_LAUNCH_PROFILE_REVOCATION_SCHEMA,
        &receipt.revocation_id,
        &receipt.revocation_digest,
        &receipt.revocation_material_digest,
        &receipt.canonicalization,
        &receipt.digest_algorithm,
    )?;
    let r = &receipt.revocation;
    identifiers([
        &r.profile_id,
        &r.provider_binding_id,
        &r.candidate_id,
        &r.revoked_by_actor_user_id,
        &r.idempotency_scope,
        &r.idempotency_key,
    ])?;
    digests([
        &r.profile_digest,
        &r.provider_binding_digest,
        &r.candidate_digest,
    ])?;
    if !actor(&r.revoked_by_actor_kind)
        || !reason(&r.reason)
        || r.revoked_at != r.recorded_at
        || r.confirmation != RUNTIME_LAUNCH_PROFILE_REVOCATION_CONFIRMATION
        || r.revocation_effect != RUNTIME_LAUNCH_PROFILE_REVOCATION_EFFECT
        || !no_effects([
            &r.runtime_effect,
            &r.adapter_effect,
            &r.provider_effect,
            &r.credential_effect,
            &r.route_effect,
            &r.execution_effect,
            &r.usage_effect,
            &r.market_effect,
            &r.settlement_effect,
        ])
    {
        bail!("runtime launch profile revocation material is not exact");
    }
    canonical_nanos(&r.revoked_at)?;
    exact_digests(
        runtime_launch_profile_revocation_material_digest(r)?,
        &receipt.revocation_material_digest,
        canonical_runtime_launch_profile_revocation_json_and_digest(receipt)?.1,
        &receipt.revocation_digest,
    )
}

pub(crate) fn validate_runtime_launch_policy(
    policy: &ExternalPoolAdapterRuntimeLaunchPolicy,
) -> Result<()> {
    validate_policy(policy)
}

fn validate_policy(p: &ExternalPoolAdapterRuntimeLaunchPolicy) -> Result<()> {
    if p != &super::policy::runtime_launch_policy_for_host(&p.host_os, &p.host_arch)? {
        bail!("runtime launch policy differs from the server-fixed catalog");
    }
    identifiers([
        &p.policy_id,
        &p.runtime_kind,
        &p.host_os,
        &p.host_arch,
        &p.host_environment,
        &p.executable_kind,
        &p.binary_format,
        &p.executable_verification_status,
        &p.materialization_kind,
        &p.argv_policy,
        &p.environment_policy,
        &p.working_directory_policy,
        &p.ipc_transport,
        &p.sidecar_protocol_id,
        &p.ipc_framing,
        &p.ipc_session_auth,
        &p.config_resolver_kind,
        &p.credential_resolver_kind,
        &p.resolver_backend_policy_id,
        &p.config_delivery_kind,
        &p.credential_delivery_kind,
        &p.secret_custody_policy,
        &p.probe_contract,
        &p.process_isolation_policy_id,
        &p.resource_policy_id,
        &p.network_egress_policy_id,
    ])?;
    digests([
        &p.resolver_backend_policy_digest,
        &p.process_isolation_policy_digest,
        &p.resource_policy_digest,
        &p.network_egress_policy_digest,
    ])?;
    if p.policy_id != RUNTIME_LAUNCH_POLICY_ID
        || p.policy_revision != RUNTIME_LAUNCH_POLICY_REVISION
        || p.runtime_kind != "server_sidecar_v1"
        || !supported_host(
            &p.host_os,
            &p.host_arch,
            &p.host_environment,
            &p.binary_format,
        )
        || p.executable_kind != "native_process_image_v1"
        || p.executable_verification_status != "deferred_to_runtime_supervisor"
        || p.materialization_kind != "copy_from_retained_handle_create_new_private_executable_v1"
        || p.shell_allowed
        || p.argv_policy != "empty_no_shell_v1"
        || p.environment_policy != "empty_allowlisted_runtime_v1"
        || p.working_directory_policy != "isolated_private_runtime_directory_v1"
        || p.ipc_transport != "anonymous_child_pipe_v1"
        || p.sidecar_protocol_id != "elon.external_pool_adapter.sidecar.v1"
        || p.sidecar_protocol_revision != 1
        || p.ipc_framing != "u32_be_length_prefixed_utf8_jcs_v1"
        || p.ipc_session_auth != "host_nonce_hmac_sha256_v1"
        || p.config_resolver_kind != "operator_mounted_runtime_bundle_v1"
        || p.credential_resolver_kind != "operator_mounted_runtime_bundle_v1"
        || p.config_delivery_kind != "authenticated_sensitive_frame_v1"
        || p.credential_delivery_kind != "authenticated_sensitive_frame_v1"
        || p.secret_custody_policy != "memory_only_no_argv_env_log_db_http_v1"
        || p.probe_contract != "authenticated_no_work_readiness_v1"
        || !safe_positive(p.max_frame_bytes)
        || !safe_positive(p.startup_timeout_ms)
        || !safe_positive(p.handshake_timeout_ms)
        || !safe_positive(p.probe_timeout_ms)
        || !safe_positive(p.shutdown_timeout_ms)
        || p.max_sidecar_processes != 1
        || !safe_positive(p.max_stderr_bytes)
        || !safe_positive(p.max_runtime_temp_bytes)
        || !safe_positive(p.resolver_backend_policy_revision)
        || !safe_positive(p.process_isolation_policy_revision)
        || !safe_positive(p.resource_policy_revision)
        || !safe_positive(p.network_egress_policy_revision)
    {
        bail!("runtime launch policy is not exact");
    }
    Ok(())
}

fn metadata(
    schema: &str,
    expected: &str,
    id: &str,
    digest: &str,
    material_digest: &str,
    canonicalization: &str,
    algorithm: &str,
) -> Result<()> {
    identifiers([id])?;
    digests([digest, material_digest])?;
    if schema != expected
        || canonicalization != RUNTIME_LAUNCH_PROFILE_CANONICALIZATION
        || algorithm != RUNTIME_LAUNCH_PROFILE_DIGEST_ALGORITHM
    {
        bail!("runtime launch receipt metadata is not exact");
    }
    Ok(())
}

fn identifiers<S: AsRef<str>>(values: impl IntoIterator<Item = S>) -> Result<()> {
    for value in values {
        let value = value.as_ref();
        if value.is_empty()
            || value.trim() != value
            || value.chars().count() > 240
            || value.chars().any(char::is_control)
        {
            bail!("runtime launch identifier is invalid");
        }
    }
    Ok(())
}

fn digests<S: AsRef<str>>(values: impl IntoIterator<Item = S>) -> Result<()> {
    for value in values {
        let value = value.as_ref();
        if value.len() != 64
            || !value
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            bail!("runtime launch digest is invalid");
        }
    }
    Ok(())
}

fn optional_identifier_and_digest(id: &Option<String>, digest: &Option<String>) -> Result<()> {
    if id.is_some() != digest.is_some() {
        bail!("runtime launch predecessor identity is incomplete");
    }
    if let Some(id) = id {
        identifiers([id])?;
    }
    if let Some(digest) = digest {
        digests([digest])?;
    }
    Ok(())
}

fn exact_digests(a: String, b: &str, c: String, d: &str) -> Result<()> {
    if a != b || c != d {
        bail!("runtime launch receipt digest is not exact");
    }
    Ok(())
}

fn canonical_nanos(value: &str) -> Result<()> {
    let parsed = DateTime::parse_from_rfc3339(value)?;
    if parsed.offset().local_minus_utc() != 0
        || parsed.to_rfc3339_opts(SecondsFormat::Nanos, true) != value
    {
        bail!("runtime launch timestamp is not canonical UTC nanoseconds");
    }
    Ok(())
}

fn actor(value: &str) -> bool {
    matches!(
        value,
        RUNTIME_LAUNCH_PROFILE_ACTOR_PROVIDER_OWNER | RUNTIME_LAUNCH_PROFILE_ACTOR_PLATFORM_ADMIN
    )
}

fn opaque_digest(value: &str) -> bool {
    !value.is_empty()
        && value.trim() == value
        && value.chars().count() <= 512
        && !value.chars().any(char::is_control)
}

fn paired(id: &Option<String>, digest: &Option<String>) -> bool {
    id.is_some() == digest.is_some()
}

fn positive(value: i64) -> bool {
    value > 0
}

fn safe_positive(value: u64) -> bool {
    (1..=MAX_SAFE_INTEGER).contains(&value)
}

fn reason(value: &str) -> bool {
    value.trim() == value
        && (12..=500).contains(&value.chars().count())
        && !value.chars().any(char::is_control)
}

fn no_effects<'a>(values: impl IntoIterator<Item = &'a String>) -> bool {
    values
        .into_iter()
        .all(|value| value == RUNTIME_LAUNCH_PROFILE_NO_EFFECT)
}

fn validate_relative_path(value: &str) -> Result<()> {
    if value.is_empty()
        || value.len() > 160
        || value.contains('\\')
        || value.starts_with('/')
        || value.ends_with('/')
        || value.chars().any(char::is_control)
        || value
            .split('/')
            .any(|part| part.is_empty() || matches!(part, "." | ".."))
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-' | b'/'))
    {
        bail!("runtime launch entrypoint relative path is invalid");
    }
    Ok(())
}

fn supported_host(os: &str, arch: &str, environment: &str, binary_format: &str) -> bool {
    arch == "x86_64"
        && matches!(
            (os, environment, binary_format),
            ("windows", "windows_native_process_v1", "pe_coff_native_v1")
                | ("linux", "linux_native_process_v1", "elf_native_v1")
        )
}

#[allow(dead_code)]
fn _external_pool_marker() -> &'static str {
    PROVIDER_KIND_EXTERNAL_POOL
}
