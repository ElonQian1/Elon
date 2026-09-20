//! A share draft contains the explicit selection and its answer, never hidden recent context.
use super::{ensure_member_and_source, selection, Store};
use anyhow::{ensure, Result};
use rusqlite::params;
use serde_json::{json, Value};

impl Store {
    pub(crate) fn group_ai_context_share_draft(
        &self,
        user: &str,
        group: &str,
        message: &str,
    ) -> Result<Value> {
        let mut conn = self.conn()?;
        let tx = conn.transaction()?;
        ensure_member_and_source(&tx, user, group, message)?;
        let (request, question, answer, created): (String, String, String, String) = tx.query_row(
            "SELECT r.id,r.selection_question,m.content,m.created_at FROM group_ai_reply_requests r
             JOIN friend_group_messages m ON m.id=r.result_message_id AND m.group_id=r.group_id
             WHERE r.group_id=?1 AND r.result_message_id=?2 AND r.requester_id=?3
               AND r.state='completed' AND r.context_scope='selected' AND r.web_provider='chatgpt_web'",
            params![group,message,user], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?)),
        )?;
        selection::validate_sources(&tx, user, group, &request)?;
        let mut stmt = tx.prepare(
            "SELECT m.id,COALESCE(u.nickname,u.email,u.id),m.content,m.created_at
             FROM group_ai_selected_sources s JOIN friend_group_messages m ON m.id=s.message_id
             JOIN users u ON u.id=m.sender_user_id WHERE s.request_id=?1 AND m.group_id=?2 ORDER BY m.rowid",
        )?;
        let mut messages: Vec<Value> = stmt
            .query_map(params![request, group], |r| {
                let id: String = r.get(0)?;
                let speaker: String = r.get(1)?;
                let text: String = r.get(2)?;
                let created: String = r.get(3)?;
                Ok(row(&id, "user", format!("{speaker}:\n{text}"), &created))
            })?
            .collect::<rusqlite::Result<_>>()?;
        ensure!(!messages.is_empty(), "选区记录不可用");
        if !question.is_empty() {
            messages.push(row("selection_question", "user", question, &created));
        }
        let summary: String = answer.chars().take(240).collect();
        messages.push(row(message, "assistant", answer, &created));
        let value = json!({"snapshot_id":"preview", "group_id":group, "owner_id":user,
            "owner_name":"群聊成员", "document":{"schema":"elon.ai_conversation_share.v1",
            "provider":"chatgpt", "title":"群聊 AI 精选讨论", "summary":summary, "messages":messages}});
        drop(stmt);
        tx.commit()?;
        Ok(value)
    }
}

fn row(id: &str, role: &str, content: String, created: &str) -> Value {
    let time = chrono::DateTime::parse_from_rfc3339(created)
        .map(|v| v.timestamp_millis())
        .unwrap_or(0);
    json!({"id":id,"role":role,"content":content,"created_at_ms":time.max(0),"gap_before":false,"parts":[]})
}
