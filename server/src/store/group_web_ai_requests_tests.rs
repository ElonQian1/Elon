use super::*;

#[test]
fn prepared_request_can_resume_without_reusing_old_dispatch_permission() {
    let f = Fixture::new();
    let old = Uuid::new_v4().to_string();
    let new = Uuid::new_v4().to_string();
    let r = f
        .store
        .prepare_group_web_ai(&f.user, &f.group, &f.trigger, &old)
        .unwrap();
    f.store
        .group_web_ai_action(&f.user, &f.group, &r.id, &old, "cancel")
        .unwrap();
    let resumed = f
        .store
        .prepare_group_web_ai(&f.user, &f.group, &f.trigger, &new)
        .unwrap();
    assert_eq!(r.id, resumed.id);
    assert!(f
        .store
        .group_web_ai_action(&f.user, &f.group, &r.id, &old, "dispatch")
        .is_err());
    assert!(
        f.store
            .group_web_ai_action(&f.user, &f.group, &r.id, &new, "dispatch")
            .unwrap()
            .dispatch_permit
    );
}

#[test]
fn web_dispatch_is_single_use_and_blocks_legacy_fallback() {
    let f = Fixture::new();
    let op = Uuid::new_v4().to_string();
    let r = f
        .store
        .prepare_group_web_ai(&f.user, &f.group, &f.trigger, &op)
        .unwrap();
    assert!(f
        .store
        .claim_group_ai_reply(&f.user, &f.group, &f.trigger)
        .unwrap()
        .is_none());
    assert!(f
        .store
        .group_web_ai_action(&f.other, &f.group, &r.id, &op, "dispatch")
        .is_err());
    assert!(f
        .store
        .group_web_ai_action(
            &f.user,
            &f.group,
            &r.id,
            &Uuid::new_v4().to_string(),
            "dispatch"
        )
        .is_err());
    assert!(
        f.store
            .group_web_ai_action(&f.user, &f.group, &r.id, &op, "dispatch")
            .unwrap()
            .dispatch_permit
    );
    assert!(
        !f.store
            .group_web_ai_action(&f.user, &f.group, &r.id, &op, "dispatch")
            .unwrap()
            .dispatch_permit
    );
    assert!(f
        .store
        .group_web_ai_action(&f.user, &f.group, &r.id, &op, "fallback")
        .is_err());
    f.store
        .group_web_ai_action(&f.user, &f.group, &r.id, &op, "uncertain")
        .unwrap();
    assert!(f
        .store
        .group_web_ai_action(&f.user, &f.group, &r.id, &op, "fallback")
        .is_err());
    f.store
        .complete_group_ai_reply(&f.user, &r.id, "web answer")
        .unwrap();
    f.store
        .complete_group_ai_reply(&f.user, &r.id, "web answer")
        .unwrap();
    assert_eq!(f.count(), 1);
}

#[test]
fn explicit_pre_dispatch_fallback_reuses_request() {
    let f = Fixture::new();
    let op = Uuid::new_v4().to_string();
    let r = f
        .store
        .prepare_group_web_ai(&f.user, &f.group, &f.trigger, &op)
        .unwrap();
    f.store
        .group_web_ai_action(&f.user, &f.group, &r.id, &op, "fallback")
        .unwrap();
    assert_eq!(f.claim(), r.id);
    assert!(f
        .store
        .claim_group_ai_reply(&f.user, &f.group, &f.trigger)
        .unwrap()
        .is_none());
    f.store
        .complete_group_ai_reply(&f.user, &r.id, "server answer")
        .unwrap();
    assert_eq!(f.count(), 1);
}

#[test]
fn atomic_send_is_idempotent_and_context_is_frozen() {
    let f = Fixture::new();
    let op = Uuid::new_v4().to_string();
    let (message, request) = f
        .store
        .send_group_web_ai_message(&f.user, &f.group, "@EL atomic", None, &op)
        .unwrap();
    f.store
        .send_friend_group_message(&f.other, &f.group, "later private marker", None)
        .unwrap();
    let (retry, snapshot) = f
        .store
        .send_group_web_ai_message(&f.user, &f.group, "@EL atomic", None, &op)
        .unwrap();
    assert_eq!(message.id, retry.id);
    assert_eq!(retry.revision, 1);
    assert!(retry.edited_at.is_none());
    assert_eq!(request.id, snapshot.id);
    assert_eq!(request.prompt, snapshot.prompt);
    assert!(!snapshot.prompt.contains("later private marker"));
    assert!(snapshot.prompt.contains("@EL atomic"));
    assert!(f
        .store
        .send_group_web_ai_message(&f.user, &f.group, "changed", None, &op)
        .is_err());
    assert!(f
        .store
        .claim_group_ai_reply(&f.user, &f.group, &message.id)
        .unwrap()
        .is_none());
}
