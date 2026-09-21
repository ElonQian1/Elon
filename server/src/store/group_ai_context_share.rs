//! A share draft contains the explicit selection and its answer, never hidden recent context.
use super::{context, Store};
use anyhow::{ensure, Result};
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
        let context = context::read(&tx, user, group, message)?;
        ensure!(
            context.owner == user || context.allowed,
            "发起人尚未开放继续讨论"
        );
        ensure!(
            context.provider == "chatgpt_web",
            "此回答不支持 ChatGPT 续聊"
        );
        let mut messages: Vec<Value> = context
            .sources
            .iter()
            .map(|source| {
                let mut text = format!(
                    "{}:\n{}",
                    source["sender_name"].as_str().unwrap_or("群成员"),
                    source["content"].as_str().unwrap_or("")
                );
                if source["attachments"]
                    .as_array()
                    .is_some_and(|a| !a.is_empty())
                {
                    text.push_str("\n[原消息包含附件；本次继续讨论不自动上传附件原文件]");
                }
                row(
                    source["id"].as_str().unwrap_or("source"),
                    "user",
                    text,
                    source["created_at"].as_str().unwrap_or(""),
                )
            })
            .collect();
        ensure!(!messages.is_empty(), "选区记录不可用");
        if !context.question.is_empty() {
            messages.push(row(
                "selection_question",
                "user",
                context.question,
                &context.created,
            ));
        }
        let summary: String = context.answer.chars().take(240).collect();
        messages.push(row(message, "assistant", context.answer, &context.created));
        let value = json!({"snapshot_id":"preview", "group_id":group, "owner_id":context.owner,
            "owner_name":"群聊成员", "document":{"schema":"elon.ai_conversation_share.v1",
            "provider":"chatgpt", "title":"群聊 AI 精选讨论", "summary":summary, "messages":messages}});
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
