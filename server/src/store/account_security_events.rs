use rusqlite::params;
use serde::Serialize;

use super::Store;

#[derive(Clone, Debug, Serialize)]
pub struct AccountSecurityEvent {
    pub id: String,
    pub action: String,
    pub outcome: String,
    pub session_id_present: bool,
    pub request_id_present: bool,
    pub reason_code: Option<String>,
    pub created_at: String,
}

impl Store {
    pub fn list_account_security_events(
        &self,
        user_id: &str,
        before: Option<&str>,
        limit: u32,
    ) -> anyhow::Result<Vec<AccountSecurityEvent>> {
        let conn = self.conn()?;
        let limit = limit.clamp(1, 100);
        let mut statement = conn.prepare(
            "SELECT id, action, outcome, session_id IS NOT NULL, request_id IS NOT NULL,
                    reason_code, created_at
             FROM auth_security_audit
             WHERE user_id = ?1 AND (?2 IS NULL OR created_at < ?2)
             ORDER BY created_at DESC, id DESC
             LIMIT ?3",
        )?;
        let rows = statement.query_map(params![user_id, before, limit], |row| {
            Ok(AccountSecurityEvent {
                id: row.get(0)?,
                action: row.get(1)?,
                outcome: row.get(2)?,
                session_id_present: row.get::<_, i64>(3)? != 0,
                request_id_present: row.get::<_, i64>(4)? != 0,
                reason_code: row.get(5)?,
                created_at: row.get(6)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn count_account_security_events(&self, user_id: &str) -> anyhow::Result<u64> {
        let conn = self.conn()?;
        Ok(conn.query_row(
            "SELECT COUNT(*) FROM auth_security_audit WHERE user_id = ?1",
            [user_id],
            |row| row.get(0),
        )?)
    }
}
