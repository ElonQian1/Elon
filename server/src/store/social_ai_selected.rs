use anyhow::{anyhow, Result};
use rusqlite::{params, Connection, OptionalExtension};

use super::{SocialAiHistoryMessage, Store, SOCIAL_AI_DISPLAY_NAME, SOCIAL_AI_USER_ID};

impl Store {
    pub(crate) fn friend_message_for_social_ai_selection(
        &self,
        user_id: &str,
        friend_id: &str,
        message_id: &str,
    ) -> Result<Option<SocialAiHistoryMessage>> {
        let conn = self.conn()?;
        ensure_friend_pair(&conn, user_id, friend_id)?;
        conn.query_row(
            "SELECT m.sender_user_id,
                    COALESCE(u.nickname, u.email, u.phone, m.sender_user_id) AS sender_name,
                    m.content
             FROM friend_messages m
             LEFT JOIN users u ON u.id = m.sender_user_id
             WHERE m.id = ?3
               AND (
                   (m.sender_user_id = ?1 AND m.receiver_user_id = ?2)
                   OR (m.sender_user_id = ?2 AND m.receiver_user_id = ?1)
                   OR (
                       m.sender_user_id = ?4
                       AND m.receiver_user_id = ?1
                       AND m.context_user_id = ?2
                   )
               )
             LIMIT 1",
            params![user_id, friend_id, message_id, SOCIAL_AI_USER_ID],
            |row| {
                Ok(selected_history_message(
                    user_id,
                    &row.get::<_, String>(0)?,
                    &row.get::<_, String>(1)?,
                    row.get(2)?,
                ))
            },
        )
        .optional()
        .map_err(Into::into)
    }

    pub(crate) fn group_message_for_social_ai_selection(
        &self,
        user_id: &str,
        group_id: &str,
        message_id: &str,
    ) -> Result<Option<SocialAiHistoryMessage>> {
        let conn = self.conn()?;
        ensure_group_member(&conn, user_id, group_id)?;
        conn.query_row(
            "SELECT m.sender_user_id,
                    COALESCE(u.nickname, u.email, u.phone, m.sender_user_id) AS sender_name,
                    m.content
             FROM friend_group_messages m
             LEFT JOIN users u ON u.id = m.sender_user_id
             WHERE m.group_id = ?1 AND m.id = ?2
             LIMIT 1",
            params![group_id, message_id],
            |row| {
                Ok(selected_history_message(
                    user_id,
                    &row.get::<_, String>(0)?,
                    &row.get::<_, String>(1)?,
                    row.get(2)?,
                ))
            },
        )
        .optional()
        .map_err(Into::into)
    }
}

fn selected_history_message(
    request_user_id: &str,
    sender_user_id: &str,
    sender_name: &str,
    content: String,
) -> SocialAiHistoryMessage {
    let from_request_user = sender_user_id == request_user_id;
    let speaker = if sender_user_id == SOCIAL_AI_USER_ID {
        SOCIAL_AI_DISPLAY_NAME.to_string()
    } else if from_request_user {
        "我".to_string()
    } else {
        sender_name.to_string()
    };
    SocialAiHistoryMessage {
        speaker,
        content,
        from_request_user,
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
            "elon_social_ai_selected_test_{}.db",
            Uuid::new_v4().simple()
        ));
        Store::open(&path).expect("store should open")
    }

    #[test]
    fn selected_friend_and_group_messages_are_scoped_to_visible_chat() {
        let store = temp_store();
        let alice = store
            .create_user("selected-alice@example.com", "secret1", Some("Alice"), None)
            .expect("alice should be created");
        let bob = store
            .create_user("selected-bob@example.com", "secret1", Some("Bob"), None)
            .expect("bob should be created");
        let carol = store
            .create_user("selected-carol@example.com", "secret1", Some("Carol"), None)
            .expect("carol should be created");
        store
            .add_friend(&alice.id, Some("email"), "selected-bob@example.com")
            .expect("alice can add bob");

        let friend_message = store
            .send_friend_message(&bob.id, &alice.id, "解释一下这句话", None)
            .expect("friend message should be stored");
        let selected = store
            .friend_message_for_social_ai_selection(&alice.id, &bob.id, &friend_message.id)
            .expect("selection lookup should work")
            .expect("message is visible");
        assert_eq!(selected.content, "解释一下这句话");
        assert!(!selected.from_request_user);
        assert!(store
            .friend_message_for_social_ai_selection(&carol.id, &bob.id, &friend_message.id)
            .is_err());

        let group = store
            .create_friend_group(&alice.id, Some("Selected Test"), &[bob.id.clone()])
            .expect("group should be created");
        let group_message = store
            .send_friend_group_message(&bob.id, &group.id, "群里这条也能选", None)
            .expect("group message should be stored");
        let selected = store
            .group_message_for_social_ai_selection(&alice.id, &group.id, &group_message.id)
            .expect("selection lookup should work")
            .expect("group message is visible");
        assert_eq!(selected.content, "群里这条也能选");
        assert!(!selected.from_request_user);
    }
}
