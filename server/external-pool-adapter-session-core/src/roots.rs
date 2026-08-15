use anyhow::{anyhow, bail, Context, Result};
use sha2::{Digest, Sha256};

mod task_conformance;

use task_conformance::ExternalPoolAdapterTaskProtocolConformanceRoots;

const ROOT_TRANSCRIPT_DOMAIN: &[u8] = b"elon.external_pool_adapter.supervisor_session.roots.v1\0";
const KDF_SALT_DOMAIN: &[u8] = b"elon.external_pool_adapter.supervisor_session.kdf_salt.v1\0";
const RUNTIME_COMPATIBILITY_ROOT_TRANSCRIPT_DOMAIN: &[u8] =
    b"elon.external_pool_adapter.runtime_compatibility_verification.session.roots.v1\0";
const RUNTIME_COMPATIBILITY_KDF_SALT_DOMAIN: &[u8] =
    b"elon.external_pool_adapter.runtime_compatibility_verification.session.kdf_salt.v1\0";

#[derive(Clone)]
pub struct ExternalPoolAdapterSessionRoots {
    roots: ExternalPoolAdapterSessionRootSet,
}

#[derive(Clone)]
enum ExternalPoolAdapterSessionRootSet {
    Production {
        policy_digest: [u8; 32],
        profile_digest: [u8; 32],
        target_digest: [u8; 32],
        companion_digest: [u8; 32],
        capsule_digest: [u8; 32],
        bundle_digest: [u8; 32],
    },
    RuntimeCompatibility {
        supervisor_session_policy_digest: [u8; 32],
        runtime_compatibility_profile_digest: [u8; 32],
        challenge_digest: [u8; 32],
        runner_policy_digest: [u8; 32],
        fixture_catalog_digest: [u8; 32],
        sandbox_verifier_key_record_digest: [u8; 32],
        registry_release_digest: [u8; 32],
        installation_content_digest: [u8; 32],
        source_capsule_sha256: [u8; 32],
        launch_image_sha256: [u8; 32],
        public_fixture_delivery_root: [u8; 32],
    },
    TaskProtocolConformance(ExternalPoolAdapterTaskProtocolConformanceRoots),
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
            roots: ExternalPoolAdapterSessionRootSet::Production {
                policy_digest: decode_digest("policy", policy_digest)?,
                profile_digest: decode_digest("profile", profile_digest)?,
                target_digest: decode_digest("target", target_digest)?,
                companion_digest: decode_digest("companion", companion_digest)?,
                capsule_digest: decode_digest("capsule", capsule_digest)?,
                bundle_digest: decode_digest("bundle", bundle_digest)?,
            },
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn new_runtime_compatibility(
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
    ) -> Result<Self> {
        Ok(Self {
            roots: ExternalPoolAdapterSessionRootSet::RuntimeCompatibility {
                supervisor_session_policy_digest: decode_digest(
                    "runtime compatibility supervisor/session policy",
                    supervisor_session_policy_digest,
                )?,
                runtime_compatibility_profile_digest: decode_digest(
                    "runtime compatibility profile",
                    runtime_compatibility_profile_digest,
                )?,
                challenge_digest: decode_digest(
                    "runtime compatibility challenge",
                    challenge_digest,
                )?,
                runner_policy_digest: decode_digest(
                    "runtime compatibility runner policy",
                    runner_policy_digest,
                )?,
                fixture_catalog_digest: decode_digest(
                    "runtime compatibility fixture catalog",
                    fixture_catalog_digest,
                )?,
                sandbox_verifier_key_record_digest: decode_digest(
                    "runtime compatibility sandbox verifier key record",
                    sandbox_verifier_key_record_digest,
                )?,
                registry_release_digest: decode_digest(
                    "runtime compatibility registry release",
                    registry_release_digest,
                )?,
                installation_content_digest: decode_digest(
                    "runtime compatibility installation content",
                    installation_content_digest,
                )?,
                source_capsule_sha256: decode_digest(
                    "runtime compatibility source capsule",
                    source_capsule_sha256,
                )?,
                launch_image_sha256: decode_digest(
                    "runtime compatibility launch image",
                    launch_image_sha256,
                )?,
                public_fixture_delivery_root: decode_digest(
                    "runtime compatibility public fixture delivery",
                    public_fixture_delivery_root,
                )?,
            },
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn new_task_protocol_conformance(
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
    ) -> Result<Self> {
        Ok(Self {
            roots: ExternalPoolAdapterSessionRootSet::TaskProtocolConformance(
                ExternalPoolAdapterTaskProtocolConformanceRoots::new(
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
                )?,
            ),
        })
    }

    pub fn launch_arguments(&self) -> ExternalPoolAdapterSessionRootArguments {
        let values = match &self.roots {
            ExternalPoolAdapterSessionRootSet::Production {
                policy_digest,
                profile_digest,
                target_digest,
                companion_digest,
                capsule_digest,
                bundle_digest,
            } => ExternalPoolAdapterSessionRootArgumentValues::Production([
                hex::encode(policy_digest),
                hex::encode(profile_digest),
                hex::encode(target_digest),
                hex::encode(companion_digest),
                hex::encode(capsule_digest),
                hex::encode(bundle_digest),
            ]),
            ExternalPoolAdapterSessionRootSet::RuntimeCompatibility {
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
            } => ExternalPoolAdapterSessionRootArgumentValues::RuntimeCompatibility([
                hex::encode(supervisor_session_policy_digest),
                hex::encode(runtime_compatibility_profile_digest),
                hex::encode(challenge_digest),
                hex::encode(runner_policy_digest),
                hex::encode(fixture_catalog_digest),
                hex::encode(sandbox_verifier_key_record_digest),
                hex::encode(registry_release_digest),
                hex::encode(installation_content_digest),
                hex::encode(source_capsule_sha256),
                hex::encode(launch_image_sha256),
                hex::encode(public_fixture_delivery_root),
            ]),
            ExternalPoolAdapterSessionRootSet::TaskProtocolConformance(roots) => {
                ExternalPoolAdapterSessionRootArgumentValues::TaskProtocolConformance(
                    roots.launch_values(),
                )
            }
        };
        ExternalPoolAdapterSessionRootArguments { values }
    }

    pub(super) fn transcript_digest(&self) -> [u8; 32] {
        match &self.roots {
            ExternalPoolAdapterSessionRootSet::Production {
                policy_digest,
                profile_digest,
                target_digest,
                companion_digest,
                capsule_digest,
                bundle_digest,
            } => {
                let mut digest = Sha256::new();
                digest.update(ROOT_TRANSCRIPT_DOMAIN);
                update_labeled_digest(&mut digest, b"policy\0", policy_digest);
                update_labeled_digest(&mut digest, b"profile\0", profile_digest);
                update_labeled_digest(&mut digest, b"target\0", target_digest);
                update_labeled_digest(&mut digest, b"companion\0", companion_digest);
                update_labeled_digest(&mut digest, b"capsule\0", capsule_digest);
                update_labeled_digest(&mut digest, b"bundle\0", bundle_digest);
                digest.finalize().into()
            }
            ExternalPoolAdapterSessionRootSet::RuntimeCompatibility { .. } => {
                runtime_compatibility_root_digest(
                    RUNTIME_COMPATIBILITY_ROOT_TRANSCRIPT_DOMAIN,
                    &self.roots,
                    None,
                )
            }
            ExternalPoolAdapterSessionRootSet::TaskProtocolConformance(roots) => {
                roots.transcript_digest()
            }
        }
    }

    pub(super) fn kdf_salt(&self, host_nonce: &[u8; 32], child_nonce: &[u8; 32]) -> [u8; 32] {
        match &self.roots {
            ExternalPoolAdapterSessionRootSet::Production {
                policy_digest,
                profile_digest,
                target_digest,
                ..
            } => {
                let mut digest = Sha256::new();
                digest.update(KDF_SALT_DOMAIN);
                update_labeled_digest(&mut digest, b"policy\0", policy_digest);
                update_labeled_digest(&mut digest, b"profile\0", profile_digest);
                update_labeled_digest(&mut digest, b"target\0", target_digest);
                update_labeled_digest(&mut digest, b"host_nonce\0", host_nonce);
                update_labeled_digest(&mut digest, b"child_nonce\0", child_nonce);
                digest.finalize().into()
            }
            ExternalPoolAdapterSessionRootSet::RuntimeCompatibility { .. } => {
                runtime_compatibility_root_digest(
                    RUNTIME_COMPATIBILITY_KDF_SALT_DOMAIN,
                    &self.roots,
                    Some((host_nonce, child_nonce)),
                )
            }
            ExternalPoolAdapterSessionRootSet::TaskProtocolConformance(roots) => {
                roots.kdf_salt(host_nonce, child_nonce)
            }
        }
    }
}

#[derive(Clone)]
pub struct ExternalPoolAdapterSessionRootArguments {
    values: ExternalPoolAdapterSessionRootArgumentValues,
}

#[derive(Clone)]
enum ExternalPoolAdapterSessionRootArgumentValues {
    Production([String; 6]),
    RuntimeCompatibility([String; 11]),
    TaskProtocolConformance([String; 14]),
}

impl ExternalPoolAdapterSessionRootArguments {
    pub fn values(&self) -> &[String] {
        match &self.values {
            ExternalPoolAdapterSessionRootArgumentValues::Production(values) => values,
            ExternalPoolAdapterSessionRootArgumentValues::RuntimeCompatibility(values) => values,
            ExternalPoolAdapterSessionRootArgumentValues::TaskProtocolConformance(values) => values,
        }
    }

    pub fn runtime_compatibility_values(&self) -> Option<&[String; 11]> {
        match &self.values {
            ExternalPoolAdapterSessionRootArgumentValues::Production(_) => None,
            ExternalPoolAdapterSessionRootArgumentValues::RuntimeCompatibility(values) => {
                Some(values)
            }
            ExternalPoolAdapterSessionRootArgumentValues::TaskProtocolConformance(_) => None,
        }
    }

    pub fn task_protocol_conformance_values(&self) -> Option<&[String; 14]> {
        match &self.values {
            ExternalPoolAdapterSessionRootArgumentValues::TaskProtocolConformance(values) => {
                Some(values)
            }
            ExternalPoolAdapterSessionRootArgumentValues::Production(_)
            | ExternalPoolAdapterSessionRootArgumentValues::RuntimeCompatibility(_) => None,
        }
    }

    #[cfg(feature = "test-support")]
    pub fn replace_for_test(&mut self, index: usize, value: String) {
        match &mut self.values {
            ExternalPoolAdapterSessionRootArgumentValues::Production(values) => {
                values[index] = value;
            }
            ExternalPoolAdapterSessionRootArgumentValues::RuntimeCompatibility(values) => {
                values[index] = value;
            }
            ExternalPoolAdapterSessionRootArgumentValues::TaskProtocolConformance(values) => {
                values[index] = value;
            }
        }
    }
}

fn runtime_compatibility_root_digest(
    domain: &[u8],
    roots: &ExternalPoolAdapterSessionRootSet,
    nonces: Option<(&[u8; 32], &[u8; 32])>,
) -> [u8; 32] {
    let ExternalPoolAdapterSessionRootSet::RuntimeCompatibility {
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
    } = roots
    else {
        unreachable!("runtime compatibility digest requires runtime compatibility roots")
    };
    let mut digest = Sha256::new();
    digest.update(domain);
    for (label, value) in [
        (
            b"supervisor_session_policy_digest\0".as_slice(),
            supervisor_session_policy_digest,
        ),
        (
            b"runtime_compatibility_profile_digest\0".as_slice(),
            runtime_compatibility_profile_digest,
        ),
        (b"challenge_digest\0".as_slice(), challenge_digest),
        (b"runner_policy_digest\0".as_slice(), runner_policy_digest),
        (
            b"fixture_catalog_digest\0".as_slice(),
            fixture_catalog_digest,
        ),
        (
            b"sandbox_verifier_key_record_digest\0".as_slice(),
            sandbox_verifier_key_record_digest,
        ),
        (
            b"registry_release_digest\0".as_slice(),
            registry_release_digest,
        ),
        (
            b"installation_content_digest\0".as_slice(),
            installation_content_digest,
        ),
        (b"source_capsule_sha256\0".as_slice(), source_capsule_sha256),
        (b"launch_image_sha256\0".as_slice(), launch_image_sha256),
        (
            b"public_fixture_delivery_root\0".as_slice(),
            public_fixture_delivery_root,
        ),
    ] {
        update_labeled_digest(&mut digest, label, value);
    }
    if let Some((host_nonce, child_nonce)) = nonces {
        update_labeled_digest(&mut digest, b"host_nonce\0", host_nonce);
        update_labeled_digest(&mut digest, b"child_nonce\0", child_nonce);
    }
    digest.finalize().into()
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
