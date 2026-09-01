use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::compute_federation::capacity::{
    ComputeCapacityClaimBinding, ComputeCapacityPoolBinding,
};

use super::{
    offer::{
        InteractiveDesktopConnectivityPolicy, InteractiveDesktopMarketAccess,
        InteractiveDesktopOfferBinding, InteractiveDesktopOfferProfile,
        InteractiveDesktopSurfaceKind, InteractiveDesktopTransportPath,
    },
    product_authority::InteractiveDesktopProductAuthorityBinding,
    session::{InteractiveDesktopPermissionSet, InteractiveDesktopSessionRequest},
    INTERACTIVE_DESKTOP_SERVICE_CLASS,
};

pub(crate) const INTERACTIVE_DESKTOP_SESSION_RESERVATION_SCHEMA: &str =
    "compute_federation.interactive_desktop.session_reservation.v1";
pub(crate) const INTERACTIVE_DESKTOP_SESSION_RESERVATION_DIGEST_DOMAIN: &str =
    "ELON-COMPUTE-INTERACTIVE-DESKTOP-SESSION-RESERVATION-V1";

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

/// Identity of the interactive execution-plane reservation itself. This is distinct from the
/// shared federation Reservation stored inside `InteractiveDesktopFederationBinding`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct InteractiveDesktopSessionReservationBinding {
    pub session_reservation_id: String,
    pub session_reservation_revision: i64,
    pub session_reservation_digest: String,
}

impl InteractiveDesktopSessionReservationBinding {
    pub(crate) fn has_complete_reference(&self) -> bool {
        !self.session_reservation_id.is_empty()
            && self.session_reservation_revision > 0
            && !self.session_reservation_digest.is_empty()
    }
}

/// Exact shared-control-plane references resolved before a Session may obtain a HostLease.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct InteractiveDesktopFederationBinding {
    pub binding_digest: String,
    pub provider_id: String,
    pub provider_policy_revision: u64,
    pub provider_digest: String,
    pub provider_owner_account_id: String,
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
            && self.provider_policy_revision > 0
            && !self.provider_digest.is_empty()
            && !self.provider_owner_account_id.is_empty()
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
pub(crate) struct InteractiveDesktopReservedMeterBudget {
    pub meter: String,
    pub maximum_quantity: u64,
}

/// Broker-created result. A request never contains these selected commercial or capacity facts.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct InteractiveDesktopSessionReservation {
    pub schema: String,
    pub service_class: String,
    pub session_reservation: InteractiveDesktopSessionReservationBinding,
    pub request_id: String,
    pub request_digest: String,
    pub session_id: String,
    pub binding: InteractiveDesktopFederationBinding,
    pub product_authority: InteractiveDesktopProductAuthorityBinding,
    pub resource_scope_digest: String,
    pub reserved_surface_kind: InteractiveDesktopSurfaceKind,
    pub reserved_permissions: InteractiveDesktopPermissionSet,
    pub reserved_width_px: u32,
    pub reserved_height_px: u32,
    pub reserved_frame_rate_milli_hz: u64,
    pub reserved_duration_ms: u64,
    pub permitted_transport_paths: Vec<InteractiveDesktopTransportPath>,
    pub video_codec: String,
    pub audio_codec: Option<String>,
    pub currency: String,
    pub consumer_max_amount_micros: u64,
    pub meter_budgets: Vec<InteractiveDesktopReservedMeterBudget>,
    pub issued_at_ms: i64,
    pub activation_deadline_ms: i64,
    pub authorization_expires_at_ms: i64,
    pub maximum_end_at_ms: i64,
}

impl InteractiveDesktopSessionReservation {
    /// Structural check only. The caller must resolve every referenced digest from its owner.
    pub(crate) fn has_safe_shape(
        &self,
        profile: &InteractiveDesktopOfferProfile,
        now_ms: i64,
    ) -> bool {
        self.schema == INTERACTIVE_DESKTOP_SESSION_RESERVATION_SCHEMA
            && self.service_class == INTERACTIVE_DESKTOP_SERVICE_CLASS
            && self.session_reservation.has_complete_reference()
            && !self.request_id.is_empty()
            && !self.request_digest.is_empty()
            && !self.session_id.is_empty()
            && self.binding.has_complete_reference()
            && profile.has_v1_shape()
            && self.binding.offer == profile.offer
            && self.binding.provider_id == profile.offer.provider_id
            && self.binding.capacity_pool == profile.resource_boundary.capacity_pool
            && self.resource_scope_digest == profile.resource_boundary.resource_scope_digest
            && self.product_authority.provider_owner_account_id
                == self.binding.provider_owner_account_id
            && self.product_authority.structurally_authorizes(
                &self.session_id,
                &self.binding.provider_id,
                &self.binding.consumer_account_id,
                self.binding.offer.product_mode,
                self.product_authority.viewer_relationship,
                self.binding.offer.title_policy.as_ref(),
                now_ms,
            )
            && self.product_authority.issued_at_ms <= self.issued_at_ms
            && profile.offer.has_current_market_authority(now_ms)
            && profile
                .capture
                .allowed_surface_kinds
                .contains(&self.reserved_surface_kind)
            && self.reserved_permissions.is_v1_safe()
            && self.reserved_permissions.capture_selected_surface
            && self.reserved_permissions.view_video
            && permissions_fit_profile(&self.reserved_permissions, profile)
            && self.reserved_width_px > 0
            && self.reserved_width_px <= profile.video.max_width_px
            && self.reserved_height_px > 0
            && self.reserved_height_px <= profile.video.max_height_px
            && self.reserved_frame_rate_milli_hz > 0
            && self.reserved_frame_rate_milli_hz <= profile.video.max_frame_rate_milli_hz
            && self.reserved_duration_ms >= profile.minimum_session_duration_ms
            && self.reserved_duration_ms <= profile.maximum_session_duration_ms
            && self.video_codec == profile.video.codec
            && audio_codec_fits(&self.reserved_permissions, &self.audio_codec, profile)
            && transport_paths_fit(self, profile)
            && commercial_terms_fit(self)
            && meter_budgets_fit(self, profile)
            && profile.created_at_ms <= self.issued_at_ms
            && profile.valid_from_ms <= self.issued_at_ms
            && profile.valid_from_ms <= now_ms
            && now_ms < profile.valid_until_ms
            && self.issued_at_ms <= now_ms
            && self.issued_at_ms < self.activation_deadline_ms
            && self.activation_deadline_ms <= self.maximum_end_at_ms
            && self.activation_deadline_ms <= self.authorization_expires_at_ms
            && now_ms < self.authorization_expires_at_ms
            && self.issued_at_ms < self.maximum_end_at_ms
            && self.maximum_end_at_ms <= self.authorization_expires_at_ms
            && self.maximum_end_at_ms <= profile.valid_until_ms
            && self.maximum_end_at_ms <= self.product_authority.expires_at_ms
            && self
                .maximum_end_at_ms
                .checked_sub(self.issued_at_ms)
                .is_some_and(|duration| duration >= 0 && duration as u64 <= self.reserved_duration_ms)
    }

    pub(crate) fn cross_validates_request(
        &self,
        request: &InteractiveDesktopSessionRequest,
        profile: &InteractiveDesktopOfferProfile,
        now_ms: i64,
    ) -> bool {
        self.has_safe_shape(profile, now_ms)
            && request.has_safe_request_shape()
            && self.request_id == request.request_id
            && self.request_digest == request.request_digest
            && self.session_id == request.session_id
            && self.binding.consumer_account_id == request.consumer_account_id
            && self.binding.offer.product_mode == request.requested_product_mode
            && self.product_authority.viewer_relationship == request.viewer_relationship
            && profile.region_or_data_zone == request.requested_region_or_data_zone
            && self.reserved_surface_kind == request.requested_surface_kind
            && self
                .reserved_permissions
                .is_subset_of(&request.requested_permissions)
            && self.reserved_width_px <= request.requested_width_px
            && self.reserved_height_px <= request.requested_height_px
            && self.reserved_frame_rate_milli_hz <= request.requested_frame_rate_milli_hz
            && self.reserved_duration_ms <= request.requested_duration_ms
            && self.currency == request.requested_currency
            && self.consumer_max_amount_micros <= request.consumer_max_amount_micros
            && self.permitted_transport_paths.iter().all(|path| {
                request.acceptable_transport_paths.contains(path)
            })
            && self.issued_at_ms >= request.requested_at_ms
            && self.issued_at_ms < request.connect_deadline_ms
            && self.activation_deadline_ms <= request.connect_deadline_ms
    }
}

fn permissions_fit_profile(
    permissions: &InteractiveDesktopPermissionSet,
    profile: &InteractiveDesktopOfferProfile,
) -> bool {
    (!permissions.receive_system_audio || profile.audio.system_audio_available)
        && (!permissions.send_keyboard_input || profile.input.keyboard_available)
        && (!permissions.send_pointer_input || profile.input.pointer_available)
        && !permissions.clipboard_sync
        && !permissions.file_transfer
        && !permissions.microphone_uplink
        && !permissions.camera_uplink
        && !permissions.privilege_elevation
        && !permissions.secure_desktop_control
}

fn audio_codec_fits(
    permissions: &InteractiveDesktopPermissionSet,
    audio_codec: &Option<String>,
    profile: &InteractiveDesktopOfferProfile,
) -> bool {
    if permissions.receive_system_audio {
        audio_codec.as_deref() == Some(profile.audio.codec.as_str())
    } else {
        audio_codec.is_none()
    }
}

fn transport_paths_fit(
    reservation: &InteractiveDesktopSessionReservation,
    profile: &InteractiveDesktopOfferProfile,
) -> bool {
    let mut paths = BTreeSet::new();
    !reservation.permitted_transport_paths.is_empty()
        && reservation
            .permitted_transport_paths
            .iter()
            .all(|path| paths.insert(*path) && profile.transport_paths.contains(path))
        && (profile.offer.connectivity_policy
            != InteractiveDesktopConnectivityPolicy::RelayOnly
            || reservation
                .permitted_transport_paths
                .iter()
                .all(|path| *path == InteractiveDesktopTransportPath::Turn))
}

fn commercial_terms_fit(reservation: &InteractiveDesktopSessionReservation) -> bool {
    !reservation.currency.is_empty()
        && match reservation.binding.offer.market_access {
            InteractiveDesktopMarketAccess::PrivateUnpaid => {
                reservation.consumer_max_amount_micros == 0
            }
            InteractiveDesktopMarketAccess::PaidMarketplace => {
                reservation.consumer_max_amount_micros > 0
            }
        }
}

fn meter_budgets_fit(
    reservation: &InteractiveDesktopSessionReservation,
    profile: &InteractiveDesktopOfferProfile,
) -> bool {
    let boundary = &profile.resource_boundary;
    let required = [
        boundary.gpu_meter.as_str(),
        boundary.encoder_slot_meter.as_str(),
        boundary.network_egress_meter.as_str(),
        boundary.interactive_login_slot_meter.as_str(),
    ];
    let mut meters = BTreeSet::new();
    reservation.meter_budgets.len() == required.len()
        && reservation
            .meter_budgets
            .iter()
            .all(|budget| {
                budget.maximum_quantity > 0
                    && meters.insert(budget.meter.as_str())
                    && required.contains(&budget.meter.as_str())
            })
        && required.iter().all(|meter| meters.contains(*meter))
}
