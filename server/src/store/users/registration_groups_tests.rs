use super::{join_new_user, DEFAULT_GROUP_ID, NOTICE_SENDER_ID};
use crate::store::{ExternalAccountSessionInput, Store, VerifiedIdentity};
use rusqlite::params;

struct Fixture {
    store: Store,
    owner: String,
}

impl Fixture {
    fn new() -> Self {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();
        crate::store_schema::apply_migrations(&conn).unwrap();
        let store = Store {
            conn: std::sync::Mutex::new(conn),
        };
        let owner = store
            .create_user("group-owner@example.com", "secret1", None, None)
            .unwrap()
            .id;
        Self { store, owner }
    }

    fn group(&self, id: &str, name: &str) {
        self.store
            .conn()
            .unwrap()
            .execute(
                "INSERT INTO friend_groups (id, name, owner_user_id, created_at, updated_at)
             VALUES (?1, ?2, ?3, '2026-01-01T00:00:00+00:00', '2026-01-01T00:00:00+00:00')",
                params![id, name, self.owner],
            )
            .unwrap();
    }

    fn memberships(&self, user_id: &str) -> i64 {
        self.store
            .conn()
            .unwrap()
            .query_row(
                "SELECT COUNT(*) FROM friend_group_members WHERE group_id = ?1 AND user_id = ?2",
                params![DEFAULT_GROUP_ID, user_id],
                |row| row.get(0),
            )
            .unwrap()
    }

    fn leave(&self, user_id: &str) {
        self.store
            .conn()
            .unwrap()
            .execute(
                "DELETE FROM friend_group_members WHERE group_id = ?1 AND user_id = ?2",
                params![DEFAULT_GROUP_ID, user_id],
            )
            .unwrap();
    }

    fn notice_count(&self) -> i64 {
        self.store
            .conn()
            .unwrap()
            .query_row(
                "SELECT COUNT(*) FROM friend_group_messages WHERE sender_user_id = ?1",
                params![NOTICE_SENDER_ID],
                |row| row.get(0),
            )
            .unwrap()
    }

    fn fail_join(&self) {
        self.store
            .conn()
            .unwrap()
            .execute_batch(
                "CREATE TRIGGER reject_registration_group BEFORE INSERT ON friend_group_members
             BEGIN SELECT RAISE(ABORT, 'injected membership failure'); END;",
            )
            .unwrap();
    }

    fn google_login(
        &self,
    ) -> Result<crate::store::IdentityCompletion, crate::store::IdentityError> {
        let challenge = self
            .store
            .create_identity_challenge("google", "login", None, "web")
            .unwrap();
        self.store.complete_identity_challenge(
            &challenge.id,
            &VerifiedIdentity {
                provider: "google".into(),
                issuer: "https://accounts.google.com".into(),
                subject: "registration-google-subject".into(),
                email: "registration-google@example.com".into(),
                display_name: Some("New user".into()),
                avatar_url: None,
                nonce: challenge.nonce,
            },
        )
    }

    fn external_login(&self) -> anyhow::Result<crate::store::ExternalAccountSession> {
        self.store.create_external_app_session(
            "fb2",
            &[],
            ExternalAccountSessionInput {
                external_user_id: "registration-external".into(),
                account: "registration-external@example.com".into(),
                display_name: None,
                avatar_url: None,
                device_name: None,
                apk_version: None,
            },
        )
    }
}

#[test]
fn dissolved_default_group_does_not_accept_new_registrations() {
    let fixture = Fixture::new();
    fixture.group(DEFAULT_GROUP_ID, "杀蟑螂");
    fixture
        .store
        .conn()
        .unwrap()
        .execute(
            "UPDATE friend_group_management SET dissolved = 1 WHERE group_id = ?1",
            params![DEFAULT_GROUP_ID],
        )
        .unwrap();
    let user = fixture
        .store
        .create_user("after-dissolve@example.com", "secret1", None, None)
        .unwrap();
    assert_eq!(fixture.memberships(&user.id), 0);
    assert_eq!(fixture.notice_count(), 0);
}

#[test]
fn password_registration_joins_once_without_old_unread_messages_or_backfill() {
    let fixture = Fixture::new();
    fixture.group(DEFAULT_GROUP_ID, "杀蟑螂");
    fixture
        .store
        .conn()
        .unwrap()
        .execute(
            "INSERT INTO friend_group_messages (id, group_id, sender_user_id, content, created_at)
         VALUES ('old-message', ?1, ?2, 'old history', '2026-01-02T00:00:00+00:00')",
            params![DEFAULT_GROUP_ID, fixture.owner],
        )
        .unwrap();
    let user = fixture
        .store
        .create_user("new@example.com", "secret1", None, None)
        .unwrap();
    let groups = fixture.store.list_friend_groups(&user.id).unwrap();
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].id, DEFAULT_GROUP_ID);
    assert_eq!(groups[0].unread_count, 0);
    assert_eq!(fixture.memberships(&fixture.owner), 0);

    let conn = fixture.store.conn().unwrap();
    let before: String = conn
        .query_row(
            "SELECT last_read_at FROM friend_group_members WHERE group_id = ?1 AND user_id = ?2",
            params![DEFAULT_GROUP_ID, user.id],
            |row| row.get(0),
        )
        .unwrap();
    let tx = conn.unchecked_transaction().unwrap();
    join_new_user(&tx, &user.id, "2099-01-01T00:00:00+00:00").unwrap();
    tx.commit().unwrap();
    let after: String = conn
        .query_row(
            "SELECT last_read_at FROM friend_group_members WHERE group_id = ?1 AND user_id = ?2",
            params![DEFAULT_GROUP_ID, user.id],
            |row| row.get(0),
        )
        .unwrap();
    drop(conn);
    assert_eq!(before, after);
    assert_eq!(fixture.memberships(&user.id), 1);
    assert_eq!(fixture.notice_count(), 1);
    assert!(fixture
        .store
        .create_user("new@example.com", "secret1", None, None)
        .is_err());
    fixture.leave(&user.id);
    fixture
        .store
        .authenticate_password("new@example.com", "secret1")
        .unwrap();
    fixture.store.create_session(&user.id, None, None).unwrap();
    assert_eq!(fixture.memberships(&user.id), 0);
    assert_eq!(fixture.notice_count(), 1);
}

#[test]
fn missing_group_never_joins_a_same_named_group_and_rename_keeps_identity() {
    let fixture = Fixture::new();
    fixture.group("same-name-group", "杀蟑螂");
    let before = fixture
        .store
        .create_user("before@example.com", "secret1", None, None)
        .unwrap();
    assert!(fixture
        .store
        .list_friend_groups(&before.id)
        .unwrap()
        .is_empty());
    fixture.group(DEFAULT_GROUP_ID, "Renamed community");
    let after = fixture
        .store
        .create_user("after@example.com", "secret1", None, Some("admin"))
        .unwrap();
    let groups = fixture.store.list_friend_groups(&after.id).unwrap();
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].id, DEFAULT_GROUP_ID);
    assert_eq!(groups[0].name, "Renamed community");
    assert_eq!(fixture.memberships(&before.id), 0);
}

#[test]
fn first_google_registration_joins_but_repeat_login_does_not_rejoin() {
    let fixture = Fixture::new();
    fixture.group(DEFAULT_GROUP_ID, "杀蟑螂");
    let first = fixture.google_login().unwrap();
    assert!(first.created_user);
    assert_eq!(fixture.memberships(&first.user.id), 1);
    fixture.leave(&first.user.id);
    let second = fixture.google_login().unwrap();
    assert!(!second.created_user);
    assert_eq!(second.user.id, first.user.id);
    assert_eq!(fixture.memberships(&first.user.id), 0);
    assert_eq!(fixture.notice_count(), 1);
}

#[test]
fn first_external_registration_joins_but_system_owner_and_repeat_login_do_not() {
    let fixture = Fixture::new();
    fixture.group(DEFAULT_GROUP_ID, "杀蟑螂");
    let first = fixture.external_login().unwrap();
    assert_eq!(fixture.memberships(&first.user.id), 1);
    assert_eq!(fixture.memberships("usr_external_fb2"), 0);
    fixture.leave(&first.user.id);
    let second = fixture.external_login().unwrap();
    assert_eq!(second.user.id, first.user.id);
    assert_eq!(fixture.memberships(&first.user.id), 0);
    assert_eq!(fixture.notice_count(), 1);
}

#[test]
fn membership_failure_rolls_back_each_registration_path() {
    let fixture = Fixture::new();
    fixture.group(DEFAULT_GROUP_ID, "杀蟑螂");
    fixture.fail_join();
    assert!(fixture
        .store
        .create_user("rollback@example.com", "secret1", None, None)
        .is_err());
    assert!(fixture.google_login().is_err());
    assert!(fixture.external_login().is_err());
    let conn = fixture.store.conn().unwrap();
    let users: i64 = conn.query_row(
        "SELECT COUNT(*) FROM users WHERE email IN
         ('rollback@example.com', 'registration-google@example.com', 'registration-external@example.com')",
        [], |row| row.get(0),
    ).unwrap();
    assert_eq!(users, 0);
    let identities: i64 = conn
        .query_row("SELECT COUNT(*) FROM user_identities", [], |row| row.get(0))
        .unwrap();
    assert_eq!(identities, 0);
    let links: i64 = conn
        .query_row("SELECT COUNT(*) FROM external_app_accounts", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(links, 0);
}

#[test]
fn notice_is_readable_without_adding_a_login_or_roster_member() {
    let fixture = Fixture::new();
    assert_eq!(fixture.notice_count(), 0);
    fixture.group(DEFAULT_GROUP_ID, "杀蟑螂");
    let user = fixture
        .store
        .create_user("notice@example.com", "secret1", Some("新成员"), None)
        .unwrap();
    assert_eq!(fixture.notice_count(), 1);
    assert_eq!(fixture.memberships(NOTICE_SENDER_ID), 0);
    assert!(fixture
        .store
        .authenticate_password(NOTICE_SENDER_ID, "secret1")
        .is_err());
    let conn = fixture.store.conn().unwrap();
    let row: (String, String, String, String, i64) = conn
        .query_row(
            "SELECT m.content, m.created_at, gm.created_at, u.status, u.password_login_enabled
         FROM friend_group_messages m JOIN users u ON u.id = m.sender_user_id
         JOIN friend_group_members gm ON gm.group_id = m.group_id AND gm.user_id = ?1
         WHERE m.sender_user_id = ?2",
            params![user.id, NOTICE_SENDER_ID],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
        )
        .unwrap();
    assert_eq!(row.0, "新成员 加入了群聊");
    assert_eq!(row.1, row.2);
    assert_eq!(row.3, "disabled");
    assert_eq!(row.4, 0);
}

#[test]
fn notice_failure_rolls_back_account_membership_and_system_sender() {
    let fixture = Fixture::new();
    fixture.group(DEFAULT_GROUP_ID, "杀蟑螂");
    fixture
        .store
        .conn()
        .unwrap()
        .execute_batch(
            "CREATE TRIGGER reject_join_notice BEFORE INSERT ON friend_group_messages
         BEGIN SELECT RAISE(ABORT, 'injected notice failure'); END;",
        )
        .unwrap();
    assert!(fixture
        .store
        .create_user("notice-rollback@example.com", "secret1", None, None)
        .is_err());
    assert!(fixture.google_login().is_err());
    assert!(fixture.external_login().is_err());
    let conn = fixture.store.conn().unwrap();
    let members: i64 = conn
        .query_row("SELECT COUNT(*) FROM friend_group_members", [], |r| {
            r.get(0)
        })
        .unwrap();
    let users: i64 = conn.query_row(
        "SELECT COUNT(*) FROM users WHERE id = ?1 OR email IN
         ('notice-rollback@example.com', 'registration-google@example.com', 'registration-external@example.com')",
        params![NOTICE_SENDER_ID], |r| r.get(0),
    ).unwrap();
    assert_eq!(members, 0);
    assert_eq!(users, 0);
}
