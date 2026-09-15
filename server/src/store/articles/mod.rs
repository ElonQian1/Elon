//! Author-owned content and immutable publications, independent of their destinations.
use super::{new_id, now, Store};
use anyhow::Result;
use rusqlite::{params, Connection, OptionalExtension};
mod drafts;
mod media;
pub(crate) mod migration;
mod model;
mod publishing;
mod reads;
pub(crate) mod square;
#[cfg(test)]
mod tests;
pub(crate) use model::*;

pub(super) fn message_preview(content: &str) -> Option<String> {
    let value: serde_json::Value =
        serde_json::from_str(content.strip_prefix(publishing::CARD_PREFIX)?).ok()?;
    let id = value.get("article_id")?.as_str()?;
    if value.get("schema")?.as_i64()? != 1
        || value.get("revision")?.as_i64()? < 1
        || !id.starts_with("article_")
        || !id
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-')
    {
        return None;
    }
    Some(format!(
        "[文章] {}",
        value
            .get("title")?
            .as_str()?
            .chars()
            .take(120)
            .collect::<String>()
    ))
}

#[derive(Debug)]
pub(crate) struct ArticleFault(pub u16, pub String);
impl std::fmt::Display for ArticleFault {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.1)
    }
}
impl std::error::Error for ArticleFault {}
fn fail(status: u16, message: &str) -> anyhow::Error {
    ArticleFault(status, message.to_owned()).into()
}

struct Head {
    owner: String,
    version: i64,
    document: ArticleDocument,
    status: String,
}
fn head(conn: &Connection, id: &str) -> Result<Head> {
    let row = conn.query_row("SELECT owner_id,edit_version,draft_json,status FROM social_contents WHERE id=?1 AND kind='article'", [id],
        |r| Ok((r.get::<_,String>(0)?,r.get::<_,i64>(1)?,r.get::<_,String>(2)?,r.get::<_,String>(3)?))).optional()?;
    let (owner, version, json, status) = row.ok_or_else(|| fail(404, "文章不存在或不可访问"))?;
    Ok(Head {
        owner,
        version,
        document: serde_json::from_str(&json)?,
        status,
    })
}
fn owned(conn: &Connection, user: &str, id: &str) -> Result<Head> {
    let value = head(conn, id)?;
    if value.owner != user {
        return Err(fail(404, "文章不存在或不可访问"));
    }
    Ok(value)
}
fn member(conn: &Connection, user: &str, group: &str) -> Result<()> {
    let allowed: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM friend_group_members WHERE group_id=?1 AND user_id=?2)",
        params![group, user],
        |r| r.get(0),
    )?;
    if !allowed {
        return Err(fail(403, "你已不在这个群聊中"));
    }
    Ok(())
}
fn author_name(conn: &Connection, user: &str) -> Result<String> {
    Ok(conn.query_row(
        "SELECT COALESCE(NULLIF(nickname,''),'作者') FROM users WHERE id=?1",
        [user],
        |r| r.get(0),
    )?)
}
