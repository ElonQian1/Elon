use std::fmt;

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const INSTALLATION_ID_DOMAIN: &[u8] = b"ELON_COMPUTE_PLUGIN_INSTALLATION_ID_V1";
const MAX_INSTALLATION_ID_BYTES: usize = 256;

/// One canonical installation identity shared by the node-data marker and authority Store. The
/// digest is derived here; callers cannot pair a raw ID from one installation with another
/// installation's authority digest.
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct ComputePluginInstallationIdentity {
    install_id: String,
    digest: String,
}

impl ComputePluginInstallationIdentity {
    pub(crate) fn derive(install_id: &str) -> Result<Self> {
        let install_id = install_id.trim();
        if install_id.is_empty()
            || install_id.len() > MAX_INSTALLATION_ID_BYTES
            || install_id.chars().any(|value| value.is_control())
        {
            bail!("COMPUTE_PLUGIN_INSTALLATION_ID_INVALID");
        }
        let mut digest = Sha256::new();
        digest.update(INSTALLATION_ID_DOMAIN);
        digest.update([0]);
        digest.update(install_id.as_bytes());
        Ok(Self {
            install_id: install_id.to_string(),
            digest: hex::encode(digest.finalize()),
        })
    }

    pub(crate) fn install_id(&self) -> &str {
        &self.install_id
    }

    pub(crate) fn digest(&self) -> &str {
        &self.digest
    }
}

impl fmt::Debug for ComputePluginInstallationIdentity {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ComputePluginInstallationIdentity")
            .field("install_id", &"<redacted>")
            .field("digest", &"<redacted>")
            .finish()
    }
}

/// Immutable release identity. `manifest_digest` is the plugin release digest; package and
/// runner digests identify different byte sets and must never be substituted for it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputePluginReleaseRef {
    pub plugin_id: String,
    pub plugin_version: String,
    pub target_id: String,
    pub manifest_digest: String,
    pub package_digest: String,
}
