use crate::{
    project_git::configured_local_project_workspace,
    store::{ProjectAccess, PublicUser},
    tools,
    types::AppState,
};
use std::path::Path;

pub fn ensure_mobile_project(
    state: &AppState,
    user_id: &str,
    project_id: &str,
    project_title: Option<&str>,
) -> anyhow::Result<(PublicUser, ProjectAccess)> {
    let user = state.store.ensure_device_user(user_id)?;
    if project_id == "elon-self" {
        let project = state.store.get_project_access(&user.id, project_id)?;
        return Ok((user, project));
    }

    let spec = mobile_project_spec(project_id, project_title);
    let project = state.store.ensure_project_for_user(
        &user.id,
        project_id,
        &spec.name,
        Some(spec.description),
        spec.source_type,
        spec.template,
        spec.workspace_path.as_deref(),
    )?;

    if project.source_type != "local_path" {
        let workspace = state
            .resolve_project_workspace(&project.workspace_key, project.workspace_path.as_deref());
        if should_initialize_mobile_workspace(&workspace) {
            tools::create_project_workspace(&workspace, "android", &project.name, &user.id)?;
        }
    }

    Ok((user, project))
}

fn should_initialize_mobile_workspace(workspace: &Path) -> bool {
    !workspace.join(".git").exists()
}

#[cfg(test)]
mod tests {
    use super::should_initialize_mobile_workspace;
    use std::{
        fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    #[test]
    fn missing_workspace_requires_initialization() {
        let workspace = unique_temp_workspace("missing");
        let _ = fs::remove_dir_all(&workspace);

        assert!(should_initialize_mobile_workspace(&workspace));
    }

    #[test]
    fn existing_git_workspace_skips_initialization() {
        let workspace = unique_temp_workspace("git");
        fs::create_dir_all(workspace.join(".git")).unwrap();

        assert!(!should_initialize_mobile_workspace(&workspace));

        let _ = fs::remove_dir_all(&workspace);
    }

    #[test]
    fn existing_non_git_workspace_requires_repair() {
        let workspace = unique_temp_workspace("non_git");
        fs::create_dir_all(&workspace).unwrap();
        fs::write(workspace.join("README.md"), "existing files").unwrap();

        assert!(should_initialize_mobile_workspace(&workspace));

        let _ = fs::remove_dir_all(&workspace);
    }

    fn unique_temp_workspace(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "elon-mobile-workspace-{label}-{}-{nanos}",
            std::process::id()
        ))
    }
}

struct MobileProjectSpec {
    name: String,
    description: &'static str,
    source_type: &'static str,
    template: &'static str,
    workspace_path: Option<String>,
}

fn mobile_project_spec(project_id: &str, project_title: Option<&str>) -> MobileProjectSpec {
    let workspace_path = configured_local_project_workspace(project_id)
        .map(|path| path.to_string_lossy().to_string());
    let name = project_title
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| {
            if project_id == "elon-self" {
                "一龙项目".into()
            } else {
                "移动端项目".into()
            }
        });

    if workspace_path.is_some() {
        MobileProjectSpec {
            name,
            description: "本地 Git 项目",
            source_type: "local_path",
            template: "local",
            workspace_path,
        }
    } else {
        MobileProjectSpec {
            name,
            description: "APK 创建的项目",
            source_type: "template",
            template: "android",
            workspace_path: None,
        }
    }
}
