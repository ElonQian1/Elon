use super::{
    offer::{
        InteractiveDesktopConnectivityPolicy, InteractiveDesktopProductMode,
        InteractiveDesktopTransportPath,
    },
    product_authority::InteractiveDesktopProductAuthorityProof,
    reservation_test_support::{active_profile, active_request, active_reservation},
    session::InteractiveDesktopViewerRelationship,
};

#[test]
fn request_is_demand_only_and_reservation_freezes_selected_authority() {
    let request = active_request();
    let profile = active_profile();
    let reservation = active_reservation();

    assert!(request.has_safe_request_shape());
    assert!(reservation.cross_validates_request(&request, &profile, 1_500));

    let json = serde_json::to_value(&request).unwrap();
    for broker_selected_field in [
        "binding",
        "offer",
        "price_snapshot",
        "reservation",
        "session_reservation",
        "capacity_pool",
        "capacity_claim",
        "provider_id",
        "provider_owner_account_id",
        "profile",
        "profile_digest",
        "product_authority",
        "resource_scope_digest",
        "meter_budgets",
    ] {
        assert!(json.get(broker_selected_field).is_none());
    }
}

#[test]
fn profile_resource_and_capacity_splices_fail_closed() {
    let request = active_request();
    let profile = active_profile();
    let reservation = active_reservation();
    assert!(reservation.cross_validates_request(&request, &profile, 1_500));

    let mut wrong_pool = reservation.clone();
    wrong_pool.binding.capacity_pool.pool_digest = "other-pool-digest".to_string();
    assert!(!wrong_pool.cross_validates_request(&request, &profile, 1_500));

    let mut wrong_scope = reservation.clone();
    wrong_scope.resource_scope_digest = "other-resource-scope".to_string();
    assert!(!wrong_scope.cross_validates_request(&request, &profile, 1_500));

    let mut wrong_profile = profile.clone();
    wrong_profile.profile_digest = "other-profile-digest".to_string();
    wrong_profile.offer.profile_digest = "other-profile-digest".to_string();
    assert!(wrong_profile.has_v1_shape());
    assert!(!reservation.cross_validates_request(&request, &wrong_profile, 1_500));

    let mut duplicate_meter = profile;
    duplicate_meter.resource_boundary.encoder_slot_meter =
        duplicate_meter.resource_boundary.gpu_meter.clone();
    assert!(!duplicate_meter.has_v1_shape());

    let mut duplicate_surface = active_profile();
    duplicate_surface.capture.allowed_surface_kinds = vec![
        duplicate_surface.capture.allowed_surface_kinds[0],
        duplicate_surface.capture.allowed_surface_kinds[0],
    ];
    assert!(!duplicate_surface.has_v1_shape());
}

#[test]
fn request_and_permissions_cannot_expand_the_offer_profile() {
    let request = active_request();
    let profile = active_profile();
    let reservation = active_reservation();

    let mut lower_resolution_request = request.clone();
    lower_resolution_request.requested_width_px = reservation.reserved_width_px - 1;
    assert!(lower_resolution_request.has_safe_request_shape());
    assert!(!reservation.cross_validates_request(&lower_resolution_request, &profile, 1_500,));

    let mut view_only_request = request.clone();
    view_only_request.requested_permissions.send_keyboard_input = false;
    assert!(view_only_request.has_safe_request_shape());
    assert!(!reservation.cross_validates_request(&view_only_request, &profile, 1_500,));

    let mut shorter_request = request.clone();
    shorter_request.requested_duration_ms = reservation.reserved_duration_ms - 1;
    assert!(shorter_request.has_safe_request_shape());
    assert!(!reservation.cross_validates_request(&shorter_request, &profile, 1_500,));

    let mut turn_only_request = request.clone();
    turn_only_request.acceptable_transport_paths = vec![InteractiveDesktopTransportPath::Turn];
    assert!(turn_only_request.has_safe_request_shape());
    assert!(!reservation.cross_validates_request(&turn_only_request, &profile, 1_500,));

    let mut unsupported_audio_profile = profile.clone();
    unsupported_audio_profile.audio.system_audio_available = false;
    assert!(!reservation.cross_validates_request(&request, &unsupported_audio_profile, 1_500,));

    let mut relay_only_profile = profile.clone();
    relay_only_profile.offer.connectivity_policy = InteractiveDesktopConnectivityPolicy::RelayOnly;
    relay_only_profile.transport_paths = vec![InteractiveDesktopTransportPath::Turn];
    let mut direct_reservation = reservation.clone();
    direct_reservation.binding.offer.connectivity_policy =
        InteractiveDesktopConnectivityPolicy::RelayOnly;
    direct_reservation.permitted_transport_paths = vec![InteractiveDesktopTransportPath::Direct];
    assert!(!direct_reservation.cross_validates_request(&request, &relay_only_profile, 1_500,));
}

#[test]
fn profile_must_preexist_reservation_and_declare_usable_codecs() {
    let request = active_request();
    let profile = active_profile();
    let reservation = active_reservation();
    assert!(profile.created_at_ms < profile.valid_from_ms);
    assert!(reservation.cross_validates_request(&request, &profile, 1_500));

    let mut created_after_validity = profile.clone();
    created_after_validity.created_at_ms = created_after_validity.valid_from_ms + 1;
    assert!(!created_after_validity.has_v1_shape());

    let mut not_yet_active_when_reserved = profile.clone();
    not_yet_active_when_reserved.valid_from_ms = 1_100;
    assert!(not_yet_active_when_reserved.has_v1_shape());
    assert!(!reservation.cross_validates_request(&request, &not_yet_active_when_reserved, 1_500,));

    let mut missing_video_profile = profile.clone();
    missing_video_profile.video.codec_profile.clear();
    assert!(!missing_video_profile.has_v1_shape());

    let mut unusable_audio = profile;
    unusable_audio.audio.codec.clear();
    unusable_audio.audio.max_channels = 0;
    unusable_audio.audio.max_sample_rate_hz = 0;
    assert!(!unusable_audio.has_v1_shape());
}

#[test]
fn relationship_labels_require_the_matching_current_proof() {
    let request = active_request();
    let profile = active_profile();
    let reservation = active_reservation();

    let mut forged_owner = reservation.clone();
    if let InteractiveDesktopProductAuthorityProof::SameOwnerAccount { account_id, .. } =
        &mut forged_owner.product_authority.proof
    {
        *account_id = "other-owner".to_string();
    }
    assert!(!forged_owner.cross_validates_request(&request, &profile, 1_500));

    let mut friend_request = request.clone();
    friend_request.requested_product_mode = InteractiveDesktopProductMode::FriendCoPlay;
    friend_request.viewer_relationship = InteractiveDesktopViewerRelationship::TrustedFriend;
    let mut friend_profile = profile.clone();
    friend_profile.offer.product_mode = InteractiveDesktopProductMode::FriendCoPlay;
    let mut friend_reservation = reservation;
    friend_reservation.binding.offer.product_mode = InteractiveDesktopProductMode::FriendCoPlay;
    friend_reservation.binding.provider_owner_account_id = "owner-1".to_string();
    friend_reservation.product_authority.product_mode = InteractiveDesktopProductMode::FriendCoPlay;
    friend_reservation.product_authority.viewer_relationship =
        InteractiveDesktopViewerRelationship::TrustedFriend;
    friend_reservation
        .product_authority
        .provider_owner_account_id = "owner-1".to_string();
    friend_reservation.product_authority.proof =
        InteractiveDesktopProductAuthorityProof::HostInvitation {
            invitation_id: "invitation-1".to_string(),
            invitation_revision: 1,
            invitation_digest: "invitation-digest".to_string(),
            inviter_account_id: "owner-1".to_string(),
            invitee_account_id: "consumer-1".to_string(),
        };
    assert!(friend_reservation.cross_validates_request(&friend_request, &friend_profile, 1_500,));

    let mut self_invitation = friend_reservation.clone();
    self_invitation.binding.provider_owner_account_id = "consumer-1".to_string();
    self_invitation.product_authority.provider_owner_account_id = "consumer-1".to_string();
    if let InteractiveDesktopProductAuthorityProof::HostInvitation {
        inviter_account_id, ..
    } = &mut self_invitation.product_authority.proof
    {
        *inviter_account_id = "consumer-1".to_string();
    }
    assert!(!self_invitation.cross_validates_request(&friend_request, &friend_profile, 1_500,));

    friend_reservation.product_authority.proof =
        InteractiveDesktopProductAuthorityProof::SameOwnerAccount {
            ownership_snapshot_id: "ownership-snapshot-1".to_string(),
            ownership_snapshot_digest: "ownership-snapshot-digest".to_string(),
            account_id: "consumer-1".to_string(),
        };
    assert!(!friend_reservation.cross_validates_request(&friend_request, &friend_profile, 1_500,));
}
