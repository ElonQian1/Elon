use crate::{
    project_git::configured_local_project_workspace,
    store::{ProjectAccess, PublicUser},
    tools,
    types::AppState,
};

pub fn ensure_mobile_project(
    state: &AppState,
    user_id: &str,
    project_id: &str,
    project_title: Option<&str>,
) -> anyhow::Result<(PublicUser, ProjectAccess)> {
    let user = state.store.ensure_device_user(user_id)?;
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
        tools::create_project_workspace(&workspace, "android", &project.name, &user.id)?;
    }

    Ok((user, project))
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
