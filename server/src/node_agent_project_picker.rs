// server/src/node_agent_project_picker.rs

use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use crate::{
    node_agent_project_manifest_identity::{
        detect_manifest_project_identity, detect_shallow_manifest_project_identity,
    },
    node_agent_project_profile::detect_project_profile,
    pc_workspace_provisioner, project_landing, project_workspace_inspect, NodeRuntime,
};

#[derive(Debug, Deserialize)]
pub(crate) struct InspectLocalProjectReq {
    workspace_path: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct DefaultLocalProjectReq {
    project_id: Option<String>,
    user_id: Option<String>,
    name: Option<String>,
    template: Option<String>,
    repo_url: Option<String>,
    branch: Option<String>,
}

#[derive(Debug, Serialize)]
struct LocalProjectInfo {
    name: String,
    workspace_path: String,
    description: Option<String>,
    identity_source: Option<String>,
    repo_url: Option<String>,
    branch: Option<String>,
    git_head: Option<String>,
    is_git_worktree: bool,
    has_uncommitted_changes: bool,
    uncommitted_count: Option<u32>,
    project_type: Option<String>,
    package_manager: Option<String>,
    run_command: Option<String>,
    test_command: Option<String>,
    build_command: Option<String>,
    detected_files: Vec<String>,
    agent_runtime: AgentRuntimeFreshness,
}

#[derive(Debug, Serialize, Clone)]
struct AgentRuntimeFreshness {
    status: String,
    summary: String,
    script_path: String,
    runtime_scope: &'static str,
    registration_required: bool,
    has_elon_agent: bool,
    has_command_budget: bool,
    has_output_limit: bool,
    max_run_commands_default: Option<u32>,
}

#[derive(Debug, Serialize)]
struct LocalProjectRegistrationReadiness {
    can_register: bool,
    status: String,
    summary: String,
    missing_fields: Vec<String>,
    warnings: Vec<String>,
    autofill_fields: Vec<String>,
    next_action: LocalProjectRegistrationNextAction,
    register_payload: LocalProjectRegisterPayload,
}

#[derive(Debug, Serialize)]
struct LocalProjectRegistrationNextAction {
    kind: String,
    label: String,
    detail: String,
}

#[derive(Debug, Serialize)]
struct LocalProjectRegisterPayload {
    name: String,
    workspace_path: String,
    description: Option<String>,
    repo_url: Option<String>,
    branch: Option<String>,
    dev_profile: Option<LocalProjectDevProfilePayload>,
}

#[derive(Debug, Serialize)]
struct LocalProjectDevProfilePayload {
    project_type: Option<String>,
    package_manager: Option<String>,
    run_command: Option<String>,
    test_command: Option<String>,
    build_command: Option<String>,
    detected_files: Vec<String>,
    source: &'static str,
}

pub(crate) async fn pick_local_project_folder() -> (StatusCode, Json<serde_json::Value>) {
    match pick_folder() {
        Ok(Some(path)) => project_info_response(&path),
        Ok(None) => (
            StatusCode::OK,
            Json(json!({ "ok": true, "cancelled": true })),
        ),
        Err(error) => json_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("打开文件夹选择器失败: {error}"),
        ),
    }
}

pub(crate) async fn inspect_local_project_folder(
    Json(req): Json<InspectLocalProjectReq>,
) -> (StatusCode, Json<serde_json::Value>) {
    project_info_response(req.workspace_path.trim())
}

pub(crate) async fn prepare_default_project_folder(
    State(runtime): State<Arc<NodeRuntime>>,
    Json(req): Json<DefaultLocalProjectReq>,
) -> (StatusCode, Json<serde_json::Value>) {
    let project_id = clean_project_text(req.project_id.as_deref().unwrap_or(""), 80)
        .unwrap_or_else(|| "default-local-project".to_string());
    let user_id = clean_project_text(req.user_id.as_deref().unwrap_or(""), 80)
        .unwrap_or_else(|| "current-user".to_string());
    let name = clean_project_text(req.name.as_deref().unwrap_or(""), 120)
        .unwrap_or_else(|| "本机默认项目".to_string());
    let template = clean_project_text(req.template.as_deref().unwrap_or(""), 80)
        .unwrap_or_else(|| "blank".to_string());

    let request = pc_workspace_provisioner::ProjectWorkspaceRequest {
        project_id,
        user_id,
        name,
        template,
        repo_url: clean_project_text(req.repo_url.as_deref().unwrap_or(""), 500),
        branch: clean_project_text(req.branch.as_deref().unwrap_or(""), 160),
    };
    let transition = runtime.node_data_root_transition.clone().lock_owned().await;
    let workspace_root = runtime
        .node_data_root
        .read()
        .await
        .paths
        .as_ref()
        .map(elon_pc_dev_runtime::NodeDataPaths::workspaces);
    let Some(workspace_root) = workspace_root else {
        drop(transition);
        return json_error(
            StatusCode::CONFLICT,
            "尚未配置有效的统一节点数据根，不能创建默认项目目录",
        );
    };
    let provisioned = tokio::task::spawn_blocking(move || {
        let _transition = transition;
        pc_workspace_provisioner::provision_project_workspace_in(&workspace_root, request)
    })
    .await;
    match provisioned {
        Ok(result) => {
            let result = match result {
                Ok(result) => result,
                Err(error) => {
                    return json_error(
                        StatusCode::BAD_REQUEST,
                        format!("准备默认项目目录失败: {error}"),
                    );
                }
            };
            let (status, Json(mut value)) = project_info_response(&result.workspace_path);
            if status.is_success() {
                if let Some(object) = value.as_object_mut() {
                    object.insert(
                        "default_workspace".to_string(),
                        json!({
                            "created": result.created,
                            "workspace_path": result.workspace_path,
                            "git_head": result.git_head,
                            "git_remote_origin": result.git_remote_origin,
                            "git_branch": result.git_branch,
                        }),
                    );
                }
            }
            (status, Json(value))
        }
        Err(error) => json_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("准备默认项目目录任务异常结束: {error}"),
        ),
    }
}

fn project_info_response(workspace_path: &str) -> (StatusCode, Json<serde_json::Value>) {
    let workspace_path = workspace_path.trim();
    if workspace_path.is_empty() {
        return json_error(StatusCode::BAD_REQUEST, "workspace_path 不能为空");
    }

    let landing = project_landing::load_workspace_landing(Path::new(workspace_path));
    match local_project_info(workspace_path, landing.as_ref()) {
        Ok((project, inspect)) => {
            let registration = local_project_registration_readiness(&project, &inspect);
            (
                StatusCode::OK,
                Json(json!({
                    "ok": true,
                    "project": project,
                    "inspect": inspect,
                    "registration": registration,
                    "landing": landing,
                })),
            )
        }
        Err(error) => json_error(StatusCode::BAD_REQUEST, error.to_string()),
    }
}

fn local_project_info(
    workspace_path: &str,
    landing: Option<&Value>,
) -> anyhow::Result<(
    LocalProjectInfo,
    homecli_proto::ProjectWorkspaceInspectStatus,
)> {
    let path = PathBuf::from(workspace_path);
    let inspect = project_workspace_inspect::inspect_project_workspace(workspace_path)?;
    if !inspect.path_exists {
        anyhow::bail!("PC 本地路径不存在: {workspace_path}");
    }
    if !inspect.is_dir {
        anyhow::bail!("workspace_path 必须指向一个目录");
    }

    let identity = detect_project_identity(&path, landing, inspect.git_remote_origin.as_deref());
    let profile = detect_project_profile(&path);
    let agent_runtime = inspect_agent_runtime_freshness(&path);
    let project = LocalProjectInfo {
        name: identity.name,
        workspace_path: inspect.workspace_path.clone(),
        description: identity.description,
        identity_source: identity.source,
        repo_url: inspect.git_remote_origin.clone(),
        branch: inspect
            .git_branch
            .as_deref()
            .filter(|value| *value != "HEAD")
            .map(ToOwned::to_owned),
        git_head: inspect.git_head.clone(),
        is_git_worktree: inspect.is_git_worktree,
        has_uncommitted_changes: inspect.has_uncommitted_changes,
        uncommitted_count: inspect.uncommitted_count,
        project_type: profile.project_type,
        package_manager: profile.package_manager,
        run_command: profile.run_command,
        test_command: profile.test_command,
        build_command: profile.build_command,
        detected_files: profile.detected_files,
        agent_runtime,
    };
    Ok((project, inspect))
}

fn local_project_registration_readiness(
    project: &LocalProjectInfo,
    inspect: &homecli_proto::ProjectWorkspaceInspectStatus,
) -> LocalProjectRegistrationReadiness {
    let mut missing_fields = Vec::new();
    if project.workspace_path.trim().is_empty() {
        missing_fields.push("项目目录".to_string());
    }
    if project.name.trim().is_empty() {
        missing_fields.push("项目名称".to_string());
    }

    let mut warnings = Vec::new();
    if !inspect.is_git_worktree {
        warnings.push("未检测到 Git 工作区，后续 AI 无法基于远端仓库判断同步状态。".to_string());
    }
    if inspect.is_git_worktree && project.repo_url.is_none() {
        warnings.push("未检测到 Git origin，注册后需要手动确认代码来源。".to_string());
    }
    if inspect.is_git_worktree && project.branch.is_none() {
        warnings
            .push("当前处于 detached HEAD 或未识别到分支，注册后会使用 HEAD 状态。".to_string());
    }
    if inspect.has_uncommitted_changes {
        warnings.push(format!(
            "目录内有 {} 个未提交改动，AI 开始开发前会看到脏工作区。",
            inspect.uncommitted_count.unwrap_or(0)
        ));
    }
    if project.project_type.is_none() {
        warnings.push("未识别到常见项目类型，运行/测试/构建命令需要后续手动补充。".to_string());
    }
    if project.agent_runtime.registration_required && project.agent_runtime.status != "current" {
        warnings.push(project.agent_runtime.summary.clone());
    }
    if !inspect.codex_available && !inspect.copilot_available {
        warnings.push(
            "本机未检测到 Codex/Copilot，开发任务会优先使用我的 API key 或平台AI。".to_string(),
        );
    }

    let mut autofill_fields = vec![
        "项目目录".to_string(),
        "项目名称".to_string(),
        "项目描述".to_string(),
    ];
    if project.repo_url.is_some() {
        autofill_fields.push("Git 远端".to_string());
    }
    if project.branch.is_some() {
        autofill_fields.push("Git 分支".to_string());
    }
    if project.project_type.is_some() {
        autofill_fields.push("项目类型".to_string());
    }
    if project.package_manager.is_some() {
        autofill_fields.push("包管理器".to_string());
    }
    if project.run_command.is_some() {
        autofill_fields.push("运行命令".to_string());
    }
    if project.test_command.is_some() {
        autofill_fields.push("测试命令".to_string());
    }
    if project.build_command.is_some() {
        autofill_fields.push("构建命令".to_string());
    }
    if project.agent_runtime.status == "current" {
        autofill_fields.push("便携一龙入口".to_string());
    }

    let can_register = missing_fields.is_empty();
    let status = if !can_register {
        "blocked"
    } else if warnings.is_empty() {
        "ready"
    } else {
        "needs_review"
    }
    .to_string();
    let summary = match status.as_str() {
        "ready" => "已自动识别关键字段，可以直接注册到云端。".to_string(),
        "needs_review" => "已自动识别关键字段，但建议确认提示项后再注册。".to_string(),
        _ => format!("还缺少 {}，暂不能注册。", missing_fields.join("、")),
    };
    let next_action = local_project_registration_next_action(&status, &missing_fields, &warnings);
    let register_payload = local_project_register_payload(project);

    LocalProjectRegistrationReadiness {
        can_register,
        status,
        summary,
        missing_fields,
        warnings,
        autofill_fields,
        next_action,
        register_payload,
    }
}

fn local_project_registration_next_action(
    status: &str,
    missing_fields: &[String],
    warnings: &[String],
) -> LocalProjectRegistrationNextAction {
    match status {
        "ready" => LocalProjectRegistrationNextAction {
            kind: "auto_register".to_string(),
            label: "直接注册".to_string(),
            detail: "选择目录后已自动填好关键字段，可以直接绑定到本 PC 节点。".to_string(),
        },
        "needs_review" => LocalProjectRegistrationNextAction {
            kind: "review_then_register".to_string(),
            label: "确认后注册".to_string(),
            detail: warnings.first().cloned().unwrap_or_else(|| {
                "已自动填好关键字段，建议确认提示项后再绑定到本 PC 节点。".to_string()
            }),
        },
        _ => LocalProjectRegistrationNextAction {
            kind: "complete_missing_fields".to_string(),
            label: "补齐字段".to_string(),
            detail: format!(
                "还缺少 {}，补齐后才能绑定到本 PC 节点。",
                if missing_fields.is_empty() {
                    "必要信息".to_string()
                } else {
                    missing_fields.join("、")
                }
            ),
        },
    }
}

fn local_project_register_payload(project: &LocalProjectInfo) -> LocalProjectRegisterPayload {
    let dev_profile = has_dev_profile(project).then(|| LocalProjectDevProfilePayload {
        project_type: project.project_type.clone(),
        package_manager: project.package_manager.clone(),
        run_command: project.run_command.clone(),
        test_command: project.test_command.clone(),
        build_command: project.build_command.clone(),
        detected_files: project.detected_files.clone(),
        source: "node_agent_project_picker",
    });

    LocalProjectRegisterPayload {
        name: project.name.clone(),
        workspace_path: project.workspace_path.clone(),
        description: project.description.clone(),
        repo_url: project.repo_url.clone(),
        branch: project.branch.clone(),
        dev_profile,
    }
}

fn has_dev_profile(project: &LocalProjectInfo) -> bool {
    project.project_type.is_some()
        || project.package_manager.is_some()
        || project.run_command.is_some()
        || project.test_command.is_some()
        || project.build_command.is_some()
        || !project.detected_files.is_empty()
}

mod helpers;
mod identity;

use helpers::{
    clean_project_text, default_project_description, inspect_agent_runtime_freshness, json_error,
    pick_folder, project_name,
};
use identity::{detect_project_identity, ProjectIdentity};

#[cfg(test)]
#[path = "node_agent_project_picker/picker_tests.rs"]
mod picker_tests;
