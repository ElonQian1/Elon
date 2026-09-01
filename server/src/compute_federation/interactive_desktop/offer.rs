use serde::{Deserialize, Serialize};

use crate::compute_federation::capacity::ComputeCapacityPoolBinding;

use super::INTERACTIVE_DESKTOP_SERVICE_CLASS;

pub(crate) const INTERACTIVE_DESKTOP_OFFER_PROFILE_SCHEMA: &str =
    "compute_federation.interactive_desktop.offer_profile.v1";
pub(crate) const INTERACTIVE_DESKTOP_OFFER_PROFILE_DIGEST_DOMAIN: &str =
    "ELON-COMPUTE-INTERACTIVE-DESKTOP-OFFER-PROFILE-V1";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum InteractiveDesktopProductMode {
    SameOwnerRemoteAccess,
    FriendCoPlay,
    LicensedCloudSeat,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum InteractiveDesktopMarketAccess {
    PrivateUnpaid,
    PaidMarketplace,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum InteractiveDesktopConnectivityPolicy {
    DirectOrRelay,
    RelayOnly,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct InteractiveDesktopTitlePolicyBinding {
    pub title_catalog_id: String,
    pub title_policy_snapshot_id: String,
    pub title_policy_version: i64,
    pub title_policy_snapshot_digest: String,
    pub rights_evidence_digest: String,
    pub territory: String,
    pub valid_until_ms: i64,
}

impl InteractiveDesktopTitlePolicyBinding {
    fn has_complete_reference(&self) -> bool {
        !self.title_catalog_id.is_empty()
            && !self.title_policy_snapshot_id.is_empty()
            && self.title_policy_version > 0
            && !self.title_policy_snapshot_digest.is_empty()
            && !self.rights_evidence_digest.is_empty()
            && !self.territory.is_empty()
            && self.valid_until_ms > 0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct InteractiveDesktopOfferBinding {
    pub provider_id: String,
    pub offer_id: String,
    pub offer_version: i64,
    pub offer_digest: String,
    pub profile_id: String,
    pub profile_version: i64,
    pub profile_digest: String,
    pub product_mode: InteractiveDesktopProductMode,
    pub market_access: InteractiveDesktopMarketAccess,
    pub connectivity_policy: InteractiveDesktopConnectivityPolicy,
    pub title_policy: Option<InteractiveDesktopTitlePolicyBinding>,
}

impl InteractiveDesktopOfferBinding {
    /// Structural market check only; callers must separately verify referenced digests.
    pub(crate) fn has_safe_market_boundary(&self) -> bool {
        self.market_access != InteractiveDesktopMarketAccess::PaidMarketplace
            || (self.product_mode == InteractiveDesktopProductMode::LicensedCloudSeat
                && self.connectivity_policy == InteractiveDesktopConnectivityPolicy::RelayOnly
                && self
                    .title_policy
                    .as_ref()
                    .is_some_and(InteractiveDesktopTitlePolicyBinding::has_complete_reference))
    }

    pub(crate) fn has_current_market_authority(&self, now_ms: i64) -> bool {
        self.has_safe_market_boundary()
            && (self.market_access != InteractiveDesktopMarketAccess::PaidMarketplace
                || self
                    .title_policy
                    .as_ref()
                    .is_some_and(|policy| now_ms < policy.valid_until_ms))
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum InteractiveDesktopSurfaceKind {
    Monitor,
    Window,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum InteractiveDesktopTransportPath {
    Direct,
    Turn,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct InteractiveDesktopCaptureCapability {
    pub allowed_surface_kinds: Vec<InteractiveDesktopSurfaceKind>,
    pub max_selected_surfaces: u32,
    pub protected_content_supported: bool,
    pub secure_desktop_supported: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct InteractiveDesktopVideoProfile {
    pub codec: String,
    pub codec_profile: String,
    pub max_width_px: u32,
    pub max_height_px: u32,
    pub max_frame_rate_milli_hz: u64,
    pub max_bitrate_bits_per_second: u64,
    pub sdr_only: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct InteractiveDesktopAudioProfile {
    pub system_audio_available: bool,
    pub codec: String,
    pub max_channels: u32,
    pub max_sample_rate_hz: u32,
    pub microphone_uplink_available: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct InteractiveDesktopInputProfile {
    pub keyboard_available: bool,
    pub pointer_available: bool,
    pub gamepad_available: bool,
    pub clipboard_available: bool,
    pub file_transfer_available: bool,
    pub privilege_elevation_available: bool,
}

/// Shared capacity identity prevents a GPU, encoder, egress link, or login slot from being sold
/// independently by the batch and interactive execution planes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct InteractiveDesktopResourceBoundary {
    pub capacity_pool: ComputeCapacityPoolBinding,
    pub resource_scope_digest: String,
    pub gpu_meter: String,
    pub encoder_slot_meter: String,
    pub network_egress_meter: String,
    pub interactive_login_slot_meter: String,
}

/// Immutable interactive extension for one exact federation Offer version.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct InteractiveDesktopOfferProfile {
    pub schema: String,
    pub service_class: String,
    pub profile_id: String,
    pub profile_version: i64,
    pub profile_digest: String,
    pub offer: InteractiveDesktopOfferBinding,
    pub capture: InteractiveDesktopCaptureCapability,
    pub video: InteractiveDesktopVideoProfile,
    pub audio: InteractiveDesktopAudioProfile,
    pub input: InteractiveDesktopInputProfile,
    pub transport_paths: Vec<InteractiveDesktopTransportPath>,
    pub resource_boundary: InteractiveDesktopResourceBoundary,
    pub region_or_data_zone: String,
    pub minimum_session_duration_ms: u64,
    pub maximum_session_duration_ms: u64,
    pub valid_from_ms: i64,
    pub valid_until_ms: i64,
    pub created_at_ms: i64,
}

impl InteractiveDesktopOfferProfile {
    pub(crate) fn has_v1_shape(&self) -> bool {
        self.schema == INTERACTIVE_DESKTOP_OFFER_PROFILE_SCHEMA
            && self.service_class == INTERACTIVE_DESKTOP_SERVICE_CLASS
            && self.profile_version > 0
            && !self.profile_id.is_empty()
            && !self.profile_digest.is_empty()
            && self.offer.offer_version > 0
            && !self.offer.provider_id.is_empty()
            && !self.offer.offer_id.is_empty()
            && !self.offer.offer_digest.is_empty()
            && self.offer.profile_id == self.profile_id
            && self.offer.profile_version == self.profile_version
            && self.offer.profile_digest == self.profile_digest
            && self.offer.has_safe_market_boundary()
            && self.capture.max_selected_surfaces == 1
            && !self.capture.allowed_surface_kinds.is_empty()
            && !self.capture.protected_content_supported
            && !self.capture.secure_desktop_supported
            && self.video.codec == "h264"
            && self.video.max_width_px > 0
            && self.video.max_height_px > 0
            && self.video.max_frame_rate_milli_hz > 0
            && self.video.max_bitrate_bits_per_second > 0
            && self.video.sdr_only
            && !self.audio.microphone_uplink_available
            && !self.input.gamepad_available
            && !self.input.clipboard_available
            && !self.input.file_transfer_available
            && !self.input.privilege_elevation_available
            && self.has_transport_shape()
            && !self.region_or_data_zone.is_empty()
            && self.minimum_session_duration_ms > 0
            && self.maximum_session_duration_ms >= self.minimum_session_duration_ms
            && self.valid_until_ms > self.valid_from_ms
            && self.created_at_ms <= self.valid_until_ms
            && (self.offer.market_access != InteractiveDesktopMarketAccess::PaidMarketplace
                || self
                    .offer
                    .title_policy
                    .as_ref()
                    .is_some_and(|policy| policy.valid_until_ms >= self.valid_until_ms))
    }

    fn has_transport_shape(&self) -> bool {
        !self.transport_paths.is_empty()
            && self
                .transport_paths
                .iter()
                .enumerate()
                .all(|(index, path)| !self.transport_paths[..index].contains(path))
            && (self.offer.connectivity_policy
                != InteractiveDesktopConnectivityPolicy::RelayOnly
                || self
                    .transport_paths
                    .iter()
                    .all(|path| *path == InteractiveDesktopTransportPath::Turn))
    }
}
