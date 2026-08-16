//! Strict wire types for cache health uploads.

use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct FleetEnvelopeV1 {
    pub(super) schema: String,
    pub(super) envelope_id: String,
    pub(super) created_at_utc: String,
    pub(super) node_id: String,
    pub(super) report: FleetEnvelopeReport,
    pub(super) security: FleetEnvelopeSecurity,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct FleetEnvelopeReport {
    pub(super) schema: String,
    pub(super) content_type: String,
    pub(super) content_sha256: String,
    pub(super) byte_length: u64,
    pub(super) json: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct FleetEnvelopeSecurity {
    pub(super) receiver_must_authenticate_node: bool,
    pub(super) destructive_actions_authorized: bool,
    pub(super) absolute_paths_included: bool,
    pub(super) secrets_included: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct FleetReportV1 {
    pub(super) schema: String,
    pub(super) generated_at_utc: String,
    pub(super) node: FleetNode,
    pub(super) project: FleetProject,
    pub(super) platform: FleetPlatform,
    pub(super) cache: FleetCache,
    pub(super) volume: FleetVolume,
    pub(super) activity: FleetActivity,
    pub(super) privacy: FleetPrivacy,
    pub(super) destructive_actions_taken: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct FleetNode {
    pub(super) node_id: String,
    pub(super) os: String,
    pub(super) powershell_edition: String,
    pub(super) powershell_version: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct FleetProject {
    pub(super) project_id: String,
    pub(super) registered: bool,
    pub(super) default_domain: String,
    pub(super) allowed_domains: Vec<String>,
    pub(super) shared_partition_count: u64,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct FleetPlatform {
    pub(super) health: String,
    pub(super) source_mode: String,
    pub(super) source_hash: String,
    pub(super) actionable_checks: Vec<FleetCheck>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct FleetCheck {
    pub(super) id: String,
    pub(super) status: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct FleetCache {
    pub(super) toolchain_epoch: String,
    pub(super) include_sizes: bool,
    pub(super) partition_count: u64,
    pub(super) managed_size_bytes: Option<u64>,
    pub(super) locked_partition_count: u64,
    pub(super) invalid_marker_count: u64,
    pub(super) quarantine_partition_count: u64,
    pub(super) retired_shared_alias_count: u64,
    pub(super) by_scope: Vec<FleetGroup>,
    pub(super) by_domain: Vec<FleetGroup>,
    pub(super) legacy_cache_count: u64,
    pub(super) retired_legacy_cache_count: u64,
    pub(super) legacy_size_bytes: Option<u64>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct FleetGroup {
    pub(super) name: String,
    pub(super) count: u64,
    pub(super) size_bytes: Option<u64>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct FleetVolume {
    pub(super) total_bytes: u64,
    pub(super) free_bytes: u64,
    pub(super) free_percent: f64,
    pub(super) warning_free_percent: f64,
    pub(super) gc_review_recommended: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct FleetActivity {
    pub(super) active_writer_count: u64,
    pub(super) active_writers: Vec<FleetWriterGroup>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct FleetWriterGroup {
    pub(super) process_name: String,
    pub(super) count: u64,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct FleetPrivacy {
    pub(super) absolute_paths_included: bool,
    pub(super) host_name_included: bool,
    pub(super) user_name_included: bool,
}
