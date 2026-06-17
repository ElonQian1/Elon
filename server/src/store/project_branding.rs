use anyhow::{anyhow, Result};
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use rusqlite::{params, OptionalExtension};
use std::sync::OnceLock;

use super::{
    is_system_project_source_type, now, ProjectSpaceSummary, ProjectSummary, PublicProjectItem,
    Store,
};

const BB64A_DISPLAY_NAME: &str = "一龙网游加速器";
const BB64A_LOGO_BYTES: &[u8] = include_bytes!("../assets/project-icons/bb64a-logo.png");
const FB2_DISPLAY_NAME: &str = "多冠体育";
const FB2_LOGO_BYTES: &[u8] = include_bytes!("../assets/project-icons/fb2-logo.png");
const JIANGXI_JIAN_CHAMBER_DISPLAY_NAME: &str = "江西吉安商会";
const JIANGXI_JIAN_CHAMBER_LOGO_BYTES: &[u8] =
    include_bytes!("../assets/project-icons/jiangxi-jian-chamber-logo.png");

static BB64A_LOGO_DATA_URL: OnceLock<String> = OnceLock::new();
static FB2_LOGO_DATA_URL: OnceLock<String> = OnceLock::new();
static JIANGXI_JIAN_CHAMBER_LOGO_DATA_URL: OnceLock<String> = OnceLock::new();

const BB64A_IDENTIFIERS: &[&str] = &["bb64a"];
const FB2_IDENTIFIERS: &[&str] = &["fb2"];
const JIANGXI_JIAN_CHAMBER_IDENTIFIERS: &[&str] =
    &["江西吉安商会", "NanchangJiAnChamber", "JiangxiJianChamber"];

#[derive(Clone, Copy)]
enum KnownProjectBrand {
    Bb64a,
    Fb2,
    JiangxiJianChamber,
}

const KNOWN_PROJECT_BRANDS: &[KnownProjectBrand] = &[
    KnownProjectBrand::Bb64a,
    KnownProjectBrand::Fb2,
    KnownProjectBrand::JiangxiJianChamber,
];

impl KnownProjectBrand {
    fn identifiers(self) -> &'static [&'static str] {
        match self {
            KnownProjectBrand::Bb64a => BB64A_IDENTIFIERS,
            KnownProjectBrand::Fb2 => FB2_IDENTIFIERS,
            KnownProjectBrand::JiangxiJianChamber => JIANGXI_JIAN_CHAMBER_IDENTIFIERS,
        }
    }

    fn display_name(self) -> &'static str {
        match self {
            KnownProjectBrand::Bb64a => BB64A_DISPLAY_NAME,
            KnownProjectBrand::Fb2 => FB2_DISPLAY_NAME,
            KnownProjectBrand::JiangxiJianChamber => JIANGXI_JIAN_CHAMBER_DISPLAY_NAME,
        }
    }

    fn logo_data_url(self) -> &'static str {
        match self {
            KnownProjectBrand::Bb64a => logo_data_url(&BB64A_LOGO_DATA_URL, BB64A_LOGO_BYTES),
            KnownProjectBrand::Fb2 => logo_data_url(&FB2_LOGO_DATA_URL, FB2_LOGO_BYTES),
            KnownProjectBrand::JiangxiJianChamber => logo_data_url(
                &JIANGXI_JIAN_CHAMBER_LOGO_DATA_URL,
                JIANGXI_JIAN_CHAMBER_LOGO_BYTES,
            ),
        }
    }

    fn matches(
        self,
        name: &str,
        source_type: &str,
        workspace_path: Option<&str>,
        storage_repo_path: Option<&str>,
        storage_worktree_path: Option<&str>,
    ) -> bool {
        let identifiers = self.identifiers();
        if identifiers
            .iter()
            .any(|expected| name.trim().eq_ignore_ascii_case(expected))
        {
            return true;
        }
        if !matches!(source_type, "local_path" | "pc_managed") {
            return false;
        }
        [workspace_path, storage_repo_path, storage_worktree_path]
            .into_iter()
            .flatten()
            .any(|path| {
                identifiers
                    .iter()
                    .any(|expected| path_ends_with_dir(path, expected))
            })
    }
}

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
    known_brand_for_project(
        name,
        source_type,
        workspace_path,
        storage_repo_path,
        storage_worktree_path,
    )
    .map(|brand| brand.display_name().to_string())
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
    known_brand_for_project(
        name,
        source_type,
        workspace_path,
        storage_repo_path,
        storage_worktree_path,
    )
    .map(|brand| brand.logo_data_url().to_string())
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

fn logo_data_url(cache: &'static OnceLock<String>, logo_bytes: &'static [u8]) -> &'static str {
    cache
        .get_or_init(|| format!("data:image/png;base64,{}", B64.encode(logo_bytes)))
        .as_str()
}

fn known_brand_for_project(
    name: &str,
    source_type: &str,
    workspace_path: Option<&str>,
    storage_repo_path: Option<&str>,
    storage_worktree_path: Option<&str>,
) -> Option<KnownProjectBrand> {
    KNOWN_PROJECT_BRANDS.iter().copied().find(|brand| {
        brand.matches(
            name,
            source_type,
            workspace_path,
            storage_repo_path,
            storage_worktree_path,
        )
    })
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
    fn known_project_branding_matches_windows_workspace_paths() {
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
        assert_eq!(
            default_display_name_for_project(
                "fb2",
                "pc_managed",
                Some(r"D:\rust\active-projects\fb2"),
                None,
                None
            )
            .as_deref(),
            Some(FB2_DISPLAY_NAME)
        );
        assert_eq!(
            default_display_name_for_project(
                "NanchangJiAnChamber",
                "local_path",
                Some(r"D:\rust\active-projects\江西吉安商会\NanchangJiAnChamber"),
                None,
                None
            )
            .as_deref(),
            Some(JIANGXI_JIAN_CHAMBER_DISPLAY_NAME)
        );
    }

    #[test]
    fn known_project_branding_preserves_manual_icon() {
        let icon = branded_icon_data_url(
            Some("data:image/png;base64,manual".to_string()),
            "fb2",
            "pc_managed",
            Some(r"D:\rust\active-projects\fb2"),
            None,
            None,
        );
        assert_eq!(icon.as_deref(), Some("data:image/png;base64,manual"));
    }

    #[test]
    fn known_project_branding_supplies_default_icons() {
        let icon = branded_icon_data_url(
            None,
            "江西吉安商会",
            "pc_managed",
            Some(r"D:\rust\active-projects\江西吉安商会"),
            None,
            None,
        )
        .expect("default icon");
        assert!(icon.starts_with("data:image/png;base64,"));
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
