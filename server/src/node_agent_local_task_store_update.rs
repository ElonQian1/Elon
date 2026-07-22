//! Update candidate queries with distinct shared-drain and installer boundaries.

use anyhow::Result;

use super::{read_record, select_sql, LocalTaskRecord, LocalTaskStore};

impl LocalTaskStore {
    /// Shared restart-drain view. Resume-required rows have no live executor
    /// and must not keep a broadcast drain open indefinitely.
    pub(crate) fn list_update_candidates(&self) -> Result<Vec<LocalTaskRecord>> {
        let conn = self.open()?;
        let mut stmt = conn.prepare(&format!(
            "{} WHERE status IN ('running','recovering','reattaching','cancel_requested')
             ORDER BY started_at_ms",
            select_sql()
        ))?;
        let records = stmt
            .query_map([], read_record)?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(anyhow::Error::from)?;
        Ok(records)
    }
}
