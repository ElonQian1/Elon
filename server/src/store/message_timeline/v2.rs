use super::{
    cursor, query,
    reading::{self, Position, Scope},
    Cursor, Store, TimelinePage,
};
use anyhow::{bail, Result};
use rusqlite::Connection;
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Deserialize, Default)]
pub(crate) struct Request {
    #[serde(flatten)]
    pub scope: Scope,
    pub before: Option<String>,
    pub after: Option<String>,
    pub sync: Option<String>,
    pub around: Option<String>,
    pub bookmark: Option<String>,
    #[serde(default)]
    pub resume: bool,
    pub limit: Option<usize>,
}

fn key(row: &Value) -> (String, String) {
    (
        row["created_at"].as_str().unwrap_or_default().into(),
        row["id"].as_str().unwrap_or_default().into(),
    )
}
fn decorate(rows: &mut [Value], owner: &str, scope: &Scope, epoch: &str) {
    let binding = cursor::binding(owner, &scope.kind, &scope.project, &scope.id);
    for row in rows {
        for (direction, field) in [
            ("before", "timeline_cursor"),
            ("after", "timeline_after_cursor"),
        ] {
            row[field] = Value::String(
                Cursor {
                    binding: format!("{binding}:{direction}"),
                    epoch: epoch.into(),
                    sequence: None,
                    before: Some(key(row)),
                }
                .encode(),
            );
        }
    }
}
fn epoch(conn: &Connection) -> Result<String> {
    Ok(conn.query_row(
        "SELECT value FROM message_timeline_epoch WHERE id=1",
        [],
        |r| r.get(0),
    )?)
}

impl Store {
    pub(crate) fn read_message_timeline_v2(&self, owner: &str, r: &Request) -> Result<Value> {
        let selectors = [
            r.before.is_some(),
            r.after.is_some(),
            r.sync.is_some(),
            r.around.is_some(),
            r.bookmark.is_some(),
        ];
        if selectors.into_iter().filter(|v| *v).count() > 1 {
            bail!("invalid_timeline_direction");
        }
        if let Some(sync) = &r.sync {
            let mut request = r.scope.request();
            request.sync = Some(sync.clone());
            request.limit = r.limit;
            return self.decorate_timeline_v2(
                owner,
                &r.scope,
                self.read_message_timeline(owner, &request)?,
            );
        }
        let mut conn = self.conn()?;
        let tx = conn.transaction()?;
        let scope = r.scope.request();
        query::authorize(&tx, owner, &scope)?;
        let epoch = epoch(&tx)?;
        let binding = cursor::binding(owner, &scope.kind, &scope.project, &scope.id);
        let limit = r.limit.unwrap_or(50).clamp(1, 100);
        let mut target = Value::Null;
        let mut rows = if r.around.is_some() || r.bookmark.is_some() {
            let position = if let Some(id) = &r.bookmark {
                reading::anchor(&tx, owner, &r.scope, id, r.resume)?
            } else {
                reading::position(
                    &tx,
                    owner,
                    &r.scope,
                    &Position {
                        message_id: r.around.clone().unwrap(),
                        ..Default::default()
                    },
                )?
            };
            let mut exact = query::messages(
                &tx,
                owner,
                &scope,
                None,
                Some(std::slice::from_ref(&position.message_id)),
                1,
            )?;
            let missing = exact.is_empty();
            if missing {
                exact = query::directed_messages(
                    &tx,
                    owner,
                    &scope,
                    Some(&position.key()),
                    None,
                    1,
                    true,
                )?;
                if exact.is_empty() {
                    exact = query::messages(&tx, owner, &scope, Some(&position.key()), None, 1)?;
                }
            }
            if let Some(center) = exact.pop() {
                let center_key = key(&center);
                let before_count = (limit - 1) / 2;
                let mut left =
                    query::messages(&tx, owner, &scope, Some(&center_key), None, before_count)?;
                let right = query::directed_messages(
                    &tx,
                    owner,
                    &scope,
                    Some(&center_key),
                    None,
                    limit - 1 - left.len(),
                    true,
                )?;
                if right.len() < limit - 1 - left.len() {
                    left = query::messages(
                        &tx,
                        owner,
                        &scope,
                        Some(&center_key),
                        None,
                        limit - 1 - right.len(),
                    )?;
                }
                target = json!({"requested_id":position.message_id,"resolved_id":center["id"],"status":if missing {"nearby"}else{"exact"},"fraction":if missing{0.0}else{position.fraction}});
                left.push(center);
                left.extend(right);
                left
            } else {
                target = json!({"requested_id":position.message_id,"resolved_id":null,"status":"empty","fraction":0});
                vec![]
            }
        } else {
            let ascending = r.after.is_some();
            let direction = if ascending { "after" } else { "before" };
            let boundary = r
                .after
                .as_ref()
                .or(r.before.as_ref())
                .map(|c| {
                    Cursor::decode(c, &format!("{binding}:{direction}"), &epoch).and_then(|c| {
                        if c.sequence.is_some() {
                            bail!("invalid_history_cursor");
                        }
                        c.before
                            .ok_or_else(|| anyhow::anyhow!("invalid_history_cursor"))
                    })
                })
                .transpose()?;
            query::directed_messages(
                &tx,
                owner,
                &scope,
                boundary.as_ref(),
                None,
                limit,
                ascending,
            )?
        };
        let has_older = if let Some(first) = rows.first() {
            !query::messages(&tx, owner, &scope, Some(&key(first)), None, 1)?.is_empty()
        } else {
            false
        };
        let has_newer = if let Some(last) = rows.last() {
            !query::directed_messages(&tx, owner, &scope, Some(&key(last)), None, 1, true)?
                .is_empty()
        } else {
            false
        };
        let watermark:i64=tx.query_row("SELECT COALESCE((SELECT seq FROM sqlite_sequence WHERE name='message_timeline_changes'),0)",[],|r|r.get(0))?;
        decorate(&mut rows, owner, &r.scope, &epoch);
        // Only a new window starts a change checkpoint; paging must not skip changes in retained rows.
        let sync = if r.before.is_none() && r.after.is_none() {
            Some(
                Cursor {
                    binding,
                    epoch,
                    sequence: Some(watermark),
                    before: None,
                }
                .encode(),
            )
        } else {
            None
        };
        let result = json!({"schema":"elon.message_timeline.v2","messages":rows,"removed_ids":[],
            "before":rows.first().map(|m|&m["timeline_cursor"]),"after":rows.last().map(|m|&m["timeline_after_cursor"]),
            "has_older":has_older,"has_newer":has_newer,"has_more":if r.after.is_some(){has_newer}else{has_older},
            "sync":sync,"reset":false,"target":target});
        tx.commit()?;
        Ok(result)
    }

    pub(crate) fn decorate_timeline_v2(
        &self,
        owner: &str,
        scope: &Scope,
        mut page: TimelinePage,
    ) -> Result<Value> {
        let conn = self.conn()?;
        query::authorize(&conn, owner, &scope.request())?;
        decorate(&mut page.messages, owner, scope, &epoch(&conn)?);
        page.schema = "elon.message_timeline.v2";
        page.before = page
            .messages
            .first()
            .and_then(|m| m["timeline_cursor"].as_str())
            .map(str::to_owned);
        let mut result = serde_json::to_value(page)?;
        result["after"] = result["messages"]
            .as_array()
            .and_then(|rows| rows.last())
            .map(|m| m["timeline_after_cursor"].clone())
            .unwrap_or(Value::Null);
        Ok(result)
    }
}
