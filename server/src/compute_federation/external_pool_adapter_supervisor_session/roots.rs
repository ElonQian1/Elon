use anyhow::{anyhow, bail, Context, Result};
use sha2::{Digest, Sha256};

use crate::compute_federation::external_pool_adapter_supervisor_session_policy_companion::server_supervisor_session_policy_catalog;

const ROOT_TRANSCRIPT_DOMAIN: &[u8] = b"elon.external_pool_adapter.supervisor_session.roots.v1\0";
const KDF_SALT_DOMAIN: &[u8] = b"elon.external_pool_adapter.supervisor_session.kdf_salt.v1\0";

pub(in crate::compute_federation) struct ExternalPoolAdapterSessionRoots {
    policy_digest: [u8; 32],
    profile_digest: [u8; 32],
    target_digest: [u8; 32],
    companion_digest: [u8; 32],
    capsule_digest: [u8; 32],
    bundle_digest: [u8; 32],
}

impl ExternalPoolAdapterSessionRoots {
    pub(in crate::compute_federation) fn new(
        profile_digest: &str,
        target_digest: &str,
        companion_digest: &str,
        capsule_digest: &str,
        bundle_digest: &str,
    ) -> Result<Self> {
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
        Ok(Self {
            policy_digest: decode_digest("policy", &policy_digest)?,
            profile_digest: decode_digest("profile", profile_digest)?,
            target_digest: decode_digest("target", target_digest)?,
            companion_digest: decode_digest("companion", companion_digest)?,
            capsule_digest: decode_digest("capsule", capsule_digest)?,
            bundle_digest: decode_digest("bundle", bundle_digest)?,
        })
    }

    pub(super) fn transcript_digest(&self) -> [u8; 32] {
        let mut digest = Sha256::new();
        digest.update(ROOT_TRANSCRIPT_DOMAIN);
        update_labeled_digest(&mut digest, b"policy\0", &self.policy_digest);
        update_labeled_digest(&mut digest, b"profile\0", &self.profile_digest);
        update_labeled_digest(&mut digest, b"target\0", &self.target_digest);
        update_labeled_digest(&mut digest, b"companion\0", &self.companion_digest);
        update_labeled_digest(&mut digest, b"capsule\0", &self.capsule_digest);
        update_labeled_digest(&mut digest, b"bundle\0", &self.bundle_digest);
        digest.finalize().into()
    }

    pub(super) fn kdf_salt(&self, host_nonce: &[u8; 32], child_nonce: &[u8; 32]) -> [u8; 32] {
        let mut digest = Sha256::new();
        digest.update(KDF_SALT_DOMAIN);
        update_labeled_digest(&mut digest, b"policy\0", &self.policy_digest);
        update_labeled_digest(&mut digest, b"profile\0", &self.profile_digest);
        update_labeled_digest(&mut digest, b"target\0", &self.target_digest);
        update_labeled_digest(&mut digest, b"host_nonce\0", host_nonce);
        update_labeled_digest(&mut digest, b"child_nonce\0", child_nonce);
        digest.finalize().into()
    }
}

fn update_labeled_digest(digest: &mut Sha256, label: &[u8], value: &[u8; 32]) {
    digest.update(label);
    digest.update(value);
}

fn decode_digest(label: &str, value: &str) -> Result<[u8; 32]> {
    if value.len() != 64
        || !value
            .as_bytes()
            .iter()
            .all(|byte| byte.is_ascii_digit() || matches!(*byte, b'a'..=b'f'))
    {
        bail!("{label} digest must be exact lowercase SHA-256 hex");
    }
    let mut output = [0_u8; 32];
    hex::decode_to_slice(value, &mut output)
        .with_context(|| format!("decode {label} SHA-256 digest"))?;
    if output.iter().all(|byte| *byte == 0) {
        return Err(anyhow!("{label} digest must not be all zero"));
    }
    Ok(output)
}
