use std::collections::HashSet;

use anyhow::{bail, Context, Result};
use chrono::{DateTime, Duration, Utc};

use super::{
    identity::ComputePluginReleaseRef,
    install_plan::{
        install_plan_shape_is_valid, ComputePluginInstallPlan, ComputePluginPlannedDownload,
        ComputeSharingAuthorizationBinding, SignedComputePluginInstallPlan,
    },
    install_plan_admission_validation::{
        is_identifier, item_plugin_id, reject_duplicate_manifests, validate_download_closure,
        validate_expected_local_state, validate_grant, validate_reason_codes,
        validate_target_compatibility,
    },
    lifecycle::{
        local_record_shape_is_valid, ComputePluginInventorySnapshot, ACTIVATION_ENABLED,
        COMPUTE_PLUGIN_INVENTORY_SCHEMA, SLOT_DOWNLOADING,
    },
    manifest_validation::{is_sha256, verify_and_validate_manifest},
    plugin_manifest::SignedComputePluginManifest,
    signed_artifact_verification::{
        jcs_sha256_hex, verify_install_plan_signature, ComputePluginControlPlaneKeyResolver,
        ComputePluginPublisherKeyResolver,
    },
};

const MAX_PLAN_ITEMS: usize = 256;
const MAX_PLAN_DOWNLOADS: usize = 4_096;
const MAX_PREVIOUS_VERSIONS: i64 = 4;
const MAX_PLAN_LIFETIME_HOURS: i64 = 24;
const MAX_GENERATED_AT_SKEW_MINUTES: i64 = 5;
const MAX_DOWNLOAD_SEGMENT_BYTES: i64 = 16 * 1_024 * 1_024;
const MAX_REDIRECT_HOPS: u8 = 5;

#[derive(Debug, Clone)]
pub(crate) struct ComputePluginLiveAdmissionState {
    pub sharing_enabled: bool,
    pub sharing_authorization: Option<ComputeSharingAuthorizationBinding>,
    pub desired_policy_revision: i64,
    pub node_profile_digest: String,
    pub manifest_catalog_revision: i64,
    pub control_keyring_revision: i64,
    pub target_id: String,
    pub host_api_protocol_id: String,
    pub host_api_revision: u32,
}

#[derive(Debug, Clone)]
pub(crate) struct AdmittedComputePluginInstallPlan {
    signed_plan: SignedComputePluginInstallPlan,
    admitted_at: String,
    manifests: Vec<AdmittedComputePluginManifestBinding>,
    downloads: Vec<AdmittedComputePluginDownload>,
    control_signing_key_fingerprint: String,
}

#[derive(Debug, Clone)]
pub(crate) struct AdmittedComputePluginManifestBinding {
    pub item_index: usize,
    pub release: ComputePluginReleaseRef,
    pub publisher_id: String,
    pub signing_key_id: String,
    pub signing_key_fingerprint: String,
}

#[derive(Debug, Clone)]
pub(crate) struct AdmittedComputePluginDownload {
    pub ordinal: usize,
    pub item_index: usize,
    pub release: ComputePluginReleaseRef,
    pub download: ComputePluginPlannedDownload,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ComputePluginDownloadSegmentRequest {
    pub ordinal: usize,
    pub offset_bytes: i64,
    pub length_bytes: i64,
    pub redirect_hop: u8,
}

#[derive(Debug, Clone)]
pub(crate) struct AuthorizedComputePluginDownloadSegment {
    pub download: AdmittedComputePluginDownload,
    pub offset_bytes: i64,
    pub length_bytes: i64,
    pub redirect_hop: u8,
}

/// Returned only by the durable InventoryStore after it atomically re-reads live authority,
/// verifies the persisted plan application and claims this exact segment/cursor.
#[derive(Debug, Clone)]
pub(crate) struct ComputePluginFetchAuthoritySnapshot {
    pub inventory: ComputePluginInventorySnapshot,
    pub live: ComputePluginLiveAdmissionState,
    pub trusted_now: DateTime<Utc>,
    pub applied_plan_id: String,
    pub applied_plan_digest: String,
    pub application_inventory_revision: i64,
    pub execution_inventory_revision: i64,
}

pub(crate) trait ComputePluginFetchAuthority {
    /// Must perform a fresh authoritative read and atomically claim the supplied cursor. A cached
    /// admission snapshot or wall clock without a persisted monotonic high-water is not valid.
    fn claim_fresh_segment(
        &self,
        plan_id: &str,
        plan_digest: &str,
        request: &ComputePluginDownloadSegmentRequest,
    ) -> Result<ComputePluginFetchAuthoritySnapshot>;
}

impl AdmittedComputePluginInstallPlan {
    pub(crate) fn plan(&self) -> &ComputePluginInstallPlan {
        &self.signed_plan.plan
    }

    pub(crate) fn plan_digest(&self) -> &str {
        &self.signed_plan.plan_digest
    }

    pub(crate) fn admitted_at(&self) -> &str {
        &self.admitted_at
    }

    pub(crate) fn manifests(&self) -> &[AdmittedComputePluginManifestBinding] {
        &self.manifests
    }

    pub(crate) fn downloads(&self) -> &[AdmittedComputePluginDownload] {
        &self.downloads
    }

    pub(crate) fn control_signing_key_fingerprint(&self) -> &str {
        &self.control_signing_key_fingerprint
    }
}

pub(crate) fn admit_install_plan(
    signed_plan: &SignedComputePluginInstallPlan,
    signed_manifests: &[SignedComputePluginManifest],
    inventory: &ComputePluginInventorySnapshot,
    live: &ComputePluginLiveAdmissionState,
    now: DateTime<Utc>,
    control_keys: &dyn ComputePluginControlPlaneKeyResolver,
    publisher_keys: &dyn ComputePluginPublisherKeyResolver,
) -> Result<AdmittedComputePluginInstallPlan> {
    let verified_plan = verify_install_plan_signature(signed_plan, control_keys)?;
    let plan = &verified_plan.signed().plan;
    validate_plan_shape_and_window(plan, now.clone())?;
    validate_live_binding(plan, live)?;
    validate_inventory(inventory, now.clone())?;
    if inventory.inventory_revision != plan.expected_inventory_revision {
        bail!("COMPUTE_PLUGIN_INVENTORY_STALE: InstallPlan inventory revision has changed");
    }
    if jcs_sha256_hex(inventory)? != plan.expected_inventory_digest {
        bail!("COMPUTE_PLUGIN_INVENTORY_DIGEST: InstallPlan inventory contents have changed");
    }
    if signed_manifests.len() > MAX_PLAN_ITEMS {
        bail!("COMPUTE_PLUGIN_MANIFEST_LIMIT: manifest set is oversized");
    }
    validate_disabled_plan_coverage(plan, inventory)?;
    let validated_manifests = signed_manifests
        .iter()
        .map(|manifest| verify_and_validate_manifest(manifest, publisher_keys))
        .collect::<Result<Vec<_>>>()?;
    reject_duplicate_manifests(&validated_manifests)?;
    if validated_manifests.iter().any(|manifest| {
        manifest.verification_key_fingerprint() == verified_plan.verification_key_fingerprint()
    }) {
        bail!("COMPUTE_PLUGIN_KEY_PURPOSE_REUSE: control and publisher keys must be distinct");
    }

    let mut used_manifest_digests = HashSet::new();
    let mut used_plugin_ids = HashSet::new();
    let mut admitted_manifests = Vec::new();
    let mut admitted_downloads = Vec::new();
    let mut minimum_disk_bytes = 0_i64;
    for (item_index, item) in plan.items.iter().enumerate() {
        validate_reason_codes(item)?;
        let plugin_id = item_plugin_id(item)?;
        if !used_plugin_ids.insert(plugin_id.to_string()) {
            bail!("COMPUTE_PLUGIN_PLAN_DUPLICATE_ITEM: a plugin may appear only once per plan");
        }
        validate_expected_local_state(item, inventory, plan.drain_before_replace)?;
        let Some(target) = item.target_release.as_ref() else {
            continue;
        };
        let manifest = validated_manifests
            .iter()
            .find(|candidate| candidate.release_ref() == *target)
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "COMPUTE_PLUGIN_MANIFEST_MISSING: target release has no verified manifest"
                )
            })?;
        validate_target_compatibility(manifest, live)?;
        validate_grant(item.grant.as_ref(), manifest)?;
        minimum_disk_bytes = minimum_disk_bytes
            .checked_add(validate_download_closure(item, manifest)?)
            .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_DISK_REQUIREMENT_OVERFLOW"))?;
        used_manifest_digests.insert(manifest.signed().manifest_digest.clone());
        admitted_manifests.push(AdmittedComputePluginManifestBinding {
            item_index,
            release: target.clone(),
            publisher_id: manifest.manifest().publisher_id.clone(),
            signing_key_id: manifest.signed().signature.signing_key_id.clone(),
            signing_key_fingerprint: manifest.verification_key_fingerprint().to_string(),
        });
        let first_ordinal = admitted_downloads.len();
        admitted_downloads.extend(item.downloads.iter().cloned().enumerate().map(
            |(offset, download)| AdmittedComputePluginDownload {
                ordinal: first_ordinal + offset,
                item_index,
                release: target.clone(),
                download,
            },
        ));
    }
    if used_manifest_digests.len() != validated_manifests.len() {
        bail!("COMPUTE_PLUGIN_MANIFEST_EXTRA: unbound manifests are forbidden");
    }
    if plan.required_disk_bytes < minimum_disk_bytes {
        bail!("COMPUTE_PLUGIN_DISK_REQUIREMENT: plan under-reports download and unpacked bytes");
    }
    Ok(AdmittedComputePluginInstallPlan {
        signed_plan: verified_plan.signed().clone(),
        admitted_at: now.to_rfc3339(),
        manifests: admitted_manifests,
        downloads: admitted_downloads,
        control_signing_key_fingerprint: verified_plan.verification_key_fingerprint().to_string(),
    })
}

/// Call immediately before every request, redirect and resumed byte range. The authority owns the
/// durable cursor claim; callers cannot authorize from the DTOs retained after initial admission.
pub(crate) fn authorize_download_segment(
    admitted: &AdmittedComputePluginInstallPlan,
    request: &ComputePluginDownloadSegmentRequest,
    authority: &dyn ComputePluginFetchAuthority,
) -> Result<AuthorizedComputePluginDownloadSegment> {
    let plan = admitted.plan();
    let download = admitted
        .downloads
        .get(request.ordinal)
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_FETCH_ORDINAL: download is not in plan"))?;
    let segment_end = request
        .offset_bytes
        .checked_add(request.length_bytes)
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_FETCH_RANGE_OVERFLOW"))?;
    if request.offset_bytes < 0
        || request.length_bytes <= 0
        || request.length_bytes > MAX_DOWNLOAD_SEGMENT_BYTES
        || segment_end > download.download.size_bytes
        || request.redirect_hop > MAX_REDIRECT_HOPS
    {
        bail!("COMPUTE_PLUGIN_FETCH_RANGE: segment or redirect hop is outside the plan");
    }
    let facts = authority.claim_fresh_segment(&plan.plan_id, admitted.plan_digest(), request)?;
    validate_plan_window(plan, facts.trusted_now.clone(), false)?;
    validate_live_binding(plan, &facts.live)?;
    validate_inventory(&facts.inventory, facts.trusted_now)?;
    let expected_application_revision = plan
        .expected_inventory_revision
        .checked_add(1)
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_INVENTORY_REVISION_OVERFLOW"))?;
    if facts.applied_plan_id != plan.plan_id
        || facts.applied_plan_digest != admitted.plan_digest()
        || facts.application_inventory_revision != expected_application_revision
        || facts.inventory.inventory_revision != facts.execution_inventory_revision
        || facts.inventory.inventory_revision < facts.application_inventory_revision
        || facts.inventory.desired_policy_revision != plan.desired_policy_revision
        || facts.inventory.sharing_enabled != plan.sharing_enabled
    {
        bail!("COMPUTE_PLUGIN_FETCH_BINDING_CHANGED: applied plan or inventory has changed");
    }
    let record = facts
        .inventory
        .plugins
        .iter()
        .find(|record| record.plugin_id == download.release.plugin_id)
        .ok_or_else(|| {
            anyhow::anyhow!("COMPUTE_PLUGIN_FETCH_INVENTORY: plugin record is missing")
        })?;
    let candidate_matches = record.candidate_slot_ref.as_ref().is_some_and(|slot_ref| {
        record.slots.iter().any(|slot| {
            &slot.slot_ref == slot_ref
                && slot.phase == SLOT_DOWNLOADING
                && slot.release == download.release
        })
    });
    if record.last_plan_id.as_deref() != Some(plan.plan_id.as_str()) || !candidate_matches {
        bail!("COMPUTE_PLUGIN_FETCH_SLOT_CHANGED: candidate slot is no longer owned by this plan");
    }
    Ok(AuthorizedComputePluginDownloadSegment {
        download: download.clone(),
        offset_bytes: request.offset_bytes,
        length_bytes: request.length_bytes,
        redirect_hop: request.redirect_hop,
    })
}

fn validate_plan_shape_and_window(
    plan: &ComputePluginInstallPlan,
    now: DateTime<Utc>,
) -> Result<()> {
    let downloads_are_bounded = plan
        .items
        .iter()
        .try_fold(0_usize, |total, item| {
            total.checked_add(item.downloads.len())
        })
        .is_some_and(|total| total <= MAX_PLAN_DOWNLOADS);
    if !install_plan_shape_is_valid(plan)
        || plan.items.len() > MAX_PLAN_ITEMS
        || !downloads_are_bounded
        || plan.previous_versions_to_keep > MAX_PREVIOUS_VERSIONS
        || (plan.total_download_bytes > 0 && plan.required_disk_bytes < plan.total_download_bytes)
        || plan.sharing_enabled != plan.sharing_authorization.is_some()
        || !is_identifier(&plan.plan_id)
        || !is_sha256(&plan.expected_inventory_digest)
        || !is_sha256(&plan.node_profile_digest)
    {
        bail!("COMPUTE_PLUGIN_PLAN_SHAPE: InstallPlan is not canonical or bounded");
    }
    validate_plan_window(plan, now, true)
}

fn validate_plan_window(
    plan: &ComputePluginInstallPlan,
    now: DateTime<Utc>,
    allow_generated_at_skew: bool,
) -> Result<()> {
    let generated = parse_utc("COMPUTE_PLUGIN_PLAN_GENERATED_AT", &plan.generated_at)?;
    let expires = parse_utc("COMPUTE_PLUGIN_PLAN_EXPIRES_AT", &plan.expires_at)?;
    let earliest_now = if allow_generated_at_skew {
        generated
            .checked_sub_signed(Duration::minutes(MAX_GENERATED_AT_SKEW_MINUTES))
            .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_PLAN_GENERATED_AT_OUT_OF_RANGE"))?
    } else {
        generated
    };
    if generated >= expires
        || expires - generated > Duration::hours(MAX_PLAN_LIFETIME_HOURS)
        || now < earliest_now
        || now >= expires
    {
        bail!("COMPUTE_PLUGIN_PLAN_EXPIRED: InstallPlan is not currently valid");
    }
    Ok(())
}

fn validate_live_binding(
    plan: &ComputePluginInstallPlan,
    live: &ComputePluginLiveAdmissionState,
) -> Result<()> {
    if live.sharing_enabled != plan.sharing_enabled
        || live.sharing_authorization != plan.sharing_authorization
        || live.desired_policy_revision != plan.desired_policy_revision
        || live.node_profile_digest != plan.node_profile_digest
        || live.manifest_catalog_revision != plan.manifest_catalog_revision
        || live.control_keyring_revision != plan.control_keyring_revision
        || live.host_api_revision == 0
        || !is_identifier(&live.target_id)
        || !is_identifier(&live.host_api_protocol_id)
    {
        bail!("COMPUTE_PLUGIN_LIVE_BINDING_CHANGED: authorization, policy or node facts changed");
    }
    if let Some(binding) = &live.sharing_authorization {
        if !is_identifier(&binding.authorization_ref)
            || binding.revision < 0
            || !is_sha256(&binding.digest)
        {
            bail!("COMPUTE_PLUGIN_AUTHORIZATION_BINDING: live sharing authorization is invalid");
        }
    }
    Ok(())
}

fn validate_inventory(
    inventory: &ComputePluginInventorySnapshot,
    now: DateTime<Utc>,
) -> Result<()> {
    if inventory.schema != COMPUTE_PLUGIN_INVENTORY_SCHEMA
        || inventory.inventory_revision < 0
        || inventory.desired_policy_revision < 0
        || inventory.plugins.len() > MAX_PLAN_ITEMS
        || parse_utc(
            "COMPUTE_PLUGIN_INVENTORY_OBSERVED_AT",
            &inventory.observed_at,
        )? > now
    {
        bail!("COMPUTE_PLUGIN_INVENTORY_INVALID: inventory snapshot is invalid");
    }
    let mut plugin_ids = HashSet::new();
    if inventory.plugins.iter().any(|record| {
        !is_identifier(&record.plugin_id)
            || !plugin_ids.insert(record.plugin_id.as_str())
            || !local_record_shape_is_valid(record)
    }) {
        bail!("COMPUTE_PLUGIN_INVENTORY_INVALID: plugin records are invalid or duplicated");
    }
    Ok(())
}

fn validate_disabled_plan_coverage(
    plan: &ComputePluginInstallPlan,
    inventory: &ComputePluginInventorySnapshot,
) -> Result<()> {
    if plan.sharing_enabled {
        return Ok(());
    }
    for record in inventory
        .plugins
        .iter()
        .filter(|record| record.desired_activation == ACTIVATION_ENABLED)
    {
        let covered = plan
            .items
            .iter()
            .any(|item| item_plugin_id(item).ok() == Some(record.plugin_id.as_str()));
        if !covered {
            bail!("COMPUTE_PLUGIN_DISABLE_INCOMPLETE: sharing-off plan leaves an enabled plugin");
        }
    }
    Ok(())
}

fn parse_utc(code: &str, value: &str) -> Result<DateTime<Utc>> {
    let parsed = DateTime::parse_from_rfc3339(value.trim())
        .with_context(|| format!("{code}: timestamp is not RFC3339"))?;
    if parsed.offset().local_minus_utc() != 0 || value.trim() != value {
        bail!("{code}: timestamp must use canonical UTC without whitespace");
    }
    Ok(parsed.with_timezone(&Utc))
}
