use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::{
    plugin_manifest::ComputePluginSignature,
    signed_artifact_verification::ComputePluginEd25519PublicKey,
};

pub(crate) const COMPUTE_PLUGIN_KEYRING_BUNDLE_SCHEMA: &str =
    "elon.compute_plugin.keyring_bundle.v1";
pub(crate) const SIGNED_COMPUTE_PLUGIN_KEYRING_BUNDLE_SCHEMA: &str =
    "elon.compute_plugin.signed_keyring_bundle.v1";
pub(crate) const COMPUTE_PLUGIN_KEYRING_SCHEMA: &str = "elon.compute_plugin.keyring.v1";
pub(crate) const COMPUTE_PLUGIN_KEYRING_SIGNATURE_DOMAIN: &str =
    "ELON-COMPUTE-PLUGIN-KEYRING-BUNDLE-V1";

pub(crate) const KEY_PURPOSE_PUBLISHER_MANIFEST: &str = "publisher_manifest";
pub(crate) const KEY_PURPOSE_CONTROL_INSTALL_PLAN: &str = "control_install_plan";
pub(crate) const KEY_STATUS_ACTIVE: &str = "active";
pub(crate) const KEY_STATUS_REVOKED: &str = "revoked";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputePluginKeyringBinding {
    pub revision: i64,
    pub digest: String,
}

/// The whole payload is signed by a Bootstrap-pinned offline root public key.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputePluginKeyringBundle {
    pub schema: String,
    pub bundle_revision: i64,
    pub publisher_keyring: ComputePluginKeyring,
    pub control_keyring: ComputePluginKeyring,
    pub generated_at: String,
    pub expires_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct SignedComputePluginKeyringBundle {
    pub schema: String,
    pub bundle: ComputePluginKeyringBundle,
    pub canonicalization: String,
    pub bundle_digest_algorithm: String,
    pub bundle_digest: String,
    pub signature: ComputePluginSignature,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputePluginKeyring {
    pub schema: String,
    pub purpose: String,
    pub revision: i64,
    pub keys: Vec<ComputePluginKeyringKey>,
}

/// Public verification material only. Private keys and node credentials are never valid here.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputePluginKeyringKey {
    pub publisher_id: Option<String>,
    pub signing_key_id: String,
    pub purpose: String,
    pub algorithm: String,
    pub public_key_base64: String,
    pub fingerprint_sha256: String,
    pub status: String,
    pub not_before: String,
    pub not_after: String,
    pub revoked_at: Option<String>,
}

/// Production implementations resolve only immutable public keys pinned into a trusted Bootstrap
/// release. AppData files and environment variables must never become an implicit root fallback.
pub(crate) trait ComputePluginBootstrapRootKeyResolver {
    fn resolve_bootstrap_root_key(
        &self,
        signing_key_id: &str,
    ) -> Result<Option<ComputePluginEd25519PublicKey>>;
}

#[derive(Debug, Clone)]
pub(crate) struct ResolvedComputePluginVerificationKey {
    key: ComputePluginEd25519PublicKey,
    keyring_binding: ComputePluginKeyringBinding,
    purpose: String,
    publisher_id: Option<String>,
    signing_key_id: String,
    fingerprint_sha256: String,
    not_before: DateTime<Utc>,
    not_after: DateTime<Utc>,
}

impl ResolvedComputePluginVerificationKey {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn new(
        key: ComputePluginEd25519PublicKey,
        keyring_binding: ComputePluginKeyringBinding,
        purpose: String,
        publisher_id: Option<String>,
        signing_key_id: String,
        fingerprint_sha256: String,
        not_before: DateTime<Utc>,
        not_after: DateTime<Utc>,
    ) -> Self {
        Self {
            key,
            keyring_binding,
            purpose,
            publisher_id,
            signing_key_id,
            fingerprint_sha256,
            not_before,
            not_after,
        }
    }

    pub(crate) fn key(&self) -> &ComputePluginEd25519PublicKey {
        &self.key
    }

    pub(crate) fn fingerprint_sha256(&self) -> &str {
        &self.fingerprint_sha256
    }

    pub(crate) fn keyring_binding(&self) -> &ComputePluginKeyringBinding {
        &self.keyring_binding
    }

    pub(crate) fn purpose(&self) -> &str {
        &self.purpose
    }

    pub(crate) fn publisher_id(&self) -> Option<&str> {
        self.publisher_id.as_deref()
    }

    pub(crate) fn signing_key_id(&self) -> &str {
        &self.signing_key_id
    }

    pub(crate) fn not_before(&self) -> DateTime<Utc> {
        self.not_before.clone()
    }

    pub(crate) fn not_after(&self) -> DateTime<Utc> {
        self.not_after.clone()
    }
}

/// Implementations return keys only from the exact, currently trusted Publisher ring binding.
pub(crate) trait ComputePluginPublisherKeyResolver {
    fn resolve_publisher_key(
        &self,
        publisher_id: &str,
        signing_key_id: &str,
        expected_keyring: &ComputePluginKeyringBinding,
        trusted_now: DateTime<Utc>,
    ) -> Result<Option<ResolvedComputePluginVerificationKey>>;
}

/// InstallPlan verification uses an independent Control ring and namespace.
pub(crate) trait ComputePluginControlPlaneKeyResolver {
    fn resolve_control_plane_key(
        &self,
        signing_key_id: &str,
        expected_keyring: &ComputePluginKeyringBinding,
        trusted_now: DateTime<Utc>,
    ) -> Result<Option<ResolvedComputePluginVerificationKey>>;
}

#[derive(Debug, Clone)]
pub(crate) struct ValidatedComputePluginKeyringBundle {
    signed: SignedComputePluginKeyringBundle,
    publisher_binding: ComputePluginKeyringBinding,
    control_binding: ComputePluginKeyringBinding,
    root_key_fingerprint: String,
}

impl ValidatedComputePluginKeyringBundle {
    pub(super) fn new(
        signed: SignedComputePluginKeyringBundle,
        publisher_binding: ComputePluginKeyringBinding,
        control_binding: ComputePluginKeyringBinding,
        root_key_fingerprint: String,
    ) -> Self {
        Self {
            signed,
            publisher_binding,
            control_binding,
            root_key_fingerprint,
        }
    }

    pub(crate) fn signed(&self) -> &SignedComputePluginKeyringBundle {
        &self.signed
    }

    pub(crate) fn publisher_binding(&self) -> &ComputePluginKeyringBinding {
        &self.publisher_binding
    }

    pub(crate) fn control_binding(&self) -> &ComputePluginKeyringBinding {
        &self.control_binding
    }

    pub(crate) fn root_key_fingerprint(&self) -> &str {
        &self.root_key_fingerprint
    }
}
