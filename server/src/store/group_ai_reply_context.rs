//! Frozen selected-message evidence and separately revocable continuation consent.
use super::{ensure_member_and_source, Store};
#[path = "group_ai_legacy_context.rs"]
mod legacy;
use anyhow::{ensure, Result};
use rusqlite::{params, Connection, OptionalExtension};
use serde_json::{json, Value};

pub(crate) fn migrate(conn: &Connection) -> Result<()> {
    conn.execute_batch("CREATE TABLE group_ai_reply_contexts (
        request_id TEXT PRIMARY KEY REFERENCES group_ai_reply_requests(id) ON DELETE CASCADE,
        sources_json TEXT NOT NULL, allow_continue INTEGER NOT NULL DEFAULT 0 CHECK(allow_continue IN (0,1)),
        version INTEGER NOT NULL DEFAULT 1
    ); CREATE INDEX group_ai_reply_result ON group_ai_reply_requests(result_message_id);")?;
    Ok(())
}

pub(crate) fn capture(conn: &Connection, request: &str, group: &str, allowed: bool) -> Result<()> {
    let mut stmt = conn.prepare(
        "SELECT m.id,m.sender_user_id,COALESCE(u.nickname,u.email,u.id),
        m.content,m.attachments_json,m.created_at,m.revision FROM group_ai_selected_sources s
        JOIN friend_group_messages m ON m.id=s.message_id JOIN users u ON u.id=m.sender_user_id
        WHERE s.request_id=?1 AND m.group_id=?2 ORDER BY m.rowid",
    )?;
    let sources = stmt.query_map(params![request,group], |r| {
        let attachments: Option<String> = r.get(4)?;
        Ok(json!({"id":r.get::<_,String>(0)?,"sender_user_id":r.get::<_,String>(1)?,
            "sender_name":r.get::<_,String>(2)?,"content":r.get::<_,String>(3)?,
            "attachments":attachments.and_then(|s| serde_json::from_str::<Value>(&s).ok()).unwrap_or(json!([])),
            "created_at":r.get::<_,String>(5)?,"revision":r.get::<_,i64>(6)?,"outgoing":false}))
    })?.collect::<rusqlite::Result<Vec<_>>>()?;
    let encoded = serde_json::to_string(&sources)?;
    ensure!(encoded.len() <= 2 * 1024 * 1024, "所选记录过大，请减少消息");
    conn.execute("INSERT INTO group_ai_reply_contexts(request_id,sources_json,allow_continue) VALUES (?1,?2,?3)",
        params![request,encoded,allowed])?;
    Ok(())
}

// Membership is checked by the caller. Batch decoration avoids one network/query per bubble.
pub(crate) fn decorate(
    conn: &Connection,
    messages: &mut [crate::store::FriendGroupMessage],
) -> Result<()> {
    let ids: Vec<_> = messages
        .iter()
        .filter(|m| m.recalled_at.is_none())
        .map(|m| &m.id)
        .collect();
    if ids.is_empty() {
        return Ok(());
    }
    let sql = format!("SELECT r.result_message_id,r.requester_id,r.web_provider,c.sources_json,c.allow_continue,c.version
        FROM group_ai_reply_requests r JOIN group_ai_reply_contexts c ON c.request_id=r.id
        WHERE r.state='completed' AND r.result_message_id IN ({})", vec!["?";ids.len()].join(","));
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt
        .query_map(rusqlite::params_from_iter(ids), |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, bool>(4)?,
                r.get::<_, i64>(5)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    for (id, owner, provider, encoded, allowed, version) in rows {
        let mut sources: Vec<Value> = serde_json::from_str(&encoded)?;
        for source in sources.iter_mut().take(3) {
            let visible: bool = conn.query_row("SELECT EXISTS(SELECT 1 FROM friend_group_messages WHERE id=?1 AND recalled_at IS NULL)",
                [source["id"].as_str()], |r| r.get(0))?;
            if !visible {
                source["content"] = json!("这条来源消息已撤回");
            }
        }
        if let Some(message) = messages.iter_mut().find(|m| m.id == id) {
            message.ai_reply = Some(json!({"schema":1,"requester_id":owner,"provider":provider,
                "allow_continue":allowed,"version":version,"source_count":sources.len(),
                "previews":sources.iter().take(3).map(|s| json!({"sender_name":s["sender_name"],
                    "text":s["content"].as_str().unwrap_or("").chars().take(90).collect::<String>()})).collect::<Vec<_>>() }));
        }
    }
    Ok(())
}

pub(super) struct Context {
    pub owner: String,
    pub question: String,
    pub answer: String,
    pub created: String,
    pub sources: Vec<Value>,
    pub allowed: bool,
    pub version: i64,
    pub provider: String,
}

pub(super) fn read(conn: &Connection, user: &str, group: &str, message: &str) -> Result<Context> {
    ensure_member_and_source(conn, user, group, message)?;
    let row = conn
        .query_row(
            "SELECT r.requester_id,r.selection_question,m.content,m.created_at,
        c.sources_json,c.allow_continue,c.version,r.web_provider FROM group_ai_reply_requests r
        JOIN group_ai_reply_contexts c ON c.request_id=r.id
        JOIN friend_group_messages m ON m.id=r.result_message_id AND m.group_id=r.group_id
        WHERE r.group_id=?1 AND r.result_message_id=?2 AND r.state='completed'",
            params![group, message],
            |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, String>(3)?,
                    r.get::<_, String>(4)?,
                    r.get::<_, bool>(5)?,
                    r.get::<_, i64>(6)?,
                    r.get::<_, String>(7)?,
                ))
            },
        )
        .optional()?;
    let Some(row) = row else {
        return legacy::read(conn, user, group, message);
    };
    let mut sources: Vec<Value> = serde_json::from_str(&row.4)?;
    // A recalled source must not be resurrected by the frozen copy; edits keep their original evidence.
    for source in &mut sources {
        let visible: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM friend_group_messages
            WHERE id=?1 AND group_id=?2 AND recalled_at IS NULL)",
            params![source["id"].as_str(), group],
            |r| r.get(0),
        )?;
        if !visible {
            source["content"] = json!("这条来源消息已撤回");
            source["attachments"] = json!([]);
            source["recalled_at"] = json!("recalled");
        }
    }
    Ok(Context {
        owner: row.0,
        question: row.1,
        answer: row.2,
        created: row.3,
        sources,
        allowed: row.5,
        version: row.6,
        provider: row.7,
    })
}

impl Store {
    pub(crate) fn group_ai_reply_sources(
        &self,
        user: &str,
        group: &str,
        message: &str,
    ) -> Result<Value> {
        let conn = self.conn()?;
        let context = read(&conn, user, group, message)?;
        Ok(
            json!({"schema":1,"group_id":group,"message_id":message,"requester_id":context.owner,
            "allow_continue":context.allowed,"version":context.version,"provider":context.provider,
            "sources":context.sources}),
        )
    }

    pub(crate) fn set_group_ai_continuation(
        &self,
        user: &str,
        group: &str,
        message: &str,
        allowed: bool,
        version: i64,
    ) -> Result<Value> {
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let context = read(&tx, user, group, message)?;
        ensure!(context.owner == user, "只有发起人可以修改分享权限");
        ensure!(
            context.version > 0,
            "旧版回答没有来源快照，请重新选择消息后分析"
        );
        ensure!(context.version == version, "分享设置已变化，请重新打开");
        ensure!(
            context.provider == "chatgpt_web",
            "此回答不支持 ChatGPT 续聊"
        );
        tx.execute("UPDATE group_ai_reply_contexts SET allow_continue=?1,version=version+1 WHERE request_id=
            (SELECT id FROM group_ai_reply_requests WHERE group_id=?2 AND result_message_id=?3)",params![allowed,group,message])?;
        tx.commit()?;
        self.group_ai_reply_sources(user, group, message)
    }
}
