use super::{
    authority::{
        InteractiveDesktopAuthorityCurrentness, InteractiveDesktopRelayAuthorityBinding,
        InteractiveDesktopRelayAuthorityScope, INTERACTIVE_DESKTOP_RELAY_AUTHORITY_SCHEMA,
    },
    offer::{
        InteractiveDesktopMarketAccess, InteractiveDesktopProductMode,
        InteractiveDesktopTransportPath,
    },
    product_authority::{
        InteractiveDesktopProductAuthorityCurrentness, InteractiveDesktopProductAuthorityProof,
    },
    reservation_test_support::{active_profile, paid_market_contracts},
    session::{InteractiveDesktopAction, InteractiveDesktopViewerRelationship},
    test_support::{
        active_control_epoch, active_grant, active_lease, active_media_epoch, active_session,
    },
};

fn relay_authority() -> InteractiveDesktopRelayAuthorityBinding {
    InteractiveDesktopRelayAuthorityBinding {
        schema: INTERACTIVE_DESKTOP_RELAY_AUTHORITY_SCHEMA.to_string(),
        service_class: super::INTERACTIVE_DESKTOP_SERVICE_CLASS.to_string(),
        relay_authority_id: "relay-authority-1".to_string(),
        relay_authority_digest: "relay-authority-digest".to_string(),
        relay_allocation_ref_digest: "relay-allocation-digest".to_string(),
        relay_grant_digest: "relay-grant-digest".to_string(),
        relay_region: "CN".to_string(),
        currentness: InteractiveDesktopAuthorityCurrentness::Current,
        scope: InteractiveDesktopRelayAuthorityScope {
            session_id: "session-1".to_string(),
            session_reservation_digest: "session-reservation-digest".to_string(),
            binding_digest: "binding-digest".to_string(),
            host_lease_id: "lease-1".to_string(),
            fencing_generation: 9,
            viewer_grant_id: "grant-1".to_string(),
            viewer_grant_generation: 2,
            viewer_transport_identity_digest: "viewer-transport-digest".to_string(),
            media_epoch_id: "media-2".to_string(),
            media_epoch_sequence: 2,
        },
        issued_at_ms: 1_100,
        expires_at_ms: 2_000,
    }
}

#[test]
fn paid_market_requires_exact_mode_budget_and_current_entitlement() {
    let (request, profile, reservation) = paid_market_contracts();
    assert!(request.has_safe_request_shape());
    assert!(profile.has_v1_shape());
    assert!(reservation.cross_validates_request(&request, &profile, 1_500));

    let mut underfunded_request = request.clone();
    underfunded_request.consumer_max_amount_micros =
        reservation.consumer_max_amount_micros - 1;
    assert!(underfunded_request.has_safe_request_shape());
    assert!(!reservation.cross_validates_request(&underfunded_request, &profile, 1_500));

    let mut wrong_currency = request.clone();
    wrong_currency.requested_currency = "USD".to_string();
    assert!(wrong_currency.has_safe_request_shape());
    assert!(!reservation.cross_validates_request(&wrong_currency, &profile, 1_500));

    let mut revoked = reservation.clone();
    revoked.product_authority.currentness =
        InteractiveDesktopProductAuthorityCurrentness::Revoked;
    assert!(!revoked.cross_validates_request(&request, &profile, 1_500));

    let mut expired = reservation.clone();
    expired.product_authority.expires_at_ms = 1_500;
    assert!(!expired.cross_validates_request(&request, &profile, 1_500));

    let mut late_proof = reservation.clone();
    late_proof.product_authority.issued_at_ms = reservation.issued_at_ms + 1;
    assert!(!late_proof.cross_validates_request(&request, &profile, 1_500));

    let mut wrong_title = reservation.clone();
    if let InteractiveDesktopProductAuthorityProof::MarketplaceEntitlement {
        title_policy_snapshot_digest,
        ..
    } = &mut wrong_title.product_authority.proof
    {
        *title_policy_snapshot_digest = "other-title-policy".to_string();
    }
    assert!(!wrong_title.cross_validates_request(&request, &profile, 1_500));

    let mut self_market = reservation.clone();
    self_market.binding.provider_owner_account_id = "consumer-1".to_string();
    self_market.product_authority.provider_owner_account_id = "consumer-1".to_string();
    assert!(!self_market.cross_validates_request(&request, &profile, 1_500));

    let mut licensed_unpaid = profile.clone();
    licensed_unpaid.offer.market_access = InteractiveDesktopMarketAccess::PrivateUnpaid;
    assert!(!licensed_unpaid.has_v1_shape());

    let mut wrong_territory = profile.clone();
    wrong_territory.offer.title_policy.as_mut().unwrap().territory = "US".to_string();
    assert!(!wrong_territory.has_v1_shape());

    let mut private_offer = active_profile();
    private_offer.offer.product_mode = InteractiveDesktopProductMode::LicensedCloudSeat;
    assert!(!private_offer.has_v1_shape());
}

#[test]
fn paid_session_requires_turn_authority_for_the_exact_region_and_scope() {
    let (_request, profile, reservation) = paid_market_contracts();
    let mut session = active_session();
    session.binding = reservation.binding.clone();
    session.session_reservation = reservation.session_reservation.clone();
    session.viewer_relationship = InteractiveDesktopViewerRelationship::MarketplaceStranger;

    assert!(session.has_safe_product_boundary(
        &profile,
        &reservation,
        InteractiveDesktopTransportPath::Turn,
        1_500,
    ));
    assert!(!session.has_safe_product_boundary(
        &profile,
        &reservation,
        InteractiveDesktopTransportPath::Direct,
        1_500,
    ));
    assert!(!session.binding.offer.has_current_market_authority(3_000));

    let lease = active_lease();
    let grant = active_grant();
    let mut media = active_media_epoch();
    let control = active_control_epoch();
    media.transport_path = InteractiveDesktopTransportPath::Turn;
    media.relay_authority = Some(relay_authority());
    assert!(session.structurally_authorizes(
        &profile,
        &reservation,
        &lease,
        &grant,
        &media,
        &control,
        "consumer-1",
        "account-session-digest",
        4,
        "viewer-device-1",
        "viewer-transport-digest",
        InteractiveDesktopAction::ViewVideo,
        1_500,
    ));

    media.relay_authority.as_mut().unwrap().relay_region = "US".to_string();
    assert!(!session.structurally_authorizes(
        &profile,
        &reservation,
        &lease,
        &grant,
        &media,
        &control,
        "consumer-1",
        "account-session-digest",
        4,
        "viewer-device-1",
        "viewer-transport-digest",
        InteractiveDesktopAction::ViewVideo,
        1_500,
    ));

    media.relay_authority = Some(relay_authority());
    media.relay_authority.as_mut().unwrap().scope.session_id = "other-session".to_string();
    assert!(!session.structurally_authorizes(
        &profile,
        &reservation,
        &lease,
        &grant,
        &media,
        &control,
        "consumer-1",
        "account-session-digest",
        4,
        "viewer-device-1",
        "viewer-transport-digest",
        InteractiveDesktopAction::ViewVideo,
        1_500,
    ));

    media.relay_authority = None;
    assert!(!session.structurally_authorizes(
        &profile,
        &reservation,
        &lease,
        &grant,
        &media,
        &control,
        "consumer-1",
        "account-session-digest",
        4,
        "viewer-device-1",
        "viewer-transport-digest",
        InteractiveDesktopAction::ViewVideo,
        1_500,
    ));
}
