use super::migration_v85;
use rusqlite::{params, Connection, OptionalExtension};

#[test]
fn migration_adds_usage_source_columns_and_backfills_untrusted_modes() {
    let conn = Connection::open_in_memory().expect("in-memory db should open");
    conn.execute_batch(
        r#"
            CREATE TABLE token_usage_events (
              id TEXT PRIMARY KEY,
              user_id TEXT NOT NULL,
              feature TEXT NOT NULL,
              usage_mode TEXT NOT NULL,
              total_tokens INTEGER NOT NULL DEFAULT 0,
              created_at TEXT NOT NULL
            );
            INSERT INTO token_usage_events
              (id, user_id, feature, usage_mode, total_tokens, created_at)
            VALUES
              ('tok_platform', 'u1', 'chat', 'server_codex_cli', 100, 'now'),
              ('tok_client', 'u1', 'chat', 'client_reported', 100, 'now'),
              ('tok_byok', 'u1', 'chat', 'user_api_key_proxy', 100, 'now');
            "#,
    )
    .expect("minimal token usage table should apply");

    migration_v85(&conn).expect("usage source migration should apply");

    let platform: String = conn
        .query_row(
            "SELECT billing_source FROM token_usage_events WHERE id = 'tok_platform'",
            [],
            |row| row.get(0),
        )
        .expect("platform row should load");
    let client: String = conn
        .query_row(
            "SELECT billing_source FROM token_usage_events WHERE id = 'tok_client'",
            [],
            |row| row.get(0),
        )
        .expect("client row should load");
    let byok: String = conn
        .query_row(
            "SELECT billing_source FROM token_usage_events WHERE id = 'tok_byok'",
            [],
            |row| row.get(0),
        )
        .expect("byok row should load");
    let owner: Option<String> = conn
        .query_row(
            "SELECT resource_owner_user_id FROM token_usage_events WHERE id = ?1",
            params!["tok_platform"],
            |row| row.get(0),
        )
        .optional()
        .expect("owner column should exist")
        .flatten();

    assert_eq!(platform, "platform");
    assert_eq!(client, "client_reported");
    assert_eq!(byok, "user_api_key");
    assert!(owner.is_none());

    migration_v85(&conn).expect("usage source migration should be idempotent");
}
