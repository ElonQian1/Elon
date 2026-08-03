use anyhow::Result;
use rusqlite::Connection;

pub(crate) fn migration_v163(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS open_commerce_data_request_followups (
           id                   TEXT PRIMARY KEY,
           data_request_id      TEXT NOT NULL,
           consumer_project_id  TEXT NOT NULL,
           consumer_user_id     TEXT NOT NULL,
           merchant_project_id  TEXT NOT NULL,
           merchant_id          TEXT NOT NULL,
           action_kind          TEXT NOT NULL
                                CHECK(action_kind IN ('reminder', 'escalate_attention')),
           idempotency_key      TEXT NOT NULL,
           note                 TEXT,
           created_at           TEXT NOT NULL,
           FOREIGN KEY(data_request_id)
             REFERENCES open_commerce_consumer_data_requests(id) ON DELETE CASCADE,
           FOREIGN KEY(consumer_project_id) REFERENCES projects(id) ON DELETE CASCADE,
           FOREIGN KEY(consumer_user_id) REFERENCES users(id) ON DELETE CASCADE,
           FOREIGN KEY(merchant_project_id) REFERENCES projects(id) ON DELETE CASCADE,
           FOREIGN KEY(merchant_id) REFERENCES open_commerce_merchants(id) ON DELETE CASCADE,
           UNIQUE(data_request_id, consumer_user_id, idempotency_key)
         );
         CREATE INDEX IF NOT EXISTS idx_open_commerce_data_request_followups_request
           ON open_commerce_data_request_followups(data_request_id, created_at DESC);
         CREATE INDEX IF NOT EXISTS idx_open_commerce_data_request_followups_merchant
           ON open_commerce_data_request_followups(
             merchant_project_id, merchant_id, created_at DESC
           );
         CREATE UNIQUE INDEX IF NOT EXISTS idx_open_commerce_data_request_one_escalation
           ON open_commerce_data_request_followups(data_request_id)
           WHERE action_kind='escalate_attention';",
    )?;
    Ok(())
}
