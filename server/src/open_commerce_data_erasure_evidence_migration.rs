use anyhow::Result;
use rusqlite::Connection;

pub(crate) fn migration_v160(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS open_commerce_data_erasure_evidence (
           id                    TEXT PRIMARY KEY,
           data_request_id       TEXT NOT NULL,
           merchant_project_id   TEXT NOT NULL,
           merchant_id           TEXT NOT NULL,
           evidence_kind         TEXT NOT NULL
                                 CHECK(evidence_kind IN (
                                   'external_system_receipt', 'merchant_attestation'
                                 )),
           external_system       TEXT NOT NULL,
           reference_id          TEXT NOT NULL,
           receipt_sha256        TEXT NOT NULL
                                 CHECK(length(receipt_sha256) = 64
                                   AND receipt_sha256 NOT GLOB '*[^0-9a-f]*'),
           summary               TEXT NOT NULL,
           submitted_by_user_id  TEXT NOT NULL,
           created_at            TEXT NOT NULL,
           FOREIGN KEY(data_request_id)
             REFERENCES open_commerce_consumer_data_requests(id) ON DELETE CASCADE,
           FOREIGN KEY(merchant_project_id) REFERENCES projects(id) ON DELETE CASCADE,
           FOREIGN KEY(merchant_id) REFERENCES open_commerce_merchants(id) ON DELETE CASCADE,
           FOREIGN KEY(submitted_by_user_id) REFERENCES users(id) ON DELETE RESTRICT,
           UNIQUE(data_request_id, external_system, receipt_sha256)
         );
         CREATE INDEX IF NOT EXISTS idx_open_commerce_erasure_evidence_request
           ON open_commerce_data_erasure_evidence(data_request_id, created_at ASC);
         CREATE INDEX IF NOT EXISTS idx_open_commerce_erasure_evidence_merchant
           ON open_commerce_data_erasure_evidence(
             merchant_project_id, merchant_id, created_at DESC
           );",
    )?;
    Ok(())
}
