use std::fs;
use std::path::{Component, Path, PathBuf};

use anyhow::{anyhow, bail, Context, Result};
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use serde_json::{json, Value};

use super::broker::LiveUiSession;
use super::ui_ir::TargetDesignUpload;

const MAX_ARTIFACT_BYTES: u64 = 512 * 1024;

pub(crate) fn project_profile(session: &LiveUiSession) -> Result<Value> {
    let root = canonical_project_root(session)?;
    read_json_artifact(&root.join(".elon/ui-design/project-ui-profile.json"))
}

pub(crate) fn design_task(session: &LiveUiSession, arguments: &Value) -> Result<Value> {
    let root = canonical_project_root(session)?;
    let tasks_root = root.join(".elon/ui-design/tasks");
    let requested = arguments
        .get("taskId")
        .and_then(Value::as_str)
        .map(safe_id)
        .filter(|value| !value.is_empty());
    let task_dir = match requested {
        Some(task_id) => tasks_root.join(task_id),
        None => latest_task_dir(&tasks_root)?,
    };
    Ok(json!({
        "task": read_json_artifact(&task_dir.join("task.json"))?,
        "attachments": read_json_artifact(&task_dir.join("attachments.json"))?,
        "taskDirectory": task_dir,
    }))
}

pub(crate) fn create_compose_screen_scaffold(
    session: &LiveUiSession,
    arguments: &Value,
) -> Result<Value> {
    let root = canonical_project_root(session)?;
    let profile = project_profile(session)?;
    if profile.pointer("/capabilities/jetpackCompose") != Some(&Value::Bool(true)) {
        bail!("项目 UI Profile 未确认 Jetpack Compose，拒绝生成 Compose 页面");
    }
    let screen_name = required_string(arguments, "screenName")?;
    let screen_id = required_string(arguments, "screenId")?;
    let package_name = optional_string(arguments, "packageName")
        .or_else(|| {
            profile
                .pointer("/android/namespace")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned)
        })
        .or_else(|| {
            profile
                .pointer("/android/applicationId")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned)
        })
        .ok_or_else(|| anyhow!("UI Profile 未识别 Android namespace；请显式提供 packageName"))?;
    let relative_file = optional_string(arguments, "relativeFile")
        .unwrap_or_else(|| default_compose_source_path(&profile, &package_name, &screen_name));
    validate_package(&package_name)?;
    validate_identifier(&screen_name, "screenName")?;
    validate_screen_id(&screen_id)?;
    let target = safe_new_kotlin_path(&root, &relative_file)?;
    if target.exists() {
        bail!("目标页面已存在，脚手架禁止覆盖: {}", target.display());
    }
    let composable_name = if screen_name.ends_with("Screen") {
        screen_name.clone()
    } else {
        format!("{screen_name}Screen")
    };
    let source = compose_source(&package_name, &composable_name, &screen_id);
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&target, source)?;
    let contract_dir = root.join(".elon/ui-design/generated");
    fs::create_dir_all(&contract_dir)?;
    let contract_path = contract_dir.join(format!("{}.scaffold.json", safe_id(&screen_id)));
    fs::write(
        &contract_path,
        serde_json::to_vec_pretty(&json!({
            "schemaVersion": 1,
            "screenId": screen_id,
            "screenName": composable_name,
            "sourceFile": target.strip_prefix(&root).unwrap_or(&target),
            "status": "SCAFFOLDED",
            "bindingStatus": "NEEDS_PROJECT_ADAPTER",
            "requiredNextActions": [
                "Apply the project theme and reusable components from project-ui-profile.json",
                "Register navigation and Preview/Preview Host scenario",
                "Replace placeholder state with the target design structure",
                "Build before starting Runtime visual fitting"
            ]
        }))?,
    )?;
    Ok(json!({
        "created": true,
        "sourceFile": target,
        "contractFile": contract_path,
        "screenId": screen_id,
        "screenName": composable_name,
        "nextPhase": "COMPILE_BOOTSTRAP"
    }))
}

pub(crate) fn target_design_upload(
    session: &LiveUiSession,
    arguments: &Value,
) -> Result<TargetDesignUpload> {
    let root = canonical_project_root(session)?;
    let bundle = design_task(session, arguments)?;
    let envelope = bundle
        .get("task")
        .ok_or_else(|| anyhow!("设计任务缺少 task.json"))?;
    let intent = envelope
        .pointer("/task/attachment_intent")
        .or_else(|| envelope.pointer("/task/attachmentIntent"))
        .and_then(Value::as_str)
        .unwrap_or("AUTO");
    if intent != "TARGET_DESIGN" {
        bail!(
            "只有 TARGET_DESIGN 可绑定为像素目标；当前是 {intent}，标注层和风格参考不得参与像素拟合"
        );
    }
    let requested_id = arguments
        .get("attachmentId")
        .and_then(Value::as_str)
        .or_else(|| {
            envelope
                .pointer("/task/target_design_attachment_id")
                .or_else(|| envelope.pointer("/task/targetDesignAttachmentId"))
                .and_then(Value::as_str)
        });
    let entries = bundle
        .pointer("/attachments/attachments")
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow!("设计任务没有本地附件"))?;
    let entry = requested_id
        .and_then(|attachment_id| {
            entries.iter().find(|entry| {
                entry
                    .pointer("/metadata/attachment_id")
                    .or_else(|| entry.pointer("/metadata/attachmentId"))
                    .and_then(Value::as_str)
                    == Some(attachment_id)
            })
        })
        .or_else(|| entries.first())
        .ok_or_else(|| anyhow!("找不到目标设计附件"))?;
    if entry.get("verified").and_then(Value::as_bool) != Some(true) {
        bail!("目标设计附件 SHA 校验未通过");
    }
    let local_path = entry
        .get("localPath")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("目标设计附件缺少 localPath"))?;
    let local_path = PathBuf::from(local_path)
        .canonicalize()
        .context("目标设计附件不存在")?;
    if !local_path.starts_with(&root) {
        bail!("目标设计附件不在项目任务目录中");
    }
    let bytes = fs::read(&local_path)?;
    if bytes.is_empty() || bytes.len() > 16 * 1024 * 1024 {
        bail!("目标设计附件大小必须在 1..16MiB");
    }
    let mime_type = entry
        .pointer("/metadata/mime_type")
        .or_else(|| entry.pointer("/metadata/mimeType"))
        .and_then(Value::as_str)
        .unwrap_or("image/png");
    if !matches!(mime_type, "image/png" | "image/jpeg") {
        bail!("目标设计只支持 PNG/JPEG");
    }
    let name = entry
        .pointer("/metadata/display_name")
        .or_else(|| entry.pointer("/metadata/displayName"))
        .and_then(Value::as_str)
        .unwrap_or("target-design.png")
        .to_string();
    Ok(TargetDesignUpload {
        name,
        data_url: format!("data:{mime_type};base64,{}", B64.encode(bytes)),
        figma_url: None,
    })
}

fn canonical_project_root(session: &LiveUiSession) -> Result<PathBuf> {
    let value = session
        .project_root
        .as_deref()
        .ok_or_else(|| anyhow!("UI 设计 MCP 未绑定项目目录"))?;
    PathBuf::from(value)
        .canonicalize()
        .with_context(|| format!("项目目录不存在: {value}"))
}

fn read_json_artifact(path: &Path) -> Result<Value> {
    let metadata =
        fs::metadata(path).with_context(|| format!("UI 设计工件不存在: {}", path.display()))?;
    if metadata.len() > MAX_ARTIFACT_BYTES {
        bail!("UI 设计工件过大: {}", path.display());
    }
    serde_json::from_slice(&fs::read(path)?)
        .with_context(|| format!("UI 设计工件 JSON 无效: {}", path.display()))
}

fn latest_task_dir(tasks_root: &Path) -> Result<PathBuf> {
    let mut entries = fs::read_dir(tasks_root)
        .with_context(|| format!("尚无 UI 设计任务: {}", tasks_root.display()))?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false))
        .collect::<Vec<_>>();
    entries.sort_by_key(|entry| {
        entry
            .metadata()
            .and_then(|metadata| metadata.modified())
            .ok()
    });
    entries
        .pop()
        .map(|entry| entry.path())
        .ok_or_else(|| anyhow!("尚无 UI 设计任务"))
}

fn safe_new_kotlin_path(root: &Path, relative: &str) -> Result<PathBuf> {
    let relative = Path::new(relative.trim());
    if relative.is_absolute()
        || relative.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
        || relative.extension().and_then(|value| value.to_str()) != Some("kt")
    {
        bail!("relativeFile 必须是项目内的 .kt 相对路径");
    }
    let normalized = relative.to_string_lossy().replace('\\', "/");
    if !normalized.contains("/src/main/") && !normalized.contains("/src/debug/") {
        bail!("relativeFile 必须位于模块 src/main 或 src/debug 下");
    }
    Ok(root.join(relative))
}

fn required_string(arguments: &Value, field: &str) -> Result<String> {
    optional_string(arguments, field).ok_or_else(|| anyhow!("缺少 {field}"))
}

fn optional_string(arguments: &Value, field: &str) -> Option<String> {
    arguments
        .get(field)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn default_compose_source_path(profile: &Value, package_name: &str, screen_name: &str) -> String {
    let module = profile
        .pointer("/candidates/buildFiles")
        .and_then(Value::as_array)
        .and_then(|files| {
            files.iter().filter_map(Value::as_str).find(|path| {
                path == &"app/build.gradle"
                    || path == &"app/build.gradle.kts"
                    || path.ends_with("/app/build.gradle")
                    || path.ends_with("/app/build.gradle.kts")
            })
        })
        .and_then(|path| Path::new(path).parent())
        .map(|path| path.to_string_lossy().replace('\\', "/"))
        .filter(|path| !path.is_empty())
        .unwrap_or_else(|| "app".to_string());
    format!(
        "{module}/src/main/kotlin/{}/{}.kt",
        package_name.replace('.', "/"),
        screen_name
    )
}

fn validate_package(value: &str) -> Result<()> {
    if value.len() > 240
        || value.split('.').any(|segment| {
            segment.is_empty()
                || !segment.chars().enumerate().all(|(index, ch)| {
                    ch == '_' || ch.is_ascii_alphanumeric() && (index > 0 || !ch.is_ascii_digit())
                })
        })
    {
        bail!("packageName 不是合法 Kotlin package");
    }
    Ok(())
}

fn validate_identifier(value: &str, field: &str) -> Result<()> {
    if value.len() > 120
        || !value.chars().enumerate().all(|(index, ch)| {
            ch == '_' || ch.is_ascii_alphanumeric() && (index > 0 || !ch.is_ascii_digit())
        })
    {
        bail!("{field} 不是合法 Kotlin 标识符");
    }
    Ok(())
}

fn validate_screen_id(value: &str) -> Result<()> {
    if value.len() > 160
        || !value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-'))
    {
        bail!("screenId 只允许字母、数字、点、下划线和短横线");
    }
    Ok(())
}

fn safe_id(value: &str) -> String {
    value
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_'))
        .take(120)
        .collect()
}

fn compose_source(package_name: &str, screen_name: &str, screen_id: &str) -> String {
    format!(
        r#"package {package_name}

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.tooling.preview.Preview

data class {screen_name}UiState(
    val title: String = "{screen_name}",
)

@Composable
fun {screen_name}(
    state: {screen_name}UiState = {screen_name}UiState(),
    modifier: Modifier = Modifier,
) {{
    Box(
        modifier = modifier
            .fillMaxSize()
            .testTag("{screen_id}.root"),
        contentAlignment = Alignment.Center,
    ) {{
        Text(text = state.title)
    }}
}}

@Preview(showBackground = true)
@Composable
private fun {screen_name}Preview() {{
    {screen_name}()
}}
"#
    )
}

#[cfg(test)]
mod scaffold_tests {
    use super::*;

    #[test]
    fn derives_compose_path_from_profile_without_source_scan() {
        let profile = json!({
            "candidates": { "buildFiles": ["android/app/build.gradle.kts"] }
        });
        assert_eq!(
            default_compose_source_path(&profile, "com.example.checkout", "CheckoutScreen"),
            "android/app/src/main/kotlin/com/example/checkout/CheckoutScreen.kt"
        );
    }
}
