use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use std::sync::OnceLock;

use super::{ProjectSpaceSummary, ProjectSummary, PublicProjectItem};

const BB64A_DIR_NAME: &str = "bb64a";
const BB64A_DISPLAY_NAME: &str = "一龙网游加速器";
const BB64A_LOGO_BYTES: &[u8] = include_bytes!("../assets/project-icons/bb64a-logo.png");

static BB64A_LOGO_DATA_URL: OnceLock<String> = OnceLock::new();

pub(crate) fn apply_project_summary_branding(project: &mut ProjectSummary) {
    let display_name = display_name_for_project(
        &project.name,
        &project.source_type,
        project.workspace_path.as_deref(),
        project.storage_repo_path.as_deref(),
        project.storage_worktree_path.as_deref(),
    );
    project.display_name = display_name;
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
    project.display_name =
        display_name_for_project(&project.name, source_type, workspace_path, None, None);
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
    project.display_name =
        display_name_for_project(&project.name, source_type, workspace_path, None, None);
    project.icon_data_url = branded_icon_data_url(
        project.icon_data_url.take(),
        &project.name,
        source_type,
        workspace_path,
        None,
        None,
    );
}

fn display_name_for_project(
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
            display_name_for_project(
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
}
