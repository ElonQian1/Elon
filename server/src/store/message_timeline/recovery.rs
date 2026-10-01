use super::{cursor, query, Cursor, Store, TimelinePage, TimelineRequest};
use anyhow::{bail, Result};
use serde_json::Value;

impl Store {
    /// Revalidate only the reader's bounded working set after a journal gap.
    /// Membership, refreshed bodies, deletions and the new watermark share a snapshot.
    pub(crate) fn recover_message_timeline_window(
        &self,
        owner: &str,
        request: &TimelineRequest,
        ids: &[String],
    ) -> Result<TimelinePage> {
        if ids.len() > 300 || ids.iter().any(|id| id.is_empty() || id.len() > 160) {
            bail!("invalid_window_ids");
        }
        let mut conn = self.conn()?;
        let tx = conn.transaction()?;
        query::authorize(&tx, owner, request)?;
        let epoch: String = tx.query_row(
            "SELECT value FROM message_timeline_epoch WHERE id=1",
            [],
            |r| r.get(0),
        )?;
        let watermark: i64 = tx.query_row("SELECT COALESCE((SELECT seq FROM sqlite_sequence WHERE name='message_timeline_changes'),0)", [], |r| r.get(0))?;
        let binding = cursor::binding(owner, &request.kind, &request.project, &request.id);
        let mut messages = query::messages(&tx, owner, request, None, Some(ids), 300)?;
        for message in &mut messages {
            message["timeline_cursor"] = Value::String(
                Cursor {
                    binding: binding.clone(),
                    epoch: epoch.clone(),
                    sequence: None,
                    before: Some((
                        message["created_at"].as_str().unwrap_or_default().into(),
                        message["id"].as_str().unwrap_or_default().into(),
                    )),
                }
                .encode(),
            );
        }
        let removed_ids = ids
            .iter()
            .filter(|id| {
                !messages
                    .iter()
                    .any(|m| m["id"].as_str() == Some(id.as_str()))
            })
            .cloned()
            .collect();
        let before = messages
            .first()
            .and_then(|m| m["timeline_cursor"].as_str())
            .map(str::to_owned);
        tx.commit()?;
        Ok(TimelinePage {
            schema: "elon.message_timeline.v1",
            messages,
            removed_ids,
            before,
            sync: Some(
                Cursor {
                    binding,
                    epoch,
                    sequence: Some(watermark),
                    before: None,
                }
                .encode(),
            ),
            has_more: false,
            reset: false,
        })
    }
}
