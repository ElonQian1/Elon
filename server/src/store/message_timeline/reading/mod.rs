mod bookmarks;
mod progress;
pub(crate) mod schema;
#[cfg(test)]
mod tests;

use super::{query, Store, TimelineRequest};
use anyhow::{bail, Result};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

#[derive(Clone, Deserialize, Serialize, Default)]
pub(crate) struct Position {
    pub message_id: String,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub fraction: f64,
}
impl Position {
    pub(super) fn key(&self) -> (String, String) {
        (self.created_at.clone(), self.message_id.clone())
    }
}

#[derive(Clone, Deserialize, Serialize, Default)]
pub(crate) struct Scope {
    pub kind: String,
    pub id: String,
    #[serde(default)]
    pub project: String,
}
impl Scope {
    pub(crate) fn request(&self) -> TimelineRequest {
        TimelineRequest {
            kind: self.kind.clone(),
            id: self.id.clone(),
            project: self.project.clone(),
            before: None,
            sync: None,
            limit: None,
        }
    }
    pub(super) fn target(&self, id: &str) -> String {
        if id.is_empty() {
            format!("conversation:{}", json!([self.kind, self.project, self.id]))
        } else {
            format!("bookmark:{id}")
        }
    }
}

#[derive(Deserialize, Serialize, Default)]
pub(crate) struct Command {
    #[serde(flatten)]
    pub scope: Scope,
    pub action: String,
    #[serde(default)]
    pub bookmark_id: String,
    #[serde(default)]
    pub operation_id: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub note: String,
    #[serde(default)]
    pub position: Position,
    #[serde(default)]
    pub base_revision: i64,
    #[serde(default)]
    pub device_id: String,
    #[serde(default)]
    pub device_seq: i64,
}

pub(super) fn position(
    conn: &Connection,
    owner: &str,
    scope: &Scope,
    p: &Position,
) -> Result<Position> {
    if p.message_id.is_empty()
        || p.message_id.len() > 160
        || !p.fraction.is_finite()
        || !(0.0..=1.0).contains(&p.fraction)
    {
        bail!("invalid_reading_position");
    }
    let rows = query::messages(
        conn,
        owner,
        &scope.request(),
        None,
        Some(std::slice::from_ref(&p.message_id)),
        1,
    )?;
    let row = rows
        .first()
        .ok_or_else(|| anyhow::anyhow!("reading_message_unavailable"))?;
    Ok(Position {
        message_id: p.message_id.clone(),
        created_at: row["created_at"].as_str().unwrap_or_default().into(),
        fraction: p.fraction,
    })
}

pub(super) fn require_bookmark(
    conn: &Connection,
    owner: &str,
    scope: &Scope,
    id: &str,
) -> Result<Value> {
    let value =
        bookmarks::find(conn, owner, id)?.ok_or_else(|| anyhow::anyhow!("reading_unavailable"))?;
    if value["deleted"] == true
        || value["kind"] != scope.kind
        || value["project"] != scope.project
        || value["scope"] != scope.id
    {
        bail!("reading_unavailable");
    }
    Ok(value)
}

pub(crate) fn anchor(
    conn: &Connection,
    owner: &str,
    scope: &Scope,
    id: &str,
    resume: bool,
) -> Result<Position> {
    query::authorize(conn, owner, &scope.request())?;
    let bookmark = if id.is_empty() {
        None
    } else {
        Some(require_bookmark(conn, owner, scope, id)?)
    };
    if resume || id.is_empty() {
        if let Some(p) = progress::read(conn, owner, &scope.target(id))? {
            return Ok(serde_json::from_value(p["position"].clone())?);
        }
    }
    bookmark
        .map(|b| serde_json::from_value(b["anchor"].clone()).map_err(Into::into))
        .unwrap_or_else(|| Err(anyhow::anyhow!("reading_position_unavailable")))
}

impl Store {
    pub(crate) fn reading_list(&self, owner: &str, scope: &Scope, after: &str) -> Result<Value> {
        let mut conn = self.conn()?;
        let tx = conn.transaction()?;
        query::authorize(&tx, owner, &scope.request())?;
        let result = bookmarks::list(&tx, owner, scope, after)?;
        tx.commit()?;
        Ok(result)
    }
    pub(crate) fn reading_command(&self, owner: &str, c: &Command) -> Result<Value> {
        if c.operation_id.len() < 8
            || c.operation_id.len() > 100
            || c.bookmark_id.len() > 100
            || c.device_id.len() > 100
            || c.title.chars().count() > 80
            || c.note.chars().count() > 1000
        {
            bail!("invalid_reading_command");
        }
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        query::authorize(&tx, owner, &c.scope.request())?;
        let target = c.scope.target(&c.bookmark_id);
        if c.action != "create" && c.action != "delete" && !c.bookmark_id.is_empty() {
            require_bookmark(&tx, owner, &c.scope, &c.bookmark_id)?;
        }
        let digest = format!("{:x}", Sha256::digest(serde_json::to_vec(c)?));
        let prior: Option<(String, String)> = tx
            .query_row(
                "SELECT digest,result FROM reading_operations WHERE owner=?1 AND operation=?2",
                params![owner, c.operation_id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
        if let Some((hash, result)) = prior {
            if hash != digest {
                bail!("invalid_reading_operation_reuse");
            }
            // Replayed creates may return an ID, but never resurrect a deleted object.
            return Ok(serde_json::from_str(&result)?);
        }
        let result = match c.action.as_str() {
            "create" | "update" | "delete" => bookmarks::write(&tx, owner, c)?,
            "progress" | "resolve" => progress::write(&tx, owner, c)?,
            _ => bail!("invalid_reading_action"),
        };
        tx.execute(
            "INSERT INTO reading_operations VALUES(?1,?2,?3,?4,?5,?6)",
            params![
                owner,
                c.operation_id,
                target,
                digest,
                result.to_string(),
                crate::store::now()
            ],
        )?;
        tx.execute("DELETE FROM reading_operations WHERE created_at < strftime('%Y-%m-%dT%H:%M:%fZ','now','-30 days')",[])?;
        tx.commit()?;
        Ok(result)
    }
}
