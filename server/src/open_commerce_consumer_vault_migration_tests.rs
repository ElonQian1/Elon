use super::*;
use rusqlite::{params, Connection};
use uuid::Uuid;

#[test]
fn v220_preserves_disk_rows_and_scopes_ids_to_the_owner() {
    let path = std::env::temp_dir().join(format!(
        "elon-consumer-vault-v220-{}.sqlite",
        Uuid::new_v4().simple()
    ));
    {
        let conn = Connection::open(&path).unwrap();
        create_parent_fixture(&conn);
        migration_v162(&conn).unwrap();
        insert_vault_row(&conn, "project_a", "user_a", "shared_record");

        migration_v220(&conn).unwrap();
        migration_v220(&conn).unwrap();
        insert_vault_row(&conn, "project_b", "user_b", "shared_record");

        assert_eq!(vault_row_count(&conn, "shared_record"), 2);
        assert!(has_owner_scoped_primary_key(&conn).unwrap());
    }
    {
        let conn = Connection::open(&path).unwrap();
        assert_eq!(vault_row_count(&conn, "shared_record"), 2);
        conn.execute("DELETE FROM users WHERE id='user_a'", [])
            .unwrap();
        assert_eq!(vault_row_count(&conn, "shared_record"), 1);
    }
    std::fs::remove_file(path).unwrap();
}

fn create_parent_fixture(conn: &Connection) {
    conn.execute_batch(
        "PRAGMA foreign_keys=ON;
         CREATE TABLE users (id TEXT PRIMARY KEY);
         CREATE TABLE projects (id TEXT PRIMARY KEY);
         INSERT INTO users (id) VALUES ('user_a'), ('user_b');
         INSERT INTO projects (id) VALUES ('project_a'), ('project_b');",
    )
    .unwrap();
}

fn insert_vault_row(conn: &Connection, project_id: &str, user_id: &str, id: &str) {
    conn.execute(
        "INSERT INTO open_commerce_consumer_data_vault_items (
           id, consumer_project_id, consumer_user_id, label, item_kind,
           envelope_json, ciphertext_sha256, ciphertext_bytes, revision,
           created_at, updated_at
         ) VALUES (?1, ?2, ?3, 'label', 'private_note', '{}', 'digest', 17, 1, 'now', 'now')",
        params![id, project_id, user_id],
    )
    .unwrap();
}

fn vault_row_count(conn: &Connection, id: &str) -> i64 {
    conn.query_row(
        "SELECT COUNT(*) FROM open_commerce_consumer_data_vault_items WHERE id=?1",
        params![id],
        |row| row.get(0),
    )
    .unwrap()
}
