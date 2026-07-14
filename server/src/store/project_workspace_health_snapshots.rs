//! Latest PC workspace health snapshots.
//!
//! The live health endpoint can inspect a PC workspace when the node is online.
//! Archive pages and capacity decisions need a cheap latest-known view, so this
//! table keeps one snapshot per project and updates it whenever health is checked.

use anyhow::Result;
use homecli_proto::ProjectWorkspaceInspectStatus;
use rusqlite::{params, OptionalExtension};
use std::collections::HashMap;

use super::{new_id, now, ProjectWorkspaceHealthSnapshot, ProjectWorkspaceHealthTarget, Store};

pub struct ProjectWorkspaceHealthSnapshotWrite<'a> {
    pub project_id: &'a str,
    pub node_id: Option<&'a str>,
    pub workspace_path: Option<&'a str>,
    pub can_run_on_pc: bool,
    pub verified_can_run_on_pc: Option<bool>,
    pub health_label: &'a str,
    pub health_tone: &'a str,
    pub recommended_action: &'a str,
    pub warnings: &'a [String],
    pub live_inspect: Option<&'a ProjectWorkspaceInspectStatus>,
    pub inspect_error: Option<&'a str>,
}

impl Store {
    pub fn upsert_project_workspace_health_snapshot(
        &self,
        snapshot: ProjectWorkspaceHealthSnapshotWrite<'_>,
    ) -> Result<ProjectWorkspaceHealthSnapshot> {
        let captured_at = now();
        let warnings_json = serde_json::to_string(snapshot.warnings)?;
        let live_inspect_json = snapshot
            .live_inspect
            .map(serde_json::to_string)
            .transpose()?;
        let disk_free_bytes = snapshot
            .live_inspect
            .and_then(|status| status.disk_free_bytes)
            .and_then(u64_to_i64);
        let path_exists = snapshot.live_inspect.map(|status| status.path_exists);
        let is_dir = snapshot.live_inspect.map(|status| status.is_dir);
        let is_git_worktree = snapshot.live_inspect.map(|status| status.is_git_worktree);
        let cli_available = snapshot
            .live_inspect
            .map(|status| status.codex_available || status.copilot_available);

        self.conn()?.execute(
            "INSERT INTO project_workspace_health_snapshots (
                id, project_id, node_id, workspace_path, can_run_on_pc,
                verified_can_run_on_pc, health_label, health_tone, recommended_action,
                warning_count, warnings_json, live_inspect_json, inspect_error,
                disk_free_bytes, path_exists, is_dir, is_git_worktree, cli_available, captured_at
             )
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19)
             ON CONFLICT(project_id) DO UPDATE SET
                node_id = excluded.node_id,
                workspace_path = excluded.workspace_path,
                can_run_on_pc = excluded.can_run_on_pc,
                verified_can_run_on_pc = excluded.verified_can_run_on_pc,
                health_label = excluded.health_label,
                health_tone = excluded.health_tone,
                recommended_action = excluded.recommended_action,
                warning_count = excluded.warning_count,
                warnings_json = excluded.warnings_json,
                live_inspect_json = excluded.live_inspect_json,
                inspect_error = excluded.inspect_error,
                disk_free_bytes = excluded.disk_free_bytes,
                path_exists = excluded.path_exists,
                is_dir = excluded.is_dir,
                is_git_worktree = excluded.is_git_worktree,
                cli_available = excluded.cli_available,
                captured_at = excluded.captured_at",
            params![
                new_id("pwh"),
                snapshot.project_id,
                snapshot.node_id,
                snapshot.workspace_path,
                snapshot.can_run_on_pc as i64,
                snapshot.verified_can_run_on_pc.map(bool_to_i64),
                snapshot.health_label,
                snapshot.health_tone,
                snapshot.recommended_action,
                snapshot.warnings.len() as i64,
                warnings_json,
                live_inspect_json,
                snapshot.inspect_error,
                disk_free_bytes,
                path_exists.map(bool_to_i64),
                is_dir.map(bool_to_i64),
                is_git_worktree.map(bool_to_i64),
                cli_available.map(bool_to_i64),
                captured_at,
            ],
        )?;

        self.latest_project_workspace_health_snapshot(snapshot.project_id)?
            .ok_or_else(|| anyhow::anyhow!("工作区健康快照写入后读取失败"))
    }

    pub fn latest_project_workspace_health_snapshot(
        &self,
        project_id: &str,
    ) -> Result<Option<ProjectWorkspaceHealthSnapshot>> {
        self.conn()?
            .query_row(
                "SELECT project_id, node_id, workspace_path, can_run_on_pc,
                        verified_can_run_on_pc, health_label, health_tone, recommended_action,
                        warning_count, warnings_json, live_inspect_json, inspect_error,
                        disk_free_bytes, path_exists, is_dir, is_git_worktree, cli_available, captured_at
                 FROM project_workspace_health_snapshots
                 WHERE project_id = ?1",
                params![project_id],
                health_snapshot_from_row,
            )
            .optional()
            .map_err(Into::into)
    }

    pub fn latest_project_workspace_health_snapshots(
        &self,
        project_ids: &[String],
    ) -> Result<HashMap<String, ProjectWorkspaceHealthSnapshot>> {
        let mut snapshots = HashMap::new();
        for project_id in project_ids {
            if let Some(snapshot) = self.latest_project_workspace_health_snapshot(project_id)? {
                snapshots.insert(project_id.clone(), snapshot);
            }
        }
        Ok(snapshots)
    }

    pub fn latest_workspace_health_snapshot_for_node(
        &self,
        node_id: &str,
    ) -> Result<Option<ProjectWorkspaceHealthSnapshot>> {
        self.conn()?
            .query_row(
                "SELECT project_id, node_id, workspace_path, can_run_on_pc,
                        verified_can_run_on_pc, health_label, health_tone, recommended_action,
                        warning_count, warnings_json, live_inspect_json, inspect_error,
                        disk_free_bytes, path_exists, is_dir, is_git_worktree, cli_available, captured_at
                 FROM project_workspace_health_snapshots
                 WHERE node_id = ?1
                 ORDER BY captured_at DESC
                 LIMIT 1",
                params![node_id],
                health_snapshot_from_row,
            )
            .optional()
            .map_err(Into::into)
    }

    pub fn list_project_workspace_health_targets(
        &self,
        limit: i64,
    ) -> Result<Vec<ProjectWorkspaceHealthTarget>> {
        let limit = limit.clamp(1, 500);
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            "SELECT id, source_type, node_id, workspace_path
             FROM projects
             WHERE status != 'deleted'
               AND node_id IS NOT NULL
               AND TRIM(node_id) != ''
               AND workspace_path IS NOT NULL
               AND TRIM(workspace_path) != ''
               AND source_type NOT IN ('agent_balloon', 'chat_memory')
             ORDER BY updated_at DESC
             LIMIT ?1",
        )?;
        let targets = stmt
            .query_map(params![limit], |row| {
                Ok(ProjectWorkspaceHealthTarget {
                    project_id: row.get(0)?,
                    source_type: row.get(1)?,
                    node_id: row.get(2)?,
                    workspace_path: row.get(3)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(targets)
    }
}

fn health_snapshot_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<ProjectWorkspaceHealthSnapshot> {
    let warnings_json: String = row.get(9)?;
    let live_inspect_json: Option<String> = row.get(10)?;
    let disk_free_bytes: Option<i64> = row.get(12)?;
    Ok(ProjectWorkspaceHealthSnapshot {
        project_id: row.get(0)?,
        node_id: row.get(1)?,
        workspace_path: row.get(2)?,
        can_run_on_pc: row.get::<_, i64>(3)? != 0,
        verified_can_run_on_pc: optional_bool(row.get(4)?),
        health_label: row.get(5)?,
        health_tone: row.get(6)?,
        recommended_action: row.get(7)?,
        warning_count: row.get(8)?,
        warnings: serde_json::from_str(&warnings_json).unwrap_or_default(),
        live_inspect: live_inspect_json
            .as_deref()
            .and_then(|value| serde_json::from_str(value).ok()),
        inspect_error: row.get(11)?,
        disk_free_bytes: disk_free_bytes.and_then(i64_to_u64),
        path_exists: optional_bool(row.get(13)?),
        is_dir: optional_bool(row.get(14)?),
        is_git_worktree: optional_bool(row.get(15)?),
        cli_available: optional_bool(row.get(16)?),
        captured_at: row.get(17)?,
    })
}

fn bool_to_i64(value: bool) -> i64 {
    if value {
        1
    } else {
        0
    }
}

fn optional_bool(value: Option<i64>) -> Option<bool> {
    value.map(|value| value != 0)
}

fn u64_to_i64(value: u64) -> Option<i64> {
    i64::try_from(value).ok()
}

fn i64_to_u64(value: i64) -> Option<u64> {
    u64::try_from(value).ok()
}

#[cfg(test)]
#[path = "project_workspace_health_snapshots_tests.rs"]
mod tests;
