use super::{
    authority::{
        InteractiveDesktopAuthorityCurrentness, InteractiveDesktopRelayAuthorityBinding,
        InteractiveDesktopRelayAuthorityScope, INTERACTIVE_DESKTOP_RELAY_AUTHORITY_SCHEMA,
    },
    offer::{
        InteractiveDesktopConnectivityPolicy, InteractiveDesktopMarketAccess,
        InteractiveDesktopProductMode, InteractiveDesktopTitlePolicyBinding,
        InteractiveDesktopTransportPath,
    },
    session::{
        InteractiveDesktopAction, InteractiveDesktopPermissionSet,
        InteractiveDesktopViewerRelationship,
    },
    test_support::{
        active_control_epoch, active_grant, active_lease, active_media_epoch, active_session,
        authorized,
    },
};

#[test]
fn host_consent_is_the_upper_bound_for_remote_permissions() {
    let session = active_session();
    let mut lease = active_lease();
    let mut grant = active_grant();
    let media = active_media_epoch();
    let mut control = active_control_epoch();

    lease.host_consent.scope.permissions.send_keyboard_input = false;
    assert!(!session.structurally_authorizes(
        &lease,
        &grant,
        &media,
        &control,
        "consumer-1",
        "account-session-digest",
        4,
        "viewer-device-1",
        "viewer-transport-digest",
        InteractiveDesktopAction::SendKeyboardInput,
        1_500,
    ));

    grant.permissions.send_keyboard_input = false;
    control.permissions.send_keyboard_input = false;
    assert!(session.structurally_authorizes(
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
    assert!(!session.structurally_authorizes(
        &lease,
        &grant,
        &media,
        &control,
        "consumer-1",
        "account-session-digest",
        4,
        "viewer-device-1",
        "viewer-transport-digest",
        InteractiveDesktopAction::SendKeyboardInput,
        1_500,
    ));

    lease.host_consent.currentness = InteractiveDesktopAuthorityCurrentness::Revoked;
    assert!(!authorized(
        &session, &lease, &grant, &media, &control, 1_500
    ));
}

#[test]
fn permissions_default_deny_and_unknown_fields_fail_closed() {
    let permissions: InteractiveDesktopPermissionSet = serde_json::from_str("{}").unwrap();
    assert_eq!(permissions, InteractiveDesktopPermissionSet::default());
    assert!(!permissions.allows(InteractiveDesktopAction::ViewVideo));
    assert!(!permissions.allows(InteractiveDesktopAction::SendKeyboardInput));
    assert!(serde_json::from_str::<InteractiveDesktopPermissionSet>(
        r#"{"unexpected_permission":true}"#
    )
    .is_err());

    let unsafe_permissions = InteractiveDesktopPermissionSet {
        capture_selected_surface: true,
        view_video: true,
        clipboard_sync: true,
        ..InteractiveDesktopPermissionSet::default()
    };
    assert!(!unsafe_permissions.is_v1_safe());
    assert!(!unsafe_permissions.allows(InteractiveDesktopAction::ClipboardSync));

    let blind_input = InteractiveDesktopPermissionSet {
        view_video: true,
        send_keyboard_input: true,
        ..InteractiveDesktopPermissionSet::default()
    };
    assert!(!blind_input.allows(InteractiveDesktopAction::SendKeyboardInput));
}

#[test]
fn current_epoch_fencing_and_expiry_are_all_required_for_authority() {
    let session = active_session();
    let lease = active_lease();
    let grant = active_grant();
    let media = active_media_epoch();
    let control = active_control_epoch();

    assert!(session.structurally_authorizes(
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
    assert!(session.structurally_authorizes(
        &lease,
        &grant,
        &media,
        &control,
        "consumer-1",
        "account-session-digest",
        4,
        "viewer-device-1",
        "viewer-transport-digest",
        InteractiveDesktopAction::SendKeyboardInput,
        1_500,
    ));

    let mut stale_media = media.clone();
    stale_media.epoch_sequence -= 1;
    assert!(!authorized(
        &session,
        &lease,
        &grant,
        &stale_media,
        &control,
        1_500
    ));

    let mut stale_control = control.clone();
    stale_control.epoch_sequence -= 1;
    assert!(!authorized(
        &session,
        &lease,
        &grant,
        &media,
        &stale_control,
        1_500
    ));

    let mut stale_grant_media = media.clone();
    stale_grant_media.viewer_grant_generation -= 1;
    assert!(!authorized(
        &session,
        &lease,
        &grant,
        &stale_grant_media,
        &control,
        1_500
    ));

    let mut wrong_surface = media.clone();
    wrong_surface.selected_surface_digest = "other-surface".to_string();
    assert!(!authorized(
        &session,
        &lease,
        &grant,
        &wrong_surface,
        &control,
        1_500
    ));

    let mut stale_auth_grant = grant.clone();
    stale_auth_grant.account_auth_epoch -= 1;
    assert!(!authorized(
        &session,
        &lease,
        &stale_auth_grant,
        &media,
        &control,
        1_500
    ));

    let mut inconsistent_grant = grant.clone();
    inconsistent_grant.revoked_at_ms = Some(1_400);
    assert!(!authorized(
        &session,
        &lease,
        &inconsistent_grant,
        &media,
        &control,
        1_500
    ));

    let mut terminal_session = session.clone();
    terminal_session.terminal_reason_code = Some("owner_revoked".to_string());
    assert!(!authorized(
        &terminal_session,
        &lease,
        &grant,
        &media,
        &control,
        1_500
    ));

    let mut zero_generation_session = session.clone();
    zero_generation_session.fencing_generation = 0;
    let mut zero_generation_lease = lease.clone();
    zero_generation_lease.fencing_generation = 0;
    let mut zero_generation_media = media.clone();
    zero_generation_media.fencing_generation = 0;
    let mut zero_generation_control = control.clone();
    zero_generation_control.fencing_generation = 0;
    assert!(!authorized(
        &zero_generation_session,
        &zero_generation_lease,
        &grant,
        &zero_generation_media,
        &zero_generation_control,
        1_500,
    ));

    let mut stale_lease = lease.clone();
    stale_lease.fencing_generation -= 1;
    assert!(!authorized(
        &session,
        &stale_lease,
        &grant,
        &media,
        &control,
        1_500
    ));

    let mut expired_grant = grant.clone();
    expired_grant.expires_at_ms = 1_500;
    assert!(!authorized(
        &session,
        &lease,
        &expired_grant,
        &media,
        &control,
        1_500
    ));
    assert!(!authorized(
        &session, &lease, &grant, &media, &control, 2_000
    ));
}

#[test]
fn paid_stranger_sessions_are_licensed_and_relay_only() {
    let mut session = active_session();
    session.binding.offer.product_mode = InteractiveDesktopProductMode::LicensedCloudSeat;
    session.binding.offer.market_access = InteractiveDesktopMarketAccess::PaidMarketplace;
    session.binding.offer.connectivity_policy = InteractiveDesktopConnectivityPolicy::RelayOnly;
    session.viewer_relationship = InteractiveDesktopViewerRelationship::MarketplaceStranger;
    assert!(!session.has_safe_product_boundary(InteractiveDesktopTransportPath::Turn));
    session.binding.offer.title_policy = Some(InteractiveDesktopTitlePolicyBinding {
        title_catalog_id: "catalog-title-1".to_string(),
        title_policy_snapshot_id: "title-policy-1".to_string(),
        title_policy_version: 1,
        title_policy_snapshot_digest: "title-policy-digest".to_string(),
        rights_evidence_digest: "rights-evidence-digest".to_string(),
        territory: "CN".to_string(),
        valid_until_ms: 3_000,
    });
    assert!(session.has_safe_product_boundary(InteractiveDesktopTransportPath::Turn));
    assert!(!session.has_safe_product_boundary(InteractiveDesktopTransportPath::Direct));
    assert!(!session.binding.offer.has_current_market_authority(3_000));

    let lease = active_lease();
    let grant = active_grant();
    let mut media = active_media_epoch();
    let control = active_control_epoch();
    media.transport_path = InteractiveDesktopTransportPath::Turn;
    media.relay_authority = Some(InteractiveDesktopRelayAuthorityBinding {
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
            binding_digest: "binding-digest".to_string(),
            host_lease_id: "lease-1".to_string(),
            fencing_generation: 9,
            viewer_grant_id: "grant-1".to_string(),
            viewer_grant_generation: 2,
            viewer_transport_identity_digest: "viewer-transport-digest".to_string(),
            media_epoch_id: "media-2".to_string(),
            media_epoch_sequence: 2,
        },
        issued_at_ms: 1_000,
        expires_at_ms: 2_000,
    });
    assert!(session.structurally_authorizes(
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
    media.relay_authority.as_mut().unwrap().scope.session_id = "other-session".to_string();
    assert!(!authorized(
        &session, &lease, &grant, &media, &control, 1_500
    ));
    media.relay_authority.as_mut().unwrap().scope.session_id = "session-1".to_string();
    media.relay_authority = None;
    assert!(!authorized(
        &session, &lease, &grant, &media, &control, 1_500
    ));

    session.binding.offer.market_access = InteractiveDesktopMarketAccess::PrivateUnpaid;
    assert!(!session.has_safe_product_boundary(InteractiveDesktopTransportPath::Turn));
    session.binding.offer.product_mode = InteractiveDesktopProductMode::FriendCoPlay;
    assert!(!session.has_safe_product_boundary(InteractiveDesktopTransportPath::Turn));
}
