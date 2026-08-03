use serde::{Deserialize, Serialize};

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
