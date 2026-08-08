use std::fmt;

use homecli_proto::{
    ComputePluginSharingAuthorizationBindingV1, ComputePluginSharingPolicySnapshotV1,
    COMPUTE_PLUGIN_SHARING_POLICY_SNAPSHOT_V1_SCHEMA,
};
use sha2::{Digest, Sha256};

const SNAPSHOT_DIGEST_DOMAIN: &[u8] = b"ELON_COMPUTE_PLUGIN_SHARING_POLICY_SNAPSHOT_V1";
const INSTALLATION_ID_DIGEST_DOMAIN: &[u8] = b"ELON_COMPUTE_PLUGIN_INSTALLATION_ID_V1";
const MAX_IJSON_SAFE_INTEGER: u64 = 9_007_199_254_740_991;
const MAX_ID_BYTES: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ComputePluginSharingPolicyValidationError {
    code: &'static str,
}

impl ComputePluginSharingPolicyValidationError {
    pub(crate) fn code(self) -> &'static str {
        self.code
    }
}

impl fmt::Display for ComputePluginSharingPolicyValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.code)
    }
}

impl std::error::Error for ComputePluginSharingPolicyValidationError {}

pub(crate) fn build_compute_plugin_sharing_policy_snapshot_v1(
    node_id: impl Into<String>,
    owner_user_id: impl Into<String>,
    installation_identity_digest: impl Into<String>,
    policy_revision: u64,
    policy_digest: impl Into<String>,
    plugin_runtime_requested: bool,
    authorization: Option<ComputePluginSharingAuthorizationBindingV1>,
) -> Result<ComputePluginSharingPolicySnapshotV1, ComputePluginSharingPolicyValidationError> {
    let snapshot = ComputePluginSharingPolicySnapshotV1 {
        schema: COMPUTE_PLUGIN_SHARING_POLICY_SNAPSHOT_V1_SCHEMA.to_string(),
        node_id: node_id.into(),
        owner_user_id: owner_user_id.into(),
        installation_identity_digest: installation_identity_digest.into(),
        policy_revision,
        policy_digest: policy_digest.into(),
        plugin_runtime_requested,
        authorization,
    };
    validate_compute_plugin_sharing_policy_snapshot_v1(&snapshot)?;
    Ok(snapshot)
}

pub(crate) fn validate_compute_plugin_sharing_policy_snapshot_v1(
    snapshot: &ComputePluginSharingPolicySnapshotV1,
) -> Result<(), ComputePluginSharingPolicyValidationError> {
    if snapshot.schema != COMPUTE_PLUGIN_SHARING_POLICY_SNAPSHOT_V1_SCHEMA {
        return invalid("COMPUTE_PLUGIN_SHARING_POLICY_SCHEMA_UNSUPPORTED");
    }
    if !bounded_identifier(&snapshot.node_id) {
        return invalid("COMPUTE_PLUGIN_SHARING_POLICY_NODE_ID_INVALID");
    }
    if !bounded_identifier(&snapshot.owner_user_id) {
        return invalid("COMPUTE_PLUGIN_SHARING_POLICY_OWNER_USER_ID_INVALID");
    }
    if !is_sha256(&snapshot.installation_identity_digest) {
        return invalid("COMPUTE_PLUGIN_SHARING_POLICY_INSTALLATION_DIGEST_INVALID");
    }
    if !safe_positive_revision(snapshot.policy_revision) {
        return invalid("COMPUTE_PLUGIN_SHARING_POLICY_REVISION_INVALID");
    }
    if !is_sha256(&snapshot.policy_digest) {
        return invalid("COMPUTE_PLUGIN_SHARING_POLICY_DIGEST_INVALID");
    }
    match (
        snapshot.plugin_runtime_requested,
        snapshot.authorization.as_ref(),
    ) {
        (true, Some(binding)) => {
            validate_authorization(binding)?;
            if binding.revision != snapshot.policy_revision {
                return invalid("COMPUTE_PLUGIN_SHARING_POLICY_AUTHORIZATION_REVISION_MISMATCH");
            }
            if binding.digest != snapshot.policy_digest {
                return invalid("COMPUTE_PLUGIN_SHARING_POLICY_AUTHORIZATION_DIGEST_MISMATCH");
            }
        }
        (true, None) => return invalid("COMPUTE_PLUGIN_SHARING_POLICY_AUTHORIZATION_REQUIRED"),
        (false, Some(_)) => {
            return invalid("COMPUTE_PLUGIN_SHARING_POLICY_AUTHORIZATION_FORBIDDEN")
        }
        (false, None) => {}
    }
    Ok(())
}

pub(crate) fn compute_plugin_sharing_policy_snapshot_digest(
    snapshot: &ComputePluginSharingPolicySnapshotV1,
) -> Result<String, ComputePluginSharingPolicyValidationError> {
    validate_compute_plugin_sharing_policy_snapshot_v1(snapshot)?;
    let mut digest = Sha256::new();
    digest.update(SNAPSHOT_DIGEST_DOMAIN);
    digest.update([0]);
    digest_string(&mut digest, b"schema", &snapshot.schema);
    digest_string(&mut digest, b"node_id", &snapshot.node_id);
    digest_string(&mut digest, b"owner_user_id", &snapshot.owner_user_id);
    digest_string(
        &mut digest,
        b"installation_identity_digest",
        &snapshot.installation_identity_digest,
    );
    digest_u64(&mut digest, b"policy_revision", snapshot.policy_revision);
    digest_string(&mut digest, b"policy_digest", &snapshot.policy_digest);
    digest_bool(
        &mut digest,
        b"plugin_runtime_requested",
        snapshot.plugin_runtime_requested,
    );
    digest_bool(
        &mut digest,
        b"authorization_present",
        snapshot.authorization.is_some(),
    );
    if let Some(binding) = &snapshot.authorization {
        digest_string(
            &mut digest,
            b"authorization_ref",
            &binding.authorization_ref,
        );
        digest_u64(&mut digest, b"authorization_revision", binding.revision);
        digest_string(&mut digest, b"authorization_digest", &binding.digest);
    }
    Ok(hex::encode(digest.finalize()))
}

pub(crate) fn derive_compute_plugin_installation_identity_digest(
    install_id: &str,
) -> Result<String, ComputePluginSharingPolicyValidationError> {
    let install_id = install_id.trim();
    if !bounded_identifier(install_id) {
        return invalid("COMPUTE_PLUGIN_SHARING_INSTALLATION_ID_INVALID");
    }
    let mut digest = Sha256::new();
    digest.update(INSTALLATION_ID_DIGEST_DOMAIN);
    digest.update([0]);
    digest.update(install_id.as_bytes());
    Ok(hex::encode(digest.finalize()))
}

fn validate_authorization(
    binding: &ComputePluginSharingAuthorizationBindingV1,
) -> Result<(), ComputePluginSharingPolicyValidationError> {
    if !bounded_identifier(&binding.authorization_ref) {
        return invalid("COMPUTE_PLUGIN_SHARING_POLICY_AUTHORIZATION_REF_INVALID");
    }
    if !safe_positive_revision(binding.revision) {
        return invalid("COMPUTE_PLUGIN_SHARING_POLICY_AUTHORIZATION_REVISION_INVALID");
    }
    if !is_sha256(&binding.digest) {
        return invalid("COMPUTE_PLUGIN_SHARING_POLICY_AUTHORIZATION_DIGEST_INVALID");
    }
    Ok(())
}

fn bounded_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_ID_BYTES
        && value.trim() == value
        && !value.chars().any(char::is_control)
}

fn safe_positive_revision(value: u64) -> bool {
    value > 0 && value <= MAX_IJSON_SAFE_INTEGER
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn digest_string(digest: &mut Sha256, label: &[u8], value: &str) {
    digest_field(digest, label, value.as_bytes());
}

fn digest_u64(digest: &mut Sha256, label: &[u8], value: u64) {
    digest_field(digest, label, &value.to_be_bytes());
}

fn digest_bool(digest: &mut Sha256, label: &[u8], value: bool) {
    digest_field(digest, label, &[u8::from(value)]);
}

fn digest_field(digest: &mut Sha256, label: &[u8], value: &[u8]) {
    digest.update(label);
    digest.update([0]);
    digest.update((value.len() as u64).to_be_bytes());
    digest.update(value);
}

fn invalid<T>(code: &'static str) -> Result<T, ComputePluginSharingPolicyValidationError> {
    Err(ComputePluginSharingPolicyValidationError { code })
}
