use std::fmt;

use anyhow::{bail, Context, Result};
use chrono::{DateTime, Utc};
use rusqlite::{OptionalExtension, Transaction};
use serde::{Deserialize, Serialize};

use super::{
    keyring_snapshot::{
        load_snapshot_for_state, read_authority_keyring_state, KeyringSnapshotValidation,
    },
    plan_application_persistence::{persist_plan_application, replay_plan_application},
    plan_application_projection::project_plan_application,
    ComputePluginLocalAuthority,
};
use crate::node_agent_compute_plugin_host::{
    install_plan::SignedComputePluginInstallPlan,
    install_plan_admission::{admit_install_plan, ComputePluginLiveAdmissionState},
    install_plan_admission_validation::is_identifier,
    keyring::{ComputePluginBootstrapRootKeyResolver, ComputePluginKeyringBinding},
    lifecycle::{
        local_record_shape_is_valid, ComputePluginInventorySnapshot,
        COMPUTE_PLUGIN_INVENTORY_SCHEMA,
    },
    manifest_validation::is_sha256,
    plugin_manifest::SignedComputePluginManifest,
    signed_artifact_verification::jcs_sha256_hex,
};

pub(super) const PLAN_APPLICATION_RECEIPT_SCHEMA: &str =
    "elon.compute_plugin.plan_application_receipt.v1";
const PLAN_APPLICATION_REQUEST_SCHEMA: &str = "elon.compute_plugin.plan_application_request.v1";
const SIGNED_MANIFEST_SET_SCHEMA: &str = "elon.compute_plugin.signed_manifest_set.v1";
const MAX_MANIFESTS: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ComputePluginPlanApplicationDisposition {
    Applied,
    Replayed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputePluginPlanApplicationReceipt {
    pub(super) schema: String,
    pub(super) plan_id: String,
    pub(super) plan_digest: String,
    pub(super) application_request_digest: String,
    pub(super) admission_bindings_digest: String,
    pub(super) inventory_before_revision: i64,
    pub(super) inventory_before_digest: String,
    pub(super) inventory_after_revision: i64,
    pub(super) inventory_after_digest: String,
    pub(super) application_state_revision: i64,
    pub(super) authority_epoch: i64,
    pub(super) keyring_bundle_revision: i64,
    pub(super) publisher_keyring: ComputePluginKeyringBinding,
    pub(super) control_keyring: ComputePluginKeyringBinding,
    pub(super) control_signing_key_fingerprint: String,
    pub(super) new_candidates: Vec<ComputePluginCandidateReceipt>,
    pub(super) released_candidates: Vec<ComputePluginReleasedCandidateReceipt>,
    pub(super) downloads: Vec<ComputePluginDownloadReceipt>,
    pub(super) download_bytes: i64,
    pub(super) applied_at_ms: i64,
}

impl ComputePluginPlanApplicationReceipt {
    pub(crate) fn plan_id(&self) -> &str {
        &self.plan_id
    }

    pub(crate) fn plan_digest(&self) -> &str {
        &self.plan_digest
    }

    pub(crate) fn inventory_after_revision(&self) -> i64 {
        self.inventory_after_revision
    }

    pub(crate) fn inventory_after_digest(&self) -> &str {
        &self.inventory_after_digest
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputePluginCandidateReceipt {
    pub(super) plugin_id: String,
    pub(super) candidate_token_digest: String,
    pub(super) slot_ref: String,
    pub(super) candidate_generation: i64,
    pub(super) release: crate::node_agent_compute_plugin_host::identity::ComputePluginReleaseRef,
    pub(super) permission_grant_digest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputePluginReleasedCandidateReceipt {
    pub(super) plugin_id: String,
    pub(super) candidate_token_digest: String,
    pub(super) slot_ref: String,
    pub(super) candidate_generation: i64,
    pub(super) release: crate::node_agent_compute_plugin_host::identity::ComputePluginReleaseRef,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputePluginDownloadReceipt {
    pub(super) ordinal: i64,
    pub(super) item_index: i64,
    pub(super) candidate_token_digest: String,
    pub(super) artifact_kind: String,
    pub(super) artifact_id: String,
    pub(super) artifact_digest: String,
    pub(super) source_ref: String,
    pub(super) cache_class: String,
    pub(super) size_bytes: i64,
    pub(super) part_relative_path: String,
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct ComputePluginCandidateHandle {
    pub(super) plugin_id: String,
    pub(super) candidate_token: String,
    pub(super) candidate_token_digest: String,
    pub(super) slot_ref: String,
    pub(super) candidate_generation: i64,
}

impl ComputePluginCandidateHandle {
    pub(crate) fn plugin_id(&self) -> &str {
        &self.plugin_id
    }

    pub(crate) fn candidate_token(&self) -> &str {
        &self.candidate_token
    }

    pub(crate) fn candidate_token_digest(&self) -> &str {
        &self.candidate_token_digest
    }

    pub(crate) fn slot_ref(&self) -> &str {
        &self.slot_ref
    }

    pub(crate) fn candidate_generation(&self) -> i64 {
        self.candidate_generation
    }
}

impl fmt::Debug for ComputePluginCandidateHandle {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ComputePluginCandidateHandle")
            .field("plugin_id", &self.plugin_id)
            .field("candidate_token", &"<redacted>")
            .field("candidate_token_digest", &self.candidate_token_digest)
            .field("slot_ref", &self.slot_ref)
            .field("candidate_generation", &self.candidate_generation)
            .finish()
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ComputePluginPlanApplicationResult {
    disposition: ComputePluginPlanApplicationDisposition,
    receipt: ComputePluginPlanApplicationReceipt,
    candidate_handles: Vec<ComputePluginCandidateHandle>,
    execution_plan:
        crate::node_agent_compute_plugin_host::install_plan_admission::AdmittedComputePluginInstallPlan,
}

impl ComputePluginPlanApplicationResult {
    pub(crate) fn disposition(&self) -> ComputePluginPlanApplicationDisposition {
        self.disposition
    }

    pub(crate) fn receipt(&self) -> &ComputePluginPlanApplicationReceipt {
        &self.receipt
    }

    pub(crate) fn candidate_handles(&self) -> &[ComputePluginCandidateHandle] {
        &self.candidate_handles
    }

    /// A sealed execution capsule. Every byte-range authorization still performs a fresh
    /// authority read; this value alone never grants network or filesystem access.
    pub(crate) fn execution_plan(
        &self,
    ) -> &crate::node_agent_compute_plugin_host::install_plan_admission::AdmittedComputePluginInstallPlan
    {
        &self.execution_plan
    }

    pub(super) fn new(
        disposition: ComputePluginPlanApplicationDisposition,
        receipt: ComputePluginPlanApplicationReceipt,
        candidate_handles: Vec<ComputePluginCandidateHandle>,
        execution_plan: crate::node_agent_compute_plugin_host::install_plan_admission::AdmittedComputePluginInstallPlan,
    ) -> Self {
        Self {
            disposition,
            receipt,
            candidate_handles,
            execution_plan,
        }
    }
}

pub(super) struct PreparedPlanApplicationRequest {
    pub signed_plan_envelope_digest: String,
    pub signed_manifest_set_digest: String,
    pub application_request_digest: String,
    pub signed_manifests: Vec<SignedComputePluginManifest>,
}

#[derive(Serialize)]
struct SignedManifestSet<'a> {
    schema: &'static str,
    manifests: &'a [SignedComputePluginManifest],
}

#[derive(Serialize)]
struct ApplicationRequestDigest<'a> {
    schema: &'static str,
    signed_plan_envelope_digest: &'a str,
    signed_manifest_set_digest: &'a str,
}

pub(super) struct AuthorityPlanApplicationState {
    pub installation_id_digest: String,
    pub state_revision: i64,
    pub authority_epoch: i64,
    pub process_owner_epoch: i64,
    pub inventory: ComputePluginInventorySnapshot,
    pub inventory_digest: String,
    pub inventory_json: String,
    pub desired_policy_revision: i64,
    pub sharing_enabled: bool,
    pub sharing_authorization: Option<
        crate::node_agent_compute_plugin_host::install_plan::ComputeSharingAuthorizationBinding,
    >,
    pub node_profile_digest: String,
    pub manifest_catalog_revision: i64,
    pub target_id: String,
    pub host_api_protocol_id: String,
    pub host_api_revision: u32,
    pub keyring_bundle_revision: i64,
    pub publisher_keyring: ComputePluginKeyringBinding,
    pub control_keyring: ComputePluginKeyringBinding,
    pub trusted_time_high_water_ms: i64,
}

impl AuthorityPlanApplicationState {
    pub(super) fn live(&self) -> ComputePluginLiveAdmissionState {
        ComputePluginLiveAdmissionState {
            sharing_enabled: self.sharing_enabled,
            sharing_authorization: self.sharing_authorization.clone(),
            desired_policy_revision: self.desired_policy_revision,
            node_profile_digest: self.node_profile_digest.clone(),
            manifest_catalog_revision: self.manifest_catalog_revision,
            publisher_keyring: self.publisher_keyring.clone(),
            control_keyring: self.control_keyring.clone(),
            target_id: self.target_id.clone(),
            host_api_protocol_id: self.host_api_protocol_id.clone(),
            host_api_revision: self.host_api_revision,
        }
    }
}

impl ComputePluginLocalAuthority {
    /// `trusted_now` must come from the future authenticated trusted-time kernel, never directly
    /// from an ordinary wall clock, and a state-changing application must observe a value strictly
    /// later than the durable authority high-water mark. This method has no filesystem, network or
    /// Sidecar effects.
    pub(crate) fn apply_install_plan(
        &self,
        signed_plan: &SignedComputePluginInstallPlan,
        signed_manifests: &[SignedComputePluginManifest],
        trusted_now: DateTime<Utc>,
        roots: &dyn ComputePluginBootstrapRootKeyResolver,
    ) -> Result<ComputePluginPlanApplicationResult> {
        let request = prepare_application_request(signed_plan, signed_manifests)?;
        self.with_immediate(|transaction| {
            if let Some(replayed) = replay_plan_application(
                transaction,
                &signed_plan.plan.plan_id,
                &signed_plan.plan_digest,
                &request.application_request_digest,
            )? {
                return Ok(replayed);
            }
            let authority = read_authority_plan_application_state(transaction, &trusted_now)?;
            let keyring_state = read_authority_keyring_state(transaction)?;
            if keyring_state.state_revision != authority.state_revision
                || keyring_state.authority_epoch != authority.authority_epoch
            {
                bail!("COMPUTE_PLUGIN_PLAN_AUTHORITY_FENCE_CHANGED");
            }
            let keyring = load_snapshot_for_state(
                transaction,
                &keyring_state,
                KeyringSnapshotValidation::Current(trusted_now.clone()),
                roots,
            )?;
            if keyring.bundle_revision() != authority.keyring_bundle_revision
                || keyring.publisher_binding() != &authority.publisher_keyring
                || keyring.control_binding() != &authority.control_keyring
            {
                bail!("COMPUTE_PLUGIN_PLAN_KEYRING_BINDING_CHANGED");
            }
            let admitted = admit_install_plan(
                signed_plan,
                &request.signed_manifests,
                &authority.inventory,
                &authority.live(),
                trusted_now.clone(),
                &keyring,
                &keyring,
            )?;
            let projected = project_plan_application(
                transaction,
                &authority,
                &admitted,
                trusted_now.timestamp_millis(),
            )?;
            persist_plan_application(
                transaction,
                &authority,
                &keyring,
                &request,
                &admitted,
                projected,
                trusted_now.timestamp_millis(),
            )
        })
    }
}

pub(super) fn prepare_application_request(
    signed_plan: &SignedComputePluginInstallPlan,
    signed_manifests: &[SignedComputePluginManifest],
) -> Result<PreparedPlanApplicationRequest> {
    if signed_manifests.len() > MAX_MANIFESTS
        || signed_plan.plan.items.len() > MAX_MANIFESTS
        || !is_identifier(&signed_plan.plan.plan_id)
        || !is_sha256(&signed_plan.plan_digest)
    {
        bail!("COMPUTE_PLUGIN_PLAN_APPLICATION_REQUEST_SHAPE");
    }
    let mut envelopes = signed_manifests
        .iter()
        .map(|manifest| Ok((jcs_sha256_hex(manifest)?, manifest.clone())))
        .collect::<Result<Vec<_>>>()?;
    envelopes.sort_by(|left, right| {
        let left_manifest = &left.1;
        let right_manifest = &right.1;
        (
            &left_manifest.manifest.plugin_id,
            &left_manifest.manifest.plugin_version,
            &left_manifest.manifest.target.target_id,
            &left_manifest.manifest_digest,
            &left.0,
        )
            .cmp(&(
                &right_manifest.manifest.plugin_id,
                &right_manifest.manifest.plugin_version,
                &right_manifest.manifest.target.target_id,
                &right_manifest.manifest_digest,
                &right.0,
            ))
    });
    let signed_manifests = envelopes
        .into_iter()
        .map(|(_, manifest)| manifest)
        .collect::<Vec<_>>();
    let signed_plan_envelope_digest = jcs_sha256_hex(signed_plan)?;
    let signed_manifest_set_digest = jcs_sha256_hex(&SignedManifestSet {
        schema: SIGNED_MANIFEST_SET_SCHEMA,
        manifests: &signed_manifests,
    })?;
    let application_request_digest = jcs_sha256_hex(&ApplicationRequestDigest {
        schema: PLAN_APPLICATION_REQUEST_SCHEMA,
        signed_plan_envelope_digest: &signed_plan_envelope_digest,
        signed_manifest_set_digest: &signed_manifest_set_digest,
    })?;
    Ok(PreparedPlanApplicationRequest {
        signed_plan_envelope_digest,
        signed_manifest_set_digest,
        application_request_digest,
        signed_manifests,
    })
}

pub(super) fn read_authority_plan_application_state(
    transaction: &Transaction<'_>,
    trusted_now: &DateTime<Utc>,
) -> Result<AuthorityPlanApplicationState> {
    read_authority_plan_application_state_with_time_mode(
        transaction,
        trusted_now,
        AuthorityStateTimeMode::StrictlyBeforeObservation,
    )
}

/// Reads the same canonical authority snapshot after a Store has advanced the durable clock to
/// this observation. This is readback/recovery only; mutation callers must retain their own strict
/// `new_time > current_high_water` gate before writing.
pub(super) fn read_authority_plan_application_state_at_or_before_observation(
    transaction: &Transaction<'_>,
    trusted_now: &DateTime<Utc>,
) -> Result<AuthorityPlanApplicationState> {
    read_authority_plan_application_state_with_time_mode(
        transaction,
        trusted_now,
        AuthorityStateTimeMode::AtOrBeforeObservation,
    )
}

#[derive(Clone, Copy)]
enum AuthorityStateTimeMode {
    StrictlyBeforeObservation,
    AtOrBeforeObservation,
}

fn read_authority_plan_application_state_with_time_mode(
    transaction: &Transaction<'_>,
    trusted_now: &DateTime<Utc>,
    time_mode: AuthorityStateTimeMode,
) -> Result<AuthorityPlanApplicationState> {
    type MetaRow = (
        String,
        i64,
        i64,
        String,
        String,
        i64,
        i64,
        Option<String>,
        Option<i64>,
        Option<String>,
        String,
        i64,
        String,
        String,
        i64,
        i64,
        i64,
        Option<i64>,
        String,
        Option<i64>,
        Option<i64>,
        Option<String>,
        Option<i64>,
        Option<String>,
    );
    let row: MetaRow = transaction
        .query_row(
            r#"SELECT installation_id_digest, state_revision, inventory_revision,
                inventory_digest, inventory_json, desired_policy_revision, sharing_enabled,
                sharing_authorization_ref, sharing_authorization_revision,
                sharing_authorization_digest, node_profile_digest, manifest_catalog_revision,
                target_id, host_api_protocol_id, host_api_revision, authority_epoch,
                process_owner_epoch, trusted_time_high_water_ms, clock_status,
                active_bundle_revision, publisher_keyring_revision, publisher_keyring_digest,
                control_keyring_revision, control_keyring_digest
            FROM authority_meta WHERE singleton = 1"#,
            [],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                    row.get(7)?,
                    row.get(8)?,
                    row.get(9)?,
                    row.get(10)?,
                    row.get(11)?,
                    row.get(12)?,
                    row.get(13)?,
                    row.get(14)?,
                    row.get(15)?,
                    row.get(16)?,
                    row.get(17)?,
                    row.get(18)?,
                    row.get(19)?,
                    row.get(20)?,
                    row.get(21)?,
                    row.get(22)?,
                    row.get(23)?,
                ))
            },
        )
        .optional()
        .context("COMPUTE_PLUGIN_PLAN_AUTHORITY_READ")?
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_AUTHORITY_UNINITIALIZED"))?;
    build_authority_plan_application_state(row, trusted_now, time_mode)
}

fn build_authority_plan_application_state(
    row: (
        String,
        i64,
        i64,
        String,
        String,
        i64,
        i64,
        Option<String>,
        Option<i64>,
        Option<String>,
        String,
        i64,
        String,
        String,
        i64,
        i64,
        i64,
        Option<i64>,
        String,
        Option<i64>,
        Option<i64>,
        Option<String>,
        Option<i64>,
        Option<String>,
    ),
    trusted_now: &DateTime<Utc>,
    time_mode: AuthorityStateTimeMode,
) -> Result<AuthorityPlanApplicationState> {
    let (
        installation_id_digest,
        state_revision,
        inventory_revision,
        inventory_digest,
        inventory_json,
        desired_policy_revision,
        sharing_flag,
        authorization_ref,
        authorization_revision,
        authorization_digest,
        node_profile_digest,
        manifest_catalog_revision,
        target_id,
        host_api_protocol_id,
        host_api_revision,
        authority_epoch,
        process_owner_epoch,
        trusted_time_high_water_ms,
        clock_status,
        bundle_revision,
        publisher_revision,
        publisher_digest,
        control_revision,
        control_digest,
    ) = row;
    let sharing_authorization = match (
        authorization_ref,
        authorization_revision,
        authorization_digest,
    ) {
        (None, None, None) => None,
        (Some(authorization_ref), Some(revision), Some(digest)) => Some(
            crate::node_agent_compute_plugin_host::install_plan::ComputeSharingAuthorizationBinding {
                authorization_ref,
                revision,
                digest,
            },
        ),
        _ => bail!("COMPUTE_PLUGIN_PLAN_AUTHORIZATION_CORRUPT"),
    };
    let (keyring_bundle_revision, publisher_keyring, control_keyring) = match (
        bundle_revision,
        publisher_revision,
        publisher_digest,
        control_revision,
        control_digest,
    ) {
        (
            Some(bundle),
            Some(publisher_revision),
            Some(publisher_digest),
            Some(control_revision),
            Some(control_digest),
        ) => (
            bundle,
            ComputePluginKeyringBinding {
                revision: publisher_revision,
                digest: publisher_digest,
            },
            ComputePluginKeyringBinding {
                revision: control_revision,
                digest: control_digest,
            },
        ),
        _ => bail!("COMPUTE_PLUGIN_PLAN_KEYRING_INACTIVE"),
    };
    let trusted_time_high_water_ms = trusted_time_high_water_ms
        .filter(|_| clock_status == "trusted")
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_AUTHORITY_CLOCK_UNTRUSTED"))?;
    let inventory: ComputePluginInventorySnapshot =
        serde_json::from_str(&inventory_json).context("COMPUTE_PLUGIN_PLAN_INVENTORY_JSON")?;
    let observed_at = DateTime::parse_from_rfc3339(&inventory.observed_at)
        .context("COMPUTE_PLUGIN_PLAN_INVENTORY_TIME")?;
    let sharing_enabled = match sharing_flag {
        0 => false,
        1 => true,
        _ => bail!("COMPUTE_PLUGIN_PLAN_SHARING_FLAG_CORRUPT"),
    };
    match time_mode {
        AuthorityStateTimeMode::StrictlyBeforeObservation
            if trusted_now.timestamp_millis() <= trusted_time_high_water_ms =>
        {
            bail!("COMPUTE_PLUGIN_PLAN_TRUSTED_TIME_NOT_ADVANCED");
        }
        AuthorityStateTimeMode::AtOrBeforeObservation
            if trusted_now.timestamp_millis() < trusted_time_high_water_ms =>
        {
            bail!("COMPUTE_PLUGIN_AUTHORITY_CLOCK_ROLLBACK");
        }
        _ => {}
    }
    if state_revision < 0
        || authority_epoch < 0
        || process_owner_epoch < 0
        || desired_policy_revision < 0
        || manifest_catalog_revision < 0
        || inventory.schema != COMPUTE_PLUGIN_INVENTORY_SCHEMA
        || inventory.inventory_revision != inventory_revision
        || inventory.desired_policy_revision != desired_policy_revision
        || inventory.sharing_enabled != sharing_enabled
        || jcs_sha256_hex(&inventory)? != inventory_digest
        || observed_at.offset().local_minus_utc() != 0
        || observed_at.with_timezone(&Utc) > *trusted_now
        || inventory
            .plugins
            .windows(2)
            .any(|pair| pair[0].plugin_id >= pair[1].plugin_id)
        || inventory
            .plugins
            .iter()
            .any(|record| !local_record_shape_is_valid(record))
        || !is_sha256(&installation_id_digest)
        || !is_sha256(&inventory_digest)
        || !is_sha256(&node_profile_digest)
        || !is_identifier(&target_id)
        || !is_identifier(&host_api_protocol_id)
    {
        bail!("COMPUTE_PLUGIN_PLAN_AUTHORITY_CORRUPT");
    }
    Ok(AuthorityPlanApplicationState {
        installation_id_digest,
        state_revision,
        authority_epoch,
        process_owner_epoch,
        inventory,
        inventory_digest,
        inventory_json,
        desired_policy_revision,
        sharing_enabled,
        sharing_authorization,
        node_profile_digest,
        manifest_catalog_revision,
        target_id,
        host_api_protocol_id,
        host_api_revision: u32::try_from(host_api_revision)
            .context("COMPUTE_PLUGIN_PLAN_HOST_API_REVISION")?,
        keyring_bundle_revision,
        publisher_keyring,
        control_keyring,
        trusted_time_high_water_ms,
    })
}
