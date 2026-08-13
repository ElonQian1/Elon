use sha2::{Digest, Sha256};

use super::types::ExternalPoolAdapterEntrypointCapsuleError;

pub(super) const ENTRYPOINT_CAPSULE_POLICY_ID: &str =
    "external_pool_adapter_entrypoint_capsule_policy_v1";
pub(super) const ENTRYPOINT_CAPSULE_POLICY_REVISION: u64 = 1;
pub(super) const ENTRYPOINT_CAPSULE_POLICY_DOMAIN: &[u8] = b"ELON-EXTERNAL-POOL-ADAPTER-ENTRYPOINT-CAPSULE-POLICY-V1\0revision=1\0linux\0x86_64\0elf64-le\0et_exec\0static-no-interp-no-dynamic\0no-wx\0sealed-memfd-v1";

pub(super) struct ExternalPoolAdapterEntrypointCapsulePolicy {
    pub(super) policy_id: &'static str,
    pub(super) policy_revision: u64,
    pub(super) host_os: &'static str,
    pub(super) host_arch: &'static str,
    pub(super) binary_format: &'static str,
    pub(super) executable_type: &'static str,
    pub(super) linking_policy: &'static str,
    pub(super) segment_policy: &'static str,
    pub(super) materialization: &'static str,
    pub(super) policy_digest: String,
}

pub(super) fn entrypoint_capsule_policy(
) -> Result<ExternalPoolAdapterEntrypointCapsulePolicy, ExternalPoolAdapterEntrypointCapsuleError> {
    let policy = ExternalPoolAdapterEntrypointCapsulePolicy {
        policy_id: ENTRYPOINT_CAPSULE_POLICY_ID,
        policy_revision: ENTRYPOINT_CAPSULE_POLICY_REVISION,
        host_os: "linux",
        host_arch: "x86_64",
        binary_format: "elf64-le",
        executable_type: "et_exec",
        linking_policy: "static-no-interp-no-dynamic",
        segment_policy: "no-wx",
        materialization: "sealed-memfd-v1",
        policy_digest: hex::encode(Sha256::digest(ENTRYPOINT_CAPSULE_POLICY_DOMAIN)),
    };
    policy.validate()?;
    Ok(policy)
}

impl ExternalPoolAdapterEntrypointCapsulePolicy {
    fn validate(&self) -> Result<(), ExternalPoolAdapterEntrypointCapsuleError> {
        if self.policy_id != ENTRYPOINT_CAPSULE_POLICY_ID
            || self.policy_revision != ENTRYPOINT_CAPSULE_POLICY_REVISION
            || self.host_os != "linux"
            || self.host_arch != "x86_64"
            || self.binary_format != "elf64-le"
            || self.executable_type != "et_exec"
            || self.linking_policy != "static-no-interp-no-dynamic"
            || self.segment_policy != "no-wx"
            || self.materialization != "sealed-memfd-v1"
            || self.policy_digest != hex::encode(Sha256::digest(ENTRYPOINT_CAPSULE_POLICY_DOMAIN))
        {
            return Err(ExternalPoolAdapterEntrypointCapsuleError::InvalidAuthority);
        }
        Ok(())
    }
}
