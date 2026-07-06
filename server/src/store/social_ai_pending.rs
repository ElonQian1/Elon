use anyhow::{anyhow, Result};
use rusqlite::{params, Connection, OptionalExtension};

use super::{Store, SOCIAL_AI_USER_ID};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SocialAiPendingMention {
    pub trigger_message_id: String,
    pub trigger_content: String,
}

impl Store {
    pub(crate) fn latest_unanswered_friend_social_ai_mention(
        &self,
        user_id: &str,
        friend_id: &str,
    ) -> Result<Option<SocialAiPendingMention>> {
        if user_id == friend_id {
            return Err(anyhow!("invalid friend pair"));
        }
        let conn = self.conn()?;
        ensure_friend_pair(&conn, user_id, friend_id)?;
        conn.query_row(
            "SELECT candidate.id, candidate.content
             FROM friend_messages candidate
             WHERE candidate.sender_user_id = ?1
               AND candidate.receiver_user_id = ?2
               AND LOWER(REPLACE(candidate.content, '＠', '@')) LIKE '%@el%'
               AND NOT EXISTS (
                   SELECT 1
                   FROM friend_messages ai
                   WHERE ai.sender_user_id = ?3
                     AND ai.receiver_user_id = ?1
                     AND ai.context_user_id = ?2
                     AND ai.created_at > candidate.created_at
                   LIMIT 1
               )
             ORDER BY candidate.created_at DESC
             LIMIT 1",
            params![user_id, friend_id, SOCIAL_AI_USER_ID],
            |row| {
                Ok(SocialAiPendingMention {
                    trigger_message_id: row.get(0)?,
                    trigger_content: row.get(1)?,
                })
            },
        )
        .optional()
        .map_err(Into::into)
    }

    pub(crate) fn latest_unanswered_group_social_ai_mention(
        &self,
        user_id: &str,
        group_id: &str,
    ) -> Result<Option<SocialAiPendingMention>> {
        let conn = self.conn()?;
        ensure_group_member(&conn, user_id, group_id)?;
        conn.query_row(
            "SELECT candidate.id, candidate.content
             FROM friend_group_messages candidate
             WHERE candidate.group_id = ?1
               AND candidate.sender_user_id != ?2
               AND LOWER(REPLACE(candidate.content, '＠', '@')) LIKE '%@el%'
               AND NOT EXISTS (
                   SELECT 1
                   FROM friend_group_messages ai
                   WHERE ai.group_id = ?1
                     AND ai.sender_user_id = ?2
                     AND ai.created_at > candidate.created_at
                   LIMIT 1
               )
             ORDER BY candidate.created_at DESC
             LIMIT 1",
            params![group_id, SOCIAL_AI_USER_ID],
            |row| {
                Ok(SocialAiPendingMention {
                    trigger_message_id: row.get(0)?,
                    trigger_content: row.get(1)?,
                })
            },
        )
        .optional()
        .map_err(Into::into)
    }
}

fn ensure_friend_pair(conn: &Connection, user_id: &str, friend_id: &str) -> Result<()> {
    let exists = conn
        .query_row(
            "SELECT 1
             FROM user_friends
             WHERE user_id = ?1 AND friend_user_id = ?2
             LIMIT 1",
            params![user_id, friend_id],
            |_| Ok(()),
        )
        .optional()?
        .is_some();
    if exists {
        Ok(())
    } else {
        Err(anyhow!("friend pair does not exist"))
    }
}

fn ensure_group_member(conn: &Connection, user_id: &str, group_id: &str) -> Result<()> {
    let exists = conn
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
        Err(anyhow!("user is not in friend group"))
    }
}


#[cfg(test)]
#[path = "social_ai_pending_tests.rs"]
mod tests;
