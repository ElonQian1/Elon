use anyhow::{anyhow, Result};
use rusqlite::{params, OptionalExtension};
use serde_json::Value;

use super::super::store_types::*;
use super::super::store_types_project::*;
use super::super::common::{now, new_id};

pub(super) fn clean_doc_path(path: &str) -> Result<&str> {
    let path = path.trim();
    if path.is_empty()
        || path.starts_with('/')
        || path.starts_with('\\')
        || path.contains("..")
        || path.contains('\\')
    {
        return Err(anyhow!("文档路径无效"));
    }
    Ok(path)
}

pub(super) fn clean_search_text(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.chars().take(120).collect())
}

pub(super) fn search_terms(query: Option<&str>) -> Vec<String> {
    query
        .into_iter()
        .flat_map(|value| value.split_whitespace())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .take(8)
        .map(|value| value.to_lowercase())
        .collect()
}

pub(super) fn score_group_chat_message(
    message: &GroupSummarySourceMessage,
    terms: &[String],
    sender: Option<&str>,
) -> (i64, Vec<String>, bool) {
    let content = message.content.to_lowercase();
    let sender_name = message.sender_name.to_lowercase();
    let sender_user_id = message.sender_user_id.to_lowercase();
    let mut score = 1;
    let mut reasons = Vec::new();
    let mut keyword_matched = terms.is_empty();
    for term in terms {
        if content.contains(term) {
            score += 20;
            keyword_matched = true;
            reasons.push(format!("content_keyword:{}", truncate_reason(term)));
        }
        if sender_name.contains(term) || sender_user_id.contains(term) {
            score += 8;
            keyword_matched = true;
            reasons.push(format!("sender_keyword:{}", truncate_reason(term)));
        }
    }
    if let Some(sender) = sender {
        let sender = sender.to_lowercase();
        if message.sender_user_id.eq_ignore_ascii_case(&sender) || sender_name.contains(&sender) {
            score += 15;
            reasons.push("sender_filter".to_string());
        }
    }
    if reasons.is_empty() {
        reasons.push("recent_message".to_string());
    }
    (score, reasons, keyword_matched)
}

pub(super) fn truncate_reason(value: &str) -> String {
    value.chars().take(24).collect()
}

pub(super) fn with_vector_status(mut strategy: Vec<String>) -> Vec<String> {
    strategy.push("group_ai_documents".to_string());
    strategy.push("vector_embedding_pending".to_string());
    strategy
}

pub(super) fn group_chat_vector_status() -> String {
    "pending_group_chat_embedding_index".to_string()
}

pub(super) fn clean_title(input: Option<&str>, messages: &[GroupSummarySourceMessage]) -> String {
    if let Some(title) = input.map(str::trim).filter(|value| !value.is_empty()) {
        return title.chars().take(120).collect();
    }
    let first = messages
        .first()
        .map(|message| message.content.trim())
        .unwrap_or("群聊总结");
    let first_line = first.lines().next().unwrap_or("群聊总结").trim();
    let title: String = first_line.chars().take(28).collect();
    if title.is_empty() {
        "群聊总结".to_string()
    } else {
        format!("群聊总结：{}", title)
    }
}

pub(super) fn group_ai_document_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<GroupAiDocument> {
    Ok(GroupAiDocument {
        group_id: row.get(0)?,
        path: row.get(1)?,
        title: row.get(2)?,
        content: row.get(3)?,
        updated_by: row.get(4)?,
        updated_by_name: row.get(5)?,
        updated_at: row.get(6)?,
    })
}

pub(super) fn group_summary_source_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<GroupSummarySourceMessage> {
    Ok(GroupSummarySourceMessage {
        id: row.get(0)?,
        group_id: row.get(1)?,
        sender_user_id: row.get(2)?,
        sender_name: row.get(3)?,
        content: row.get(4)?,
        created_at: row.get(5)?,
    })
}

pub(super) fn group_summary_post_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<GroupSummaryPost> {
    Ok(GroupSummaryPost {
        id: row.get(0)?,
        group_id: row.get(1)?,
        title: row.get(2)?,
        topic: row.get(3)?,
        summary: row.get(4)?,
        status: row.get(5)?,
        context_pack_id: row.get(6)?,
        source_start_at: row.get(7)?,
        source_end_at: row.get(8)?,
        source_message_count: row.get(9)?,
        model_used: row.get(10)?,
        error: row.get(11)?,
        pinned_at: row.get(12)?,
        pinned_by: row.get(13)?,
        pinned_by_name: row.get(14)?,
        created_by: row.get(15)?,
        created_by_name: row.get(16)?,
        created_at: row.get(17)?,
        updated_at: row.get(18)?,
    })
}

pub(super) fn group_summary_context_pack_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<GroupSummaryContextPack> {
    Ok(GroupSummaryContextPack {
        id: row.get(0)?,
        group_id: row.get(1)?,
        purpose: row.get(2)?,
        query: row.get(3)?,
        payload_json: row.get(4)?,
        source_start_at: row.get(5)?,
        source_end_at: row.get(6)?,
        message_count: row.get(7)?,
        created_by: row.get(8)?,
        created_by_name: row.get(9)?,
        created_at: row.get(10)?,
    })
}

pub(super) fn load_group_ai_document(
    conn: &rusqlite::Connection,
    group_id: &str,
    path: &str,
) -> Result<GroupAiDocument> {
    conn.query_row(
        "SELECT d.group_id, d.path, d.title, d.content, d.updated_by,
                COALESCE(u.nickname, u.email, u.phone, d.updated_by) AS updated_by_name,
                d.updated_at
         FROM friend_group_ai_documents d
         LEFT JOIN users u ON u.id = d.updated_by
         WHERE d.group_id = ?1 AND d.path = ?2",
        params![group_id, path],
        group_ai_document_from_row,
    )
    .optional()?
    .ok_or_else(|| anyhow!("文档不存在"))
}

pub(super) fn load_group_summary_post(
    conn: &rusqlite::Connection,
    group_id: &str,
    post_id: &str,
) -> Result<GroupSummaryPost> {
    conn.query_row(
        "SELECT p.id, p.group_id, p.title, p.topic, p.summary, p.status,
                p.context_pack_id, p.source_start_at, p.source_end_at,
                p.source_message_count, p.model_used, p.error,
                p.pinned_at, p.pinned_by,
                COALESCE(pin_user.nickname, pin_user.email, pin_user.phone, p.pinned_by) AS pinned_by_name,
                p.created_by,
                COALESCE(author.nickname, author.email, author.phone, p.created_by) AS created_by_name,
                p.created_at, p.updated_at
         FROM friend_group_summary_posts p
         LEFT JOIN users pin_user ON pin_user.id = p.pinned_by
         LEFT JOIN users author ON author.id = p.created_by
         WHERE p.group_id = ?1 AND p.id = ?2",
        params![group_id, post_id],
        group_summary_post_from_row,
    )
    .optional()?
    .ok_or_else(|| anyhow!("总结帖不存在"))
}

pub(super) fn load_group_summary_context_pack(
    conn: &rusqlite::Connection,
    group_id: &str,
    context_pack_id: &str,
) -> Result<GroupSummaryContextPack> {
    conn.query_row(
        "SELECT c.id, c.group_id, c.purpose, c.query, c.payload_json,
                c.source_start_at, c.source_end_at, c.message_count,
                c.created_by,
                COALESCE(u.nickname, u.email, u.phone, c.created_by) AS created_by_name,
                c.created_at
         FROM friend_group_summary_context_packs c
         LEFT JOIN users u ON u.id = c.created_by
         WHERE c.group_id = ?1 AND c.id = ?2",
        params![group_id, context_pack_id],
        group_summary_context_pack_from_row,
    )
    .optional()?
    .ok_or_else(|| anyhow!("Context Pack 不存在"))
}

pub(super) fn load_group_summary_sources(
    conn: &rusqlite::Connection,
    group_id: &str,
    post_id: &str,
) -> Result<Vec<GroupSummarySourceMessage>> {
    let mut stmt = conn.prepare(
        "SELECT m.id, m.group_id, m.sender_user_id,
                COALESCE(u.nickname, u.email, u.phone, m.sender_user_id) AS sender_name,
                m.content, m.created_at
         FROM friend_group_summary_post_sources s
         JOIN friend_group_messages m ON m.id = s.message_id
         JOIN users u ON u.id = m.sender_user_id
         WHERE s.post_id = ?1 AND m.group_id = ?2
         ORDER BY s.position",
    )?;
    let sources = stmt
        .query_map(params![post_id, group_id], group_summary_source_from_row)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(sources)
}
