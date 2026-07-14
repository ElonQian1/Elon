use super::*;

#[test]
fn migrates_legacy_single_model_embedding_table() {
    let conn = Connection::open_in_memory().expect("open sqlite");
    conn.execute_batch(
        r#"
            CREATE TABLE chunks(id TEXT PRIMARY KEY);
            INSERT INTO chunks(id) VALUES ('chunk-1');

            CREATE TABLE embeddings (
                chunk_id TEXT PRIMARY KEY,
                model TEXT NOT NULL,
                dim INTEGER NOT NULL,
                vector BLOB NOT NULL,
                content_hash TEXT NOT NULL,
                created_at INTEGER NOT NULL
            );

            INSERT INTO embeddings(chunk_id, model, dim, vector, content_hash, created_at)
            VALUES ('chunk-1', 'local-hash-v1', 256, zeroblob(1024), 'hash-1', 1);
            "#,
    )
    .expect("seed legacy schema");

    create_embedding_schema(&conn).expect("migrate schema");

    assert!(!legacy_embedding_primary_key(&conn).expect("check pk"));
    conn.execute(
        r#"
            INSERT INTO embeddings(chunk_id, model, dim, vector, content_hash, created_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            "#,
        params![
            "chunk-1",
            "future-semantic-model",
            768_i64,
            vec![1_u8; 3072],
            "hash-1",
            2_i64
        ],
    )
    .expect("insert second model for same chunk");

    let count = conn
        .query_row(
            "SELECT COUNT(*) FROM embeddings WHERE chunk_id = 'chunk-1'",
            [],
            |row| row.get::<_, i64>(0),
        )
        .expect("count embeddings");
    assert_eq!(count, 2);
}
