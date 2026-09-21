//! Preserve the previous owner-only export for pre-snapshot replies, without inventing old rich data.
use super::Context;
use anyhow::{ensure, Result};
use rusqlite::{params, Connection};
use serde_json::json;

pub(super) fn read(conn: &Connection, user: &str, group: &str, message: &str) -> Result<Context> {
    let (request, question, answer, created): (String, String, String, String) = conn.query_row(
        "SELECT r.id,r.selection_question,m.content,m.created_at FROM group_ai_reply_requests r
         JOIN friend_group_messages m ON m.id=r.result_message_id AND m.group_id=r.group_id
         WHERE r.group_id=?1 AND r.result_message_id=?2 AND r.requester_id=?3
         AND r.state='completed' AND r.context_scope='selected' AND r.web_provider='chatgpt_web'",
        params![group, message, user],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
    )?;
    super::super::selection::validate_sources(conn, user, group, &request)?;
    let mut stmt=conn.prepare("SELECT m.id,m.sender_user_id,COALESCE(u.nickname,u.email,u.id),m.content,m.created_at
        FROM group_ai_selected_sources s JOIN friend_group_messages m ON m.id=s.message_id
        JOIN users u ON u.id=m.sender_user_id WHERE s.request_id=?1 AND m.group_id=?2 ORDER BY m.rowid")?;
    let sources=stmt.query_map(params![request,group],|r| Ok(json!({"id":r.get::<_,String>(0)?,
        "sender_user_id":r.get::<_,String>(1)?,"sender_name":r.get::<_,String>(2)?,"content":r.get::<_,String>(3)?,
        "created_at":r.get::<_,String>(4)?,"attachments":[]})))?.collect::<rusqlite::Result<Vec<_>>>()?;
    ensure!(!sources.is_empty(), "选区记录不可用");
    Ok(Context {
        owner: user.into(),
        question,
        answer,
        created,
        sources,
        allowed: false,
        version: 0,
        provider: "chatgpt_web".into(),
    })
}
