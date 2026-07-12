use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use ignore::WalkBuilder;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

const TASK_MARKER_BEGIN: &str = "<elon-ui-design-task version=\"1\">";
const TASK_MARKER_END: &str = "</elon-ui-design-task>";
const MAX_SCANNED_FILES: usize = 4_000;
const MAX_CANDIDATES: usize = 80;

pub(crate) fn prepare_ui_design_workspace(
    prompt: String,
    cwd: Option<&str>,
    resolved_args: &[String],
) -> Result<String> {
    let Some(envelope) = task_envelope(&prompt) else {
        return Ok(prompt);
    };
    let cwd = cwd.ok_or_else(|| anyhow!("UI 设计任务缺少会话工作目录"))?;
    let project_root = PathBuf::from(cwd)
        .canonicalize()
        .with_context(|| format!("UI 设计任务工作目录不可用: {cwd}"))?;
    let task_id = safe_task_id(
        envelope
            .pointer("/task/task_id")
            .or_else(|| envelope.pointer("/task/taskId"))
            .and_then(Value::as_str)
            .unwrap_or("ui_design_task"),
    );
    let task_root = project_root
        .join(".elon")
        .join("ui-design")
        .join("tasks")
        .join(task_id);
    fs::create_dir_all(&task_root)?;

    let task_path = task_root.join("task.json");
    fs::write(&task_path, serde_json::to_vec_pretty(&envelope)?)?;
    let attachment_manifest = materialize_attachment_manifest(
        &task_root,
        envelope.get("attachments"),
        resolved_args,
    )?;
    let attachment_path = task_root.join("attachments.json");
    fs::write(
        &attachment_path,
        serde_json::to_vec_pretty(&attachment_manifest)?,
    )?;

    let profile = build_project_ui_profile(&project_root)?;
    let profile_root = project_root.join(".elon").join("ui-design");
    fs::create_dir_all(&profile_root)?;
    let profile_path = profile_root.join("project-ui-profile.json");
    fs::write(&profile_path, serde_json::to_vec_pretty(&profile)?)?;

    Ok(format!(
        "{prompt}\n\nNode-prepared UI design artifacts (read these small indexes before searching source):\n\
         - structured task: {}\n\
         - local attachment manifest: {}\n\
         - cached project UI profile: {}\n\
         For CREATE_NEW, use the profile's component/theme/navigation candidates to create a Preview-first screen. Do not scan the whole repository unless these indexes are insufficient.",
        task_path.display(),
        attachment_path.display(),
        profile_path.display(),
    ))
}

fn task_envelope(prompt: &str) -> Option<Value> {
    let (_, rest) = prompt.rsplit_once(TASK_MARKER_BEGIN)?;
    let (json, _) = rest.split_once(TASK_MARKER_END)?;
    serde_json::from_str(json.trim()).ok()
}

fn materialize_attachment_manifest(
    task_root: &Path,
    metadata: Option<&Value>,
    resolved_args: &[String],
) -> Result<Value> {
    let source_paths = cli_attachment_paths(resolved_args);
    let metadata = metadata
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let attachment_root = task_root.join("attachments");
    fs::create_dir_all(&attachment_root)?;
    let mut entries = Vec::new();
    for (index, source) in source_paths.into_iter().enumerate() {
        let source = source.canonicalize().with_context(|| {
            format!("UI 设计附件临时文件不存在: {}", source.display())
        })?;
        let extension = source
            .extension()
            .and_then(|value| value.to_str())
            .filter(|value| value.len() <= 8)
            .unwrap_or("bin");
        let target = attachment_root.join(format!("image_{:02}.{extension}", index + 1));
        fs::copy(&source, &target)?;
        let bytes = fs::read(&target)?;
        let actual_sha = hex::encode(Sha256::digest(&bytes));
        let expected = metadata.get(index).cloned().unwrap_or(Value::Null);
        let expected_sha = expected.get("sha256").and_then(Value::as_str);
        let verified = expected_sha
            .filter(|value| !value.trim().is_empty())
            .map(|value| value.eq_ignore_ascii_case(&actual_sha))
            .unwrap_or(true);
        entries.push(json!({
            "index": index,
            "localPath": target,
            "sha256": actual_sha,
            "expectedSha256": expected_sha,
            "verified": verified,
            "metadata": expected,
        }));
    }
    let all_verified = entries
        .iter()
        .all(|entry| entry["verified"].as_bool() == Some(true));
    Ok(json!({
        "schemaVersion": 1,
        "attachments": entries,
        "allVerified": all_verified,
    }))
}

fn cli_attachment_paths(args: &[String]) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    let mut index = 0;
    while index + 1 < args.len() {
        if matches!(args[index].as_str(), "-i" | "--attachment") {
            let path = PathBuf::from(&args[index + 1]);
            if path.is_absolute() && path.is_file() {
                paths.push(path);
            }
            index += 2;
        } else {
            index += 1;
        }
    }
    paths
}

fn build_project_ui_profile(root: &Path) -> Result<Value> {
    let mut scanned = 0usize;
    let mut compose = false;
    let mut android_views = false;
    let mut pwa = false;
    let mut themes = Vec::new();
    let mut components = Vec::new();
    let mut navigation = Vec::new();
    let mut previews = Vec::new();
    let mut build_files = Vec::new();

    let walker = WalkBuilder::new(root)
        .hidden(false)
        .git_ignore(true)
        .git_exclude(true)
        .max_depth(Some(10))
        .build();
    for entry in walker.filter_map(|entry| entry.ok()) {
        if scanned >= MAX_SCANNED_FILES {
            break;
        }
        let path = entry.path();
        if !entry.file_type().is_some_and(|kind| kind.is_file()) || ignored_dir(path) {
            continue;
        }
        let relative = path.strip_prefix(root).unwrap_or(path);
        let relative_text = relative.to_string_lossy().replace('\\', "/");
        let name = path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase();
        let extension = path
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase();
        if is_build_file(&name) {
            push_unique(&mut build_files, &relative_text);
        }
        if name == "package.json" || name.ends_with(".webmanifest") {
            pwa = true;
        }
        if extension == "xml" && relative_text.contains("/res/layout/") {
            android_views = true;
            push_unique(&mut components, &relative_text);
        }
        if !matches!(extension.as_str(), "kt" | "java" | "xml" | "json" | "kts" | "gradle" | "ts" | "tsx") {
            continue;
        }
        scanned += 1;
        let content = read_prefix(path, 192 * 1024).unwrap_or_default();
        if content.contains("androidx.compose") || content.contains("@Composable") {
            compose = true;
        }
        if content.contains("@Composable") && looks_like_component_file(&name) {
            push_unique(&mut components, &relative_text);
        }
        if content.contains("@Preview") || name.contains("preview") {
            push_unique(&mut previews, &relative_text);
        }
        if content.contains("NavHost")
            || content.contains("NavController")
            || name.contains("navigation")
            || name.starts_with("nav_")
        {
            push_unique(&mut navigation, &relative_text);
        }
        if ["theme", "color", "typography", "shape", "token", "style", "design"]
            .iter()
            .any(|marker| name.contains(marker))
        {
            push_unique(&mut themes, &relative_text);
        }
    }

    Ok(json!({
        "schemaVersion": 1,
        "projectRoot": root,
        "generatedAt": chrono::Utc::now().to_rfc3339(),
        "capabilities": {
            "jetpackCompose": compose,
            "androidViews": android_views,
            "pwa": pwa,
        },
        "candidates": {
            "buildFiles": build_files,
            "themesAndTokens": themes,
            "components": components,
            "navigation": navigation,
            "previews": previews,
        },
        "scan": {
            "filesInspected": scanned,
            "truncated": scanned >= MAX_SCANNED_FILES,
            "contentIncludedInProfile": false,
        }
    }))
}

fn ignored_dir(path: &Path) -> bool {
    path.components().any(|component| {
        matches!(
            component.as_os_str().to_string_lossy().as_ref(),
            ".git" | "target" | "build" | ".gradle" | "node_modules" | "dist"
        )
    })
}

fn is_build_file(name: &str) -> bool {
    matches!(
        name,
        "settings.gradle" | "settings.gradle.kts" | "build.gradle" | "build.gradle.kts" | "package.json"
    )
}

fn looks_like_component_file(name: &str) -> bool {
    ["screen", "view", "component", "card", "button", "dialog", "sheet", "item"]
        .iter()
        .any(|marker| name.contains(marker))
}

fn read_prefix(path: &Path, max_bytes: usize) -> Result<String> {
    let bytes = fs::read(path)?;
    let end = bytes.len().min(max_bytes);
    Ok(String::from_utf8_lossy(&bytes[..end]).to_string())
}

fn push_unique(values: &mut Vec<String>, value: &str) {
    if values.len() < MAX_CANDIDATES && !values.iter().any(|item| item == value) {
        values.push(value.to_string());
    }
}

fn safe_task_id(value: &str) -> String {
    let safe = value
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_'))
        .take(96)
        .collect::<String>();
    if safe.is_empty() {
        "ui_design_task".to_string()
    } else {
        safe
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_indexes_compose_without_embedding_source() {
        let root = std::env::temp_dir().join(format!(
            "elon_ui_profile_{}",
            uuid::Uuid::new_v4().simple()
        ));
        let source = root.join("app/src/main/kotlin/demo/CheckoutScreen.kt");
        fs::create_dir_all(source.parent().unwrap()).unwrap();
        fs::write(
            &source,
            "import androidx.compose.runtime.Composable\n@Composable fun CheckoutScreen() = Unit\n",
        )
        .unwrap();

        let profile = build_project_ui_profile(&root).unwrap();

        assert_eq!(profile["capabilities"]["jetpackCompose"], true);
        assert!(profile["candidates"]["components"]
            .as_array()
            .unwrap()
            .iter()
            .any(|value| value.as_str().unwrap().ends_with("CheckoutScreen.kt")));
        assert_eq!(profile["scan"]["contentIncludedInProfile"], false);
        fs::remove_dir_all(root).unwrap();
    }
}
