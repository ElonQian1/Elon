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

#[cfg(test)]
mod linux_tests;
