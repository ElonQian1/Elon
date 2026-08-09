use serde::{Deserialize, Serialize};

use super::{
    identity::ComputePluginReleaseRef,
    keyring::ComputePluginKeyringBinding,
    plugin_manifest::{
        resource_limits_are_non_negative, ComputePluginPermissionProfile,
        ComputePluginResourceLimits, ComputePluginSignature,
    },
};

pub(crate) const COMPUTE_PLUGIN_INSTALL_PLAN_SCHEMA: &str = "elon.compute_plugin.install_plan.v1";
pub(crate) const SIGNED_COMPUTE_PLUGIN_INSTALL_PLAN_SCHEMA: &str =
    "elon.compute_plugin.signed_install_plan.v1";
pub(crate) const COMPUTE_PLUGIN_INSTALL_PLAN_SIGNATURE_DOMAIN: &str =
    "ELON-COMPUTE-PLUGIN-INSTALL-PLAN-V1";

pub(crate) const PLAN_ACTION_INSTALL: &str = "install";
pub(crate) const PLAN_ACTION_UPGRADE: &str = "upgrade";
pub(crate) const PLAN_ACTION_KEEP: &str = "keep";
pub(crate) const PLAN_ACTION_REAUTHORIZE_EXISTING: &str = "reauthorize_existing";
pub(crate) const PLAN_ACTION_DISABLE: &str = "disable";
pub(crate) const PLAN_ACTION_REMOVE: &str = "remove";
pub(crate) const PLAN_ACTION_CANCEL_CANDIDATE: &str = "cancel_candidate";

pub(crate) const PLAN_TARGET_ENABLED: &str = "enabled";
pub(crate) const PLAN_TARGET_DISABLED: &str = "disabled";
pub(crate) const PLAN_TARGET_ABSENT: &str = "absent";

/// Immutable desired plan. Executors must re-check live authorization before every fetch.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputePluginInstallPlan {
    pub schema: String,
    pub plan_id: String,
    pub expected_inventory_revision: i64,
    pub expected_inventory_digest: String,
    pub desired_policy_revision: i64,
    pub sharing_enabled: bool,
    pub sharing_authorization: Option<ComputeSharingAuthorizationBinding>,
    pub node_profile_digest: String,
    pub manifest_catalog_revision: i64,
    pub publisher_keyring: ComputePluginKeyringBinding,
    pub control_keyring: ComputePluginKeyringBinding,
    pub items: Vec<ComputePluginPlanItem>,
    pub total_download_bytes: i64,
    pub required_disk_bytes: i64,
    pub previous_versions_to_keep: i64,
    pub drain_before_replace: bool,
    pub generated_at: String,
    pub expires_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeSharingAuthorizationBinding {
    pub authorization_ref: String,
    pub revision: i64,
    pub digest: String,
}

/// Control-plane plan signatures use a separate keyring from publisher manifest signatures.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct SignedComputePluginInstallPlan {
    pub schema: String,
    pub plan: ComputePluginInstallPlan,
    pub canonicalization: String,
    pub plan_digest_algorithm: String,
    pub plan_digest: String,
    pub signature: ComputePluginSignature,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputePluginPlanItem {
    pub expected_current_release: Option<ComputePluginReleaseRef>,
    pub expected_candidate_release: Option<ComputePluginReleaseRef>,
    pub expected_install_generation: Option<i64>,
    pub target_release: Option<ComputePluginReleaseRef>,
    pub action: String,
    pub reason_codes: Vec<String>,
    pub downloads: Vec<ComputePluginPlannedDownload>,
    pub grant: Option<ComputePluginGrantBinding>,
    pub target_activation: String,
}

/// A grant is local authority, not a publisher request. Executors verify it is a strict subset.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputePluginGrantBinding {
    pub grant_ref: String,
    pub grant_digest: String,
    pub granted_permissions: ComputePluginPermissionProfile,
    pub granted_resources: ComputePluginResourceLimits,
}

/// Plugin packages and runtimes are independent downloads. Workload models use attempt prefetch.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputePluginPlannedDownload {
    pub artifact_kind: String,
    pub artifact_id: String,
    pub digest: String,
    pub size_bytes: i64,
    /// Opaque lookup reference only. Credentials and signed URLs are supplied out-of-band.
    pub source_ref: String,
    pub cache_class: String,
}

pub(crate) fn install_plan_shape_is_valid(plan: &ComputePluginInstallPlan) -> bool {
    if plan.expected_inventory_revision < 0
        || plan.desired_policy_revision < 0
        || plan.manifest_catalog_revision < 0
        || !keyring_binding_shape_is_valid(&plan.publisher_keyring)
        || !keyring_binding_shape_is_valid(&plan.control_keyring)
        || plan.total_download_bytes < 0
        || plan.required_disk_bytes < 0
        || plan.previous_versions_to_keep < 0
        || (plan.sharing_enabled && plan.sharing_authorization.is_none())
        || plan
            .sharing_authorization
            .as_ref()
            .is_some_and(|binding| binding.revision < 0)
    {
        return false;
    }

    let downloads_total = plan
        .items
        .iter()
        .flat_map(|item| item.downloads.iter())
        .try_fold(0_i64, |total, item| {
            (item.size_bytes >= 0)
                .then_some(item.size_bytes)
                .and_then(|size| total.checked_add(size))
        });

    downloads_total == Some(plan.total_download_bytes)
        && plan.items.iter().all(plan_item_shape_is_valid)
        && install_plan_respects_sharing_intent(plan)
}

fn keyring_binding_shape_is_valid(binding: &ComputePluginKeyringBinding) -> bool {
    binding.revision > 0
        && binding.digest.len() == 64
        && binding
            .digest
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn plan_item_shape_is_valid(item: &ComputePluginPlanItem) -> bool {
    if item
        .expected_install_generation
        .is_some_and(|value| value < 0)
    {
        return false;
    }
    let plugin_ids = [
        item.expected_current_release.as_ref(),
        item.expected_candidate_release.as_ref(),
        item.target_release.as_ref(),
    ]
    .into_iter()
    .flatten()
    .map(|release| release.plugin_id.as_str())
    .collect::<Vec<_>>();
    let same_plugin = plugin_ids.windows(2).all(|pair| pair[0] == pair[1]);
    let grant_is_valid = item
        .grant
        .as_ref()
        .is_none_or(|grant| resource_limits_are_non_negative(&grant.granted_resources));
    same_plugin
        && grant_is_valid
        && match item.action.as_str() {
            PLAN_ACTION_INSTALL => {
                item.expected_current_release.is_none()
                    && item.expected_candidate_release.is_none()
                    && item.expected_install_generation.is_none()
                    && item.target_release.is_some()
                    && item.grant.is_some()
                    && item.target_activation == PLAN_TARGET_ENABLED
            }
            PLAN_ACTION_UPGRADE => {
                item.expected_current_release.is_some()
                    && item.expected_candidate_release.is_none()
                    && item.expected_install_generation.is_some()
                    && item.target_release.is_some()
                    && item.grant.is_some()
                    && item.target_activation == PLAN_TARGET_ENABLED
            }
            PLAN_ACTION_KEEP => {
                item.expected_current_release.is_some()
                    && item.expected_candidate_release.is_none()
                    && item.expected_install_generation.is_some()
                    && item.target_release.is_none()
                    && item.downloads.is_empty()
                    && item.grant.is_none()
                    && matches!(
                        item.target_activation.as_str(),
                        PLAN_TARGET_ENABLED | PLAN_TARGET_DISABLED
                    )
            }
            PLAN_ACTION_REAUTHORIZE_EXISTING => {
                super::install_plan_reauthorization::reauthorization_shape_is_valid(item)
            }
            PLAN_ACTION_DISABLE => {
                item.expected_current_release.is_some()
                    && item.expected_candidate_release.is_none()
                    && item.expected_install_generation.is_some()
                    && item.target_release.is_none()
                    && item.downloads.is_empty()
                    && item.grant.is_none()
                    && item.target_activation == PLAN_TARGET_DISABLED
            }
            PLAN_ACTION_REMOVE => {
                item.expected_current_release.is_some()
                    && item.expected_candidate_release.is_none()
                    && item.expected_install_generation.is_some()
                    && item.target_release.is_none()
                    && item.downloads.is_empty()
                    && item.grant.is_none()
                    && item.target_activation == PLAN_TARGET_ABSENT
            }
            PLAN_ACTION_CANCEL_CANDIDATE => {
                item.expected_candidate_release.is_some()
                    && item.expected_install_generation.is_some()
                    && item.target_release.is_none()
                    && item.downloads.is_empty()
                    && item.grant.is_none()
                    && item.target_activation == PLAN_TARGET_DISABLED
            }
            _ => false,
        }
}

/// This is a structural guard, not authorization. Executors must re-read current authorization
/// before each fetch and stop before the next byte if sharing has since been disabled.
pub(crate) fn install_plan_respects_sharing_intent(plan: &ComputePluginInstallPlan) -> bool {
    if plan.sharing_enabled {
        return true;
    }
    plan.total_download_bytes == 0
        && plan.required_disk_bytes == 0
        && plan.items.iter().all(|item| {
            item.downloads.is_empty()
                && matches!(
                    item.action.as_str(),
                    PLAN_ACTION_KEEP
                        | PLAN_ACTION_DISABLE
                        | PLAN_ACTION_REMOVE
                        | PLAN_ACTION_CANCEL_CANDIDATE
                )
                && matches!(
                    item.target_activation.as_str(),
                    PLAN_TARGET_DISABLED | PLAN_TARGET_ABSENT
                )
        })
}
