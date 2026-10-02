//! Shared bounded timeline, independent of the renderer and model prompt history.
mod cursor;
#[cfg(test)]
mod integration_tests;
mod query;
pub(crate) mod reading;
mod recovery;
pub(crate) mod schema;
#[cfg(test)]
mod tests;
pub(crate) mod v2;

use super::Store;
use anyhow::{bail, Result};
use cursor::Cursor;
use rusqlite::named_params;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Deserialize)]
pub(crate) struct TimelineRequest {
    pub kind: String,
    pub id: String,
    #[serde(default)]
    pub project: String,
    pub before: Option<String>,
    pub sync: Option<String>,
    pub limit: Option<usize>,
}

#[derive(Serialize)]
pub(crate) struct TimelinePage {
    pub schema: &'static str,
    pub messages: Vec<Value>,
    pub removed_ids: Vec<String>,
    pub before: Option<String>,
    pub sync: Option<String>,
    pub has_more: bool,
    pub reset: bool,
}

impl Store {
    pub(crate) fn read_message_timeline(
        &self,
        owner: &str,
        request: &TimelineRequest,
    ) -> Result<TimelinePage> {
        let mut conn = self.conn()?;
        let tx = conn.transaction()?;
        query::authorize(&tx, owner, request)?;
        let epoch: String = tx.query_row(
            "SELECT value FROM message_timeline_epoch WHERE id=1",
            [],
            |r| r.get(0),
        )?;
        let binding = cursor::binding(owner, &request.kind, &request.project, &request.id);
        let limit = request.limit.unwrap_or(50).clamp(1, 100);
        if request.before.is_some() && request.sync.is_some() {
            bail!("choose_history_or_sync");
        }
        let watermark: i64 = tx.query_row("SELECT COALESCE((SELECT seq FROM sqlite_sequence WHERE name='message_timeline_changes'),0)", [], |r| r.get(0))?;
        let make_cursor = |sequence, before| {
            Cursor {
                binding: binding.clone(),
                epoch: epoch.clone(),
                sequence,
                before,
            }
            .encode()
        };
        let mut page = TimelinePage {
            schema: "elon.message_timeline.v1",
            messages: vec![],
            removed_ids: vec![],
            before: None,
            sync: None,
            has_more: false,
            reset: false,
        };
        if let Some(sync) = &request.sync {
            let decoded = match Cursor::decode(sync, &binding, &epoch) {
                Err(e) if e.to_string() == "timeline_cursor_expired" => {
                    page.reset = true;
                    return Ok(page);
                }
                result => result?,
            };
            let sequence = decoded
                .sequence
                .ok_or_else(|| anyhow::anyhow!("invalid_sync_cursor"))?;
            let floor: i64 = tx.query_row(
                "SELECT COALESCE(MIN(seq),0) FROM message_timeline_changes",
                [],
                |r| r.get(0),
            )?;
            if sequence > watermark || (floor > 0 && sequence < floor - 1) {
                page.reset = true;
                return Ok(page);
            }
            let scope = query::event_scope(owner, request);
            let backlog: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM message_timeline_changes WHERE kind=?1 AND project=?2 AND scope=?3 AND (audience='' OR audience=?4) AND seq>?5 AND seq<=?6 ORDER BY seq LIMIT 1 OFFSET 1000)", rusqlite::params![request.kind,request.project,scope,owner,sequence,watermark], |row| row.get(0))?;
            if backlog {
                page.reset = true;
                return Ok(page);
            }
            let mut stmt = tx.prepare("SELECT seq,message_id FROM message_timeline_changes
                WHERE kind=:kind AND project=:project AND scope=:scope AND (audience='' OR audience=:owner)
                  AND seq>:seq AND seq<=:watermark ORDER BY seq LIMIT :limit")?;
            let events = stmt.query_map(named_params!{ ":kind":request.kind, ":project":request.project,
                ":scope":scope, ":owner":owner, ":seq":sequence, ":watermark":watermark, ":limit":(limit+1) as i64 },
                |r| Ok((r.get::<_,i64>(0)?,r.get::<_,String>(1)?)))?.collect::<rusqlite::Result<Vec<_>>>()?;
            page.has_more = events.len() > limit;
            let events = &events[..events.len().min(limit)];
            let checkpoint = if page.has_more {
                events.last().unwrap().0
            } else {
                watermark
            };
            let ids: Vec<String> = events
                .iter()
                .map(|(_, id)| id.clone())
                .collect::<std::collections::BTreeSet<_>>()
                .into_iter()
                .collect();
            page.messages = query::messages(&tx, owner, request, None, Some(&ids), limit)?;
            page.removed_ids = ids
                .into_iter()
                .filter(|id| !page.messages.iter().any(|m| m["id"].as_str() == Some(id)))
                .collect();
            page.sync = Some(make_cursor(Some(checkpoint), None));
        } else {
            let before = request
                .before
                .as_ref()
                .map(|value| {
                    Cursor::decode(value, &binding, &epoch).and_then(|c| {
                        c.before
                            .ok_or_else(|| anyhow::anyhow!("invalid_history_cursor"))
                    })
                })
                .transpose()?;
            page.messages = query::messages(&tx, owner, request, before.as_ref(), None, limit + 1)?;
            page.has_more = page.messages.len() > limit;
            if page.has_more {
                page.messages.remove(0);
            }
            page.before = page.messages.first().map(|m| {
                make_cursor(
                    None,
                    Some((
                        m["created_at"].as_str().unwrap_or_default().into(),
                        m["id"].as_str().unwrap_or_default().into(),
                    )),
                )
            });
            if request.before.is_none() {
                page.sync = Some(make_cursor(Some(watermark), None));
            }
        }
        for message in &mut page.messages {
            message["timeline_cursor"] = Value::String(make_cursor(
                None,
                Some((
                    message["created_at"].as_str().unwrap_or_default().into(),
                    message["id"].as_str().unwrap_or_default().into(),
                )),
            ));
        }
        tx.commit()?;
        Ok(page)
    }

    pub(crate) fn mark_timeline_read(
        &self,
        owner: &str,
        request: &TimelineRequest,
        message_id: &str,
    ) -> Result<()> {
        let mut conn = self.conn()?;
        let tx = conn.transaction()?;
        query::authorize(&tx, owner, request)?;
        let messages =
            query::messages(&tx, owner, request, None, Some(&[message_id.to_owned()]), 1)?;
        let at = messages
            .first()
            .and_then(|m| m["created_at"].as_str())
            .ok_or_else(|| anyhow::anyhow!("message_not_found"))?;
        match request.kind.as_str() {
            "group" => {
                tx.execute("UPDATE friend_group_members SET last_read_at=MAX(COALESCE(last_read_at,''),?3) WHERE group_id=?1 AND user_id=?2", rusqlite::params![request.id,owner,at])?;
            }
            "friend" => {
                tx.execute("INSERT INTO friend_read_states(user_id,friend_user_id,last_read_at) VALUES(?1,?2,?3) ON CONFLICT(user_id,friend_user_id) DO UPDATE SET last_read_at=MAX(last_read_at,excluded.last_read_at)", rusqlite::params![owner,request.id,at])?;
            }
            "channel" => {
                tx.execute("INSERT INTO project_channel_read_states(project_id,channel_id,user_id,last_read_at) VALUES(?1,?2,?3,?4) ON CONFLICT(project_id,channel_id,user_id) DO UPDATE SET last_read_at=MAX(last_read_at,excluded.last_read_at)", rusqlite::params![request.project,request.id,owner,at])?;
            }
            _ => {}
        }
        tx.commit()?;
        if request.kind == "friend" {
            crate::read_receipt_events::publish(owner.into(), request.id.clone(), at.into());
        }
        Ok(())
    }
}
