// server/src/node_agent_project_picker.rs

use axum::{http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

use crate::{project_landing, project_workspace_inspect};

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
}

#[derive(Debug, Serialize)]
struct LocalProjectRegistrationReadiness {
    can_register: bool,
    status: String,
    summary: String,
    missing_fields: Vec<String>,
    warnings: Vec<String>,
    autofill_fields: Vec<String>,
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

    let identity = detect_project_identity(&path, landing);
    let profile = detect_project_profile(&path);
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

    LocalProjectRegistrationReadiness {
        can_register,
        status,
        summary,
        missing_fields,
        warnings,
        autofill_fields,
    }
}

#[derive(Debug, Default)]
struct ProjectProfile {
    project_type: Option<String>,
    package_manager: Option<String>,
    run_command: Option<String>,
    test_command: Option<String>,
    build_command: Option<String>,
    detected_files: Vec<String>,
}

#[derive(Debug)]
struct ProjectIdentity {
    name: String,
    description: Option<String>,
    source: Option<String>,
}

fn detect_project_identity(path: &Path, landing: Option<&Value>) -> ProjectIdentity {
    let fallback_name = project_name(path);
    if let Some(identity) = identity_from_landing(&fallback_name, landing) {
        return identity;
    }
    if let Some(identity) = identity_from_package_json(&fallback_name, &path.join("package.json")) {
        return identity;
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

fn detect_project_profile(path: &Path) -> ProjectProfile {
    let mut profile = ProjectProfile::default();
    let cargo = path.join("Cargo.toml");
    let package_json = path.join("package.json");
    let gradle = path.join("build.gradle");
    let gradle_kts = path.join("build.gradle.kts");
    let pyproject = path.join("pyproject.toml");

    if cargo.exists() {
        profile.project_type = Some("Rust".to_string());
        profile.package_manager = Some("Cargo".to_string());
        profile.run_command = Some("cargo run".to_string());
        profile.test_command = Some("cargo test".to_string());
        profile.build_command = Some("cargo build".to_string());
        profile.detected_files.push("Cargo.toml".to_string());
        return profile;
    }

    if package_json.exists() {
        profile.project_type = Some("Node.js".to_string());
        profile.package_manager = Some(node_package_manager(path));
        profile.detected_files.push("package.json".to_string());
        if path.join("pnpm-lock.yaml").exists() {
            profile.detected_files.push("pnpm-lock.yaml".to_string());
        } else if path.join("yarn.lock").exists() {
            profile.detected_files.push("yarn.lock".to_string());
        } else if path.join("package-lock.json").exists() {
            profile.detected_files.push("package-lock.json".to_string());
        }
        apply_package_json_scripts(&mut profile, &package_json);
        return profile;
    }

    if gradle.exists() || gradle_kts.exists() {
        profile.project_type = Some("Gradle".to_string());
        profile.package_manager = Some("Gradle".to_string());
        profile.build_command = Some(gradle_command(path, "build"));
        profile.test_command = Some(gradle_command(path, "test"));
        profile.detected_files.push(
            if gradle.exists() {
                "build.gradle"
            } else {
                "build.gradle.kts"
            }
            .to_string(),
        );
        return profile;
    }

    if pyproject.exists() {
        profile.project_type = Some("Python".to_string());
        profile.package_manager = Some("pyproject".to_string());
        profile.test_command = Some("python -m pytest".to_string());
        profile.detected_files.push("pyproject.toml".to_string());
    }

    profile
}

fn node_package_manager(path: &Path) -> String {
    if path.join("pnpm-lock.yaml").exists() {
        "pnpm".to_string()
    } else if path.join("yarn.lock").exists() {
        "yarn".to_string()
    } else if path.join("package-lock.json").exists() {
        "npm".to_string()
    } else {
        "npm".to_string()
    }
}

fn apply_package_json_scripts(profile: &mut ProjectProfile, package_json: &Path) {
    let Some(scripts) = std::fs::read_to_string(package_json)
        .ok()
        .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok())
        .and_then(|value| value.get("scripts").cloned())
        .and_then(|scripts| scripts.as_object().cloned())
    else {
        return;
    };
    let manager = profile.package_manager.as_deref().unwrap_or("npm");
    profile.run_command = first_script_command(manager, &scripts, &["dev", "start"]);
    profile.test_command = first_script_command(manager, &scripts, &["test"]);
    profile.build_command = first_script_command(manager, &scripts, &["build"]);
}

fn first_script_command(
    manager: &str,
    scripts: &serde_json::Map<String, serde_json::Value>,
    names: &[&str],
) -> Option<String> {
    names
        .iter()
        .find(|name| scripts.get(**name).is_some())
        .map(|name| match manager {
            "yarn" => format!("yarn {name}"),
            "pnpm" => format!("pnpm {name}"),
            _ => format!("npm run {name}"),
        })
}

fn gradle_command(path: &Path, task: &str) -> String {
    if cfg!(windows) && path.join("gradlew.bat").exists() {
        format!("gradlew.bat {task}")
    } else if path.join("gradlew").exists() {
        format!("./gradlew {task}")
    } else {
        format!("gradle {task}")
    }
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
if ($dialog.ShowDialog() -eq [System.Windows.Forms.DialogResult]::OK) {
  Write-Output $dialog.SelectedPath
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
        detect_project_identity, detect_project_profile, local_project_info,
        local_project_registration_readiness, LocalProjectInfo,
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
        assert!(!readiness.autofill_fields.contains(&"Git 远端".to_string()));
    }

    #[test]
    fn detects_project_identity_from_landing_manifest() {
        let landing = json!({
            "title": "智能客服工作台",
            "tagline": "给运营团队使用的客服项目"
        });

        let identity = detect_project_identity(PathBuf::from("C:\\demo").as_path(), Some(&landing));

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

        let identity = detect_project_identity(&dir, None);
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

        let identity = detect_project_identity(&dir, None);
        let _ = std::fs::remove_dir_all(&dir);

        assert_eq!(identity.name, "repair-agent");
        assert_eq!(identity.description.as_deref(), Some("Windows 维修代理"));
        assert_eq!(identity.source.as_deref(), Some("Cargo.toml"));
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

    #[test]
    fn detects_rust_project_commands() {
        let dir = temp_project("rust");
        std::fs::write(dir.join("Cargo.toml"), "[package]\nname='demo'\n").unwrap();

        let profile = detect_project_profile(&dir);
        let _ = std::fs::remove_dir_all(&dir);

        assert_eq!(profile.project_type.as_deref(), Some("Rust"));
        assert_eq!(profile.package_manager.as_deref(), Some("Cargo"));
        assert_eq!(profile.test_command.as_deref(), Some("cargo test"));
        assert!(profile.detected_files.contains(&"Cargo.toml".to_string()));
    }

    #[test]
    fn detects_node_package_manager_and_scripts() {
        let dir = temp_project("node");
        std::fs::write(dir.join("pnpm-lock.yaml"), "").unwrap();
        std::fs::write(
            dir.join("package.json"),
            r#"{"scripts":{"dev":"vite","test":"vitest","build":"vite build"}}"#,
        )
        .unwrap();

        let profile = detect_project_profile(&dir);
        let _ = std::fs::remove_dir_all(&dir);

        assert_eq!(profile.project_type.as_deref(), Some("Node.js"));
        assert_eq!(profile.package_manager.as_deref(), Some("pnpm"));
        assert_eq!(profile.run_command.as_deref(), Some("pnpm dev"));
        assert_eq!(profile.test_command.as_deref(), Some("pnpm test"));
        assert_eq!(profile.build_command.as_deref(), Some("pnpm build"));
    }

    #[test]
    fn detects_gradle_and_python_projects() {
        let gradle = temp_project("gradle");
        std::fs::write(gradle.join("build.gradle.kts"), "plugins {}\n").unwrap();
        let gradle_profile = detect_project_profile(&gradle);
        let _ = std::fs::remove_dir_all(&gradle);
        assert_eq!(gradle_profile.project_type.as_deref(), Some("Gradle"));
        assert!(gradle_profile
            .build_command
            .as_deref()
            .unwrap_or_default()
            .contains("build"));

        let python = temp_project("python");
        std::fs::write(python.join("pyproject.toml"), "[project]\nname='demo'\n").unwrap();
        let python_profile = detect_project_profile(&python);
        let _ = std::fs::remove_dir_all(&python);
        assert_eq!(python_profile.project_type.as_deref(), Some("Python"));
        assert_eq!(
            python_profile.test_command.as_deref(),
            Some("python -m pytest")
        );
    }
}
