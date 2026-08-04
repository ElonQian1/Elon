use anyhow::Result;
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
