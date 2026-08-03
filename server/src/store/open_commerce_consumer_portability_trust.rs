use anyhow::Result;
use rusqlite::{params, OptionalExtension, Row, TransactionBehavior};

use crate::open_commerce_portability_trust_model::{
    ConsumerPortabilityTrustKey, CONSUMER_PORTABILITY_TRUST_KEY_SCHEMA,
};

use super::{new_id, now, Store};

impl Store {
    pub(crate) fn save_consumer_portability_trust_key(
        &self,
        destination_project_id: &str,
        consumer_user_id: &str,
        source_operator: &str,
        key_id: &str,
        algorithm: &str,
        public_key_pem: &str,
    ) -> Result<(ConsumerPortabilityTrustKey, bool)> {
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let existing = tx
            .query_row(
                &format!(
                    "{TRUST_KEY_SELECT}
                      WHERE destination_project_id=?1 AND consumer_user_id=?2
                        AND source_operator=?3 AND key_id=?4"
                ),
                params![
                    destination_project_id.trim(),
                    consumer_user_id.trim(),
                    source_operator,
                    key_id,
                ],
                trust_key_from_row,
            )
            .optional()?;
        if let Some(existing) = existing {
            tx.commit()?;
            return Ok((existing, false));
        }
        let id = new_id("portability-trust-key");
        let created_at = now();
        tx.execute(
            "INSERT INTO open_commerce_consumer_portability_trust_keys (
               id, destination_project_id, consumer_user_id, source_operator,
               key_id, algorithm, public_key_pem, status, created_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'active', ?8)",
            params![
                id,
                destination_project_id.trim(),
                consumer_user_id.trim(),
                source_operator,
                key_id,
                algorithm,
                public_key_pem,
                created_at,
            ],
        )?;
        let saved = tx.query_row(
            &format!("{TRUST_KEY_SELECT} WHERE id=?1"),
            params![id],
            trust_key_from_row,
        )?;
        tx.commit()?;
        Ok((saved, true))
    }

    pub(crate) fn list_consumer_portability_trust_keys(
        &self,
        destination_project_id: &str,
        consumer_user_id: &str,
        limit: usize,
    ) -> Result<Vec<ConsumerPortabilityTrustKey>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(&format!(
            "{TRUST_KEY_SELECT}
              WHERE destination_project_id=?1 AND consumer_user_id=?2
              ORDER BY created_at DESC, rowid DESC LIMIT ?3"
        ))?;
        let rows = stmt.query_map(
            params![
                destination_project_id.trim(),
                consumer_user_id.trim(),
                limit.clamp(1, 100) as i64,
            ],
            trust_key_from_row,
        )?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub(crate) fn active_consumer_portability_trust_key(
        &self,
        destination_project_id: &str,
        consumer_user_id: &str,
        source_operator: &str,
        key_id: &str,
    ) -> Result<Option<ConsumerPortabilityTrustKey>> {
        self.conn()?
            .query_row(
                &format!(
                    "{TRUST_KEY_SELECT}
                      WHERE destination_project_id=?1 AND consumer_user_id=?2
                        AND source_operator=?3 AND key_id=?4 AND status='active'"
                ),
                params![
                    destination_project_id.trim(),
                    consumer_user_id.trim(),
                    source_operator,
                    key_id,
                ],
                trust_key_from_row,
            )
            .optional()
            .map_err(Into::into)
    }

    pub(crate) fn revoke_consumer_portability_trust_key(
        &self,
        destination_project_id: &str,
        consumer_user_id: &str,
        record_id: &str,
    ) -> Result<Option<ConsumerPortabilityTrustKey>> {
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        tx.execute(
            "UPDATE open_commerce_consumer_portability_trust_keys
                SET status='revoked', revoked_at=COALESCE(revoked_at, ?4)
              WHERE id=?1 AND destination_project_id=?2 AND consumer_user_id=?3
                AND status='active'",
            params![
                record_id.trim(),
                destination_project_id.trim(),
                consumer_user_id.trim(),
                now(),
            ],
        )?;
        let value = tx
            .query_row(
                &format!(
                    "{TRUST_KEY_SELECT}
                      WHERE id=?1 AND destination_project_id=?2 AND consumer_user_id=?3"
                ),
                params![
                    record_id.trim(),
                    destination_project_id.trim(),
                    consumer_user_id.trim(),
                ],
                trust_key_from_row,
            )
            .optional()?;
        tx.commit()?;
        Ok(value)
    }
}

fn trust_key_from_row(row: &Row<'_>) -> rusqlite::Result<ConsumerPortabilityTrustKey> {
    Ok(ConsumerPortabilityTrustKey {
        schema: CONSUMER_PORTABILITY_TRUST_KEY_SCHEMA.to_string(),
        id: row.get(0)?,
        source_operator: row.get(1)?,
        key_id: row.get(2)?,
        algorithm: row.get(3)?,
        public_key_pem: row.get(4)?,
        status: row.get(5)?,
        created_at: row.get(6)?,
        revoked_at: row.get(7)?,
    })
}

const TRUST_KEY_SELECT: &str = "SELECT id, source_operator, key_id, algorithm,
       public_key_pem, status, created_at, revoked_at
  FROM open_commerce_consumer_portability_trust_keys";
