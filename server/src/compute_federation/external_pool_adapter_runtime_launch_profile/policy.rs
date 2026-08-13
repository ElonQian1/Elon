use anyhow::Result;
use sha2::{Digest, Sha256};

use super::{
    runtime_launch_policy_digest, validate_runtime_launch_policy,
    ExternalPoolAdapterRuntimeLaunchPolicy, RUNTIME_LAUNCH_POLICY_ID,
    RUNTIME_LAUNCH_POLICY_REVISION,
};

pub(crate) fn server_runtime_launch_policy_catalog(
) -> Result<(ExternalPoolAdapterRuntimeLaunchPolicy, String)> {
    let policy = runtime_launch_policy_for_host(std::env::consts::OS, std::env::consts::ARCH)?;
    validate_runtime_launch_policy(&policy)?;
    let digest = runtime_launch_policy_digest(&policy)?;
    Ok((policy, digest))
}

pub(super) fn runtime_launch_policy_for_host(
    requested_os: &str,
    requested_arch: &str,
) -> Result<ExternalPoolAdapterRuntimeLaunchPolicy> {
    let host_os = normalized(requested_os);
    let host_arch = normalized_arch(requested_arch);
    if !matches!(host_os.as_str(), "windows" | "linux") || host_arch != "x86_64" {
        anyhow::bail!("external-pool runtime launch policy has no supported host profile");
    }
    let (host_environment, binary_format) = match host_os.as_str() {
        "windows" => ("windows_native_process_v1", "pe_coff_native_v1"),
        "linux" => ("linux_native_process_v1", "elf_native_v1"),
        _ => unreachable!("host OS was checked above"),
    };
    Ok(ExternalPoolAdapterRuntimeLaunchPolicy {
        policy_id: RUNTIME_LAUNCH_POLICY_ID.into(),
        policy_revision: RUNTIME_LAUNCH_POLICY_REVISION,
        runtime_kind: "server_sidecar_v1".into(),
        host_os,
        host_arch,
        host_environment: host_environment.into(),
        executable_kind: "native_process_image_v1".into(),
        binary_format: binary_format.into(),
        executable_verification_status: "deferred_to_runtime_supervisor".into(),
        materialization_kind: "copy_from_retained_handle_create_new_private_executable_v1".into(),
        shell_allowed: false,
        argv_policy: "empty_no_shell_v1".into(),
        environment_policy: "empty_allowlisted_runtime_v1".into(),
        working_directory_policy: "isolated_private_runtime_directory_v1".into(),
        ipc_transport: "anonymous_child_pipe_v1".into(),
        sidecar_protocol_id: "elon.external_pool_adapter.sidecar.v1".into(),
        sidecar_protocol_revision: 1,
        ipc_framing: "u32_be_length_prefixed_utf8_jcs_v1".into(),
        max_frame_bytes: 1_048_576,
        ipc_session_auth: "host_nonce_hmac_sha256_v1".into(),
        config_resolver_kind: "operator_mounted_runtime_bundle_v1".into(),
        credential_resolver_kind: "operator_mounted_runtime_bundle_v1".into(),
        resolver_backend_policy_id: "operator_mounted_runtime_bundle_policy_v1".into(),
        resolver_backend_policy_revision: 1,
        resolver_backend_policy_digest: fixed_digest(
            b"ELON-EXTERNAL-POOL-OPERATOR-MOUNTED-RUNTIME-BUNDLE-POLICY-V1",
        ),
        config_delivery_kind: "authenticated_sensitive_frame_v1".into(),
        credential_delivery_kind: "authenticated_sensitive_frame_v1".into(),
        secret_custody_policy: "memory_only_no_argv_env_log_db_http_v1".into(),
        probe_contract: "authenticated_no_work_readiness_v1".into(),
        process_isolation_policy_id: "external_pool_sidecar_process_isolation_v1".into(),
        process_isolation_policy_revision: 1,
        process_isolation_policy_digest: fixed_digest(
            b"ELON-EXTERNAL-POOL-SIDECAR-PROCESS-ISOLATION-POLICY-V1",
        ),
        resource_policy_id: "external_pool_sidecar_resource_policy_v1".into(),
        resource_policy_revision: 1,
        resource_policy_digest: fixed_digest(b"ELON-EXTERNAL-POOL-SIDECAR-RESOURCE-POLICY-V1"),
        network_egress_policy_id: "external_pool_sidecar_network_egress_policy_v1".into(),
        network_egress_policy_revision: 1,
        network_egress_policy_digest: fixed_digest(
            b"ELON-EXTERNAL-POOL-SIDECAR-NETWORK-EGRESS-POLICY-V1",
        ),
        startup_timeout_ms: 10_000,
        handshake_timeout_ms: 5_000,
        probe_timeout_ms: 15_000,
        shutdown_timeout_ms: 5_000,
        max_sidecar_processes: 1,
        max_stderr_bytes: 1_048_576,
        max_runtime_temp_bytes: 67_108_864,
    })
}

fn normalized(value: &str) -> String {
    value.replace('-', "_").to_ascii_lowercase()
}

fn normalized_arch(value: &str) -> String {
    match normalized(value).as_str() {
        "x86_64" | "amd64" => "x86_64".into(),
        other => other.into(),
    }
}

fn fixed_digest(domain: &[u8]) -> String {
    let mut digest = Sha256::new();
    digest.update(domain);
    digest.update([0]);
    digest.update(b"revision=1");
    hex::encode(digest.finalize())
}
