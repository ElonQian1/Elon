use std::fs;
use std::path::{Component, Path, PathBuf};

use anyhow::{anyhow, bail, Context, Result};
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use serde_json::{json, Value};

use super::broker::LiveUiSession;
use super::ui_ir::TargetDesignUpload;

const MAX_ARTIFACT_BYTES: u64 = 512 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AndroidUiToolkit {
    Compose,
    Views,
}

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
    let profile = project_profile(session)?;
    if profile.pointer("/capabilities/jetpackCompose") != Some(&Value::Bool(true)) {
        bail!("兼容工具只允许已确认的 Compose 项目；空白或混合项目请使用统一 Android 页面脚手架");
    }
    let mut arguments = arguments.clone();
    arguments["uiToolkit"] = Value::String("COMPOSE".to_string());
    create_android_screen_scaffold(session, &arguments)
}

pub(crate) fn create_android_screen_scaffold(
    session: &LiveUiSession,
    arguments: &Value,
) -> Result<Value> {
    let root = canonical_project_root(session)?;
    let profile = project_profile(session)?;
    match resolve_android_ui_toolkit(&profile, arguments)? {
        AndroidUiToolkit::Compose => create_compose_scaffold(&root, &profile, arguments),
        AndroidUiToolkit::Views => create_view_scaffold(&root, &profile, arguments),
    }
}

fn create_compose_scaffold(root: &Path, profile: &Value, arguments: &Value) -> Result<Value> {
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
    ensure_scaffold_contract_absent(root, &screen_id)?;
    let target = safe_new_kotlin_path(root, &relative_file)?;
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
    let stable_node_ids = stable_node_ids(&screen_id);
    let toolkit_confirmed =
        profile.pointer("/capabilities/jetpackCompose") == Some(&Value::Bool(true));
    let contract_path = write_scaffold_contract(
        root,
        &screen_id,
        json!({
            "schemaVersion": 2,
            "screenId": screen_id,
            "screenName": composable_name,
            "uiToolkit": "COMPOSE",
            "sourceFiles": [target.strip_prefix(root).unwrap_or(&target)],
            "workflowStatus": "SCAFFOLDED",
            "rendererStatus": "NEEDS_BUILD_AND_NAVIGATION",
            "sourceBindingStatus": "NEEDS_COMPOSE_RUNTIME_ADAPTER",
            "projectToolkitConfirmed": toolkit_confirmed,
            "toolkitSetupRequired": !toolkit_confirmed,
            "stableNodeIds": stable_node_ids,
            "requiredNextActions": [
                "Use Codex to replace the placeholder with the target business structure",
                "Apply the project theme and reusable components from project-ui-profile.json",
                "Register navigation and a deterministic Preview Host scenario",
                "Add Compose Runtime style bindings for the editable nodes",
                "Build and reconnect the real Android renderer before visual fitting"
            ]
        }),
    )?;
    Ok(json!({
        "created": true,
        "sourceFile": target,
        "contractFile": contract_path,
        "screenId": screen_id,
        "screenName": composable_name,
        "uiToolkit": "COMPOSE",
        "workflowStatus": "SCAFFOLDED",
        "runtimeReady": false,
        "nextPhase": "CODEX_STRUCTURE"
    }))
}

fn create_view_scaffold(root: &Path, profile: &Value, arguments: &Value) -> Result<Value> {
    let screen_name = required_string(arguments, "screenName")?;
    let screen_id = required_string(arguments, "screenId")?;
    if screen_name.chars().count() > 160 {
        bail!("screenName 最多 160 个字符");
    }
    validate_screen_id(&screen_id)?;
    ensure_scaffold_contract_absent(root, &screen_id)?;
    let layout_name = optional_string(arguments, "layoutName")
        .unwrap_or_else(|| format!("screen_{}", resource_name(&screen_id)));
    validate_resource_name(&layout_name, "layoutName")?;
    let relative_file = optional_string(arguments, "relativeFile")
        .unwrap_or_else(|| default_view_layout_path(profile, &layout_name));
    let target = safe_new_layout_path(root, &relative_file)?;
    if target.exists() {
        bail!("目标页面已存在，脚手架禁止覆盖: {}", target.display());
    }
    let node_ids = stable_node_ids(&screen_id);
    let source = view_layout_source(&screen_name, &resource_name(&screen_id));
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&target, source)?;
    let contract_path = write_scaffold_contract(
        root,
        &screen_id,
        json!({
            "schemaVersion": 2,
            "screenId": screen_id,
            "screenName": screen_name,
            "uiToolkit": "VIEWS",
            "layoutName": layout_name,
            "sourceFiles": [target.strip_prefix(root).unwrap_or(&target)],
            "workflowStatus": "SCAFFOLDED",
            "rendererStatus": "NEEDS_BUILD_AND_NAVIGATION",
            "sourceBindingStatus": "XML_RESOURCE_IDS_READY",
            "stableNodeIds": node_ids,
            "requiredNextActions": [
                "Use Codex to replace the placeholder with the target business structure",
                "Inflate this layout from the target Activity or Fragment and register navigation",
                "Add deterministic Preview Host data for normal/loading/empty/error as needed",
                "Build and reconnect the real Android renderer before visual fitting"
            ]
        }),
    )?;
    Ok(json!({
        "created": true,
        "sourceFile": target,
        "contractFile": contract_path,
        "screenId": screen_id,
        "screenName": screen_name,
        "layoutName": layout_name,
        "uiToolkit": "VIEWS",
        "workflowStatus": "SCAFFOLDED",
        "runtimeReady": false,
        "nextPhase": "CODEX_STRUCTURE"
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

fn safe_new_layout_path(root: &Path, relative: &str) -> Result<PathBuf> {
    let relative = Path::new(relative.trim());
    if relative.is_absolute()
        || relative.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
        || relative.extension().and_then(|value| value.to_str()) != Some("xml")
    {
        bail!("relativeFile 必须是项目内的 .xml 相对路径");
    }
    let normalized = relative.to_string_lossy().replace('\\', "/");
    if !normalized.contains("/src/main/res/layout/") {
        bail!("View relativeFile 必须位于模块 src/main/res/layout 下");
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
    let module = default_android_module(profile);
    format!(
        "{module}/src/main/kotlin/{}/{}.kt",
        package_name.replace('.', "/"),
        screen_name
    )
}

fn default_view_layout_path(profile: &Value, layout_name: &str) -> String {
    format!(
        "{}/src/main/res/layout/{layout_name}.xml",
        default_android_module(profile)
    )
}

fn default_android_module(profile: &Value) -> String {
    profile
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
        .unwrap_or_else(|| "app".to_string())
}

fn resolve_android_ui_toolkit(profile: &Value, arguments: &Value) -> Result<AndroidUiToolkit> {
    let explicit = optional_string(arguments, "uiToolkit");
    let requested = explicit
        .clone()
        .unwrap_or_else(|| {
            profile
                .pointer("/capabilities/preferredAndroidUiToolkit")
                .and_then(Value::as_str)
                .unwrap_or("UNKNOWN")
                .to_string()
        })
        .to_ascii_uppercase();
    let compose = profile.pointer("/capabilities/jetpackCompose") == Some(&Value::Bool(true));
    let views = profile.pointer("/capabilities/androidViews") == Some(&Value::Bool(true));
    let android_project = is_android_project_profile(profile);
    match requested.as_str() {
        "COMPOSE" if compose || explicit.is_some() && android_project => {
            Ok(AndroidUiToolkit::Compose)
        }
        "VIEWS" | "VIEW" | "XML" if views || explicit.is_some() && android_project => {
            Ok(AndroidUiToolkit::Views)
        }
        "COMPOSE" | "VIEWS" | "VIEW" | "XML" => {
            bail!("UI Profile 没有识别到 Android 工程，不能安全生成页面")
        }
        "HYBRID" => bail!("混合项目必须显式传 uiToolkit=COMPOSE 或 VIEWS，禁止静默猜测"),
        _ if compose && !views => Ok(AndroidUiToolkit::Compose),
        _ if views && !compose => Ok(AndroidUiToolkit::Views),
        _ if compose && views => {
            bail!("混合项目必须显式传 uiToolkit=COMPOSE 或 VIEWS，禁止静默猜测")
        }
        _ => bail!("UI Profile 没有识别到 Compose 或 View/XML，不能安全生成 Android 页面"),
    }
}

pub(crate) fn is_android_project_profile(profile: &Value) -> bool {
    profile
        .pointer("/android/namespace")
        .and_then(Value::as_str)
        .is_some_and(|value| !value.trim().is_empty())
        || profile
            .pointer("/android/applicationId")
            .and_then(Value::as_str)
            .is_some_and(|value| !value.trim().is_empty())
        || profile.pointer("/capabilities/jetpackCompose") == Some(&Value::Bool(true))
        || profile.pointer("/capabilities/androidViews") == Some(&Value::Bool(true))
}

fn write_scaffold_contract(root: &Path, screen_id: &str, contract: Value) -> Result<PathBuf> {
    let contract_dir = root.join(".elon/ui-design/generated");
    fs::create_dir_all(&contract_dir)?;
    let contract_path = contract_dir.join(format!("{}.scaffold.json", safe_id(screen_id)));
    fs::write(&contract_path, serde_json::to_vec_pretty(&contract)?)?;
    Ok(contract_path)
}

fn ensure_scaffold_contract_absent(root: &Path, screen_id: &str) -> Result<()> {
    let contract_path = root
        .join(".elon/ui-design/generated")
        .join(format!("{}.scaffold.json", safe_id(screen_id)));
    if contract_path.exists() {
        bail!(
            "页面脚手架契约已存在，禁止覆盖: {}",
            contract_path.display()
        );
    }
    Ok(())
}

fn stable_node_ids(screen_id: &str) -> Value {
    json!({
        "root": format!("{screen_id}.root"),
        "title": format!("{screen_id}.title"),
        "primaryAction": format!("{screen_id}.primary_action"),
    })
}

fn resource_name(value: &str) -> String {
    let mut result = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect::<String>();
    while result.contains("__") {
        result = result.replace("__", "_");
    }
    result = result.trim_matches('_').to_string();
    if result.chars().next().is_some_and(|ch| ch.is_ascii_digit()) {
        result.insert_str(0, "ui_");
    }
    if result.is_empty() {
        "ui_screen".to_string()
    } else {
        result
    }
}

fn validate_resource_name(value: &str, field: &str) -> Result<()> {
    if value.len() > 120
        || value.is_empty()
        || value.chars().next().is_some_and(|ch| ch.is_ascii_digit())
        || !value
            .chars()
            .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_')
    {
        bail!("{field} 必须是小写 Android resource name");
    }
    Ok(())
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

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Button
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.TextUnit
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp

data class {screen_name}UiState(
    val title: String = "{screen_name}",
    val primaryActionText: String = "继续",
)

data class {screen_name}Style(
    val contentPadding: Dp = 24.dp,
    val itemSpacing: Dp = 16.dp,
    val titleSize: TextUnit = 24.sp,
    val primaryActionHeight: Dp = 52.dp,
    val primaryActionRadius: Dp = 14.dp,
)

@Composable
fun {screen_name}(
    state: {screen_name}UiState = {screen_name}UiState(),
    style: {screen_name}Style = {screen_name}Style(),
    onPrimaryAction: () -> Unit = {{}},
    modifier: Modifier = Modifier,
) {{
    Column(
        modifier = modifier
            .fillMaxSize()
            .testTag("{screen_id}.root")
            .padding(style.contentPadding),
        verticalArrangement = Arrangement.spacedBy(style.itemSpacing),
    ) {{
        Text(
            text = state.title,
            modifier = Modifier.testTag("{screen_id}.title"),
            fontSize = style.titleSize,
        )
        Button(
            onClick = onPrimaryAction,
            modifier = Modifier
                .fillMaxWidth()
                .height(style.primaryActionHeight)
                .testTag("{screen_id}.primary_action"),
            shape = RoundedCornerShape(style.primaryActionRadius),
        ) {{
            Text(state.primaryActionText)
        }}
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

fn view_layout_source(screen_name: &str, resource_prefix: &str) -> String {
    let title = xml_escape(screen_name);
    format!(
        r#"<?xml version="1.0" encoding="utf-8"?>
<LinearLayout xmlns:android="http://schemas.android.com/apk/res/android"
    android:id="@+id/{resource_prefix}_root"
    android:layout_width="match_parent"
    android:layout_height="match_parent"
    android:background="?android:attr/colorBackground"
    android:orientation="vertical"
    android:padding="24dp">

    <TextView
        android:id="@+id/{resource_prefix}_title"
        android:layout_width="match_parent"
        android:layout_height="wrap_content"
        android:text="{title}"
        android:textColor="?android:attr/textColorPrimary"
        android:textSize="24sp"
        android:textStyle="bold" />

    <Button
        android:id="@+id/{resource_prefix}_primary_action"
        android:layout_width="match_parent"
        android:layout_height="52dp"
        android:layout_marginTop="16dp"
        android:text="继续" />

</LinearLayout>
"#
    )
}

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
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

    #[test]
    fn derives_view_layout_path_and_stable_resource_prefix() {
        let profile = json!({
            "candidates": { "buildFiles": ["android/app/build.gradle"] }
        });
        assert_eq!(
            default_view_layout_path(&profile, "screen_checkout"),
            "android/app/src/main/res/layout/screen_checkout.xml"
        );
        assert_eq!(resource_name("checkout.pay-button"), "checkout_pay_button");
    }

    #[test]
    fn hybrid_project_requires_explicit_toolkit() {
        let profile = json!({
            "capabilities": {
                "jetpackCompose": true,
                "androidViews": true,
                "preferredAndroidUiToolkit": "HYBRID"
            }
        });
        let error = resolve_android_ui_toolkit(&profile, &json!({})).unwrap_err();
        assert!(error.to_string().contains("必须显式"));
        assert_eq!(
            resolve_android_ui_toolkit(&profile, &json!({"uiToolkit":"VIEWS"})).unwrap(),
            AndroidUiToolkit::Views
        );
    }

    #[test]
    fn blank_android_project_accepts_explicit_first_toolkit() {
        let profile = json!({
            "android": {"namespace":"com.example.blank"},
            "capabilities": {
                "jetpackCompose": false,
                "androidViews": false,
                "preferredAndroidUiToolkit": "UNKNOWN"
            }
        });
        assert_eq!(
            resolve_android_ui_toolkit(&profile, &json!({"uiToolkit":"COMPOSE"})).unwrap(),
            AndroidUiToolkit::Compose
        );
        assert!(resolve_android_ui_toolkit(&profile, &json!({})).is_err());
    }

    #[test]
    fn creates_view_scaffold_with_runtime_addressable_resource_ids() {
        let root = std::env::temp_dir().join(format!(
            "elon_view_scaffold_{}",
            uuid::Uuid::new_v4().simple()
        ));
        fs::create_dir_all(&root).unwrap();
        let profile = json!({
            "capabilities": {
                "jetpackCompose": false,
                "androidViews": true,
                "preferredAndroidUiToolkit": "VIEWS"
            },
            "candidates": { "buildFiles": ["android/app/build.gradle"] }
        });

        let result = create_view_scaffold(
            &root,
            &profile,
            &json!({"screenName":"结算页", "screenId":"checkout.pay"}),
        )
        .unwrap();

        assert_eq!(result["uiToolkit"], "VIEWS");
        let layout = root.join("android/app/src/main/res/layout/screen_checkout_pay.xml");
        let source = fs::read_to_string(layout).unwrap();
        assert!(source.contains("@+id/checkout_pay_root"));
        assert!(source.contains("@+id/checkout_pay_primary_action"));
        let contract =
            fs::read_to_string(root.join(".elon/ui-design/generated/checkoutpay.scaffold.json"))
                .unwrap();
        assert!(contract.contains("XML_RESOURCE_IDS_READY"));
        fs::remove_dir_all(root).unwrap();
    }
}
