use serde::{Deserialize, Serialize};

use crate::compute_federation::capacity::{
    ComputeCapacityClaimBinding, ComputeCapacityPoolBinding,
};

pub(crate) use super::authority::{
    InteractiveDesktopHostConsentBinding, InteractiveDesktopRelayAuthorityBinding,
};
use super::{
    offer::{
        InteractiveDesktopConnectivityPolicy, InteractiveDesktopMarketAccess,
        InteractiveDesktopOfferBinding, InteractiveDesktopProductMode,
        InteractiveDesktopSurfaceKind, InteractiveDesktopTransportPath,
    },
    INTERACTIVE_DESKTOP_SERVICE_CLASS,
};

pub(crate) const INTERACTIVE_DESKTOP_SESSION_REQUEST_SCHEMA: &str =
    "compute_federation.interactive_desktop.session_request.v1";
pub(crate) const INTERACTIVE_DESKTOP_SESSION_SCHEMA: &str =
    "compute_federation.interactive_desktop.session.v1";
pub(crate) const INTERACTIVE_DESKTOP_HOST_LEASE_SCHEMA: &str =
    "compute_federation.interactive_desktop.host_lease.v1";
pub(crate) const INTERACTIVE_DESKTOP_VIEWER_GRANT_SCHEMA: &str =
    "compute_federation.interactive_desktop.viewer_grant.v1";
pub(crate) const INTERACTIVE_DESKTOP_MEDIA_EPOCH_SCHEMA: &str =
    "compute_federation.interactive_desktop.media_epoch.v1";
pub(crate) const INTERACTIVE_DESKTOP_CONTROL_EPOCH_SCHEMA: &str =
    "compute_federation.interactive_desktop.control_epoch.v1";

pub(crate) const INTERACTIVE_DESKTOP_SESSION_REQUEST_DIGEST_DOMAIN: &str =
    "ELON-COMPUTE-INTERACTIVE-DESKTOP-SESSION-REQUEST-V1";
pub(crate) const INTERACTIVE_DESKTOP_SESSION_DIGEST_DOMAIN: &str =
    "ELON-COMPUTE-INTERACTIVE-DESKTOP-SESSION-V1";
pub(crate) const INTERACTIVE_DESKTOP_HOST_LEASE_DIGEST_DOMAIN: &str =
    "ELON-COMPUTE-INTERACTIVE-DESKTOP-HOST-LEASE-V1";
pub(crate) const INTERACTIVE_DESKTOP_VIEWER_GRANT_DIGEST_DOMAIN: &str =
    "ELON-COMPUTE-INTERACTIVE-DESKTOP-VIEWER-GRANT-V1";
pub(crate) const INTERACTIVE_DESKTOP_MEDIA_EPOCH_DIGEST_DOMAIN: &str =
    "ELON-COMPUTE-INTERACTIVE-DESKTOP-MEDIA-EPOCH-V1";
pub(crate) const INTERACTIVE_DESKTOP_CONTROL_EPOCH_DIGEST_DOMAIN: &str =
    "ELON-COMPUTE-INTERACTIVE-DESKTOP-CONTROL-EPOCH-V1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct InteractiveDesktopPriceSnapshotBinding {
    pub price_snapshot_id: String,
    pub price_snapshot_digest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct InteractiveDesktopReservationBinding {
    pub reservation_id: String,
    pub reservation_revision: i64,
    pub reservation_digest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct InteractiveDesktopFederationBinding {
    pub binding_digest: String,
    pub provider_id: String,
    pub consumer_account_id: String,
    pub offer: InteractiveDesktopOfferBinding,
    pub price_snapshot: InteractiveDesktopPriceSnapshotBinding,
    pub reservation: InteractiveDesktopReservationBinding,
    pub capacity_pool: ComputeCapacityPoolBinding,
    pub capacity_claim: ComputeCapacityClaimBinding,
}

impl InteractiveDesktopFederationBinding {
    pub(crate) fn has_complete_reference(&self) -> bool {
        !self.binding_digest.is_empty()
            && !self.provider_id.is_empty()
            && !self.consumer_account_id.is_empty()
            && self.offer.provider_id == self.provider_id
            && !self.offer.offer_id.is_empty()
            && self.offer.offer_version > 0
            && !self.offer.offer_digest.is_empty()
            && !self.offer.profile_id.is_empty()
            && self.offer.profile_version > 0
            && !self.offer.profile_digest.is_empty()
            && !self.price_snapshot.price_snapshot_id.is_empty()
            && !self.price_snapshot.price_snapshot_digest.is_empty()
            && !self.reservation.reservation_id.is_empty()
            && self.reservation.reservation_revision > 0
            && !self.reservation.reservation_digest.is_empty()
            && !self.capacity_pool.pool_id.is_empty()
            && self.capacity_pool.capacity_epoch > 0
            && self.capacity_pool.pool_revision > 0
            && !self.capacity_pool.pool_digest.is_empty()
            && !self.capacity_claim.claim_id.is_empty()
            && self.capacity_claim.claim_revision > 0
            && !self.capacity_claim.claim_digest.is_empty()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct InteractiveDesktopSurfaceSelection {
    pub surface_kind: InteractiveDesktopSurfaceKind,
    pub selection_digest: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum InteractiveDesktopViewerRelationship {
    SameOwner,
    TrustedFriend,
    MarketplaceStranger,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct InteractiveDesktopPermissionSet {
    pub capture_selected_surface: bool,
    pub view_video: bool,
    pub receive_system_audio: bool,
    pub send_keyboard_input: bool,
    pub send_pointer_input: bool,
    pub clipboard_sync: bool,
    pub file_transfer: bool,
    pub microphone_uplink: bool,
    pub camera_uplink: bool,
    pub privilege_elevation: bool,
    pub secure_desktop_control: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum InteractiveDesktopAction {
    ViewVideo,
    ReceiveSystemAudio,
    SendKeyboardInput,
    SendPointerInput,
    ClipboardSync,
    FileTransfer,
    MicrophoneUplink,
    CameraUplink,
    PrivilegeElevation,
    SecureDesktopControl,
}

impl InteractiveDesktopPermissionSet {
    pub(crate) fn is_v1_safe(&self) -> bool {
        !self.clipboard_sync
            && !self.file_transfer
            && !self.microphone_uplink
            && !self.camera_uplink
            && !self.privilege_elevation
            && !self.secure_desktop_control
    }

    pub(crate) fn is_subset_of(&self, parent: &Self) -> bool {
        (!self.capture_selected_surface || parent.capture_selected_surface)
            && (!self.view_video || parent.view_video)
            && (!self.receive_system_audio || parent.receive_system_audio)
            && (!self.send_keyboard_input || parent.send_keyboard_input)
            && (!self.send_pointer_input || parent.send_pointer_input)
            && (!self.clipboard_sync || parent.clipboard_sync)
            && (!self.file_transfer || parent.file_transfer)
            && (!self.microphone_uplink || parent.microphone_uplink)
            && (!self.camera_uplink || parent.camera_uplink)
            && (!self.privilege_elevation || parent.privilege_elevation)
            && (!self.secure_desktop_control || parent.secure_desktop_control)
    }

    pub(crate) fn allows(&self, action: InteractiveDesktopAction) -> bool {
        match action {
            InteractiveDesktopAction::ViewVideo => {
                self.capture_selected_surface && self.view_video
            }
            InteractiveDesktopAction::ReceiveSystemAudio => {
                self.capture_selected_surface && self.view_video && self.receive_system_audio
            }
            InteractiveDesktopAction::SendKeyboardInput => {
                self.capture_selected_surface && self.view_video && self.send_keyboard_input
            }
            InteractiveDesktopAction::SendPointerInput => {
                self.capture_selected_surface && self.view_video && self.send_pointer_input
            }
            InteractiveDesktopAction::ClipboardSync
            | InteractiveDesktopAction::FileTransfer
            | InteractiveDesktopAction::MicrophoneUplink
            | InteractiveDesktopAction::CameraUplink
            | InteractiveDesktopAction::PrivilegeElevation
            | InteractiveDesktopAction::SecureDesktopControl => false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct InteractiveDesktopSessionRequest {
    pub schema: String,
    pub service_class: String,
    pub request_id: String,
    pub request_digest: String,
    pub session_id: String,
    pub idempotency_key: String,
    pub binding: InteractiveDesktopFederationBinding,
    pub viewer_relationship: InteractiveDesktopViewerRelationship,
    pub requested_surface_kind: InteractiveDesktopSurfaceKind,
    pub requested_permissions: InteractiveDesktopPermissionSet,
    pub requested_width_px: u32,
    pub requested_height_px: u32,
    pub requested_frame_rate_milli_hz: u64,
    pub requested_duration_ms: u64,
    pub requested_at_ms: i64,
    pub connect_deadline_ms: i64,
}

impl InteractiveDesktopSessionRequest {
    pub(crate) fn has_safe_product_boundary(&self) -> bool {
        self.schema == INTERACTIVE_DESKTOP_SESSION_REQUEST_SCHEMA
            && self.service_class == INTERACTIVE_DESKTOP_SERVICE_CLASS
            && !self.request_id.is_empty()
            && !self.request_digest.is_empty()
            && !self.session_id.is_empty()
            && !self.idempotency_key.is_empty()
            && self.binding.has_complete_reference()
            && self.binding.offer.has_safe_market_boundary()
            && self.requested_permissions.is_v1_safe()
            && self.requested_permissions.capture_selected_surface
            && self.requested_permissions.view_video
            && self.requested_width_px > 0
            && self.requested_height_px > 0
            && self.requested_frame_rate_milli_hz > 0
            && self.requested_duration_ms > 0
            && self.connect_deadline_ms > self.requested_at_ms
            && matches!(
                (
                    self.binding.offer.product_mode,
                    self.viewer_relationship,
                ),
                (
                    InteractiveDesktopProductMode::SameOwnerRemoteAccess,
                    InteractiveDesktopViewerRelationship::SameOwner
                ) | (
                    InteractiveDesktopProductMode::FriendCoPlay,
                    InteractiveDesktopViewerRelationship::TrustedFriend
                ) | (
                    InteractiveDesktopProductMode::LicensedCloudSeat,
                    InteractiveDesktopViewerRelationship::MarketplaceStranger
                )
            )
            && (self.viewer_relationship
                != InteractiveDesktopViewerRelationship::MarketplaceStranger
                || (self.binding.offer.connectivity_policy
                    == InteractiveDesktopConnectivityPolicy::RelayOnly
                    && self.binding.offer.market_access
                        == InteractiveDesktopMarketAccess::PaidMarketplace))
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum InteractiveDesktopSessionState {
    Requested,
    Reserved,
    HostLeased,
    ViewerGranted,
    Connecting,
    Active,
    Reconnecting,
    Ending,
    Ended,
    Canceled,
    Failed,
}

impl InteractiveDesktopSessionState {
    pub(crate) fn allows_transition(self, next: Self) -> bool {
        self == next
            || matches!(
                (self, next),
                (Self::Requested, Self::Reserved | Self::Canceled | Self::Failed)
                    | (Self::Reserved, Self::HostLeased | Self::Canceled | Self::Failed)
                    | (Self::HostLeased, Self::ViewerGranted | Self::Ending | Self::Failed)
                    | (Self::ViewerGranted, Self::Connecting | Self::Ending | Self::Failed)
                    | (
                        Self::Connecting,
                        Self::Active | Self::Reconnecting | Self::Ending | Self::Failed
                    )
                    | (Self::Active, Self::Reconnecting | Self::Ending | Self::Failed)
                    | (Self::Reconnecting, Self::Connecting | Self::Ending | Self::Failed)
                    | (Self::Ending, Self::Ended | Self::Failed)
            )
    }

    pub(crate) fn is_terminal(self) -> bool {
        matches!(self, Self::Ended | Self::Canceled | Self::Failed)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct InteractiveDesktopSession {
    pub schema: String,
    pub service_class: String,
    pub session_id: String,
    pub session_root_digest: String,
    pub session_revision: i64,
    pub session_digest: String,
    pub request_id: String,
    pub request_digest: String,
    pub binding: InteractiveDesktopFederationBinding,
    pub viewer_relationship: InteractiveDesktopViewerRelationship,
    pub state: InteractiveDesktopSessionState,
    pub host_lease_id: String,
    pub selected_surface_digest: String,
    pub viewer_grant_id: String,
    pub viewer_grant_generation: u64,
    pub media_epoch_id: String,
    pub media_epoch_sequence: u64,
    pub control_epoch_id: String,
    pub control_epoch_sequence: u64,
    pub fencing_generation: u64,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
    pub maximum_end_at_ms: i64,
    pub terminal_reason_code: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum InteractiveDesktopHostLeaseState {
    Issued,
    Active,
    Released,
    Expired,
    Revoked,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct InteractiveDesktopHostLease {
    pub schema: String,
    pub service_class: String,
    pub host_lease_id: String,
    pub host_lease_digest: String,
    pub session_id: String,
    pub binding_digest: String,
    pub provider_id: String,
    pub host_node_id: String,
    pub provider_node_binding_digest: String,
    pub endpoint_credential_digest: String,
    pub selected_surface: InteractiveDesktopSurfaceSelection,
    pub state: InteractiveDesktopHostLeaseState,
    pub fencing_generation: u64,
    pub host_consent: InteractiveDesktopHostConsentBinding,
    pub issued_at_ms: i64,
    pub last_heartbeat_at_ms: Option<i64>,
    pub expires_at_ms: i64,
    pub hard_deadline_at_ms: i64,
    pub terminal_reason_code: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum InteractiveDesktopGrantState {
    Issued,
    Active,
    Expired,
    Revoked,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct InteractiveDesktopViewerGrant {
    pub schema: String,
    pub service_class: String,
    pub viewer_grant_id: String,
    pub viewer_grant_digest: String,
    pub grant_generation: u64,
    pub session_id: String,
    pub binding_digest: String,
    pub consumer_account_id: String,
    pub consumer_account_session_digest: String,
    pub account_auth_epoch: u64,
    pub viewer_device_key_digest: String,
    pub viewer_transport_identity_digest: String,
    pub permissions: InteractiveDesktopPermissionSet,
    pub state: InteractiveDesktopGrantState,
    pub issued_at_ms: i64,
    pub expires_at_ms: i64,
    pub revoked_at_ms: Option<i64>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum InteractiveDesktopEpochState {
    Issued,
    Active,
    Closed,
    Expired,
    Revoked,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct InteractiveDesktopMediaEpoch {
    pub schema: String,
    pub service_class: String,
    pub media_epoch_id: String,
    pub media_epoch_digest: String,
    pub epoch_sequence: u64,
    pub session_id: String,
    pub binding_digest: String,
    pub host_lease_id: String,
    pub viewer_grant_id: String,
    pub viewer_grant_generation: u64,
    pub viewer_transport_identity_digest: String,
    pub selected_surface_digest: String,
    pub fencing_generation: u64,
    pub state: InteractiveDesktopEpochState,
    pub transport_path: InteractiveDesktopTransportPath,
    pub relay_authority: Option<InteractiveDesktopRelayAuthorityBinding>,
    pub video_codec: String,
    pub audio_codec: Option<String>,
    pub issued_at_ms: i64,
    pub expires_at_ms: i64,
    pub ended_at_ms: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct InteractiveDesktopControlEpoch {
    pub schema: String,
    pub service_class: String,
    pub control_epoch_id: String,
    pub control_epoch_digest: String,
    pub epoch_sequence: u64,
    pub session_id: String,
    pub binding_digest: String,
    pub host_lease_id: String,
    pub viewer_grant_id: String,
    pub viewer_grant_generation: u64,
    pub media_epoch_id: String,
    pub selected_surface_digest: String,
    pub fencing_generation: u64,
    pub permissions: InteractiveDesktopPermissionSet,
    pub state: InteractiveDesktopEpochState,
    pub issued_at_ms: i64,
    pub expires_at_ms: i64,
    pub ended_at_ms: Option<i64>,
}
