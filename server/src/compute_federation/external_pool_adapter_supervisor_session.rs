//! Server policy gate for the shared ephemeral external-pool Adapter session core.
//!
//! The reusable protocol implementation lives in `elon-external-pool-adapter-session-core` so the
//! trusted host and sealed child use one HKDF/HMAC/framing implementation. This wrapper binds it to
//! the current server-fixed V259 policy. It does not launch a process, deliver secrets, connect to
//! an upstream, or change Provider readiness.

use anyhow::{bail, Result};

pub(crate) use elon_external_pool_adapter_session_core::{
    prepare_external_pool_adapter_ephemeral_bundle_delivery,
    prepare_external_pool_adapter_supervisor_session, AuthenticatedExternalPoolAdapterSession,
    AuthenticatedExternalPoolAdapterSessionFrame, ExternalPoolAdapterChildBootstrap,
    ExternalPoolAdapterHostBootstrap, ExternalPoolAdapterSessionFrameKind,
    ExternalPoolAdapterSessionRootArguments, ExternalPoolAdapterSessionRoots,
    ExternalPoolAdapterSupervisorDescriptorTransfer, PreparedExternalPoolAdapterSupervisorSession,
};

use crate::compute_federation::external_pool_adapter_supervisor_session_policy_companion::server_supervisor_session_policy_catalog;

pub(crate) fn external_pool_adapter_session_roots(
    profile_digest: &str,
    target_digest: &str,
    companion_digest: &str,
    capsule_digest: &str,
    bundle_digest: &str,
) -> Result<ExternalPoolAdapterSessionRoots> {
    let (policy, policy_digest) = server_supervisor_session_policy_catalog()?;
    if policy.wire.transport != "anonymous_child_socketpair_seqpacket_v1"
        || policy.wire.protocol_id != "elon.external_pool_adapter.sidecar.v1"
        || policy.wire.protocol_revision != 1
        || policy.wire.frame_magic_ascii != "ELSP"
        || policy.crypto.kdf != "hkdf_sha256_extract_expand_v1"
        || policy.crypto.mac != "hmac_sha256_32_v1"
        || policy.crypto.seed_bytes != 32
        || policy.crypto.nonce_bytes != 32
        || policy.crypto.directional_key_bytes != 32
    {
        bail!("V259 supervisor/session policy is not compatible with the V260 session core");
    }
    ExternalPoolAdapterSessionRoots::new(
        &policy_digest,
        profile_digest,
        target_digest,
        companion_digest,
        capsule_digest,
        bundle_digest,
    )
}

/// Dedicated V268 public-fixture binding. None of these values occupies a production target,
/// companion, or secret-delivery slot; the shared core applies separate transcript/KDF domains.
#[allow(clippy::too_many_arguments)]
pub(crate) fn external_pool_adapter_runtime_compatibility_session_roots(
    supervisor_session_policy_digest: &str,
    runtime_compatibility_profile_digest: &str,
    challenge_digest: &str,
    runner_policy_digest: &str,
    fixture_catalog_digest: &str,
    sandbox_verifier_key_record_digest: &str,
    registry_release_digest: &str,
    installation_content_digest: &str,
    source_capsule_sha256: &str,
    launch_image_sha256: &str,
    public_fixture_delivery_root: &str,
) -> Result<ExternalPoolAdapterSessionRoots> {
    let (_, current_policy_digest) = server_supervisor_session_policy_catalog()?;
    if current_policy_digest != supervisor_session_policy_digest {
        bail!("V268 runtime compatibility session policy is not current and exact");
    }
    ExternalPoolAdapterSessionRoots::new_runtime_compatibility(
        supervisor_session_policy_digest,
        runtime_compatibility_profile_digest,
        challenge_digest,
        runner_policy_digest,
        fixture_catalog_digest,
        sandbox_verifier_key_record_digest,
        registry_release_digest,
        installation_content_digest,
        source_capsule_sha256,
        launch_image_sha256,
        public_fixture_delivery_root,
    )
}

/// Dedicated V272 controlled task-protocol binding. The synthetic lane/executor roots are
/// explicitly non-production and none of these values occupies a Provider target or Secret slot.
#[allow(clippy::too_many_arguments)]
pub(crate) fn external_pool_adapter_task_protocol_conformance_session_roots(
    supervisor_session_policy_digest: &str,
    task_protocol_profile_digest: &str,
    run_nonce_digest: &str,
    fixture_catalog_digest: &str,
    registry_release_digest: &str,
    installation_content_digest: &str,
    capability_set_digest: &str,
    sandbox_reattestation_receipt_digest: &str,
    runtime_compatibility_verification_receipt_digest: &str,
    source_capsule_sha256: &str,
    launch_image_sha256: &str,
    public_fixture_delivery_root: &str,
    synthetic_fixture_lane_digest: &str,
    synthetic_fixture_executor_digest: &str,
) -> Result<ExternalPoolAdapterSessionRoots> {
    let (_, current_policy_digest) = server_supervisor_session_policy_catalog()?;
    if current_policy_digest != supervisor_session_policy_digest {
        bail!("V272 task-protocol conformance session policy is not current and exact");
    }
    ExternalPoolAdapterSessionRoots::new_task_protocol_conformance(
        supervisor_session_policy_digest,
        task_protocol_profile_digest,
        run_nonce_digest,
        fixture_catalog_digest,
        registry_release_digest,
        installation_content_digest,
        capability_set_digest,
        sandbox_reattestation_receipt_digest,
        runtime_compatibility_verification_receipt_digest,
        source_capsule_sha256,
        launch_image_sha256,
        public_fixture_delivery_root,
        synthetic_fixture_lane_digest,
        synthetic_fixture_executor_digest,
    )
}

#[cfg(test)]
mod linux_tests;
