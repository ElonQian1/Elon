use std::fmt;

use anyhow::{bail, Result};
use chrono::{DateTime, Duration, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};

use super::{
    identity::{ComputePluginInstallationIdentity, ComputePluginReleaseRef},
    install_plan_admission::validate_inventory,
    install_plan_admission_validation::is_identifier,
    lifecycle::{
        ComputePluginHealthObservation, ComputePluginInventorySnapshot, ComputePluginLocalRecord,
        ACTIVATION_ENABLED, ADMISSION_ALLOWED, DESIRED_PRESENCE_PRESENT, RUNTIME_READY,
        SLOT_INSTALLED,
    },
    manifest_validation::is_sha256,
    signed_artifact_verification::jcs_sha256_hex,
    trusted_time::ComputePluginTrustedTimeObservation,
};

pub(crate) const COMPUTE_READY_CAPABILITY_SCHEMA: &str = "elon.compute_plugin.ready_capability.v1";
pub(crate) const HASHED_COMPUTE_READY_CAPABILITY_SCHEMA: &str =
    "elon.compute_plugin.hashed_ready_capability.v1";
pub(crate) const COMPUTE_READY_HEALTHY: &str = "healthy";
const COMPUTE_READY_HEALTH_DIGEST_SCHEMA: &str =
    "elon.compute_plugin.ready_health_digest_payload.v1";
const MAX_READY_HEALTH_LIFETIME_SECONDS: i64 = 5 * 60;
const MAX_READY_HEALTH_REASON_CODES: usize = 16;

/// Short-lived technical evidence. Price, reservable capacity and account policy belong to Offer.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeReadyCapability {
    pub schema: String,
    pub capability_id: String,
    pub executor_id: String,
    pub inventory_revision: i64,
    pub install_generation: i64,
    pub activation_generation: i64,
    pub runtime_generation: i64,
    pub slot_ref: String,
    pub release: ComputePluginReleaseRef,
    pub runner_id: String,
    pub runner_digest: String,
    pub runtime_digest: String,
    pub health_observation_digest: String,
    pub task_kinds: Vec<String>,
    pub model_bindings: Vec<ComputeReadyModelBinding>,
    pub supported_precisions: Vec<String>,
    pub resource_profile_digest: String,
    /// Local technical ceiling only; the versioned Offer owns market concurrency.
    pub technical_concurrency_limit: i64,
    pub observed_at: String,
    pub expires_at: String,
}

/// Digest is outside the payload and covers canonical capability bytes only.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct HashedComputeReadyCapability {
    pub schema: String,
    pub capability: ComputeReadyCapability,
    pub canonicalization: String,
    pub capability_digest_algorithm: String,
    pub capability_digest: String,
    pub evidence_ref: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeReadyModelBinding {
    pub model_id: String,
    pub model_digest: String,
    pub tokenizer_digest: Option<String>,
}

#[derive(Serialize)]
struct ComputeReadyHealthDigestPayload<'a> {
    schema: &'static str,
    plugin_id: &'a str,
    install_generation: i64,
    activation_generation: i64,
    status: &'a str,
    runtime_generation: i64,
    slot_ref: &'a str,
    runner_digest: &'a str,
    reason_codes: &'a [String],
    observed_at: &'a str,
    expires_at: &'a str,
}

/// Opaque proof that one immutable inventory record is ready to be transformed into a short-lived
/// capability. It deliberately owns the authenticated time observation so freshness cannot be
/// checked once and then reused after a later state transition.
pub(in crate::node_agent_compute_plugin_host) struct ValidatedComputeReadyPublication {
    inventory_revision: i64,
    desired_policy_revision: i64,
    record: ComputePluginLocalRecord,
    trusted_time: ComputePluginTrustedTimeObservation,
}

impl ValidatedComputeReadyPublication {
    pub(in crate::node_agent_compute_plugin_host) fn inventory_revision(&self) -> i64 {
        self.inventory_revision
    }

    pub(in crate::node_agent_compute_plugin_host) fn desired_policy_revision(&self) -> i64 {
        self.desired_policy_revision
    }

    pub(in crate::node_agent_compute_plugin_host) fn record(&self) -> &ComputePluginLocalRecord {
        &self.record
    }

    pub(in crate::node_agent_compute_plugin_host) fn trusted_time(
        &self,
    ) -> &ComputePluginTrustedTimeObservation {
        &self.trusted_time
    }
}

impl fmt::Debug for ValidatedComputeReadyPublication {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ValidatedComputeReadyPublication")
            .field("inventory_revision", &self.inventory_revision)
            .field("desired_policy_revision", &self.desired_policy_revision)
            .field("plugin_id", &self.record.plugin_id)
            .field("trusted_time", &"<authenticated>")
            .finish()
    }
}

/// Validates sharing, active runtime identity and health freshness from one authenticated time
/// observation. Ordinary wall time and caller-provided freshness booleans are not accepted.
pub(in crate::node_agent_compute_plugin_host) fn validate_ready_capability_publication(
    inventory: &ComputePluginInventorySnapshot,
    plugin_id: &str,
    installation: &ComputePluginInstallationIdentity,
    trusted_time: ComputePluginTrustedTimeObservation,
) -> Result<ValidatedComputeReadyPublication> {
    if trusted_time.installation_id_digest() != installation.digest() {
        bail!("COMPUTE_READY_INSTALLATION_MISMATCH");
    }

    let trusted_now = trusted_time.trusted_now().to_owned();
    validate_inventory(inventory, trusted_now.to_owned())?;
    if !inventory.sharing_enabled {
        bail!("COMPUTE_READY_SHARING_DISABLED");
    }
    if !is_identifier(plugin_id) {
        bail!("COMPUTE_READY_PLUGIN_ID_INVALID");
    }

    let record = inventory
        .plugins
        .iter()
        .find(|record| record.plugin_id == plugin_id)
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_READY_PLUGIN_NOT_FOUND"))?;
    validate_ready_record(record, &trusted_now)?;

    Ok(ValidatedComputeReadyPublication {
        inventory_revision: inventory.inventory_revision,
        desired_policy_revision: inventory.desired_policy_revision,
        record: record.clone(),
        trusted_time,
    })
}

fn validate_ready_record(
    record: &ComputePluginLocalRecord,
    trusted_now: &DateTime<Utc>,
) -> Result<()> {
    if record.desired_presence != DESIRED_PRESENCE_PRESENT
        || record.desired_activation != ACTIVATION_ENABLED
        || record.admission != ADMISSION_ALLOWED
        || record.runtime.phase != RUNTIME_READY
        || record.install_generation <= 0
        || record.activation_generation <= 0
        || record.runtime.runtime_generation <= 0
    {
        bail!("COMPUTE_READY_RECORD_NOT_READY");
    }

    let permission_grant_digest = record
        .permission_grant_digest
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_READY_PERMISSION_GRANT_MISSING"))?;
    if !is_sha256(permission_grant_digest) {
        bail!("COMPUTE_READY_PERMISSION_GRANT_INVALID");
    }

    let active_slot_ref = record
        .active_slot_ref
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_READY_ACTIVE_SLOT_MISSING"))?;
    let active_slot = record
        .slots
        .iter()
        .find(|slot| slot.slot_ref == active_slot_ref && slot.phase == SLOT_INSTALLED)
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_READY_ACTIVE_SLOT_INVALID"))?;
    if !is_identifier(&active_slot.release.plugin_version)
        || !is_identifier(&active_slot.release.target_id)
        || !is_sha256(&active_slot.release.manifest_digest)
        || !is_sha256(&active_slot.release.package_digest)
    {
        bail!("COMPUTE_READY_RELEASE_INVALID");
    }

    let health = record
        .health
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_READY_HEALTH_MISSING"))?;
    validate_ready_health(record, health, trusted_now)
}

fn validate_ready_health(
    record: &ComputePluginLocalRecord,
    health: &ComputePluginHealthObservation,
    trusted_now: &DateTime<Utc>,
) -> Result<()> {
    if health.status != COMPUTE_READY_HEALTHY
        || health.runtime_generation <= 0
        || health.runtime_generation != record.runtime.runtime_generation
        || record.runtime.slot_ref.as_deref() != Some(health.slot_ref.as_str())
        || record.active_slot_ref.as_deref() != Some(health.slot_ref.as_str())
        || record.runtime.runner_digest.as_deref() != Some(health.runner_digest.as_str())
        || !is_sha256(&health.runner_digest)
        || !is_sha256(&health.observation_digest)
    {
        bail!("COMPUTE_READY_HEALTH_BINDING_INVALID");
    }
    validate_reason_codes(&health.reason_codes)?;

    let observed_at = parse_canonical_utc("COMPUTE_READY_HEALTH_OBSERVED_AT", &health.observed_at)?;
    let expires_at = parse_canonical_utc("COMPUTE_READY_HEALTH_EXPIRES_AT", &health.expires_at)?;
    let state_changed_at =
        parse_canonical_utc("COMPUTE_READY_STATE_CHANGED_AT", &record.state_changed_at)?;
    if observed_at < state_changed_at
        || observed_at > *trusted_now
        || expires_at <= *trusted_now
        || expires_at <= observed_at
        || (expires_at - observed_at) > Duration::seconds(MAX_READY_HEALTH_LIFETIME_SECONDS)
    {
        bail!("COMPUTE_READY_HEALTH_STALE");
    }

    let digest = jcs_sha256_hex(&ComputeReadyHealthDigestPayload {
        schema: COMPUTE_READY_HEALTH_DIGEST_SCHEMA,
        plugin_id: &record.plugin_id,
        install_generation: record.install_generation,
        activation_generation: record.activation_generation,
        status: &health.status,
        runtime_generation: health.runtime_generation,
        slot_ref: &health.slot_ref,
        runner_digest: &health.runner_digest,
        reason_codes: &health.reason_codes,
        observed_at: &health.observed_at,
        expires_at: &health.expires_at,
    })?;
    if digest != health.observation_digest {
        bail!("COMPUTE_READY_HEALTH_DIGEST_MISMATCH");
    }
    Ok(())
}

fn validate_reason_codes(reason_codes: &[String]) -> Result<()> {
    if reason_codes.len() > MAX_READY_HEALTH_REASON_CODES {
        bail!("COMPUTE_READY_HEALTH_REASON_LIMIT");
    }
    if reason_codes.iter().any(|code| !is_identifier(code))
        || !reason_codes.windows(2).all(|pair| pair[0] < pair[1])
    {
        bail!("COMPUTE_READY_HEALTH_REASON_INVALID");
    }
    Ok(())
}

fn parse_canonical_utc(code: &str, value: &str) -> Result<DateTime<Utc>> {
    let parsed = DateTime::parse_from_rfc3339(value)
        .map_err(|_| anyhow::anyhow!("{code}: timestamp must be RFC3339"))?
        .with_timezone(&Utc);
    if parsed.timestamp_millis() < 0 || parsed.to_rfc3339_opts(SecondsFormat::Millis, true) != value
    {
        bail!("{code}: timestamp must be canonical UTC milliseconds");
    }
    Ok(parsed)
}
