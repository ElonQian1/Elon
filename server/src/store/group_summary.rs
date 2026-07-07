use anyhow::{anyhow, Result};
use rusqlite::{params, OptionalExtension};
use std::collections::BTreeSet;

use crate::group_chat_project_docs::default_group_chat_docs;

use super::{
    new_id, now, GroupAiDocument, GroupChatRetrievalHit, GroupChatRetrievalInput,
    GroupChatRetrievalResult, GroupSummaryContextPack, GroupSummaryCreateInput, GroupSummaryPost,
    GroupSummaryPostDetail, GroupSummarySourceMessage, Store,
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

    pub fn search_group_chat_messages(
        &self,
        user_id: &str,
        group_id: &str,
        input: &GroupChatRetrievalInput,
    ) -> Result<GroupChatRetrievalResult> {
        self.ensure_summary_group_member(user_id, group_id)?;
        let mut strategy = Vec::new();
        let query = clean_search_text(input.query.as_deref());
        let sender = clean_search_text(input.sender.as_deref());
        let limit = input.limit.clamp(1, 100);
        if !input.message_ids.is_empty() {
            strategy.push("exact_message_ids".to_string());
            let messages = self.group_summary_messages_by_id(group_id, &input.message_ids)?;
            return Ok(GroupChatRetrievalResult {
                strategy: with_vector_status(strategy),
                vector_status: group_chat_vector_status(),
                hits: messages
                    .into_iter()
                    .map(|message| GroupChatRetrievalHit {
                        message,
                        score: 100,
                        match_reasons: vec!["exact_message_id".to_string()],
                    })
                    .take(limit as usize)
                    .collect(),
            });
        }
        if query.is_some() {
            strategy.push("keyword_full_text".to_string());
        }
        if sender.is_some() {
            strategy.push("sender_filter".to_string());
        }
        if input.start_at.is_some() || input.end_at.is_some() {
            strategy.push("time_window".to_string());
        }
        if strategy.is_empty() {
            strategy.push("recent_messages".to_string());
        }
        let candidate_limit = if query.is_some() {
            (limit * 12).clamp(limit, 1000)
        } else {
            limit
        };
        let sender_like = sender.as_ref().map(|value| format!("%{}%", value));
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
               AND (?4 IS NULL
                    OR m.sender_user_id = ?4
                    OR COALESCE(u.nickname, u.email, u.phone, m.sender_user_id) LIKE ?5)
             ORDER BY m.created_at DESC
             LIMIT ?6",
        )?;
        let mut messages = stmt
            .query_map(
                params![
                    group_id,
                    input.start_at.as_deref(),
                    input.end_at.as_deref(),
                    sender.as_deref(),
                    sender_like.as_deref(),
                    candidate_limit
                ],
                group_summary_source_from_row,
            )?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        messages.reverse();
        let terms = search_terms(query.as_deref());
        let mut hits = messages
            .into_iter()
            .filter_map(|message| {
                let (score, reasons, keyword_matched) =
                    score_group_chat_message(&message, &terms, sender.as_deref());
                if query.is_some() && !keyword_matched {
                    return None;
                }
                Some(GroupChatRetrievalHit {
                    message,
                    score,
                    match_reasons: reasons,
                })
            })
            .collect::<Vec<_>>();
        hits.sort_by(|left, right| {
            right
                .score
                .cmp(&left.score)
                .then_with(|| right.message.created_at.cmp(&left.message.created_at))
        });
        hits.truncate(limit as usize);
        Ok(GroupChatRetrievalResult {
            strategy: with_vector_status(strategy),
            vector_status: group_chat_vector_status(),
            hits,
        })
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
            conn.execute(
                "UPDATE friend_group_ai_documents
                    SET title = ?1, content = ?2, position = ?3, updated_at = ?4
                  WHERE group_id = ?5 AND path = ?6 AND updated_by IS NULL",
                params![
                    doc.title,
                    doc.content.trim(),
                    position as i64,
                    now,
                    group_id,
                    doc.path
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


mod helpers;
use self::helpers::*;
