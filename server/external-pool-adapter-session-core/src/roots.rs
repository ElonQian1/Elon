use anyhow::{anyhow, bail, Context, Result};
use sha2::{Digest, Sha256};

const ROOT_TRANSCRIPT_DOMAIN: &[u8] = b"elon.external_pool_adapter.supervisor_session.roots.v1\0";
const KDF_SALT_DOMAIN: &[u8] = b"elon.external_pool_adapter.supervisor_session.kdf_salt.v1\0";

#[derive(Clone)]
pub struct ExternalPoolAdapterSessionRoots {
    policy_digest: [u8; 32],
    profile_digest: [u8; 32],
    target_digest: [u8; 32],
    companion_digest: [u8; 32],
    capsule_digest: [u8; 32],
    bundle_digest: [u8; 32],
}

impl ExternalPoolAdapterSessionRoots {
    pub fn new(
        policy_digest: &str,
        profile_digest: &str,
        target_digest: &str,
        companion_digest: &str,
        capsule_digest: &str,
        bundle_digest: &str,
    ) -> Result<Self> {
        Ok(Self {
            policy_digest: decode_digest("policy", policy_digest)?,
            profile_digest: decode_digest("profile", profile_digest)?,
            target_digest: decode_digest("target", target_digest)?,
            companion_digest: decode_digest("companion", companion_digest)?,
            capsule_digest: decode_digest("capsule", capsule_digest)?,
            bundle_digest: decode_digest("bundle", bundle_digest)?,
        })
    }

    pub fn launch_arguments(&self) -> ExternalPoolAdapterSessionRootArguments {
        ExternalPoolAdapterSessionRootArguments {
            values: [
                hex::encode(self.policy_digest),
                hex::encode(self.profile_digest),
                hex::encode(self.target_digest),
                hex::encode(self.companion_digest),
                hex::encode(self.capsule_digest),
                hex::encode(self.bundle_digest),
            ],
        }
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

#[derive(Clone)]
pub struct ExternalPoolAdapterSessionRootArguments {
    values: [String; 6],
}

impl ExternalPoolAdapterSessionRootArguments {
    pub fn values(&self) -> &[String; 6] {
        &self.values
    }

    #[cfg(feature = "test-support")]
    pub fn replace_for_test(&mut self, index: usize, value: String) {
        self.values[index] = value;
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
