use super::super::work::{self, GroupWorkAiOptions};
use super::*;

fn selection() -> GroupWorkAiOptions {
    GroupWorkAiOptions {
        agent: Some("server-model-a".into()),
        allow_fallback: false,
    }
}

#[test]
fn work_options_are_frozen_before_single_dispatch_and_survive_reopen() {
    let f = Fixture::new();
    let op = Uuid::new_v4().to_string();
    let r = f
        .store
        .prepare_group_web_ai(&f.user, &f.group, &f.trigger, &op)
        .unwrap();
    let options = selection();
    let ready = f
        .store
        .activate_group_work_ai(&f.user, &f.group, &r.id, &op, &options)
        .unwrap();
    assert_eq!(ready.state, "server_ready");
    assert_eq!(f.claim(), r.id);
    assert!(f
        .store
        .claim_group_ai_reply(&f.user, &f.group, &f.trigger)
        .unwrap()
        .is_none());
    let reopened = Store::open(&f.path).unwrap();
    assert_eq!(
        reopened.group_work_ai_options(&f.user, &r.id).unwrap(),
        Some(options.clone())
    );
    assert_eq!(
        reopened
            .activate_group_work_ai(&f.user, &f.group, &r.id, &op, &options)
            .unwrap()
            .state,
        "dispatched"
    );
    let changed = GroupWorkAiOptions {
        agent: Some("other-model".into()),
        ..options
    };
    assert!(reopened
        .activate_group_work_ai(&f.user, &f.group, &r.id, &op, &changed)
        .is_err());
    assert_eq!(
        reopened.group_work_ai_options(&f.user, &r.id).unwrap(),
        Some(selection())
    );
}

#[test]
fn work_switch_cannot_take_other_users_request_or_replay_a_web_write() {
    let f = Fixture::new();
    let op = Uuid::new_v4().to_string();
    let r = f
        .store
        .prepare_group_web_ai(&f.user, &f.group, &f.trigger, &op)
        .unwrap();
    assert!(f
        .store
        .activate_group_work_ai(&f.other, &f.group, &r.id, &op, &selection())
        .is_err());
    assert!(f
        .store
        .activate_group_work_ai(
            &f.user,
            &f.group,
            &r.id,
            &Uuid::new_v4().to_string(),
            &selection()
        )
        .is_err());
    f.store
        .group_web_ai_action(&f.user, &f.group, &r.id, &op, "dispatch")
        .unwrap();
    assert!(f
        .store
        .activate_group_work_ai(&f.user, &f.group, &r.id, &op, &selection())
        .is_err());
}

#[test]
fn failed_activation_rolls_back_options_and_reservation_transition() {
    let f = Fixture::new();
    let op = Uuid::new_v4().to_string();
    let r = f
        .store
        .prepare_group_web_ai(&f.user, &f.group, &f.trigger, &op)
        .unwrap();
    f.store
        .conn()
        .unwrap()
        .execute_batch(
            "CREATE TRIGGER refuse_work BEFORE UPDATE OF engine ON group_ai_reply_requests
        BEGIN SELECT RAISE(ABORT,'injected'); END;",
        )
        .unwrap();
    assert!(f
        .store
        .activate_group_work_ai(&f.user, &f.group, &r.id, &op, &selection())
        .is_err());
    let count: i64 = f
        .store
        .conn()
        .unwrap()
        .query_row("SELECT COUNT(*) FROM group_ai_work_options", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(count, 0);
    assert_eq!(
        f.store
            .group_web_ai_action(&f.user, &f.group, &r.id, &op, "status")
            .unwrap()
            .state,
        "prepared"
    );
}

#[test]
fn work_options_migration_is_idempotent_and_legacy_default_stays_empty() {
    let f = Fixture::new();
    work::migrate(&f.store.conn().unwrap()).unwrap();
    work::migrate(&f.store.conn().unwrap()).unwrap();
    let id = f.claim();
    assert_eq!(f.store.group_work_ai_options(&f.user, &id).unwrap(), None);
    assert!(f.store.group_work_ai_options(&f.other, &id).is_err());
    assert!(GroupWorkAiOptions {
        agent: Some(" ".into()),
        allow_fallback: true
    }
    .validate()
    .is_err());
}
