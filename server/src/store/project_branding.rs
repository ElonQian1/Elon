use anyhow::{anyhow, Result};
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use rusqlite::{params, OptionalExtension};
use std::sync::OnceLock;

use super::{
    is_system_project_source_type, now, ProjectSpaceSummary, ProjectSummary, PublicProjectItem,
    Store,
};

const BB64A_DIR_NAME: &str = "bb64a";
const BB64A_DISPLAY_NAME: &str = "一龙网游加速器";
const BB64A_LOGO_BYTES: &[u8] = include_bytes!("../assets/project-icons/bb64a-logo.png");

static BB64A_LOGO_DATA_URL: OnceLock<String> = OnceLock::new();

pub(crate) fn apply_project_summary_branding(project: &mut ProjectSummary) {
    let fallback_display_name = default_display_name_for_project(
        &project.name,
        &project.source_type,
        project.workspace_path.as_deref(),
        project.storage_repo_path.as_deref(),
        project.storage_worktree_path.as_deref(),
    );
    project.display_name =
        clean_display_name(project.display_name.take()).or(fallback_display_name);
    project.icon_data_url = branded_icon_data_url(
        project.icon_data_url.take(),
        &project.name,
        &project.source_type,
        project.workspace_path.as_deref(),
        project.storage_repo_path.as_deref(),
        project.storage_worktree_path.as_deref(),
    );
}

pub(crate) fn apply_public_project_branding(
    project: &mut PublicProjectItem,
    source_type: &str,
    workspace_path: Option<&str>,
) {
    let fallback_display_name =
        default_display_name_for_project(&project.name, source_type, workspace_path, None, None);
    project.display_name =
        clean_display_name(project.display_name.take()).or(fallback_display_name);
    project.icon_data_url = branded_icon_data_url(
        project.icon_data_url.take(),
        &project.name,
        source_type,
        workspace_path,
        None,
        None,
    );
}

pub(crate) fn apply_project_space_branding(
    project: &mut ProjectSpaceSummary,
    source_type: &str,
    workspace_path: Option<&str>,
) {
    let fallback_display_name =
        default_display_name_for_project(&project.name, source_type, workspace_path, None, None);
    project.display_name =
        clean_display_name(project.display_name.take()).or(fallback_display_name);
    project.icon_data_url = branded_icon_data_url(
        project.icon_data_url.take(),
        &project.name,
        source_type,
        workspace_path,
        None,
        None,
    );
}

impl Store {
    pub fn update_project_branding(
        &self,
        project_id: &str,
        display_name: Option<Option<&str>>,
        icon_data_url: Option<Option<&str>>,
    ) -> Result<()> {
        if display_name.is_none() && icon_data_url.is_none() {
            return Ok(());
        }

        let conn = self.conn()?;
        let source_type: String = conn
            .query_row(
                "SELECT source_type FROM projects WHERE id = ?1 AND status != 'deleted'",
                params![project_id],
                |row| row.get(0),
            )
            .optional()?
            .ok_or_else(|| anyhow!("项目不存在"))?;
        if is_system_project_source_type(&source_type) {
            anyhow::bail!("系统归档项目不能修改展示资料");
        }

        let (current_display_name, current_icon_data_url): (Option<String>, Option<String>) = conn
            .query_row(
                "SELECT display_name, icon_data_url FROM projects WHERE id = ?1 AND status != 'deleted'",
                params![project_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )?;

        let next_display_name = match display_name {
            Some(value) => clean_display_name(value.map(ToOwned::to_owned)),
            None => current_display_name,
        };
        let next_icon_data_url = match icon_data_url {
            Some(value) => clean_icon_data_url(value.map(ToOwned::to_owned)),
            None => current_icon_data_url,
        };

        let n = conn.execute(
            "UPDATE projects
                SET display_name = ?1,
                    icon_data_url = ?2,
                    updated_at = ?3
              WHERE id = ?4 AND status != 'deleted'",
            params![next_display_name, next_icon_data_url, now(), project_id],
        )?;
        if n == 0 {
            anyhow::bail!("项目不存在");
        }
        Ok(())
    }
}

fn default_display_name_for_project(
    name: &str,
    source_type: &str,
    workspace_path: Option<&str>,
    storage_repo_path: Option<&str>,
    storage_worktree_path: Option<&str>,
) -> Option<String> {
    if is_bb64a_project(
        name,
        source_type,
        workspace_path,
        storage_repo_path,
        storage_worktree_path,
    ) {
        Some(BB64A_DISPLAY_NAME.to_string())
    } else {
        None
    }
}

fn branded_icon_data_url(
    icon_data_url: Option<String>,
    name: &str,
    source_type: &str,
    workspace_path: Option<&str>,
    storage_repo_path: Option<&str>,
    storage_worktree_path: Option<&str>,
) -> Option<String> {
    if let Some(icon_data_url) = clean_icon_data_url(icon_data_url) {
        return Some(icon_data_url);
    }
    if is_bb64a_project(
        name,
        source_type,
        workspace_path,
        storage_repo_path,
        storage_worktree_path,
    ) {
        return Some(bb64a_logo_data_url().to_string());
    }
    None
}

fn clean_icon_data_url(icon_data_url: Option<String>) -> Option<String> {
    icon_data_url
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty() && !value.eq_ignore_ascii_case("null"))
}

fn clean_display_name(display_name: Option<String>) -> Option<String> {
    display_name
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty() && !value.eq_ignore_ascii_case("null"))
}

fn bb64a_logo_data_url() -> &'static str {
    BB64A_LOGO_DATA_URL
        .get_or_init(|| format!("data:image/png;base64,{}", B64.encode(BB64A_LOGO_BYTES)))
        .as_str()
}

fn is_bb64a_project(
    name: &str,
    source_type: &str,
    workspace_path: Option<&str>,
    storage_repo_path: Option<&str>,
    storage_worktree_path: Option<&str>,
) -> bool {
    if name.trim().eq_ignore_ascii_case(BB64A_DIR_NAME) {
        return true;
    }
    if !matches!(source_type, "local_path" | "pc_managed") {
        return false;
    }
    [workspace_path, storage_repo_path, storage_worktree_path]
        .into_iter()
        .flatten()
        .any(|path| path_ends_with_dir(path, BB64A_DIR_NAME))
}

fn path_ends_with_dir(path: &str, expected: &str) -> bool {
    path.trim()
        .trim_end_matches(|ch| ch == '/' || ch == '\\')
        .rsplit(|ch| ch == '/' || ch == '\\')
        .next()
        .is_some_and(|name| name.eq_ignore_ascii_case(expected))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bb64a_branding_matches_windows_workspace_path() {
        assert_eq!(
            default_display_name_for_project(
                "bb64a",
                "pc_managed",
                Some(r"D:\rust\active-projects\bb64a"),
                None,
                None
            )
            .as_deref(),
            Some(BB64A_DISPLAY_NAME)
        );
    }

    #[test]
    fn bb64a_branding_preserves_manual_icon() {
        let icon = branded_icon_data_url(
            Some("data:image/png;base64,manual".to_string()),
            "bb64a",
            "pc_managed",
            Some(r"D:\rust\active-projects\bb64a"),
            None,
            None,
        );
        assert_eq!(icon.as_deref(), Some("data:image/png;base64,manual"));
    }

    #[test]
    fn configured_display_name_overrides_bb64a_default() {
        let mut project = ProjectSummary {
            id: "prj-test".to_string(),
            name: "bb64a".to_string(),
            display_name: Some("自定义加速器".to_string()),
            description: None,
            workspace_key: "prj-test".to_string(),
            template: "local".to_string(),
            source_type: "pc_managed".to_string(),
            repo_url: None,
            branch: None,
            workspace_path: Some(r"D:\rust\active-projects\bb64a".to_string()),
            node_id: None,
            storage_node_id: None,
            storage_repo_path: None,
            storage_repo_url: None,
            storage_worktree_path: None,
            storage_status: "none".to_string(),
            status: "active".to_string(),
            role: "owner".to_string(),
            member_count: 1,
            is_public: false,
            join_mode: "invite".to_string(),
            last_task_status: None,
            last_apk_url: None,
            icon_data_url: None,
            updated_at: "now".to_string(),
        };
        apply_project_summary_branding(&mut project);
        assert_eq!(project.display_name.as_deref(), Some("自定义加速器"));
    }
}
