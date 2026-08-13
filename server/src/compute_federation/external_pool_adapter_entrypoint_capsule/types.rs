use std::fs::File;

pub(in super::super) const ENTRYPOINT_CAPSULE_EFFECT: &str = "materialized_ephemeral";
pub(in super::super) const PROBE_OBSERVED: bool = false;
pub(in super::super) const RUNTIME_LAUNCH_READY: bool = false;
pub(in super::super) const ACTIVATION_READY: bool = false;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ExternalPoolAdapterEntrypointCapsuleError {
    InvalidAuthority,
    Unavailable,
    ContentDrift,
    UnsafeExecutable,
    MaterializationFailed,
}

/// An ephemeral, sealed image. It is intentionally neither Clone, Debug, nor serializable.
pub(in super::super) struct PreparedExternalPoolAdapterEntrypointCapsule {
    pub(super) sealed_image: File,
    pub(super) entrypoint_sha256: String,
    pub(super) entrypoint_size_bytes: u64,
    pub(super) policy_digest: String,
}

impl PreparedExternalPoolAdapterEntrypointCapsule {
    pub(in super::super) fn entrypoint_sha256(&self) -> &str {
        &self.entrypoint_sha256
    }

    pub(in super::super) fn entrypoint_size_bytes(&self) -> u64 {
        self.entrypoint_size_bytes
    }

    pub(in super::super) fn policy_digest(&self) -> &str {
        &self.policy_digest
    }
}
