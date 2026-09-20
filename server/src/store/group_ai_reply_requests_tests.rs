use super::*;
use uuid::Uuid;

#[path = "group_web_ai_selection_tests.rs"]
mod selection_cases;
#[path = "group_web_ai_requests_tests.rs"]
mod web_cases;
#[path = "group_work_ai_options_tests.rs"]
mod work_cases;

struct Fixture {
    store: Store,
    path: std::path::PathBuf,
    user: String,
    other: String,
    group: String,
    trigger: String,
}

impl Fixture {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!("group_ai_requests_{}.db", Uuid::new_v4()));
        let store = Store::open(&path).unwrap();
        let user = store
            .create_user("owner@example.com", "secret1", Some("Owner"), None)
            .unwrap()
            .id;
        let other = store
            .create_user("member@example.com", "secret1", Some("Member"), None)
            .unwrap()
            .id;
        store
            .add_friend(&user, Some("email"), "member@example.com")
            .unwrap();
        let group = store
            .create_friend_group(&user, Some("Test"), std::slice::from_ref(&other))
            .unwrap()
            .id;
        let trigger = store
            .send_friend_group_message(&user, &group, "@EL explain", None)
            .unwrap()
            .id;
        Self {
            store,
            path,
            user,
            other,
            group,
            trigger,
        }
    }

    fn claim(&self) -> String {
        self.store
            .claim_group_ai_reply(&self.user, &self.group, &self.trigger)
            .unwrap()
            .unwrap()
    }

    fn count(&self) -> i64 {
        self.store.conn().unwrap().query_row(
            "SELECT COUNT(*) FROM friend_group_messages WHERE group_id = ?1 AND sender_user_id = ?2",
            params![self.group, SOCIAL_AI_USER_ID], |r| r.get(0),
        ).unwrap()
    }
}

#[test]
fn mention_and_selected_share_ownership_across_connections() {
    let f = Fixture::new();
    let id = f.claim();
    let reopened = Store::open(&f.path).unwrap();
    assert!(reopened
        .claim_group_ai_reply(&f.other, &f.group, &f.trigger)
        .unwrap()
        .is_none());
    let result = reopened
        .complete_group_ai_reply(&f.user, &id, "answer")
        .unwrap();
    let retry = f
        .store
        .complete_group_ai_reply(&f.user, &id, "answer")
        .unwrap();
    assert_eq!(result.id, retry.id);
    assert_eq!(f.count(), 1);
    assert!(f
        .store
        .complete_group_ai_reply(&f.user, &id, "different")
        .is_err());
    assert!(f
        .store
        .claim_group_ai_reply(&f.user, &f.group, &f.trigger)
        .unwrap()
        .is_none());
}

#[test]
fn unknown_result_is_not_redispatched_and_late_result_reconciles() {
    let f = Fixture::new();
    let id = f.claim();
    f.store.mark_group_ai_reply_indeterminate(&id).unwrap();
    assert!(f
        .store
        .claim_group_ai_reply(&f.user, &f.group, &f.trigger)
        .unwrap()
        .is_none());
    f.store
        .complete_group_ai_reply(&f.user, &id, "late answer")
        .unwrap();
    f.store.mark_group_ai_reply_indeterminate(&id).unwrap();
    let state: String = f
        .store
        .conn()
        .unwrap()
        .query_row(
            "SELECT state FROM group_ai_reply_requests WHERE id = ?1",
            [&id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(state, "completed");
    assert_eq!(f.count(), 1);
}

#[test]
fn result_requires_original_requester_membership_and_live_source() {
    let f = Fixture::new();
    let id = f.claim();
    assert!(f
        .store
        .complete_group_ai_reply(&f.other, &id, "forged")
        .is_err());
    assert!(f
        .store
        .claim_group_ai_reply("outsider", &f.group, &f.trigger)
        .is_err());
    assert!(f
        .store
        .claim_group_ai_reply(&f.user, "other-group", &f.trigger)
        .is_err());
    f.store
        .conn()
        .unwrap()
        .execute(
            "DELETE FROM friend_group_members WHERE group_id = ?1 AND user_id = ?2",
            params![f.group, f.user],
        )
        .unwrap();
    assert!(f
        .store
        .complete_group_ai_reply(&f.user, &id, "revoked")
        .is_err());
    assert_eq!(f.count(), 0);
}

#[test]
fn recalled_message_and_empty_result_cannot_be_published() {
    let f = Fixture::new();
    let id = f.claim();
    assert!(f.store.complete_group_ai_reply(&f.user, &id, " ").is_err());
    f.store
        .conn()
        .unwrap()
        .execute(
            "UPDATE friend_group_messages SET recalled_at = ?1 WHERE id = ?2",
            params![now(), f.trigger],
        )
        .unwrap();
    assert!(f
        .store
        .complete_group_ai_reply(&f.user, &id, "recalled")
        .is_err());
    assert_eq!(f.count(), 0);
}

#[test]
fn commit_failure_rolls_back_group_message() {
    let f = Fixture::new();
    let id = f.claim();
    f.store.conn().unwrap().execute_batch(
        "CREATE TRIGGER reject_group_ai_commit BEFORE UPDATE OF state ON group_ai_reply_requests
         WHEN NEW.state = 'completed' BEGIN SELECT RAISE(ABORT, 'injected'); END;",
    ).unwrap();
    assert!(f
        .store
        .complete_group_ai_reply(&f.user, &id, "answer")
        .is_err());
    assert_eq!(f.count(), 0);
    f.store
        .conn()
        .unwrap()
        .execute_batch("DROP TRIGGER reject_group_ai_commit")
        .unwrap();
    f.store
        .complete_group_ai_reply(&f.user, &id, "answer")
        .unwrap();
    assert_eq!(f.count(), 1);
}

#[test]
fn concurrent_claims_have_exactly_one_owner() {
    let f = Fixture::new();
    let stores: Vec<_> = (0..4).map(|_| Store::open(&f.path).unwrap()).collect();
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(stores.len()));
    let threads: Vec<_> = stores
        .into_iter()
        .map(|store| {
            let user = f.user.clone();
            let group = f.group.clone();
            let trigger = f.trigger.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                barrier.wait();
                store
                    .claim_group_ai_reply(&user, &group, &trigger)
                    .unwrap()
                    .is_some()
            })
        })
        .collect();
    assert_eq!(
        threads
            .into_iter()
            .map(|t| usize::from(t.join().unwrap()))
            .sum::<usize>(),
        1
    );
}

#[test]
fn migration_is_idempotent() {
    let f = Fixture::new();
    let id = f.claim();
    migrate(&f.store.conn().unwrap()).unwrap();
    assert!(f
        .store
        .claim_group_ai_reply(&f.user, &f.group, &f.trigger)
        .unwrap()
        .is_none());
    f.store
        .complete_group_ai_reply(&f.user, &id, "answer")
        .unwrap();
}
