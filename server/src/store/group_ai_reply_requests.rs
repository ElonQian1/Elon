//! Durable ownership shared by group mention and selected-message AI replies.

use anyhow::{anyhow, Result};
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};

use super::super::{new_id, now, FriendGroupMessage, Store, SOCIAL_AI_USER_ID};
use super::{ensure_social_ai_user, SOCIAL_AI_DISPLAY_NAME};
#[path = "group_ai_source_access.rs"]
mod source;
use source::ensure_member_and_source;
#[path = "group_web_ai_attachments.rs"]
pub(crate) mod attachments;
#[path = "group_ai_reply_context.rs"]
pub(crate) mod context;
#[path = "group_ai_context_share.rs"]
mod context_share;
#[path = "group_web_ai_provider.rs"]
pub(crate) mod provider;
#[path = "group_web_ai_selection.rs"]
pub(crate) mod selection;
#[path = "group_ai_selection_schema.rs"]
pub(crate) mod selection_schema;
#[path = "group_web_ai_requests.rs"]
pub(crate) mod web;
#[path = "group_work_ai_options.rs"]
pub(crate) mod work;

pub(crate) fn migrate(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS group_ai_reply_requests (
            id TEXT PRIMARY KEY,
            group_id TEXT NOT NULL REFERENCES friend_groups(id) ON DELETE CASCADE,
            trigger_message_id TEXT NOT NULL,
            requester_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            state TEXT NOT NULL CHECK(state IN ('dispatched', 'completed', 'indeterminate')),
            result_message_id TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            UNIQUE(group_id, trigger_message_id),
            CHECK((state = 'completed') = (result_message_id IS NOT NULL))
        );",
    )?;
    Ok(())
}

impl Store {
    /// None means this source already has an owner, including an uncertain write.
    pub(crate) fn claim_group_ai_reply(
        &self,
        user_id: &str,
        group_id: &str,
        trigger_message_id: &str,
    ) -> Result<Option<String>> {
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        ensure_member_and_source(&tx, user_id, group_id, trigger_message_id)?;
        let id = new_id("gaireq");
        let inserted = tx.execute(
            "INSERT INTO group_ai_reply_requests
                (id, group_id, trigger_message_id, requester_id, state, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, 'dispatched', ?5, ?5)
             ON CONFLICT DO NOTHING",
            params![id, group_id, trigger_message_id, user_id, now()],
        )?;
        let fallback = if inserted == 0 {
            tx.query_row("SELECT id FROM group_ai_reply_requests WHERE group_id=?1 AND trigger_message_id=?2 AND requester_id=?3 AND state='server_ready' AND engine='server_api' AND context_scope='recent'",
                params![group_id,trigger_message_id,user_id], |r| r.get::<_,String>(0)).optional()?
        } else {
            None
        };
        if let Some(ref existing) = fallback {
            tx.execute(
                "UPDATE group_ai_reply_requests SET state='dispatched',updated_at=?1 WHERE id=?2",
                params![now(), existing],
            )?;
        }
        tx.commit()?;
        Ok(if inserted == 1 { Some(id) } else { fallback })
    }

    /// Inserting the shared message and completing the request are one transaction.
    pub(crate) fn complete_group_ai_reply(
        &self,
        user_id: &str,
        request_id: &str,
        content: &str,
    ) -> Result<FriendGroupMessage> {
        let content = content.trim();
        if content.is_empty() || content.chars().count() > 20_000 {
            return Err(anyhow!("invalid group AI reply length"));
        }
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let (group_id, trigger, state, result_id): (String, String, String, Option<String>) = tx
            .query_row(
                "SELECT group_id, trigger_message_id, state, result_message_id
                 FROM group_ai_reply_requests WHERE id = ?1 AND requester_id = ?2",
                params![request_id, user_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )?;
        ensure_member_and_source(&tx, user_id, &group_id, &trigger)?;
        selection::validate_sources(&tx, user_id, &group_id, request_id)?;
        let (id, created_at) = if let Some(id) = result_id {
            let (stored, created): (String, String) = tx.query_row(
                "SELECT content, created_at FROM friend_group_messages WHERE id = ?1 AND group_id = ?2",
                params![id, group_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )?;
            if stored != content {
                return Err(anyhow!(
                    "group AI result already committed with different content"
                ));
            }
            (id, created)
        } else {
            // A late response may reconcile an indeterminate attempt; never dispatch it again.
            if state != "dispatched" && state != "indeterminate" {
                return Err(anyhow!("group AI request cannot accept a result"));
            }
            ensure_social_ai_user(&tx)?;
            let id = new_id("gai");
            let created_at = now();
            tx.execute(
                "INSERT INTO friend_group_messages (id, group_id, sender_user_id, content, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![id, group_id, SOCIAL_AI_USER_ID, content, created_at],
            )?;
            tx.execute(
                "UPDATE friend_groups SET updated_at = ?1 WHERE id = ?2",
                params![created_at, group_id],
            )?;
            tx.execute(
                "UPDATE group_ai_reply_requests SET state = 'completed', result_message_id = ?1,
                    updated_at = ?2 WHERE id = ?3",
                params![id, created_at, request_id],
            )?;
            (id, created_at)
        };
        let mut message = FriendGroupMessage {
            id,
            group_id,
            sender_user_id: SOCIAL_AI_USER_ID.to_owned(),
            sender_name: SOCIAL_AI_DISPLAY_NAME.to_owned(),
            content: content.to_owned(),
            attachments: Vec::new(),
            created_at,
            outgoing: false,
            recalled_at: None,
            recalled_by: None,
            revision: 1,
            edited_at: None,
            ai_reply: None,
        };
        context::decorate(&tx, std::slice::from_mut(&mut message))?;
        tx.commit()?;
        Ok(message)
    }

    pub(crate) fn mark_group_ai_reply_indeterminate(&self, request_id: &str) -> Result<()> {
        self.conn()?.execute(
            "UPDATE group_ai_reply_requests SET state = 'indeterminate', updated_at = ?1
             WHERE id = ?2 AND state = 'dispatched'",
            params![now(), request_id],
        )?;
        Ok(())
    }
}

#[cfg(test)]
#[path = "group_ai_reply_requests_tests.rs"]
mod tests;
