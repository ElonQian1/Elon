use anyhow::{anyhow, Result};
use rusqlite::{params, OptionalExtension};
use std::collections::BTreeSet;

use crate::group_chat_project_docs::default_group_chat_docs;

use super::{
    new_id, now, GroupAiDocument, GroupSummaryContextPack, GroupSummaryCreateInput,
    GroupSummaryPost, GroupSummaryPostDetail, GroupSummarySourceMessage, Store,
};

impl Store {
    pub fn list_group_ai_documents(
        &self,
        user_id: &str,
        group_id: &str,
    ) -> Result<Vec<GroupAiDocument>> {
        self.ensure_summary_group_member(user_id, group_id)?;
        self.ensure_group_ai_documents(group_id)?;
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            "SELECT d.group_id, d.path, d.title, d.content, d.updated_by,
                    COALESCE(u.nickname, u.email, u.phone, d.updated_by) AS updated_by_name,
                    d.updated_at
             FROM friend_group_ai_documents d
             LEFT JOIN users u ON u.id = d.updated_by
             WHERE d.group_id = ?1
             ORDER BY d.position, d.path",
        )?;
        let docs = stmt
            .query_map(params![group_id], group_ai_document_from_row)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(docs)
    }

    pub fn update_group_ai_document(
        &self,
        user_id: &str,
        group_id: &str,
        path: &str,
        content: &str,
    ) -> Result<GroupAiDocument> {
        self.ensure_summary_group_member(user_id, group_id)?;
        self.ensure_group_ai_documents(group_id)?;
        let path = clean_doc_path(path)?;
        let content = content.trim();
        if content.is_empty() {
            return Err(anyhow!("文档内容不能为空"));
        }
        if content.chars().count() > 20_000 {
            return Err(anyhow!("文档内容不能超过 20000 字"));
        }
        let doc = default_group_chat_docs()
            .iter()
            .find(|doc| doc.path == path)
            .ok_or_else(|| anyhow!("只能编辑群聊项目默认 AI 文档"))?;
        let updated_at = now();
        let conn = self.conn()?;
        conn.execute(
            "UPDATE friend_group_ai_documents
                SET title = ?1, content = ?2, updated_by = ?3, updated_at = ?4
              WHERE group_id = ?5 AND path = ?6",
            params![doc.title, content, user_id, updated_at, group_id, path],
        )?;
        load_group_ai_document(&conn, group_id, path)
    }

    pub fn group_summary_messages_for_context(
        &self,
        user_id: &str,
        group_id: &str,
        input: &GroupSummaryCreateInput,
    ) -> Result<Vec<GroupSummarySourceMessage>> {
        self.ensure_summary_group_member(user_id, group_id)?;
        if !input.message_ids.is_empty() {
            return self.group_summary_messages_by_id(group_id, &input.message_ids);
        }
        let limit = input.limit.clamp(2, 200);
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            "SELECT m.id, m.group_id, m.sender_user_id,
                    COALESCE(u.nickname, u.email, u.phone, m.sender_user_id) AS sender_name,
                    m.content, m.created_at
             FROM friend_group_messages m
             JOIN users u ON u.id = m.sender_user_id
             WHERE m.group_id = ?1
               AND (?2 IS NULL OR m.created_at >= ?2)
               AND (?3 IS NULL OR m.created_at <= ?3)
             ORDER BY m.created_at DESC
             LIMIT ?4",
        )?;
        let mut messages = stmt
            .query_map(
                params![
                    group_id,
                    input.start_at.as_deref(),
                    input.end_at.as_deref(),
                    limit
                ],
                group_summary_source_from_row,
            )?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        messages.reverse();
        if messages.is_empty() {
            return Err(anyhow!("没有找到可总结的群聊消息"));
        }
        Ok(messages)
    }

    pub fn create_group_summary_post_draft(
        &self,
        user_id: &str,
        group_id: &str,
        input: &GroupSummaryCreateInput,
        messages: &[GroupSummarySourceMessage],
        context_payload_json: &str,
    ) -> Result<GroupSummaryPostDetail> {
        self.ensure_summary_group_member(user_id, group_id)?;
        if messages.is_empty() {
            return Err(anyhow!("没有可绑定的源消息"));
        }
        let title = clean_title(input.title.as_deref(), messages);
        let topic = input
            .topic
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| value.chars().take(120).collect::<String>());
        let context_pack_id = new_id("gcp");
        let post_id = new_id("gsp");
        let created_at = now();
        let source_start_at = messages.first().map(|message| message.created_at.clone());
        let source_end_at = messages.last().map(|message| message.created_at.clone());
        let query = input
            .instructions
            .as_deref()
            .or(input.topic.as_deref())
            .or(input.title.as_deref())
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string);
        let pinned_at = input.pin.then_some(created_at.clone());
        let pinned_by = input.pin.then_some(user_id.to_string());
        let conn = self.conn()?;
        let tx = conn.unchecked_transaction()?;
        tx.execute(
            "INSERT INTO friend_group_summary_context_packs (
                id, group_id, purpose, query, payload_json,
                source_start_at, source_end_at, message_count, created_by, created_at
             )
             VALUES (?1, ?2, 'summary_post', ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                context_pack_id,
                group_id,
                query,
                context_payload_json,
                source_start_at,
                source_end_at,
                messages.len() as i64,
                user_id,
                created_at
            ],
        )?;
        tx.execute(
            "INSERT INTO friend_group_summary_posts (
                id, group_id, title, topic, summary, status, context_pack_id,
                source_start_at, source_end_at, source_message_count,
                pinned_at, pinned_by, created_by, created_at, updated_at
             )
             VALUES (?1, ?2, ?3, ?4, ?5, 'generating', ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?13)",
            params![
                post_id,
                group_id,
                title,
                topic,
                "AI 正在根据群聊 Context Pack 生成总结帖...",
                context_pack_id,
                source_start_at,
                source_end_at,
                messages.len() as i64,
                pinned_at,
                pinned_by,
                user_id,
                created_at
            ],
        )?;
        for (position, message) in messages.iter().enumerate() {
            tx.execute(
                "INSERT INTO friend_group_summary_post_sources (
                    post_id, message_id, position, excerpt
                 )
                 VALUES (?1, ?2, ?3, ?4)",
                params![
                    post_id,
                    message.id,
                    position as i64,
                    message.content.chars().take(500).collect::<String>()
                ],
            )?;
        }
        tx.commit()?;
        drop(conn);
        self.group_summary_post_detail(user_id, group_id, &post_id)
    }

    pub fn list_group_summary_posts(
        &self,
        user_id: &str,
        group_id: &str,
        limit: i64,
    ) -> Result<Vec<GroupSummaryPost>> {
        self.ensure_summary_group_member(user_id, group_id)?;
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
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
             WHERE p.group_id = ?1
             ORDER BY (p.pinned_at IS NULL), p.pinned_at DESC, p.updated_at DESC
             LIMIT ?2",
        )?;
        let posts = stmt
            .query_map(
                params![group_id, limit.clamp(1, 100)],
                group_summary_post_from_row,
            )?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(posts)
    }

    pub fn group_summary_post_detail(
        &self,
        user_id: &str,
        group_id: &str,
        post_id: &str,
    ) -> Result<GroupSummaryPostDetail> {
        self.ensure_summary_group_member(user_id, group_id)?;
        let conn = self.conn()?;
        let post = load_group_summary_post(&conn, group_id, post_id)?;
        let context_pack = load_group_summary_context_pack(&conn, group_id, &post.context_pack_id)?;
        let sources = load_group_summary_sources(&conn, group_id, post_id)?;
        Ok(GroupSummaryPostDetail {
            post,
            context_pack,
            sources,
        })
    }

    pub fn update_group_summary_post_result(
        &self,
        group_id: &str,
        post_id: &str,
        summary: &str,
        status: &str,
        model_used: Option<&str>,
        error: Option<&str>,
    ) -> Result<()> {
        let summary = summary.trim();
        if summary.is_empty() {
            return Err(anyhow!("总结内容不能为空"));
        }
        let updated = self.conn()?.execute(
            "UPDATE friend_group_summary_posts
                SET summary = ?1, status = ?2, model_used = ?3, error = ?4, updated_at = ?5
              WHERE group_id = ?6 AND id = ?7",
            params![summary, status, model_used, error, now(), group_id, post_id],
        )?;
        if updated == 0 {
            return Err(anyhow!("总结帖不存在"));
        }
        Ok(())
    }

    pub fn edit_group_summary_post(
        &self,
        user_id: &str,
        group_id: &str,
        post_id: &str,
        title: Option<&str>,
        summary: Option<&str>,
        pinned: Option<bool>,
    ) -> Result<GroupSummaryPostDetail> {
        self.ensure_summary_group_member(user_id, group_id)?;
        let mut changed = false;
        let conn = self.conn()?;
        if let Some(title) = title.map(str::trim).filter(|value| !value.is_empty()) {
            conn.execute(
                "UPDATE friend_group_summary_posts
                    SET title = ?1, updated_at = ?2
                  WHERE group_id = ?3 AND id = ?4",
                params![
                    title.chars().take(120).collect::<String>(),
                    now(),
                    group_id,
                    post_id
                ],
            )?;
            changed = true;
        }
        if let Some(summary) = summary.map(str::trim).filter(|value| !value.is_empty()) {
            conn.execute(
                "UPDATE friend_group_summary_posts
                    SET summary = ?1, status = 'edited', updated_at = ?2
                  WHERE group_id = ?3 AND id = ?4",
                params![summary, now(), group_id, post_id],
            )?;
            changed = true;
        }
        if let Some(pinned) = pinned {
            if pinned {
                conn.execute(
                    "UPDATE friend_group_summary_posts
                        SET pinned_at = ?1, pinned_by = ?2, updated_at = ?1
                      WHERE group_id = ?3 AND id = ?4",
                    params![now(), user_id, group_id, post_id],
                )?;
            } else {
                conn.execute(
                    "UPDATE friend_group_summary_posts
                        SET pinned_at = NULL, pinned_by = NULL, updated_at = ?1
                      WHERE group_id = ?2 AND id = ?3",
                    params![now(), group_id, post_id],
                )?;
            }
            changed = true;
        }
        if !changed {
            return Err(anyhow!("没有可更新的内容"));
        }
        drop(conn);
        self.group_summary_post_detail(user_id, group_id, post_id)
    }

    fn ensure_group_ai_documents(&self, group_id: &str) -> Result<()> {
        let now = now();
        let conn = self.conn()?;
        for (position, doc) in default_group_chat_docs().iter().enumerate() {
            conn.execute(
                "INSERT OR IGNORE INTO friend_group_ai_documents (
                    group_id, path, title, content, position, updated_by, updated_at
                 )
                 VALUES (?1, ?2, ?3, ?4, ?5, NULL, ?6)",
                params![
                    group_id,
                    doc.path,
                    doc.title,
                    doc.content.trim(),
                    position as i64,
                    now
                ],
            )?;
        }
        Ok(())
    }

    fn ensure_summary_group_member(&self, user_id: &str, group_id: &str) -> Result<()> {
        let exists = self
            .conn()?
            .query_row(
                "SELECT 1
                 FROM friend_group_members
                 WHERE group_id = ?1 AND user_id = ?2
                 LIMIT 1",
                params![group_id, user_id],
                |_| Ok(()),
            )
            .optional()?
            .is_some();
        if exists {
            Ok(())
        } else {
            Err(anyhow!("你不在这个群聊中"))
        }
    }

    fn group_summary_messages_by_id(
        &self,
        group_id: &str,
        message_ids: &[String],
    ) -> Result<Vec<GroupSummarySourceMessage>> {
        let mut seen = BTreeSet::new();
        let mut messages = Vec::new();
        let conn = self.conn()?;
        for message_id in message_ids
            .iter()
            .map(|value| value.trim())
            .filter(|value| !value.is_empty())
        {
            if !seen.insert(message_id.to_string()) {
                continue;
            }
            let message = conn
                .query_row(
                    "SELECT m.id, m.group_id, m.sender_user_id,
                            COALESCE(u.nickname, u.email, u.phone, m.sender_user_id) AS sender_name,
                            m.content, m.created_at
                     FROM friend_group_messages m
                     JOIN users u ON u.id = m.sender_user_id
                     WHERE m.group_id = ?1 AND m.id = ?2",
                    params![group_id, message_id],
                    group_summary_source_from_row,
                )
                .optional()?;
            if let Some(message) = message {
                messages.push(message);
            }
        }
        messages.sort_by(|left, right| left.created_at.cmp(&right.created_at));
        if messages.is_empty() {
            return Err(anyhow!("没有找到可总结的群聊消息"));
        }
        Ok(messages)
    }
}

fn clean_doc_path(path: &str) -> Result<&str> {
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

fn clean_title(input: Option<&str>, messages: &[GroupSummarySourceMessage]) -> String {
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

fn group_ai_document_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<GroupAiDocument> {
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

fn group_summary_source_from_row(
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

fn group_summary_post_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<GroupSummaryPost> {
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

fn group_summary_context_pack_from_row(
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

fn load_group_ai_document(
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

fn load_group_summary_post(
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

fn load_group_summary_context_pack(
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

fn load_group_summary_sources(
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
