use anyhow::{anyhow, Result};
use rusqlite::{params, OptionalExtension};

use super::{now, Store, UserPresenceSettings};

const PRESENCE_STATUSES: &[&str] = &["online", "idle", "dnd", "invisible"];

impl Store {
    pub fn user_presence_settings(&self, user_id: &str) -> Result<UserPresenceSettings> {
        let conn = self.conn()?;
        let existing = conn
            .query_row(
                "SELECT user_id, status, custom_status, activity, updated_at
                   FROM user_presence_settings
                  WHERE user_id = ?1",
                params![user_id],
                presence_from_row,
            )
            .optional()?;
        Ok(existing.unwrap_or_else(|| UserPresenceSettings {
            user_id: user_id.to_string(),
            status: "online".to_string(),
            custom_status: None,
            activity: None,
            updated_at: now(),
        }))
    }

    pub fn set_user_presence_settings(
        &self,
        user_id: &str,
        status: &str,
        custom_status: Option<&str>,
        activity: Option<&str>,
    ) -> Result<UserPresenceSettings> {
        let status = status.trim().to_ascii_lowercase();
        if !PRESENCE_STATUSES.contains(&status.as_str()) {
            return Err(anyhow!("状态必须为 online / idle / dnd / invisible"));
        }
        let custom_status = clean_presence_text(custom_status, 80);
        let activity = clean_presence_text(activity, 80);
        let updated_at = now();
        let conn = self.conn()?;
        conn.execute(
            "INSERT INTO user_presence_settings
             (user_id, status, custom_status, activity, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(user_id) DO UPDATE SET
               status = excluded.status,
               custom_status = excluded.custom_status,
               activity = excluded.activity,
               updated_at = excluded.updated_at",
            params![user_id, status, custom_status, activity, updated_at],
        )?;
        drop(conn);
        self.user_presence_settings(user_id)
    }
}

fn clean_presence_text(value: Option<&str>, max_chars: usize) -> Option<String> {
    let value = value.map(str::trim).filter(|value| !value.is_empty())?;
    Some(value.chars().take(max_chars).collect())
}

fn presence_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<UserPresenceSettings> {
    Ok(UserPresenceSettings {
        user_id: row.get(0)?,
        status: row.get(1)?,
        custom_status: row.get(2)?,
        activity: row.get(3)?,
        updated_at: row.get(4)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn temp_store() -> Store {
        let path =
            std::env::temp_dir().join(format!("elon_user_presence_{}.db", Uuid::new_v4().simple()));
        Store::open(&path).expect("store should open")
    }

    #[test]
    fn user_presence_settings_can_be_saved() {
        let store = temp_store();
        let user = store
            .create_user("presence@example.com", "secret1", None, None)
            .expect("user should be created");

        let default_presence = store
            .user_presence_settings(&user.id)
            .expect("default presence should load");
        assert_eq!(default_presence.status, "online");

        let updated = store
            .set_user_presence_settings(
                &user.id,
                "dnd",
                Some("  Coding  "),
                Some("Reviewing members"),
            )
            .expect("presence should update");
        assert_eq!(updated.status, "dnd");
        assert_eq!(updated.custom_status.as_deref(), Some("Coding"));
        assert_eq!(updated.activity.as_deref(), Some("Reviewing members"));

        let error = store
            .set_user_presence_settings(&user.id, "busy", None, None)
            .expect_err("invalid status should fail");
        assert!(error.to_string().contains("状态必须"));
    }
}
