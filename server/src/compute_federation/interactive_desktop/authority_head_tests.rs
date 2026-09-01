use super::test_support::{
    active_control_epoch, active_grant, active_lease, active_media_epoch, active_session,
};

#[test]
fn exact_authority_head_rejects_same_id_with_different_digest() {
    let session = active_session();
    let lease = active_lease();
    let grant = active_grant();
    let media = active_media_epoch();
    let control = active_control_epoch();

    assert!(session.authority_head.matches(&lease, &grant, &media, &control));

    let mut substituted_lease = lease.clone();
    substituted_lease.host_lease_digest = "other-lease-digest".to_string();
    assert!(!session
        .authority_head
        .matches(&substituted_lease, &grant, &media, &control));

    let mut substituted_grant = grant.clone();
    substituted_grant.viewer_grant_digest = "other-grant-digest".to_string();
    assert!(!session
        .authority_head
        .matches(&lease, &substituted_grant, &media, &control));

    let mut substituted_media = media.clone();
    substituted_media.media_epoch_digest = "other-media-digest".to_string();
    assert!(!session
        .authority_head
        .matches(&lease, &grant, &substituted_media, &control));

    let mut substituted_control = control.clone();
    substituted_control.control_epoch_digest = "other-control-digest".to_string();
    assert!(!session
        .authority_head
        .matches(&lease, &grant, &media, &substituted_control));
}

#[test]
fn control_epoch_cannot_cross_a_media_takeover() {
    let session = active_session();
    let lease = active_lease();
    let grant = active_grant();
    let media = active_media_epoch();
    let mut control = active_control_epoch();

    control.media_epoch_digest = "stale-media-digest".to_string();
    assert!(!session.authority_head.matches(&lease, &grant, &media, &control));

    control = active_control_epoch();
    control.media_epoch_sequence -= 1;
    assert!(!session.authority_head.matches(&lease, &grant, &media, &control));

    control = active_control_epoch();
    control.viewer_transport_identity_digest = "stale-transport".to_string();
    assert!(!session.authority_head.matches(&lease, &grant, &media, &control));
}
