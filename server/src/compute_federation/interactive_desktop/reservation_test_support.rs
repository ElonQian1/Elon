use super::{
    offer::{
        InteractiveDesktopAudioProfile, InteractiveDesktopCaptureCapability,
        InteractiveDesktopConnectivityPolicy, InteractiveDesktopInputProfile,
        InteractiveDesktopMarketAccess, InteractiveDesktopOfferProfile,
        InteractiveDesktopProductMode, InteractiveDesktopResourceBoundary,
        InteractiveDesktopSurfaceKind, InteractiveDesktopTitlePolicyBinding,
        InteractiveDesktopTransportPath,
        InteractiveDesktopVideoProfile, INTERACTIVE_DESKTOP_OFFER_PROFILE_SCHEMA,
    },
    product_authority::{
        InteractiveDesktopProductAuthorityBinding,
        InteractiveDesktopProductAuthorityCurrentness,
        InteractiveDesktopProductAuthorityProof, INTERACTIVE_DESKTOP_PRODUCT_AUTHORITY_SCHEMA,
    },
    reservation::{
        InteractiveDesktopReservedMeterBudget, InteractiveDesktopSessionReservation,
        INTERACTIVE_DESKTOP_SESSION_RESERVATION_SCHEMA,
    },
    session::{
        InteractiveDesktopSessionRequest, InteractiveDesktopViewerRelationship,
        INTERACTIVE_DESKTOP_SESSION_REQUEST_SCHEMA,
    },
    test_support::{allowed_permissions, binding, capacity_pool, offer_binding},
};

pub(super) fn active_profile() -> InteractiveDesktopOfferProfile {
    InteractiveDesktopOfferProfile {
        schema: INTERACTIVE_DESKTOP_OFFER_PROFILE_SCHEMA.to_string(),
        service_class: super::INTERACTIVE_DESKTOP_SERVICE_CLASS.to_string(),
        profile_id: "profile-1".to_string(),
        profile_version: 1,
        profile_digest: "profile-digest".to_string(),
        offer: offer_binding(),
        capture: InteractiveDesktopCaptureCapability {
            allowed_surface_kinds: vec![
                InteractiveDesktopSurfaceKind::Monitor,
                InteractiveDesktopSurfaceKind::Window,
            ],
            max_selected_surfaces: 1,
            protected_content_supported: false,
            secure_desktop_supported: false,
        },
        video: InteractiveDesktopVideoProfile {
            codec: "h264".to_string(),
            codec_profile: "high".to_string(),
            max_width_px: 1_920,
            max_height_px: 1_080,
            max_frame_rate_milli_hz: 60_000,
            max_bitrate_bits_per_second: 20_000_000,
            sdr_only: true,
        },
        audio: InteractiveDesktopAudioProfile {
            system_audio_available: true,
            codec: "opus".to_string(),
            max_channels: 2,
            max_sample_rate_hz: 48_000,
            microphone_uplink_available: false,
        },
        input: InteractiveDesktopInputProfile {
            keyboard_available: true,
            pointer_available: true,
            gamepad_available: false,
            clipboard_available: false,
            file_transfer_available: false,
            privilege_elevation_available: false,
        },
        transport_paths: vec![
            InteractiveDesktopTransportPath::Direct,
            InteractiveDesktopTransportPath::Turn,
        ],
        resource_boundary: InteractiveDesktopResourceBoundary {
            capacity_pool: capacity_pool(),
            resource_scope_digest: "resource-scope-digest".to_string(),
            gpu_meter: "encode_gpu_ms".to_string(),
            encoder_slot_meter: "encoder_slot".to_string(),
            network_egress_meter: "egress_bytes".to_string(),
            interactive_login_slot_meter: "interactive_session_slot".to_string(),
        },
        region_or_data_zone: "CN".to_string(),
        minimum_session_duration_ms: 100,
        maximum_session_duration_ms: 1_000,
        valid_from_ms: 500,
        valid_until_ms: 3_000,
        created_at_ms: 400,
    }
}

pub(super) fn active_request() -> InteractiveDesktopSessionRequest {
    InteractiveDesktopSessionRequest {
        schema: INTERACTIVE_DESKTOP_SESSION_REQUEST_SCHEMA.to_string(),
        service_class: super::INTERACTIVE_DESKTOP_SERVICE_CLASS.to_string(),
        request_id: "request-1".to_string(),
        request_digest: "request-digest".to_string(),
        session_id: "session-1".to_string(),
        idempotency_key: "request-idempotency".to_string(),
        consumer_account_id: "consumer-1".to_string(),
        requested_product_mode: InteractiveDesktopProductMode::SameOwnerRemoteAccess,
        viewer_relationship: InteractiveDesktopViewerRelationship::SameOwner,
        requested_surface_kind: InteractiveDesktopSurfaceKind::Window,
        requested_permissions: allowed_permissions(),
        requested_width_px: 1_920,
        requested_height_px: 1_080,
        requested_frame_rate_milli_hz: 60_000,
        requested_duration_ms: 1_000,
        requested_currency: "CNY".to_string(),
        consumer_max_amount_micros: 0,
        acceptable_transport_paths: vec![
            InteractiveDesktopTransportPath::Direct,
            InteractiveDesktopTransportPath::Turn,
        ],
        requested_region_or_data_zone: "CN".to_string(),
        requested_at_ms: 900,
        connect_deadline_ms: 1_200,
    }
}

pub(super) fn same_owner_product_authority() -> InteractiveDesktopProductAuthorityBinding {
    InteractiveDesktopProductAuthorityBinding {
        schema: INTERACTIVE_DESKTOP_PRODUCT_AUTHORITY_SCHEMA.to_string(),
        service_class: super::INTERACTIVE_DESKTOP_SERVICE_CLASS.to_string(),
        authority_id: "product-authority-1".to_string(),
        authority_revision: 1,
        authority_digest: "product-authority-digest".to_string(),
        issuer_id: "account-authority".to_string(),
        issuer_policy_digest: "account-authority-policy-digest".to_string(),
        session_id: "session-1".to_string(),
        provider_id: "provider-1".to_string(),
        provider_owner_account_id: "consumer-1".to_string(),
        consumer_account_id: "consumer-1".to_string(),
        product_mode: InteractiveDesktopProductMode::SameOwnerRemoteAccess,
        viewer_relationship: InteractiveDesktopViewerRelationship::SameOwner,
        currentness: InteractiveDesktopProductAuthorityCurrentness::Current,
        proof: InteractiveDesktopProductAuthorityProof::SameOwnerAccount {
            ownership_snapshot_id: "ownership-snapshot-1".to_string(),
            ownership_snapshot_digest: "ownership-snapshot-digest".to_string(),
            account_id: "consumer-1".to_string(),
        },
        issued_at_ms: 800,
        expires_at_ms: 2_500,
    }
}

pub(super) fn active_reservation() -> InteractiveDesktopSessionReservation {
    InteractiveDesktopSessionReservation {
        schema: INTERACTIVE_DESKTOP_SESSION_RESERVATION_SCHEMA.to_string(),
        service_class: super::INTERACTIVE_DESKTOP_SERVICE_CLASS.to_string(),
        session_reservation: super::reservation::InteractiveDesktopSessionReservationBinding {
            session_reservation_id: "session-reservation-1".to_string(),
            session_reservation_revision: 1,
            session_reservation_digest: "session-reservation-digest".to_string(),
        },
        request_id: "request-1".to_string(),
        request_digest: "request-digest".to_string(),
        session_id: "session-1".to_string(),
        binding: binding(),
        product_authority: same_owner_product_authority(),
        resource_scope_digest: "resource-scope-digest".to_string(),
        reserved_surface_kind: InteractiveDesktopSurfaceKind::Window,
        reserved_permissions: allowed_permissions(),
        reserved_width_px: 1_920,
        reserved_height_px: 1_080,
        reserved_frame_rate_milli_hz: 60_000,
        reserved_duration_ms: 1_000,
        permitted_transport_paths: vec![
            InteractiveDesktopTransportPath::Direct,
            InteractiveDesktopTransportPath::Turn,
        ],
        video_codec: "h264".to_string(),
        audio_codec: Some("opus".to_string()),
        currency: "CNY".to_string(),
        consumer_max_amount_micros: 0,
        meter_budgets: vec![
            InteractiveDesktopReservedMeterBudget {
                meter: "encode_gpu_ms".to_string(),
                maximum_quantity: 1_000,
            },
            InteractiveDesktopReservedMeterBudget {
                meter: "encoder_slot".to_string(),
                maximum_quantity: 1,
            },
            InteractiveDesktopReservedMeterBudget {
                meter: "egress_bytes".to_string(),
                maximum_quantity: 10_000_000,
            },
            InteractiveDesktopReservedMeterBudget {
                meter: "interactive_session_slot".to_string(),
                maximum_quantity: 1,
            },
        ],
        issued_at_ms: 1_000,
        activation_deadline_ms: 1_200,
        authorization_expires_at_ms: 2_500,
        maximum_end_at_ms: 2_000,
    }
}

pub(super) fn paid_market_contracts() -> (
    InteractiveDesktopSessionRequest,
    InteractiveDesktopOfferProfile,
    InteractiveDesktopSessionReservation,
) {
    let title_policy = InteractiveDesktopTitlePolicyBinding {
        title_catalog_id: "catalog-title-1".to_string(),
        title_policy_snapshot_id: "title-policy-1".to_string(),
        title_policy_version: 1,
        title_policy_snapshot_digest: "title-policy-digest".to_string(),
        rights_evidence_digest: "rights-evidence-digest".to_string(),
        territory: "CN".to_string(),
        valid_until_ms: 3_000,
    };

    let mut request = active_request();
    request.requested_product_mode = InteractiveDesktopProductMode::LicensedCloudSeat;
    request.viewer_relationship = InteractiveDesktopViewerRelationship::MarketplaceStranger;
    request.consumer_max_amount_micros = 1_000_000;
    request.acceptable_transport_paths = vec![InteractiveDesktopTransportPath::Turn];

    let mut profile = active_profile();
    profile.offer.product_mode = InteractiveDesktopProductMode::LicensedCloudSeat;
    profile.offer.market_access = InteractiveDesktopMarketAccess::PaidMarketplace;
    profile.offer.connectivity_policy = InteractiveDesktopConnectivityPolicy::RelayOnly;
    profile.offer.title_policy = Some(title_policy.clone());
    profile.transport_paths = vec![InteractiveDesktopTransportPath::Turn];

    let mut reservation = active_reservation();
    reservation.binding.offer = profile.offer.clone();
    reservation.binding.provider_owner_account_id = "owner-1".to_string();
    reservation.product_authority.provider_owner_account_id = "owner-1".to_string();
    reservation.product_authority.product_mode = InteractiveDesktopProductMode::LicensedCloudSeat;
    reservation.product_authority.viewer_relationship =
        InteractiveDesktopViewerRelationship::MarketplaceStranger;
    reservation.product_authority.proof =
        InteractiveDesktopProductAuthorityProof::MarketplaceEntitlement {
            entitlement_id: "entitlement-1".to_string(),
            entitlement_revision: 1,
            entitlement_digest: "entitlement-digest".to_string(),
            consumer_account_id: "consumer-1".to_string(),
            title_policy_snapshot_digest: title_policy.title_policy_snapshot_digest,
        };
    reservation.consumer_max_amount_micros = 800_000;
    reservation.permitted_transport_paths = vec![InteractiveDesktopTransportPath::Turn];

    (request, profile, reservation)
}
