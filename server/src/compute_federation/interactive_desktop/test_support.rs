use crate::compute_federation::capacity::{
    ComputeCapacityClaimBinding, ComputeCapacityPoolBinding,
};

use super::{
    authority::{
        InteractiveDesktopAuthorityCurrentness, InteractiveDesktopHostConsentBinding,
        InteractiveDesktopHostConsentScope, INTERACTIVE_DESKTOP_HOST_CONSENT_SCHEMA,
    },
    metering::{
        InteractiveDesktopCumulativeCounter, InteractiveDesktopMeter, InteractiveDesktopUsageLayer,
        InteractiveDesktopUsageReceipt, InteractiveDesktopUsageSourceKind,
        INTERACTIVE_DESKTOP_USAGE_RECEIPT_SCHEMA,
    },
    offer::{
        InteractiveDesktopConnectivityPolicy, InteractiveDesktopMarketAccess,
        InteractiveDesktopOfferBinding, InteractiveDesktopProductMode,
        InteractiveDesktopSurfaceKind, InteractiveDesktopTransportPath,
    },
    reservation::{
        InteractiveDesktopFederationBinding, InteractiveDesktopPriceSnapshotBinding,
        InteractiveDesktopReservationBinding, InteractiveDesktopSessionReservationBinding,
    },
    reservation_test_support::{active_profile, active_reservation},
    session::{
        InteractiveDesktopAction, InteractiveDesktopAuthorityHead, InteractiveDesktopControlEpoch,
        InteractiveDesktopEpochState, InteractiveDesktopGrantState,
        InteractiveDesktopHostLease, InteractiveDesktopHostLeaseState,
        InteractiveDesktopMediaEpoch, InteractiveDesktopPermissionSet,
        InteractiveDesktopSession, InteractiveDesktopSessionState,
        InteractiveDesktopSurfaceSelection, InteractiveDesktopViewerGrant,
        InteractiveDesktopViewerRelationship,
        INTERACTIVE_DESKTOP_CONTROL_EPOCH_SCHEMA, INTERACTIVE_DESKTOP_HOST_LEASE_SCHEMA,
        INTERACTIVE_DESKTOP_MEDIA_EPOCH_SCHEMA, INTERACTIVE_DESKTOP_SESSION_SCHEMA,
        INTERACTIVE_DESKTOP_VIEWER_GRANT_SCHEMA,
    },
};

pub(super) fn offer_binding() -> InteractiveDesktopOfferBinding {
    InteractiveDesktopOfferBinding {
        provider_id: "provider-1".to_string(),
        offer_id: "offer-1".to_string(),
        offer_version: 1,
        offer_digest: "offer-digest".to_string(),
        profile_id: "profile-1".to_string(),
        profile_version: 1,
        profile_digest: "profile-digest".to_string(),
        product_mode: InteractiveDesktopProductMode::SameOwnerRemoteAccess,
        market_access: InteractiveDesktopMarketAccess::PrivateUnpaid,
        connectivity_policy: InteractiveDesktopConnectivityPolicy::DirectOrRelay,
        title_policy: None,
    }
}

pub(super) fn capacity_pool() -> ComputeCapacityPoolBinding {
    ComputeCapacityPoolBinding {
        pool_id: "pool-1".to_string(),
        capacity_epoch: 1,
        pool_revision: 1,
        pool_digest: "pool-digest".to_string(),
    }
}

pub(super) fn binding() -> InteractiveDesktopFederationBinding {
    InteractiveDesktopFederationBinding {
        binding_digest: "binding-digest".to_string(),
        provider_id: "provider-1".to_string(),
        provider_policy_revision: 1,
        provider_digest: "provider-digest".to_string(),
        provider_owner_account_id: "consumer-1".to_string(),
        consumer_account_id: "consumer-1".to_string(),
        offer: offer_binding(),
        price_snapshot: InteractiveDesktopPriceSnapshotBinding {
            price_snapshot_id: "price-1".to_string(),
            price_snapshot_digest: "price-digest".to_string(),
        },
        reservation: InteractiveDesktopReservationBinding {
            reservation_id: "reservation-1".to_string(),
            reservation_revision: 1,
            reservation_digest: "reservation-digest".to_string(),
        },
        capacity_pool: capacity_pool(),
        capacity_claim: ComputeCapacityClaimBinding {
            claim_id: "claim-1".to_string(),
            claim_revision: 1,
            claim_digest: "claim-digest".to_string(),
        },
    }
}

pub(super) fn allowed_permissions() -> InteractiveDesktopPermissionSet {
    InteractiveDesktopPermissionSet {
        capture_selected_surface: true,
        view_video: true,
        receive_system_audio: true,
        send_keyboard_input: true,
        send_pointer_input: true,
        ..InteractiveDesktopPermissionSet::default()
    }
}

pub(super) fn active_session() -> InteractiveDesktopSession {
    InteractiveDesktopSession {
        schema: INTERACTIVE_DESKTOP_SESSION_SCHEMA.to_string(),
        service_class: super::INTERACTIVE_DESKTOP_SERVICE_CLASS.to_string(),
        session_id: "session-1".to_string(),
        session_root_digest: "session-root-digest".to_string(),
        session_revision: 5,
        session_digest: "session-digest".to_string(),
        request_id: "request-1".to_string(),
        request_digest: "request-digest".to_string(),
        session_reservation: InteractiveDesktopSessionReservationBinding {
            session_reservation_id: "session-reservation-1".to_string(),
            session_reservation_revision: 1,
            session_reservation_digest: "session-reservation-digest".to_string(),
        },
        binding: binding(),
        viewer_relationship: InteractiveDesktopViewerRelationship::SameOwner,
        state: InteractiveDesktopSessionState::Active,
        authority_head: InteractiveDesktopAuthorityHead {
            host_lease_id: "lease-1".to_string(),
            host_lease_digest: "lease-digest".to_string(),
            viewer_grant_id: "grant-1".to_string(),
            viewer_grant_digest: "grant-digest".to_string(),
            viewer_grant_generation: 2,
            media_epoch_id: "media-2".to_string(),
            media_epoch_digest: "media-digest".to_string(),
            media_epoch_sequence: 2,
            control_epoch_id: "control-2".to_string(),
            control_epoch_digest: "control-digest".to_string(),
            control_epoch_sequence: 2,
            selected_surface_digest: "surface-selection-digest".to_string(),
            viewer_transport_identity_digest: "viewer-transport-digest".to_string(),
            fencing_generation: 9,
        },
        created_at_ms: 1_000,
        updated_at_ms: 1_400,
        maximum_end_at_ms: 2_000,
        terminal_reason_code: None,
    }
}

pub(super) fn active_lease() -> InteractiveDesktopHostLease {
    InteractiveDesktopHostLease {
        schema: INTERACTIVE_DESKTOP_HOST_LEASE_SCHEMA.to_string(),
        service_class: super::INTERACTIVE_DESKTOP_SERVICE_CLASS.to_string(),
        host_lease_id: "lease-1".to_string(),
        host_lease_digest: "lease-digest".to_string(),
        session_id: "session-1".to_string(),
        session_reservation_digest: "session-reservation-digest".to_string(),
        binding_digest: "binding-digest".to_string(),
        provider_id: "provider-1".to_string(),
        host_node_id: "host-node-1".to_string(),
        provider_node_binding_digest: "node-binding-digest".to_string(),
        endpoint_credential_digest: "endpoint-credential-digest".to_string(),
        selected_surface: InteractiveDesktopSurfaceSelection {
            surface_kind: InteractiveDesktopSurfaceKind::Window,
            selection_digest: "surface-selection-digest".to_string(),
        },
        state: InteractiveDesktopHostLeaseState::Active,
        fencing_generation: 9,
        host_consent: InteractiveDesktopHostConsentBinding {
            schema: INTERACTIVE_DESKTOP_HOST_CONSENT_SCHEMA.to_string(),
            service_class: super::INTERACTIVE_DESKTOP_SERVICE_CLASS.to_string(),
            consent_id: "consent-1".to_string(),
            consent_digest: "consent-digest".to_string(),
            consent_revision: 1,
            policy_id: "host-policy-1".to_string(),
            policy_revision: 1,
            policy_digest: "host-policy-digest".to_string(),
            currentness: InteractiveDesktopAuthorityCurrentness::Current,
            scope: InteractiveDesktopHostConsentScope {
                session_id: "session-1".to_string(),
                session_reservation_digest: "session-reservation-digest".to_string(),
                binding_digest: "binding-digest".to_string(),
                host_lease_id: "lease-1".to_string(),
                fencing_generation: 9,
                selected_surface_digest: "surface-selection-digest".to_string(),
                permissions: allowed_permissions(),
            },
            issued_at_ms: 1_000,
            expires_at_ms: 2_000,
        },
        issued_at_ms: 1_000,
        activated_at_ms: Some(1_100),
        last_heartbeat_at_ms: Some(1_400),
        expires_at_ms: 2_000,
        hard_deadline_at_ms: 2_000,
        terminal_reason_code: None,
    }
}

pub(super) fn active_grant() -> InteractiveDesktopViewerGrant {
    InteractiveDesktopViewerGrant {
        schema: INTERACTIVE_DESKTOP_VIEWER_GRANT_SCHEMA.to_string(),
        service_class: super::INTERACTIVE_DESKTOP_SERVICE_CLASS.to_string(),
        viewer_grant_id: "grant-1".to_string(),
        viewer_grant_digest: "grant-digest".to_string(),
        grant_generation: 2,
        session_id: "session-1".to_string(),
        session_reservation_digest: "session-reservation-digest".to_string(),
        binding_digest: "binding-digest".to_string(),
        consumer_account_id: "consumer-1".to_string(),
        consumer_account_session_digest: "account-session-digest".to_string(),
        account_auth_epoch: 4,
        viewer_device_key_digest: "viewer-device-1".to_string(),
        viewer_transport_identity_digest: "viewer-transport-digest".to_string(),
        permissions: allowed_permissions(),
        state: InteractiveDesktopGrantState::Active,
        issued_at_ms: 1_100,
        expires_at_ms: 2_000,
        revoked_at_ms: None,
    }
}

pub(super) fn active_media_epoch() -> InteractiveDesktopMediaEpoch {
    InteractiveDesktopMediaEpoch {
        schema: INTERACTIVE_DESKTOP_MEDIA_EPOCH_SCHEMA.to_string(),
        service_class: super::INTERACTIVE_DESKTOP_SERVICE_CLASS.to_string(),
        media_epoch_id: "media-2".to_string(),
        media_epoch_digest: "media-digest".to_string(),
        epoch_sequence: 2,
        session_id: "session-1".to_string(),
        session_reservation_digest: "session-reservation-digest".to_string(),
        binding_digest: "binding-digest".to_string(),
        host_lease_id: "lease-1".to_string(),
        viewer_grant_id: "grant-1".to_string(),
        viewer_grant_generation: 2,
        viewer_transport_identity_digest: "viewer-transport-digest".to_string(),
        selected_surface_digest: "surface-selection-digest".to_string(),
        fencing_generation: 9,
        state: InteractiveDesktopEpochState::Active,
        transport_path: InteractiveDesktopTransportPath::Direct,
        relay_authority: None,
        video_codec: "h264".to_string(),
        audio_codec: Some("opus".to_string()),
        issued_at_ms: 1_100,
        expires_at_ms: 2_000,
        ended_at_ms: None,
    }
}

pub(super) fn active_control_epoch() -> InteractiveDesktopControlEpoch {
    InteractiveDesktopControlEpoch {
        schema: INTERACTIVE_DESKTOP_CONTROL_EPOCH_SCHEMA.to_string(),
        service_class: super::INTERACTIVE_DESKTOP_SERVICE_CLASS.to_string(),
        control_epoch_id: "control-2".to_string(),
        control_epoch_digest: "control-digest".to_string(),
        epoch_sequence: 2,
        session_id: "session-1".to_string(),
        session_reservation_digest: "session-reservation-digest".to_string(),
        binding_digest: "binding-digest".to_string(),
        host_lease_id: "lease-1".to_string(),
        viewer_grant_id: "grant-1".to_string(),
        viewer_grant_generation: 2,
        media_epoch_id: "media-2".to_string(),
        media_epoch_digest: "media-digest".to_string(),
        media_epoch_sequence: 2,
        viewer_transport_identity_digest: "viewer-transport-digest".to_string(),
        selected_surface_digest: "surface-selection-digest".to_string(),
        fencing_generation: 9,
        permissions: allowed_permissions(),
        state: InteractiveDesktopEpochState::Active,
        issued_at_ms: 1_100,
        expires_at_ms: 2_000,
        ended_at_ms: None,
    }
}

pub(super) fn authorized(
    session: &InteractiveDesktopSession,
    lease: &InteractiveDesktopHostLease,
    grant: &InteractiveDesktopViewerGrant,
    media: &InteractiveDesktopMediaEpoch,
    control: &InteractiveDesktopControlEpoch,
    now_ms: i64,
) -> bool {
    session.structurally_authorizes(
        &active_profile(),
        &active_reservation(),
        lease,
        grant,
        media,
        control,
        "consumer-1",
        "account-session-digest",
        4,
        "viewer-device-1",
        "viewer-transport-digest",
        InteractiveDesktopAction::ViewVideo,
        now_ms,
    )
}

fn usage_layer(
    source_kind: InteractiveDesktopUsageSourceKind,
    source_ref_digest: &str,
    closing_quantity: u64,
) -> InteractiveDesktopUsageLayer {
    InteractiveDesktopUsageLayer {
        source_kind,
        source_ref_digest: source_ref_digest.to_string(),
        sample_sequence: 1,
        previous_sample_digest: None,
        counters: vec![InteractiveDesktopCumulativeCounter {
            meter: InteractiveDesktopMeter::MediaActiveMilliseconds,
            opening_quantity: 0,
            closing_quantity,
        }],
        observation_digest: format!("{source_ref_digest}-observation-1"),
        observed_at_ms: 1_500,
    }
}

pub(super) fn pending_usage_receipt() -> InteractiveDesktopUsageReceipt {
    InteractiveDesktopUsageReceipt {
        schema: INTERACTIVE_DESKTOP_USAGE_RECEIPT_SCHEMA.to_string(),
        service_class: super::INTERACTIVE_DESKTOP_SERVICE_CLASS.to_string(),
        usage_receipt_id: "usage-1".to_string(),
        usage_receipt_digest: "usage-digest-1".to_string(),
        usage_sequence: 1,
        previous_usage_receipt_digest: None,
        session_id: "session-1".to_string(),
        session_root_digest: "session-root-digest".to_string(),
        session_revision: 5,
        session_digest: "session-digest".to_string(),
        binding: binding(),
        host_lease_id: "lease-1".to_string(),
        fencing_generation: 9,
        viewer_grant_id: "grant-1".to_string(),
        viewer_grant_generation: 2,
        selected_surface_digest: "surface-selection-digest".to_string(),
        media_epoch_id: "media-2".to_string(),
        media_epoch_sequence: 2,
        control_epoch_id: "control-2".to_string(),
        control_epoch_sequence: 2,
        transport_path: InteractiveDesktopTransportPath::Direct,
        interval_started_at_ms: 1_000,
        interval_ended_at_ms: 1_500,
        declared: usage_layer(
            InteractiveDesktopUsageSourceKind::ProviderDeclared,
            "provider-source",
            500,
        ),
        transport_observed: usage_layer(
            InteractiveDesktopUsageSourceKind::TransportObserved,
            "transport-source",
            490,
        ),
        consumer_observed: usage_layer(
            InteractiveDesktopUsageSourceKind::ConsumerObserved,
            "consumer-source",
            480,
        ),
        created_at_ms: 1_500,
    }
}
