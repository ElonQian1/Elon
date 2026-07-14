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

    pub fn can_receive_presence(
        &self,
        viewer_user_id: &str,
        subject_user_id: &str,
    ) -> Result<bool> {
        if viewer_user_id == subject_user_id {
            return Ok(true);
        }
        let conn = self.conn()?;
        let allowed: i64 = conn.query_row(
            "SELECT
               EXISTS(
                 SELECT 1
                   FROM user_friends
                  WHERE user_id = ?1 AND friend_user_id = ?2
               )
               OR EXISTS(
                 SELECT 1
                   FROM project_members viewer
                   JOIN project_members subject
                     ON subject.project_id = viewer.project_id
                  WHERE viewer.user_id = ?1 AND subject.user_id = ?2
               )",
            params![viewer_user_id, subject_user_id],
            |row| row.get(0),
        )?;
        Ok(allowed != 0)
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
#[path = "user_presence_tests.rs"]
mod tests;
