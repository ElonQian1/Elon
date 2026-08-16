use anyhow::{anyhow, Result};
use chrono::DateTime;
use sha2::{Digest, Sha256};

use crate::store::rust_cache::fleet_reports::RustCacheFleetReportInput;

const ENVELOPE_SCHEMA: &str = "elon.rust_cache.fleet_envelope.v1";
const REPORT_SCHEMA: &str = "elon.rust_cache.fleet_report.v1";
const MAX_REPORT_BYTES: usize = 1_048_576;

pub(super) mod types;
use types::*;

#[cfg(test)]
mod tests;

#[derive(Debug)]
pub(crate) struct ValidatedFleetEnvelope {
    pub input: RustCacheFleetReportInput,
}

pub(crate) fn validate_envelope(
    route_node_id: &str,
    envelope: FleetEnvelopeV1,
) -> Result<ValidatedFleetEnvelope> {
    let route_node_id = route_node_id.trim();
    if envelope.schema != ENVELOPE_SCHEMA
        || envelope.report.schema != REPORT_SCHEMA
        || envelope.report.content_type != "application/json"
    {
        return Err(anyhow!("unsupported Rust cache fleet envelope contract"));
    }
    require_stable_id(route_node_id, 128, "route node_id")?;
    require_stable_id(&envelope.node_id, 128, "envelope node_id")?;
    require_lower_hex(&envelope.envelope_id, 32, "envelope_id")?;
    require_lower_hex(&envelope.report.content_sha256, 64, "report SHA-256")?;
    parse_rfc3339(&envelope.created_at_utc, "envelope timestamp")?;
    if route_node_id != envelope.node_id {
        return Err(anyhow!("route and envelope node identity mismatch"));
    }
    if !envelope.security.receiver_must_authenticate_node
        || envelope.security.destructive_actions_authorized
        || envelope.security.absolute_paths_included
        || envelope.security.secrets_included
    {
        return Err(anyhow!(
            "unsafe Rust cache fleet envelope security contract"
        ));
    }

    let report_bytes = envelope.report.json.as_bytes();
    if report_bytes.is_empty()
        || report_bytes.len() > MAX_REPORT_BYTES
        || envelope.report.byte_length != report_bytes.len() as u64
    {
        return Err(anyhow!("Rust cache fleet report byte length mismatch"));
    }
    let actual_hash = hex::encode(Sha256::digest(report_bytes));
    if actual_hash != envelope.report.content_sha256 {
        return Err(anyhow!("Rust cache fleet report hash mismatch"));
    }
    reject_path_or_identity_fields(&envelope.report.json)?;

    let report: FleetReportV1 = serde_json::from_str(&envelope.report.json)
        .map_err(|error| anyhow!("invalid Rust cache fleet report: {error}"))?;
    validate_report(route_node_id, &report)?;

    Ok(ValidatedFleetEnvelope {
        input: RustCacheFleetReportInput {
            envelope_id: envelope.envelope_id,
            node_id: route_node_id.to_string(),
            report_sha256: actual_hash,
            report_json: envelope.report.json,
            platform_health: report.platform.health,
            gc_review_recommended: report.volume.gc_review_recommended,
            active_writer_count: report.activity.active_writer_count,
            managed_size_bytes: report.cache.managed_size_bytes,
            generated_at: report.generated_at_utc,
        },
    })
}

fn validate_report(route_node_id: &str, report: &FleetReportV1) -> Result<()> {
    if report.schema != REPORT_SCHEMA || report.node.node_id != route_node_id {
        return Err(anyhow!("embedded Rust cache report identity mismatch"));
    }
    if report.destructive_actions_taken
        || report.privacy.absolute_paths_included
        || report.privacy.host_name_included
        || report.privacy.user_name_included
    {
        return Err(anyhow!(
            "embedded Rust cache report violates read-only privacy policy"
        ));
    }
    parse_rfc3339(&report.generated_at_utc, "report timestamp")?;
    require_stable_id(&report.project.project_id, 128, "project_id")?;
    require_stable_id(&report.project.default_domain, 128, "default_domain")?;
    for domain in &report.project.allowed_domains {
        require_stable_id(domain, 128, "allowed_domain")?;
    }
    require_bounded_text(&report.platform.health, 64, "platform health")?;
    require_bounded_text(&report.platform.source_mode, 64, "source mode")?;
    require_lower_hex(&report.platform.source_hash, 64, "platform source hash")?;
    require_bounded_text(&report.cache.toolchain_epoch, 128, "toolchain epoch")?;
    require_bounded_text(&report.node.os, 32, "node os")?;
    require_bounded_text(&report.node.powershell_edition, 32, "PowerShell edition")?;
    require_bounded_text(&report.node.powershell_version, 64, "PowerShell version")?;
    for check in &report.platform.actionable_checks {
        require_stable_id(&check.id, 128, "check id")?;
        require_bounded_text(&check.status, 32, "check status")?;
    }
    for group in report.cache.by_scope.iter().chain(&report.cache.by_domain) {
        require_stable_id(&group.name, 128, "cache group")?;
        let _ = (group.count, group.size_bytes);
    }
    for writer in &report.activity.active_writers {
        require_stable_id(&writer.process_name, 64, "writer process")?;
        let _ = writer.count;
    }
    if !report.volume.free_percent.is_finite()
        || !report.volume.warning_free_percent.is_finite()
        || report.volume.free_percent < 0.0
        || report.volume.free_percent > 100.0
        || report.volume.warning_free_percent < 0.0
        || report.volume.warning_free_percent > 100.0
        || report.volume.free_bytes > report.volume.total_bytes
    {
        return Err(anyhow!("invalid Rust cache volume metrics"));
    }
    let _ = (
        report.project.registered,
        report.project.shared_partition_count,
        report.cache.include_sizes,
        report.cache.partition_count,
        report.cache.locked_partition_count,
        report.cache.invalid_marker_count,
        report.cache.quarantine_partition_count,
        report.cache.retired_shared_alias_count,
        report.cache.legacy_cache_count,
        report.cache.retired_legacy_cache_count,
        report.cache.legacy_size_bytes,
    );
    Ok(())
}

fn reject_path_or_identity_fields(json: &str) -> Result<()> {
    let lower = json.to_ascii_lowercase();
    let forbidden = [
        "project_root",
        "cache_root",
        "user_launcher_path",
        "computer_name",
        "hostname",
        "username",
        "file://",
        ":\\\\",
        "/home/",
        "/users/",
    ];
    if forbidden.iter().any(|needle| lower.contains(needle)) {
        return Err(anyhow!(
            "Rust cache fleet report contains a forbidden local identity or path signal"
        ));
    }
    Ok(())
}

fn require_stable_id(value: &str, max: usize, field: &str) -> Result<()> {
    if value.is_empty()
        || value.len() > max
        || !value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | ':' | '-'))
    {
        return Err(anyhow!("invalid {field}"));
    }
    Ok(())
}

fn require_bounded_text(value: &str, max: usize, field: &str) -> Result<()> {
    if value.trim().is_empty() || value.len() > max || value.chars().any(char::is_control) {
        return Err(anyhow!("invalid {field}"));
    }
    Ok(())
}

fn require_lower_hex(value: &str, len: usize, field: &str) -> Result<()> {
    if value.len() != len
        || value != value.to_ascii_lowercase()
        || !value.chars().all(|ch| ch.is_ascii_hexdigit())
    {
        return Err(anyhow!("invalid {field}"));
    }
    Ok(())
}

fn parse_rfc3339(value: &str, field: &str) -> Result<()> {
    DateTime::parse_from_rfc3339(value)
        .map(|_| ())
        .map_err(|_| anyhow!("invalid {field}"))
}
