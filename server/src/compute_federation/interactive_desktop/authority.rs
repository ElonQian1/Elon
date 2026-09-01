use serde::{Deserialize, Serialize};

use super::{
    offer::{
        InteractiveDesktopConnectivityPolicy, InteractiveDesktopMarketAccess,
        InteractiveDesktopOfferProfile, InteractiveDesktopProductMode,
        InteractiveDesktopTransportPath,
    },
    reservation::InteractiveDesktopSessionReservation,
    session::{
        InteractiveDesktopAction, InteractiveDesktopControlEpoch, InteractiveDesktopEpochState,
        InteractiveDesktopGrantState, InteractiveDesktopHostLease,
        InteractiveDesktopHostLeaseState, InteractiveDesktopMediaEpoch, InteractiveDesktopSession,
        InteractiveDesktopSessionState, InteractiveDesktopViewerGrant,
        InteractiveDesktopViewerRelationship, INTERACTIVE_DESKTOP_CONTROL_EPOCH_SCHEMA,
        INTERACTIVE_DESKTOP_HOST_LEASE_SCHEMA, INTERACTIVE_DESKTOP_MEDIA_EPOCH_SCHEMA,
        INTERACTIVE_DESKTOP_SESSION_SCHEMA, INTERACTIVE_DESKTOP_VIEWER_GRANT_SCHEMA,
    },
    INTERACTIVE_DESKTOP_SERVICE_CLASS,
};

pub(crate) const INTERACTIVE_DESKTOP_HOST_CONSENT_SCHEMA: &str =
    "compute_federation.interactive_desktop.host_consent.v1";
pub(crate) const INTERACTIVE_DESKTOP_RELAY_AUTHORITY_SCHEMA: &str =
    "compute_federation.interactive_desktop.relay_authority.v1";
pub(crate) const INTERACTIVE_DESKTOP_HOST_CONSENT_DIGEST_DOMAIN: &str =
    "ELON-COMPUTE-INTERACTIVE-DESKTOP-HOST-CONSENT-V1";
pub(crate) const INTERACTIVE_DESKTOP_RELAY_AUTHORITY_DIGEST_DOMAIN: &str =
    "ELON-COMPUTE-INTERACTIVE-DESKTOP-RELAY-AUTHORITY-V1";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum InteractiveDesktopAuthorityCurrentness {
    Current,
    Superseded,
    Revoked,
    Expired,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct InteractiveDesktopHostConsentScope {
    pub session_id: String,
    pub session_reservation_digest: String,
    pub binding_digest: String,
    pub host_lease_id: String,
    pub fencing_generation: u64,
    pub selected_surface_digest: String,
    pub permissions: super::session::InteractiveDesktopPermissionSet,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct InteractiveDesktopHostConsentBinding {
    pub schema: String,
    pub service_class: String,
    pub consent_id: String,
    pub consent_digest: String,
    pub consent_revision: u64,
    pub policy_id: String,
    pub policy_revision: u64,
    pub policy_digest: String,
    pub currentness: InteractiveDesktopAuthorityCurrentness,
    pub scope: InteractiveDesktopHostConsentScope,
    pub issued_at_ms: i64,
    pub expires_at_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct InteractiveDesktopRelayAuthorityScope {
    pub session_id: String,
    pub session_reservation_digest: String,
    pub binding_digest: String,
    pub host_lease_id: String,
    pub fencing_generation: u64,
    pub viewer_grant_id: String,
    pub viewer_grant_generation: u64,
    pub viewer_transport_identity_digest: String,
    pub media_epoch_id: String,
    pub media_epoch_sequence: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct InteractiveDesktopRelayAuthorityBinding {
    pub schema: String,
    pub service_class: String,
    pub relay_authority_id: String,
    pub relay_authority_digest: String,
    pub relay_allocation_ref_digest: String,
    pub relay_grant_digest: String,
    pub relay_region: String,
    pub currentness: InteractiveDesktopAuthorityCurrentness,
    pub scope: InteractiveDesktopRelayAuthorityScope,
    pub issued_at_ms: i64,
    pub expires_at_ms: i64,
}

impl InteractiveDesktopSession {
    pub(crate) fn has_safe_product_boundary(
        &self,
        profile: &InteractiveDesktopOfferProfile,
        reservation: &InteractiveDesktopSessionReservation,
        transport_path: InteractiveDesktopTransportPath,
        now_ms: i64,
    ) -> bool {
        let mode_matches_relationship = matches!(
            (self.binding.offer.product_mode, self.viewer_relationship),
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
        );
        self.binding.offer.has_safe_market_boundary()
            && reservation.has_safe_shape(profile, now_ms)
            && self.request_id == reservation.request_id
            && self.request_digest == reservation.request_digest
            && self.session_id == reservation.session_id
            && self.session_reservation == reservation.session_reservation
            && self.binding == reservation.binding
            && self.viewer_relationship == reservation.product_authority.viewer_relationship
            && self.maximum_end_at_ms <= reservation.maximum_end_at_ms
            && mode_matches_relationship
            && (self.binding.offer.market_access != InteractiveDesktopMarketAccess::PaidMarketplace
                || self.binding.offer.product_mode
                    == InteractiveDesktopProductMode::LicensedCloudSeat)
            && reservation
                .permitted_transport_paths
                .contains(&transport_path)
            && profile.transport_paths.contains(&transport_path)
            && (self.binding.offer.connectivity_policy
                != InteractiveDesktopConnectivityPolicy::RelayOnly
                || transport_path == InteractiveDesktopTransportPath::Turn)
            && (self.viewer_relationship
                != InteractiveDesktopViewerRelationship::MarketplaceStranger
                || (self.binding.offer.market_access
                    == InteractiveDesktopMarketAccess::PaidMarketplace
                    && transport_path == InteractiveDesktopTransportPath::Turn))
    }

    /// Structural gate only. Runtime callers must first verify every referenced digest and proof.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn structurally_authorizes(
        &self,
        profile: &InteractiveDesktopOfferProfile,
        reservation: &InteractiveDesktopSessionReservation,
        lease: &InteractiveDesktopHostLease,
        grant: &InteractiveDesktopViewerGrant,
        media: &InteractiveDesktopMediaEpoch,
        control: &InteractiveDesktopControlEpoch,
        viewer_account_id: &str,
        viewer_account_session_digest: &str,
        viewer_account_auth_epoch: u64,
        viewer_device_key_digest: &str,
        viewer_transport_identity_digest: &str,
        action: InteractiveDesktopAction,
        now_ms: i64,
    ) -> bool {
        self.schema == INTERACTIVE_DESKTOP_SESSION_SCHEMA
            && self.service_class == INTERACTIVE_DESKTOP_SERVICE_CLASS
            && lease.schema == INTERACTIVE_DESKTOP_HOST_LEASE_SCHEMA
            && lease.service_class == INTERACTIVE_DESKTOP_SERVICE_CLASS
            && grant.schema == INTERACTIVE_DESKTOP_VIEWER_GRANT_SCHEMA
            && grant.service_class == INTERACTIVE_DESKTOP_SERVICE_CLASS
            && media.schema == INTERACTIVE_DESKTOP_MEDIA_EPOCH_SCHEMA
            && media.service_class == INTERACTIVE_DESKTOP_SERVICE_CLASS
            && control.schema == INTERACTIVE_DESKTOP_CONTROL_EPOCH_SCHEMA
            && control.service_class == INTERACTIVE_DESKTOP_SERVICE_CLASS
            && self.state == InteractiveDesktopSessionState::Active
            && self.terminal_reason_code.is_none()
            && has_nonempty_authority_roots(self, lease, grant, media, control)
            && self.binding.has_complete_reference()
            && self.has_safe_product_boundary(profile, reservation, media.transport_path, now_ms)
            && self.binding.offer.has_current_market_authority(now_ms)
            && reservation.issued_at_ms <= lease.host_consent.issued_at_ms
            && has_host_consent_authority(self, lease, grant, control, action, now_ms)
            && has_transport_authority(media, lease, grant, &profile.region_or_data_zone, now_ms)
            && self.binding.offer.provider_id == self.binding.provider_id
            && self.authority_head.matches(lease, grant, media, control)
            && grant.grant_generation == media.viewer_grant_generation
            && grant.grant_generation == control.viewer_grant_generation
            && grant.viewer_transport_identity_digest == media.viewer_transport_identity_digest
            && grant.viewer_transport_identity_digest == control.viewer_transport_identity_digest
            && self.binding.binding_digest == lease.binding_digest
            && self.binding.binding_digest == grant.binding_digest
            && self.binding.binding_digest == media.binding_digest
            && self.binding.binding_digest == control.binding_digest
            && self.session_reservation.session_reservation_digest
                == lease.session_reservation_digest
            && self.session_reservation.session_reservation_digest
                == grant.session_reservation_digest
            && self.session_reservation.session_reservation_digest
                == media.session_reservation_digest
            && self.session_reservation.session_reservation_digest
                == control.session_reservation_digest
            && self.binding.provider_id == lease.provider_id
            && self.binding.consumer_account_id == grant.consumer_account_id
            && lease.selected_surface.surface_kind == reservation.reserved_surface_kind
            && grant
                .permissions
                .is_subset_of(&reservation.reserved_permissions)
            && control
                .permissions
                .is_subset_of(&reservation.reserved_permissions)
            && media.video_codec == reservation.video_codec
            && media.audio_codec == reservation.audio_codec
            && self.session_id == lease.session_id
            && self.session_id == grant.session_id
            && self.session_id == media.session_id
            && self.session_id == control.session_id
            && lease.host_lease_id == media.host_lease_id
            && lease.host_lease_id == control.host_lease_id
            && grant.viewer_grant_id == media.viewer_grant_id
            && grant.viewer_grant_id == control.viewer_grant_id
            && media.media_epoch_id == control.media_epoch_id
            && media.media_epoch_digest == control.media_epoch_digest
            && media.epoch_sequence == control.media_epoch_sequence
            && lease.state == InteractiveDesktopHostLeaseState::Active
            && grant.state == InteractiveDesktopGrantState::Active
            && media.state == InteractiveDesktopEpochState::Active
            && control.state == InteractiveDesktopEpochState::Active
            && lease.terminal_reason_code.is_none()
            && grant.revoked_at_ms.is_none()
            && media.ended_at_ms.is_none()
            && control.ended_at_ms.is_none()
            && !lease.provider_node_binding_digest.is_empty()
            && !lease.endpoint_credential_digest.is_empty()
            && !lease.selected_surface.selection_digest.is_empty()
            && grant.account_auth_epoch > 0
            && !grant.consumer_account_session_digest.is_empty()
            && !grant.viewer_device_key_digest.is_empty()
            && !grant.viewer_transport_identity_digest.is_empty()
            && self.created_at_ms <= self.updated_at_ms
            && reservation.issued_at_ms <= self.updated_at_ms
            && self.updated_at_ms <= now_ms
            && lease.issued_at_ms <= now_ms
            && lease.issued_at_ms >= reservation.issued_at_ms
            && lease.activated_at_ms.is_some_and(|activated_at_ms| {
                lease.issued_at_ms <= activated_at_ms
                    && activated_at_ms <= reservation.activation_deadline_ms
                    && activated_at_ms <= now_ms
                    && activated_at_ms <= self.updated_at_ms
                    && activated_at_ms <= grant.issued_at_ms
                    && activated_at_ms <= media.issued_at_ms
            })
            && now_ms < lease.expires_at_ms
            && now_ms < lease.hard_deadline_at_ms
            && grant.issued_at_ms <= now_ms
            && now_ms < grant.expires_at_ms
            && media.issued_at_ms <= now_ms
            && grant.issued_at_ms <= media.issued_at_ms
            && now_ms < media.expires_at_ms
            && control.issued_at_ms <= now_ms
            && media.issued_at_ms <= control.issued_at_ms
            && control.issued_at_ms <= self.updated_at_ms
            && now_ms < control.expires_at_ms
            && now_ms < self.maximum_end_at_ms
            && lease.expires_at_ms <= self.maximum_end_at_ms
            && lease.hard_deadline_at_ms <= self.maximum_end_at_ms
            && grant.expires_at_ms <= self.maximum_end_at_ms
            && media.expires_at_ms <= self.maximum_end_at_ms
            && control.expires_at_ms <= self.maximum_end_at_ms
            && grant.consumer_account_id == viewer_account_id
            && grant.consumer_account_session_digest == viewer_account_session_digest
            && grant.account_auth_epoch == viewer_account_auth_epoch
            && grant.viewer_device_key_digest == viewer_device_key_digest
            && grant.viewer_transport_identity_digest == viewer_transport_identity_digest
            && grant.permissions.is_v1_safe()
            && control.permissions.is_v1_safe()
            && control.permissions.is_subset_of(&grant.permissions)
            && control.permissions.allows(action)
    }
}

fn has_nonempty_authority_roots(
    session: &InteractiveDesktopSession,
    lease: &InteractiveDesktopHostLease,
    grant: &InteractiveDesktopViewerGrant,
    media: &InteractiveDesktopMediaEpoch,
    control: &InteractiveDesktopControlEpoch,
) -> bool {
    !session.session_id.is_empty()
        && !session.session_root_digest.is_empty()
        && session.session_revision > 0
        && !session.session_digest.is_empty()
        && !session.request_id.is_empty()
        && !session.request_digest.is_empty()
        && !session.binding.binding_digest.is_empty()
        && !session.binding.provider_id.is_empty()
        && !session.binding.consumer_account_id.is_empty()
        && session.authority_head.has_complete_reference()
        && !lease.host_lease_id.is_empty()
        && !lease.host_lease_digest.is_empty()
        && !lease.host_node_id.is_empty()
        && lease.fencing_generation > 0
        && !grant.viewer_grant_id.is_empty()
        && !grant.viewer_grant_digest.is_empty()
        && grant.grant_generation > 0
        && !media.media_epoch_id.is_empty()
        && !media.media_epoch_digest.is_empty()
        && media.epoch_sequence > 0
        && media.fencing_generation > 0
        && !control.control_epoch_id.is_empty()
        && !control.control_epoch_digest.is_empty()
        && control.epoch_sequence > 0
        && control.fencing_generation > 0
}

fn has_host_consent_authority(
    session: &InteractiveDesktopSession,
    lease: &InteractiveDesktopHostLease,
    grant: &InteractiveDesktopViewerGrant,
    control: &InteractiveDesktopControlEpoch,
    action: InteractiveDesktopAction,
    now_ms: i64,
) -> bool {
    let consent = &lease.host_consent;
    let scope = &consent.scope;
    consent.schema == INTERACTIVE_DESKTOP_HOST_CONSENT_SCHEMA
        && consent.service_class == INTERACTIVE_DESKTOP_SERVICE_CLASS
        && !consent.consent_id.is_empty()
        && !consent.consent_digest.is_empty()
        && consent.consent_revision > 0
        && !consent.policy_id.is_empty()
        && consent.policy_revision > 0
        && !consent.policy_digest.is_empty()
        && consent.currentness == InteractiveDesktopAuthorityCurrentness::Current
        && consent.issued_at_ms <= lease.issued_at_ms
        && consent.issued_at_ms <= now_ms
        && lease.expires_at_ms <= consent.expires_at_ms
        && now_ms < consent.expires_at_ms
        && scope.session_id == session.session_id
        && scope.session_reservation_digest
            == session.session_reservation.session_reservation_digest
        && scope.binding_digest == session.binding.binding_digest
        && scope.host_lease_id == lease.host_lease_id
        && scope.fencing_generation == lease.fencing_generation
        && scope.fencing_generation == session.authority_head.fencing_generation
        && scope.selected_surface_digest == session.authority_head.selected_surface_digest
        && scope.selected_surface_digest == lease.selected_surface.selection_digest
        && scope.selected_surface_digest == control.selected_surface_digest
        && scope.permissions.is_v1_safe()
        && scope.permissions.capture_selected_surface
        && scope.permissions.view_video
        && grant.permissions.is_subset_of(&scope.permissions)
        && control.permissions.is_subset_of(&scope.permissions)
        && scope.permissions.allows(action)
}

fn has_transport_authority(
    media: &InteractiveDesktopMediaEpoch,
    lease: &InteractiveDesktopHostLease,
    grant: &InteractiveDesktopViewerGrant,
    expected_region_or_data_zone: &str,
    now_ms: i64,
) -> bool {
    match media.transport_path {
        InteractiveDesktopTransportPath::Direct => media.relay_authority.is_none(),
        InteractiveDesktopTransportPath::Turn => {
            media.relay_authority.as_ref().is_some_and(|authority| {
                let scope = &authority.scope;
                authority.schema == INTERACTIVE_DESKTOP_RELAY_AUTHORITY_SCHEMA
                    && authority.service_class == INTERACTIVE_DESKTOP_SERVICE_CLASS
                    && !authority.relay_authority_id.is_empty()
                    && !authority.relay_authority_digest.is_empty()
                    && !authority.relay_allocation_ref_digest.is_empty()
                    && !authority.relay_grant_digest.is_empty()
                    && authority.relay_region == expected_region_or_data_zone
                    && authority.currentness == InteractiveDesktopAuthorityCurrentness::Current
                    && authority.issued_at_ms == media.issued_at_ms
                    && authority.expires_at_ms == media.expires_at_ms
                    && authority.issued_at_ms <= now_ms
                    && now_ms < authority.expires_at_ms
                    && scope.session_id == media.session_id
                    && scope.session_reservation_digest == media.session_reservation_digest
                    && scope.binding_digest == media.binding_digest
                    && scope.host_lease_id == media.host_lease_id
                    && scope.host_lease_id == lease.host_lease_id
                    && scope.fencing_generation == media.fencing_generation
                    && scope.fencing_generation == lease.fencing_generation
                    && scope.viewer_grant_id == media.viewer_grant_id
                    && scope.viewer_grant_id == grant.viewer_grant_id
                    && scope.viewer_grant_generation == media.viewer_grant_generation
                    && scope.viewer_grant_generation == grant.grant_generation
                    && scope.viewer_transport_identity_digest
                        == media.viewer_transport_identity_digest
                    && scope.viewer_transport_identity_digest
                        == grant.viewer_transport_identity_digest
                    && scope.media_epoch_id == media.media_epoch_id
                    && scope.media_epoch_sequence == media.epoch_sequence
            })
        }
    }
}
