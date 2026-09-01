use super::{
    authority::InteractiveDesktopAuthorityCurrentness,
    offer::{
        InteractiveDesktopConnectivityPolicy, InteractiveDesktopTransportPath,
    },
    session::{InteractiveDesktopAction, InteractiveDesktopPermissionSet},
    reservation_test_support::{active_profile, active_request, active_reservation},
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
    let profile = active_profile();
    let reservation = active_reservation();

    lease.host_consent.scope.permissions.send_keyboard_input = false;
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
        InteractiveDesktopAction::SendKeyboardInput,
        1_500,
    ));

    grant.permissions.send_keyboard_input = false;
    control.permissions.send_keyboard_input = false;
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
        InteractiveDesktopAction::SendKeyboardInput,
        1_500,
    ));

    lease.host_consent.currentness = InteractiveDesktopAuthorityCurrentness::Revoked;
    assert!(!authorized(
        &session, &lease, &grant, &media, &control, 1_500
    ));

    lease.host_consent.currentness = InteractiveDesktopAuthorityCurrentness::Current;
    lease.host_consent.issued_at_ms = active_reservation().issued_at_ms - 1;
    assert!(!authorized(
        &session, &lease, &grant, &media, &control, 1_500
    ));

    lease.host_consent.issued_at_ms = active_reservation().issued_at_ms;
    lease.host_consent.scope.session_reservation_digest =
        "other-session-reservation".to_string();
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
    let profile = active_profile();
    let reservation = active_reservation();

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
    zero_generation_session.authority_head.fencing_generation = 0;
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
fn activation_deadline_expires_only_unstarted_sessions() {
    let session = active_session();
    let mut lease = active_lease();
    let mut grant = active_grant();
    let mut media = active_media_epoch();
    let mut control = active_control_epoch();
    let request = active_request();
    let profile = active_profile();
    let reservation = active_reservation();

    assert!(request.connect_deadline_ms < 1_500);
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

    let late_activation = reservation.activation_deadline_ms + 1;
    lease.activated_at_ms = Some(late_activation);
    grant.issued_at_ms = late_activation;
    media.issued_at_ms = late_activation;
    control.issued_at_ms = late_activation;
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

#[test]
fn exact_session_reservation_and_temporal_authority_chain_are_required() {
    let session = active_session();
    let lease = active_lease();
    let grant = active_grant();
    let media = active_media_epoch();
    let control = active_control_epoch();
    let profile = active_profile();
    let reservation = active_reservation();

    let mut replaced_reservation = reservation.clone();
    replaced_reservation
        .session_reservation
        .session_reservation_digest = "replacement-reservation-digest".to_string();
    assert!(replaced_reservation.has_safe_shape(&profile, 1_500));
    assert!(!session.structurally_authorizes(
        &profile,
        &replaced_reservation,
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

    let mut replaced_session = session.clone();
    replaced_session
        .session_reservation
        .session_reservation_digest = "replacement-reservation-digest".to_string();
    assert!(replaced_session.has_safe_product_boundary(
        &profile,
        &replaced_reservation,
        media.transport_path,
        1_500,
    ));
    assert!(!replaced_session.structurally_authorizes(
        &profile,
        &replaced_reservation,
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

    let mut grant_after_media = grant.clone();
    grant_after_media.issued_at_ms = media.issued_at_ms + 1;
    assert!(!authorized(
        &session,
        &lease,
        &grant_after_media,
        &media,
        &control,
        1_500,
    ));

    let mut future_session_revision = session;
    future_session_revision.updated_at_ms = 1_501;
    assert!(!authorized(
        &future_session_revision,
        &lease,
        &grant,
        &media,
        &control,
        1_500,
    ));
}

#[test]
fn reservation_profile_is_the_upper_bound_for_grants_codecs_and_transport() {
    let session = active_session();
    let lease = active_lease();
    let grant = active_grant();
    let media = active_media_epoch();
    let control = active_control_epoch();
    let profile = active_profile();
    let reservation = active_reservation();

    let mut view_only = reservation.clone();
    view_only.reserved_permissions.send_keyboard_input = false;
    view_only.reserved_permissions.send_pointer_input = false;
    assert!(!session.structurally_authorizes(
        &profile,
        &view_only,
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

    let mut substituted_codec = media.clone();
    substituted_codec.video_codec = "other-codec".to_string();
    assert!(!authorized(
        &session,
        &lease,
        &grant,
        &substituted_codec,
        &control,
        1_500,
    ));

    let mut relay_only_profile = profile;
    relay_only_profile.offer.connectivity_policy =
        InteractiveDesktopConnectivityPolicy::RelayOnly;
    relay_only_profile.transport_paths = vec![InteractiveDesktopTransportPath::Turn];
    let mut relay_only_reservation = reservation;
    relay_only_reservation.binding.offer.connectivity_policy =
        InteractiveDesktopConnectivityPolicy::RelayOnly;
    relay_only_reservation.permitted_transport_paths = vec![InteractiveDesktopTransportPath::Turn];
    let mut relay_only_session = session;
    relay_only_session.binding = relay_only_reservation.binding.clone();
    assert!(!relay_only_session.has_safe_product_boundary(
        &relay_only_profile,
        &relay_only_reservation,
        InteractiveDesktopTransportPath::Direct,
        1_500,
    ));
}
