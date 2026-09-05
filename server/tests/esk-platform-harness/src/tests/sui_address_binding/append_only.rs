use rusqlite::params;

use super::super::{token, Fixture};
use super::helpers::{address, material, verified};
use crate::esk_asset::platform::sui_address_binding::subject_commitment;

#[test]
fn insert_or_replace_cannot_rewrite_append_only_evidence() {
    let fixture = Fixture::new();
    let subject = subject_commitment(&[8_u8; 32]);
    let conn = fixture.store.conn().unwrap();
    conn.execute(
        "INSERT INTO esk_platform_sui_subjects(
           user_id,subject_commitment,created_session_id,created_at
         ) VALUES('bob',?1,'bob','2026-09-04T09:50:00.000Z')",
        params![subject],
    )
    .unwrap();
    let replacement_subject = subject_commitment(&[7_u8; 32]);
    assert!(
        conn.execute(
            "INSERT OR REPLACE INTO esk_platform_sui_subjects(
               rowid,user_id,subject_commitment,created_session_id,created_at
             ) SELECT rowid,'alice',?1,'alice','2026-09-04T09:51:00.000Z'
                 FROM esk_platform_sui_subjects WHERE user_id='bob'",
            params![replacement_subject],
        )
        .is_err(),
        "hidden rowid must not permit replacement with fresh declared keys"
    );
    for table in [
        "esk_platform_sui_subjects",
        "esk_platform_sui_address_binding_challenges",
        "esk_platform_sui_address_bindings",
    ] {
        assert!(
            conn.prepare(&format!("SELECT rowid FROM {table}")).is_err(),
            "{table} must not expose an undeclared replacement key"
        );
    }
    let guarded_keys: [(&str, &[&str]); 3] = [
        (
            "esk_platform_sui_subjects",
            &[
                "existing.user_id=NEW.user_id",
                "existing.subject_commitment=NEW.subject_commitment",
            ],
        ),
        (
            "esk_platform_sui_address_binding_challenges",
            &["existing.challenge_id=NEW.challenge_id"],
        ),
        (
            "esk_platform_sui_address_bindings",
            &[
                "existing.binding_id=NEW.binding_id",
                "existing.challenge_id=NEW.challenge_id",
                "existing.user_id=NEW.user_id",
                "existing.address=NEW.address",
            ],
        ),
    ];
    for (table, predicates) in guarded_keys {
        let trigger_name = format!("trg_{table}_no_replacement_insert");
        let sql: String = conn
            .query_row(
                "SELECT sql FROM sqlite_master WHERE type='trigger' AND name=?1",
                [trigger_name],
                |row| row.get(0),
            )
            .unwrap();
        for predicate in predicates {
            assert!(
                sql.contains(predicate),
                "{table} replacement guard must cover {predicate}"
            );
        }
    }
    assert!(conn
        .execute(
            "INSERT OR REPLACE INTO esk_platform_sui_subjects
             SELECT * FROM esk_platform_sui_subjects WHERE user_id='bob'",
            [],
        )
        .is_err());
    drop(conn);

    let challenge = fixture
        .store
        .create_esk_sui_address_binding_challenge("alice", &token("alice"), &material(address(), 6))
        .unwrap();
    let conn = fixture.store.conn().unwrap();
    assert!(conn
        .execute(
            "INSERT OR REPLACE INTO esk_platform_sui_address_binding_challenges
             SELECT * FROM esk_platform_sui_address_binding_challenges WHERE challenge_id=?1",
            params![challenge.challenge_id],
        )
        .is_err());
    drop(conn);

    fixture
        .store
        .complete_esk_sui_address_binding(
            "alice",
            &token("alice"),
            &challenge.challenge_id,
            &verified(&challenge),
        )
        .unwrap();
    let conn = fixture.store.conn().unwrap();
    assert!(conn
        .execute(
            "INSERT OR REPLACE INTO esk_platform_sui_address_bindings
             SELECT * FROM esk_platform_sui_address_bindings WHERE challenge_id=?1",
            params![challenge.challenge_id],
        )
        .is_err());
    drop(conn);
    fixture.assert_empty_posting();
}
