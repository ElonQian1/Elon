use anyhow::{anyhow, Result};
use rusqlite::params;

use super::super::{new_id, now, ProjectChannel, Store};
use super::{
    permissions::{
        ensure_project_channel_category_exists_locked, ensure_project_channel_exists_locked,
    },
    DEFAULT_CHANNELS,
};

const PROJECT_CHANNEL_NAME_MAX: usize = 32;

impl Store {
    pub fn create_project_channel(
        &self,
        user_id: &str,
        project_id: &str,
        name: &str,
        category_id: Option<&str>,
    ) -> Result<ProjectChannel> {
        self.ensure_project_default_channels(project_id)?;
        let name = clean_channel_name(name)?;
        let conn = self.conn()?;
        let category_id = clean_category_id(category_id);
        if let Some(category_id) = category_id {
            ensure_project_channel_category_exists_locked(&conn, project_id, category_id)?;
        }
        let position: i64 = conn
            .query_row(
                "SELECT COALESCE(MAX(position), 0) + 10 FROM project_channels WHERE project_id = ?1",
                params![project_id],
                |row| row.get(0),
            )
            .unwrap_or(10);
        let id = new_id("pch");
        let kind = new_id("custom");
        conn.execute(
            "INSERT INTO project_channels (
                id, project_id, category_id, name, kind, position, created_at, updated_at
             )
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)",
            params![id, project_id, category_id, name, kind, position, now()],
        )?;
        drop(conn);
        self.visible_project_channel(user_id, project_id, &id)
    }

    pub fn rename_project_channel(
        &self,
        user_id: &str,
        project_id: &str,
        channel_id: &str,
        name: &str,
    ) -> Result<ProjectChannel> {
        self.ensure_project_default_channels(project_id)?;
        let name = clean_channel_name(name)?;
        let conn = self.conn()?;
        ensure_project_channel_exists_locked(&conn, project_id, channel_id)?;
        let updated = conn.execute(
            "UPDATE project_channels
                SET name = ?1, updated_at = ?2
              WHERE project_id = ?3 AND id = ?4",
            params![name, now(), project_id, channel_id],
        )?;
        if updated == 0 {
            anyhow::bail!("频道不存在");
        }
        drop(conn);
        self.visible_project_channel(user_id, project_id, channel_id)
    }

    pub fn delete_project_channel(&self, project_id: &str, channel_id: &str) -> Result<()> {
        self.ensure_project_default_channels(project_id)?;
        let conn = self.conn()?;
        let kind = ensure_project_channel_exists_locked(&conn, project_id, channel_id)?;
        if is_default_channel_kind(&kind) {
            anyhow::bail!("默认频道不能删除，可以改名或调整权限");
        }
        conn.execute(
            "DELETE FROM project_channel_read_states WHERE project_id = ?1 AND channel_id = ?2",
            params![project_id, channel_id],
        )?;
        conn.execute(
            "DELETE FROM project_channel_role_permissions WHERE project_id = ?1 AND channel_id = ?2",
            params![project_id, channel_id],
        )?;
        conn.execute(
            "DELETE FROM project_channel_member_permissions WHERE project_id = ?1 AND channel_id = ?2",
            params![project_id, channel_id],
        )?;
        conn.execute(
            "DELETE FROM project_channel_messages WHERE project_id = ?1 AND channel_id = ?2",
            params![project_id, channel_id],
        )?;
        conn.execute(
            "DELETE FROM project_channels WHERE project_id = ?1 AND id = ?2",
            params![project_id, channel_id],
        )?;
        Ok(())
    }

    fn visible_project_channel(
        &self,
        user_id: &str,
        project_id: &str,
        channel_id: &str,
    ) -> Result<ProjectChannel> {
        self.list_project_space_channels(user_id, project_id)?
            .into_iter()
            .find(|channel| channel.id == channel_id)
            .ok_or_else(|| anyhow!("频道已保存，但当前用户无权查看"))
    }
}

fn clean_channel_name(raw: &str) -> Result<String> {
    let name = raw.split_whitespace().collect::<Vec<_>>().join(" ");
    if name.is_empty() {
        anyhow::bail!("频道名称不能为空");
    }
    if name.chars().count() > PROJECT_CHANNEL_NAME_MAX {
        anyhow::bail!("频道名称不能超过 {PROJECT_CHANNEL_NAME_MAX} 个字");
    }
    Ok(name)
}

fn clean_category_id(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

fn is_default_channel_kind(kind: &str) -> bool {
    DEFAULT_CHANNELS
        .iter()
        .any(|(_, default_kind, _, _)| *default_kind == kind)
}
