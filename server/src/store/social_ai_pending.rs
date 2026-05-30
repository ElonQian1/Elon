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
mod tests {
    use crate::store::Store;
    use uuid::Uuid;

    fn temp_store() -> Store {
        let path = std::env::temp_dir().join(format!(
            "elon_social_ai_pending_test_{}.db",
            Uuid::new_v4().simple()
        ));
        Store::open(&path).expect("store should open")
    }

    #[test]
    fn finds_latest_unanswered_friend_and_group_mentions() {
        let store = temp_store();
        let alice = store
            .create_user("pending-alice@example.com", "secret1", Some("Alice"), None)
            .expect("alice should be created");
        let bob = store
            .create_user("pending-bob@example.com", "secret1", Some("Bob"), None)
            .expect("bob should be created");
        store
            .add_friend(&alice.id, Some("email"), "pending-bob@example.com")
            .expect("alice can add bob");

        store
            .send_friend_message(&alice.id, &bob.id, "how to remove fleas?", None)
            .expect("question should be stored");
        let trigger = store
            .send_friend_message(&alice.id, &bob.id, "@EL", None)
            .expect("mention should be stored");
        let pending = store
            .latest_unanswered_friend_social_ai_mention(&alice.id, &bob.id)
            .expect("pending lookup should work")
            .expect("friend mention should be pending");
        assert_eq!(pending.trigger_message_id, trigger.id);
        store
            .insert_friend_social_ai_reply(&alice.id, &bob.id, "use a flea treatment plan")
            .expect("ai reply should be stored");
        assert!(store
            .latest_unanswered_friend_social_ai_mention(&alice.id, &bob.id)
            .expect("pending lookup should work")
            .is_none());

        let group = store
            .create_friend_group(&alice.id, Some("Pending Test"), &[bob.id.clone()])
            .expect("group should be created");
        let trigger = store
            .send_friend_group_message(&alice.id, &group.id, "＠EL explain this", None)
            .expect("group mention should be stored");
        let pending = store
            .latest_unanswered_group_social_ai_mention(&alice.id, &group.id)
            .expect("pending lookup should work")
            .expect("group mention should be pending");
        assert_eq!(pending.trigger_message_id, trigger.id);
        store
            .insert_group_social_ai_reply(&group.id, "group answer")
            .expect("group ai reply should be stored");
        assert!(store
            .latest_unanswered_group_social_ai_mention(&alice.id, &group.id)
            .expect("pending lookup should work")
            .is_none());
    }
}
