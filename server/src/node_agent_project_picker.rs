// server/src/node_agent_project_picker.rs

use axum::{http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

use crate::{
    node_agent_project_manifest_identity::{
        detect_manifest_project_identity, detect_shallow_manifest_project_identity,
    },
    node_agent_project_profile::detect_project_profile,
    project_landing, project_workspace_inspect,
};

#[derive(Debug, Deserialize)]
pub(crate) struct InspectLocalProjectReq {
    workspace_path: String,
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
    if project.agent_runtime.status != "current" {
        warnings.push(project.agent_runtime.summary.clone());
    }
    if !inspect.codex_available && !inspect.copilot_available {
        warnings.push(
            "本机未检测到 Codex/Copilot CLI，开发任务会优先走 Route B/C 模型能力。".to_string(),
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
        autofill_fields.push("Agent Runtime".to_string());
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

#[derive(Debug)]
struct ProjectIdentity {
    name: String,
    description: Option<String>,
    source: Option<String>,
}

fn detect_project_identity(
    path: &Path,
    landing: Option<&Value>,
    git_remote_origin: Option<&str>,
) -> ProjectIdentity {
    let fallback_name = project_name(path);
    if let Some(identity) = identity_from_landing(&fallback_name, landing) {
        return identity;
    }
    if let Some(identity) = identity_from_package_json(&fallback_name, &path.join("package.json")) {
        return identity;
    }
    if let Some(identity) = detect_manifest_project_identity(&fallback_name, path) {
        return ProjectIdentity {
            name: identity.name,
            description: identity.description,
            source: Some(identity.source),
        };
    }
    if let Some(identity) = identity_from_toml_manifest(
        &fallback_name,
        &path.join("Cargo.toml"),
        "package",
        "Cargo.toml",
    ) {
        return identity;
    }
    if let Some(identity) = identity_from_toml_manifest(
        &fallback_name,
        &path.join("pyproject.toml"),
        "project",
        "pyproject.toml",
    ) {
        return identity;
    }
    if let Some(identity) = identity_from_go_mod(&fallback_name, &path.join("go.mod")) {
        return identity;
    }
    if let Some(identity) = detect_shallow_manifest_project_identity(&fallback_name, path) {
        return ProjectIdentity {
            name: identity.name,
            description: identity.description,
            source: Some(identity.source),
        };
    }
    if let Some(identity) = identity_from_readme(&fallback_name, path) {
        return identity;
    }
    if let Some(identity) = identity_from_git_remote(git_remote_origin) {
        return identity;
    }
    ProjectIdentity {
        description: Some(default_project_description(&fallback_name)),
        name: fallback_name,
        source: Some("目录名".to_string()),
    }
}

fn identity_from_landing(fallback_name: &str, landing: Option<&Value>) -> Option<ProjectIdentity> {
    let object = landing?.as_object()?;
    let name = first_json_string(object, &["title"]);
    let description = first_json_string(object, &["tagline", "summary", "description"]);
    identity_from_parts(
        fallback_name,
        name,
        description,
        ".elon/project-landing.json",
    )
}

fn identity_from_package_json(fallback_name: &str, package_json: &Path) -> Option<ProjectIdentity> {
    let object = std::fs::read_to_string(package_json)
        .ok()
        .and_then(|text| serde_json::from_str::<Value>(&text).ok())
        .and_then(|value| value.as_object().cloned())?;
    identity_from_parts(
        fallback_name,
        first_json_string(&object, &["displayName", "display_name", "name"]),
        first_json_string(&object, &["description"]),
        "package.json",
    )
}

fn identity_from_toml_manifest(
    fallback_name: &str,
    manifest_path: &Path,
    section: &str,
    source: &str,
) -> Option<ProjectIdentity> {
    if !manifest_path.is_file() {
        return None;
    }
    identity_from_parts(
        fallback_name,
        toml_section_string(manifest_path, section, "name"),
        toml_section_string(manifest_path, section, "description"),
        source,
    )
}

fn identity_from_go_mod(fallback_name: &str, go_mod: &Path) -> Option<ProjectIdentity> {
    let module_path = go_module_path(go_mod)?;
    let name = go_module_name(&module_path)?;
    identity_from_parts(fallback_name, Some(name), None, "go.mod")
}

fn identity_from_readme(fallback_name: &str, path: &Path) -> Option<ProjectIdentity> {
    let (readme_path, source) = ["README.md", "README.MD", "Readme.md", "README"]
        .into_iter()
        .map(|file| (path.join(file), file))
        .find(|(candidate, _)| candidate.is_file())?;
    let text = std::fs::read_to_string(readme_path).ok()?;
    let mut title = None;
    let mut description_lines = Vec::new();
    let mut in_code_fence = false;
    let mut seen_heading = false;

    for raw_line in text.lines().take(120) {
        let line = raw_line.trim();
        if line.starts_with("```") || line.starts_with("~~~") {
            in_code_fence = !in_code_fence;
            continue;
        }
        if in_code_fence || line.is_empty() {
            if !description_lines.is_empty() {
                break;
            }
            continue;
        }
        if line.starts_with("<!--") || line.starts_with("[!") || line.starts_with("![") {
            continue;
        }
        if title.is_none() {
            if let Some(heading) = markdown_heading_text(line) {
                title = Some(heading);
                seen_heading = true;
                continue;
            }
        } else if markdown_heading_text(line).is_some() {
            if !description_lines.is_empty() {
                break;
            }
            continue;
        }
        if seen_heading || title.is_none() {
            if let Some(text) = clean_readme_line(line) {
                description_lines.push(text);
            }
        }
    }

    let description = clean_project_text(&description_lines.join(" "), 240);
    identity_from_parts(fallback_name, title, description, source)
}

fn identity_from_git_remote(remote: Option<&str>) -> Option<ProjectIdentity> {
    let remote = remote.map(str::trim).filter(|value| !value.is_empty())?;
    let trimmed = remote.trim_end_matches('/');
    let mut name_part = trimmed.rsplit(['/', ':']).next().unwrap_or(trimmed).trim();
    if let Some(stripped) = name_part.strip_suffix(".git") {
        name_part = stripped;
    }
    let name = clean_project_text(name_part, 120)?;
    if name == "." || name == ".." {
        return None;
    }
    Some(ProjectIdentity {
        description: Some(default_project_description(&name)),
        name,
        source: Some("Git 远端".to_string()),
    })
}

fn identity_from_parts(
    fallback_name: &str,
    name: Option<String>,
    description: Option<String>,
    source: &str,
) -> Option<ProjectIdentity> {
    if name.is_none() && description.is_none() {
        return None;
    }
    let name = name.unwrap_or_else(|| fallback_name.to_string());
    let description = description.or_else(|| Some(default_project_description(&name)));
    Some(ProjectIdentity {
        name,
        description,
        source: Some(source.to_string()),
    })
}

fn markdown_heading_text(line: &str) -> Option<String> {
    let text = line.strip_prefix('#')?;
    if !text.starts_with('#') && !text.starts_with(' ') {
        return None;
    }
    let text = line.trim_start_matches('#').trim();
    clean_project_text(&strip_markdown_inline(text), 120)
}

fn clean_readme_line(line: &str) -> Option<String> {
    if line.starts_with('#') || line.starts_with('|') || line.starts_with('>') {
        return None;
    }
    let text = strip_markdown_inline(line);
    clean_project_text(&text, 240)
}

fn strip_markdown_inline(value: &str) -> String {
    let mut output = String::new();
    let mut chars = value.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '`' | '*' | '_' => {}
            '[' => {
                let mut label = String::new();
                for next in chars.by_ref() {
                    if next == ']' {
                        break;
                    }
                    label.push(next);
                }
                if chars.peek() == Some(&'(') {
                    for next in chars.by_ref() {
                        if next == ')' {
                            break;
                        }
                    }
                }
                output.push_str(&label);
            }
            _ => output.push(ch),
        }
    }
    output.trim().to_string()
}

fn first_json_string(object: &serde_json::Map<String, Value>, keys: &[&str]) -> Option<String> {
    keys.iter()
        .find_map(|key| object.get(*key).and_then(|value| value.as_str()))
        .and_then(|value| clean_project_text(value, 240))
}

fn toml_section_string(path: &Path, section: &str, key: &str) -> Option<String> {
    let text = std::fs::read_to_string(path).ok()?;
    let mut in_section = false;
    for raw_line in text.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            in_section = line.trim_start_matches('[').trim_end_matches(']').trim() == section;
            continue;
        }
        if !in_section {
            continue;
        }
        let Some((left, right)) = line.split_once('=') else {
            continue;
        };
        if left.trim() == key {
            return parse_toml_string(right.trim())
                .and_then(|value| clean_project_text(&value, 240));
        }
    }
    None
}

fn parse_toml_string(value: &str) -> Option<String> {
    let value = value.trim();
    if let Some(rest) = value.strip_prefix('"') {
        let mut escaped = false;
        let mut output = String::new();
        for ch in rest.chars() {
            if escaped {
                output.push(ch);
                escaped = false;
                continue;
            }
            if ch == '\\' {
                escaped = true;
                continue;
            }
            if ch == '"' {
                return Some(output);
            }
            output.push(ch);
        }
        return None;
    }
    if let Some(rest) = value.strip_prefix('\'') {
        return rest.split_once('\'').map(|(text, _)| text.to_string());
    }
    None
}

fn go_module_path(go_mod: &Path) -> Option<String> {
    let text = std::fs::read_to_string(go_mod).ok()?;
    text.lines().find_map(|raw_line| {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with("//") {
            return None;
        }
        line.strip_prefix("module ")
            .map(str::trim)
            .and_then(|value| clean_project_text(value, 240))
    })
}

fn go_module_name(module_path: &str) -> Option<String> {
    let mut parts = module_path
        .trim()
        .trim_end_matches('/')
        .rsplit('/')
        .filter(|part| !part.trim().is_empty());
    let first = parts.next()?.trim();
    let name = if is_go_major_version_suffix(first) {
        parts.next().unwrap_or(first).trim()
    } else {
        first
    };
    clean_project_text(name.trim_end_matches(".git"), 120)
}

fn is_go_major_version_suffix(value: &str) -> bool {
    let Some(rest) = value.strip_prefix('v') else {
        return false;
    };
    rest.len() <= 3 && rest.chars().all(|ch| ch.is_ascii_digit())
}

fn inspect_agent_runtime_freshness(project_root: &Path) -> AgentRuntimeFreshness {
    let script = project_root.join("scripts").join("elon-agent.ps1");
    let script_path = script.to_string_lossy().to_string();
    let Ok(text) = std::fs::read_to_string(&script) else {
        return AgentRuntimeFreshness {
            status: "missing".to_string(),
            summary: "项目缺少 scripts\\elon-agent.ps1，Route B/C 自研运行时需要重新生成项目脚本。"
                .to_string(),
            script_path,
            has_elon_agent: false,
            has_command_budget: false,
            has_output_limit: false,
            max_run_commands_default: None,
        };
    };

    let has_command_budget =
        text.contains("MaxRunCommands") && text.contains("Use-AgentRunCommandBudget");
    let has_output_limit =
        text.contains("AgentCommandOutputMaxChars") && text.contains("Limit-AgentText");
    let max_run_commands_default = parse_max_run_commands_default(&text);
    if has_command_budget && has_output_limit {
        AgentRuntimeFreshness {
            status: "current".to_string(),
            summary: format!(
                "Agent Runtime 已包含命令预算和输出截断保护，默认每轮最多 {} 个 run_command。",
                max_run_commands_default.unwrap_or(8)
            ),
            script_path,
            has_elon_agent: true,
            has_command_budget,
            has_output_limit,
            max_run_commands_default,
        }
    } else {
        AgentRuntimeFreshness {
            status: "stale".to_string(),
            summary: "项目内 scripts\\elon-agent.ps1 是旧版模板，缺少 run_command 预算或输出截断保护；建议重新生成后再长期使用 Route B/C。".to_string(),
            script_path,
            has_elon_agent: true,
            has_command_budget,
            has_output_limit,
            max_run_commands_default,
        }
    }
}

fn parse_max_run_commands_default(script: &str) -> Option<u32> {
    let marker = "[int]$MaxRunCommands =";
    let start = script.find(marker)? + marker.len();
    let digits = script[start..]
        .trim_start()
        .chars()
        .take_while(|ch| ch.is_ascii_digit())
        .collect::<String>();
    digits.parse().ok()
}

fn clean_project_text(value: &str, max_chars: usize) -> Option<String> {
    let value = value.trim();
    if value.is_empty() {
        return None;
    }
    Some(value.chars().take(max_chars).collect())
}

fn default_project_description(name: &str) -> String {
    format!("绑定到本 PC 节点的本地项目: {name}")
}

fn project_name(path: &Path) -> String {
    path.file_name()
        .and_then(|value| value.to_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("本地项目")
        .to_string()
}

fn json_error(
    status: StatusCode,
    error: impl Into<String>,
) -> (StatusCode, Json<serde_json::Value>) {
    (
        status,
        Json(json!({
            "ok": false,
            "error": error.into(),
        })),
    )
}

#[cfg(windows)]
fn pick_folder() -> anyhow::Result<Option<String>> {
    use std::os::windows::process::CommandExt;
    use std::process::Command;

    const CREATE_NO_WINDOW: u32 = 0x08000000;
    const CREATE_NEW_PROCESS_GROUP: u32 = 0x00000200;
    let script = r#"
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8
Add-Type -AssemblyName System.Windows.Forms
$dialog = New-Object System.Windows.Forms.FolderBrowserDialog
$dialog.Description = '选择要绑定到一龙 PC 节点的项目目录'
$dialog.ShowNewFolderButton = $false
$owner = New-Object System.Windows.Forms.Form
$owner.StartPosition = 'CenterScreen'
$owner.Width = 1
$owner.Height = 1
$owner.ShowInTaskbar = $false
$owner.TopMost = $true
$owner.Opacity = 0
try {
  $owner.Show()
  $owner.Activate()
  $result = $dialog.ShowDialog($owner)
  if ($result -eq [System.Windows.Forms.DialogResult]::OK) {
    Write-Output $dialog.SelectedPath
  }
} finally {
  $owner.Dispose()
}
"#;
    let output = Command::new("powershell")
        .args([
            "-NoProfile",
            "-STA",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            script,
        ])
        // PowerShell 仅用于系统文件夹选择器；隐藏并隔离控制台，避免用户看到黑窗。
        .creation_flags(CREATE_NO_WINDOW | CREATE_NEW_PROCESS_GROUP)
        .output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        anyhow::bail!(if stderr.is_empty() {
            "PowerShell 文件夹选择器返回失败".to_string()
        } else {
            stderr
        });
    }
    let selected = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok((!selected.is_empty()).then_some(selected))
}

#[cfg(not(windows))]
fn pick_folder() -> anyhow::Result<Option<String>> {
    anyhow::bail!("本机文件夹选择器目前仅支持 Windows 客户端");
}

#[cfg(test)]
mod tests {
    use super::{
        detect_project_identity, inspect_agent_runtime_freshness, local_project_info,
        local_project_registration_readiness, AgentRuntimeFreshness, LocalProjectInfo,
    };
    use homecli_proto::ProjectWorkspaceInspectStatus;
    use serde_json::json;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_project(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "elon-project-picker-{label}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn inspect_status() -> ProjectWorkspaceInspectStatus {
        ProjectWorkspaceInspectStatus {
            workspace_path: "C:\\demo".to_string(),
            path_exists: true,
            is_dir: true,
            is_git_worktree: true,
            git_branch: Some("main".to_string()),
            git_head: Some("abc1234".to_string()),
            git_remote_origin: Some("https://example.com/demo.git".to_string()),
            has_uncommitted_changes: false,
            uncommitted_count: Some(0),
            disk_free_bytes: Some(1024 * 1024 * 1024),
            codex_available: true,
            copilot_available: false,
        }
    }

    fn local_project() -> LocalProjectInfo {
        LocalProjectInfo {
            name: "demo".to_string(),
            workspace_path: "C:\\demo".to_string(),
            description: Some("绑定到本 PC 节点的本地项目: demo".to_string()),
            identity_source: Some("目录名".to_string()),
            repo_url: Some("https://example.com/demo.git".to_string()),
            branch: Some("main".to_string()),
            git_head: Some("abc1234".to_string()),
            is_git_worktree: true,
            has_uncommitted_changes: false,
            uncommitted_count: Some(0),
            project_type: Some("Rust".to_string()),
            package_manager: Some("Cargo".to_string()),
            run_command: Some("cargo run".to_string()),
            test_command: Some("cargo test".to_string()),
            build_command: Some("cargo build".to_string()),
            detected_files: vec!["Cargo.toml".to_string()],
            agent_runtime: current_agent_runtime(),
        }
    }

    fn current_agent_runtime() -> AgentRuntimeFreshness {
        AgentRuntimeFreshness {
            status: "current".to_string(),
            summary: "Agent Runtime 已包含命令预算和输出截断保护，默认每轮最多 8 个 run_command。"
                .to_string(),
            script_path: "C:\\demo\\scripts\\elon-agent.ps1".to_string(),
            has_elon_agent: true,
            has_command_budget: true,
            has_output_limit: true,
            max_run_commands_default: Some(8),
        }
    }

    #[test]
    fn registration_readiness_reports_ready_project_autofill() {
        let project = local_project();
        let inspect = inspect_status();

        let readiness = local_project_registration_readiness(&project, &inspect);

        assert!(readiness.can_register);
        assert_eq!(readiness.status, "ready");
        assert!(readiness.missing_fields.is_empty());
        assert!(readiness.warnings.is_empty());
        assert!(readiness.autofill_fields.contains(&"Git 远端".to_string()));
        assert!(readiness.autofill_fields.contains(&"构建命令".to_string()));
        assert!(readiness
            .autofill_fields
            .contains(&"Agent Runtime".to_string()));
        assert_eq!(readiness.next_action.kind, "auto_register");
        assert_eq!(readiness.register_payload.name, "demo");
        assert_eq!(
            readiness.register_payload.repo_url.as_deref(),
            Some("https://example.com/demo.git")
        );
        assert_eq!(readiness.register_payload.branch.as_deref(), Some("main"));
        assert_eq!(
            readiness
                .register_payload
                .dev_profile
                .as_ref()
                .and_then(|profile| profile.build_command.as_deref()),
            Some("cargo build")
        );
    }

    #[test]
    fn registration_readiness_warns_about_gitless_unknown_project() {
        let mut project = local_project();
        project.repo_url = None;
        project.branch = None;
        project.is_git_worktree = false;
        project.project_type = None;
        project.package_manager = None;
        project.run_command = None;
        project.test_command = None;
        project.build_command = None;
        project.detected_files.clear();
        project.agent_runtime = AgentRuntimeFreshness {
            status: "missing".to_string(),
            summary: "项目缺少 scripts\\elon-agent.ps1，Route B/C 自研运行时需要重新生成项目脚本。"
                .to_string(),
            script_path: "C:\\demo\\scripts\\elon-agent.ps1".to_string(),
            has_elon_agent: false,
            has_command_budget: false,
            has_output_limit: false,
            max_run_commands_default: None,
        };

        let mut inspect = inspect_status();
        inspect.is_git_worktree = false;
        inspect.git_remote_origin = None;
        inspect.git_branch = None;
        inspect.codex_available = false;
        inspect.copilot_available = false;

        let readiness = local_project_registration_readiness(&project, &inspect);

        assert!(readiness.can_register);
        assert_eq!(readiness.status, "needs_review");
        assert!(readiness
            .warnings
            .iter()
            .any(|warning| warning.contains("未检测到 Git 工作区")));
        assert!(readiness
            .warnings
            .iter()
            .any(|warning| warning.contains("Route B/C")));
        assert!(readiness
            .warnings
            .iter()
            .any(|warning| warning.contains("elon-agent.ps1")));
        assert!(!readiness.autofill_fields.contains(&"Git 远端".to_string()));
        assert_eq!(readiness.next_action.kind, "review_then_register");
        assert!(readiness.next_action.detail.contains("未检测到 Git 工作区"));
        assert!(readiness.register_payload.repo_url.is_none());
        assert!(readiness.register_payload.dev_profile.is_none());
    }

    #[test]
    fn registration_readiness_blocks_missing_required_payload_fields() {
        let mut project = local_project();
        project.name = " ".to_string();
        project.workspace_path = " ".to_string();
        let inspect = inspect_status();

        let readiness = local_project_registration_readiness(&project, &inspect);

        assert!(!readiness.can_register);
        assert_eq!(readiness.status, "blocked");
        assert_eq!(readiness.next_action.kind, "complete_missing_fields");
        assert!(readiness.next_action.detail.contains("项目目录"));
        assert!(readiness.next_action.detail.contains("项目名称"));
    }

    #[test]
    fn agent_runtime_freshness_detects_missing_stale_and_current_templates() {
        let dir = temp_project("agent-runtime-freshness");

        let missing = inspect_agent_runtime_freshness(&dir);
        assert_eq!(missing.status, "missing");
        assert!(!missing.has_elon_agent);

        let scripts = dir.join("scripts");
        std::fs::create_dir_all(&scripts).unwrap();
        std::fs::write(
            scripts.join("elon-agent.ps1"),
            "function Invoke-AgentAction {}\n",
        )
        .unwrap();
        let stale = inspect_agent_runtime_freshness(&dir);
        assert_eq!(stale.status, "stale");
        assert!(stale.has_elon_agent);
        assert!(!stale.has_command_budget);
        assert!(!stale.has_output_limit);

        std::fs::write(
            scripts.join("elon-agent.ps1"),
            "[int]$MaxRunCommands = 8\n$AgentCommandOutputMaxChars = 12000\nfunction Use-AgentRunCommandBudget {}\nfunction Limit-AgentText {}\n",
        )
        .unwrap();
        let current = inspect_agent_runtime_freshness(&dir);
        assert_eq!(current.status, "current");
        assert_eq!(current.max_run_commands_default, Some(8));
        assert!(current.has_command_budget);
        assert!(current.has_output_limit);

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn detects_project_identity_from_landing_manifest() {
        let landing = json!({
            "title": "智能客服工作台",
            "tagline": "给运营团队使用的客服项目"
        });

        let identity =
            detect_project_identity(PathBuf::from("C:\\demo").as_path(), Some(&landing), None);

        assert_eq!(identity.name, "智能客服工作台");
        assert_eq!(
            identity.description.as_deref(),
            Some("给运营团队使用的客服项目")
        );
        assert_eq!(
            identity.source.as_deref(),
            Some(".elon/project-landing.json")
        );
    }

    #[test]
    fn detects_project_identity_from_package_json() {
        let dir = temp_project("identity-node");
        std::fs::write(
            dir.join("package.json"),
            r#"{"name":"agent-desk","description":"本地 AI 工作台"}"#,
        )
        .unwrap();

        let identity = detect_project_identity(&dir, None, None);
        let _ = std::fs::remove_dir_all(&dir);

        assert_eq!(identity.name, "agent-desk");
        assert_eq!(identity.description.as_deref(), Some("本地 AI 工作台"));
        assert_eq!(identity.source.as_deref(), Some("package.json"));
    }

    #[test]
    fn detects_project_identity_from_cargo_manifest() {
        let dir = temp_project("identity-rust");
        std::fs::write(
            dir.join("Cargo.toml"),
            "[package]\nname = \"repair-agent\"\ndescription = 'Windows 维修代理'\n",
        )
        .unwrap();

        let identity = detect_project_identity(&dir, None, None);
        let _ = std::fs::remove_dir_all(&dir);

        assert_eq!(identity.name, "repair-agent");
        assert_eq!(identity.description.as_deref(), Some("Windows 维修代理"));
        assert_eq!(identity.source.as_deref(), Some("Cargo.toml"));
    }

    #[test]
    fn detects_project_identity_from_go_mod() {
        let dir = temp_project("identity-go");
        std::fs::write(
            dir.join("go.mod"),
            "module github.com/example/pc-node-runtime/v2\n\ngo 1.22\n",
        )
        .unwrap();

        let identity = detect_project_identity(&dir, None, None);
        let _ = std::fs::remove_dir_all(&dir);

        assert_eq!(identity.name, "pc-node-runtime");
        assert_eq!(
            identity.description.as_deref(),
            Some("绑定到本 PC 节点的本地项目: pc-node-runtime")
        );
        assert_eq!(identity.source.as_deref(), Some("go.mod"));
    }

    #[test]
    fn detects_project_identity_from_shallow_module_manifest() {
        let dir = temp_project("identity-shallow-module");
        std::fs::create_dir_all(dir.join("web")).unwrap();
        std::fs::write(
            dir.join("web").join("package.json"),
            r#"{"name":"desktop-workbench","description":"PC 端项目工作台"}"#,
        )
        .unwrap();
        std::fs::write(dir.join("README.md"), "# 根目录 README\n\n通用仓库说明").unwrap();

        let identity = detect_project_identity(&dir, None, None);
        let _ = std::fs::remove_dir_all(&dir);

        assert_eq!(identity.name, "desktop-workbench");
        assert_eq!(identity.description.as_deref(), Some("PC 端项目工作台"));
        assert_eq!(identity.source.as_deref(), Some("web/package.json"));
    }

    #[test]
    fn detects_project_identity_from_readme_heading_and_intro() {
        let dir = temp_project("identity-readme");
        std::fs::write(
            dir.join("README.md"),
            "# 网络诊断助手\n\n![badge](https://example.com/badge.svg)\n\n帮助用户自动检查代理、DNS 和网卡配置。\n第二句会合并进项目描述。\n\n## 安装\n",
        )
        .unwrap();

        let identity = detect_project_identity(&dir, None, None);
        let _ = std::fs::remove_dir_all(&dir);

        assert_eq!(identity.name, "网络诊断助手");
        assert_eq!(
            identity.description.as_deref(),
            Some("帮助用户自动检查代理、DNS 和网卡配置。 第二句会合并进项目描述。")
        );
        assert_eq!(identity.source.as_deref(), Some("README.md"));
    }

    #[test]
    fn structured_manifest_identity_takes_precedence_over_readme() {
        let dir = temp_project("identity-priority");
        std::fs::write(
            dir.join("package.json"),
            r#"{"name":"package-name","description":"manifest desc"}"#,
        )
        .unwrap();
        std::fs::write(dir.join("README.md"), "# README 名称\n\nREADME 描述").unwrap();

        let identity = detect_project_identity(&dir, None, None);
        let _ = std::fs::remove_dir_all(&dir);

        assert_eq!(identity.name, "package-name");
        assert_eq!(identity.description.as_deref(), Some("manifest desc"));
        assert_eq!(identity.source.as_deref(), Some("package.json"));
    }

    #[test]
    fn detects_project_identity_from_git_remote_when_no_manifest_or_readme() {
        let dir = temp_project("identity-git-remote");

        let identity = detect_project_identity(
            &dir,
            None,
            Some("https://github.com/example/acme-desktop-agent.git"),
        );
        let _ = std::fs::remove_dir_all(&dir);

        assert_eq!(identity.name, "acme-desktop-agent");
        assert_eq!(
            identity.description.as_deref(),
            Some("绑定到本 PC 节点的本地项目: acme-desktop-agent")
        );
        assert_eq!(identity.source.as_deref(), Some("Git 远端"));
    }

    #[test]
    fn detects_project_identity_from_ssh_git_remote() {
        let dir = temp_project("identity-ssh-git-remote");

        let identity = detect_project_identity(
            &dir,
            None,
            Some("git@github.com:example/win-client-runtime.git"),
        );
        let _ = std::fs::remove_dir_all(&dir);

        assert_eq!(identity.name, "win-client-runtime");
        assert_eq!(identity.source.as_deref(), Some("Git 远端"));
    }

    #[test]
    fn readme_identity_takes_precedence_over_git_remote() {
        let dir = temp_project("identity-readme-before-git");
        std::fs::write(dir.join("README.md"), "# README 名称\n\nREADME 描述").unwrap();

        let identity = detect_project_identity(
            &dir,
            None,
            Some("https://github.com/example/git-remote-name.git"),
        );
        let _ = std::fs::remove_dir_all(&dir);

        assert_eq!(identity.name, "README 名称");
        assert_eq!(identity.description.as_deref(), Some("README 描述"));
        assert_eq!(identity.source.as_deref(), Some("README.md"));
    }

    #[test]
    fn local_project_info_uses_landing_identity() {
        let dir = temp_project("landing-info");
        std::fs::create_dir_all(dir.join(".elon")).unwrap();
        std::fs::write(
            dir.join(".elon").join("project-landing.json"),
            r#"{"title":"项目元信息名称","summary":"项目元信息描述"}"#,
        )
        .unwrap();
        let landing = crate::project_landing::load_workspace_landing(&dir);

        let (project, _) = local_project_info(dir.to_string_lossy().as_ref(), landing.as_ref())
            .expect("local project should inspect");
        let _ = std::fs::remove_dir_all(&dir);

        assert_eq!(project.name, "项目元信息名称");
        assert_eq!(project.description.as_deref(), Some("项目元信息描述"));
        assert_eq!(
            project.identity_source.as_deref(),
            Some(".elon/project-landing.json")
        );
    }
}
