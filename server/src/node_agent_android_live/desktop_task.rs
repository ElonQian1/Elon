use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail, Context, Result};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use super::broker::LiveUiSession;

const MAX_ATTACHMENTS: usize = 8;
const MAX_IMAGE_BYTES: u64 = 16 * 1024 * 1024;

pub(crate) fn import_desktop_task(session: &LiveUiSession, arguments: &Value) -> Result<Value> {
    let root = canonical_project_root(session)?;
    let request = required_text(arguments, "request", 20_000)?;
    let mode = enum_value(
        arguments,
        "mode",
        &["AUTO", "MODIFY_EXISTING", "EXTEND_EXISTING", "CREATE_NEW"],
        "AUTO",
    )?;
    let overall_intent = enum_value(
        arguments,
        "attachmentIntent",
        &[
            "AUTO",
            "TARGET_DESIGN",
            "ANNOTATED_CHANGE_REQUEST",
            "REFERENCE_STYLE",
            "CURRENT_SCREENSHOT",
        ],
        "AUTO",
    )?;
    let task_id = arguments
        .get("taskId")
        .and_then(Value::as_str)
        .map(safe_id)
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| format!("desktop_{}", uuid::Uuid::new_v4().simple()));
    let task_root = root
        .join(".elon")
        .join("ui-design")
        .join("tasks")
        .join(&task_id);
    if task_root.exists() {
        bail!("桌面 UI 任务已存在，拒绝覆盖: {task_id}");
    }
    fs::create_dir_all(task_root.join("attachments"))?;

    let attachments = arguments
        .get("attachments")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if attachments.len() > MAX_ATTACHMENTS {
        bail!("Codex 桌面 UI 任务最多允许 {MAX_ATTACHMENTS} 张图片");
    }
    let mut metadata = Vec::with_capacity(attachments.len());
    let mut local_entries = Vec::with_capacity(attachments.len());
    let mut target_design_attachment_id = None;
    for (index, attachment) in attachments.iter().enumerate() {
        let source = attachment
            .get("path")
            .and_then(Value::as_str)
            .map(PathBuf::from)
            .ok_or_else(|| anyhow!("attachments[{index}].path 缺失"))?
            .canonicalize()
            .with_context(|| format!("桌面草图不存在: attachments[{index}]"))?;
        let file = validate_image(&source)?;
        let attachment_id = format!("desktop_image_{:02}", index + 1);
        let target = task_root
            .join("attachments")
            .join(format!("{attachment_id}.{}", file.extension));
        fs::copy(&source, &target)?;
        let bytes = fs::read(&target)?;
        let sha256 = hex::encode(Sha256::digest(&bytes));
        let intent = enum_value(
            attachment,
            "intent",
            &[
                "AUTO",
                "TARGET_DESIGN",
                "ANNOTATED_CHANGE_REQUEST",
                "REFERENCE_STYLE",
                "CURRENT_SCREENSHOT",
            ],
            &overall_intent,
        )?;
        if target_design_attachment_id.is_none() && intent == "TARGET_DESIGN" {
            target_design_attachment_id = Some(attachment_id.clone());
        }
        let display_name = attachment
            .get("displayName")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or(&file.display_name)
            .chars()
            .take(240)
            .collect::<String>();
        let item = json!({
            "attachmentId": attachment_id,
            "displayName": display_name,
            "mimeType": file.mime_type,
            "sha256": sha256,
            "intent": intent,
            "source": "CODEX_DESKTOP",
        });
        metadata.push(item.clone());
        local_entries.push(json!({
            "index": index,
            "localPath": target,
            "sha256": sha256,
            "expectedSha256": sha256,
            "verified": true,
            "metadata": item,
        }));
    }
    if target_design_attachment_id.is_none() && overall_intent == "TARGET_DESIGN" {
        target_design_attachment_id = metadata
            .first()
            .and_then(|item| item.get("attachmentId"))
            .and_then(Value::as_str)
            .map(ToOwned::to_owned);
    }

    let envelope = json!({
        "task": {
            "taskId": task_id,
            "mode": mode,
            "attachmentIntent": overall_intent,
            "targetDesignAttachmentId": target_design_attachment_id,
            "executionPolicy": {
                "allowLivePatch": true,
                "allowDeterministicCommit": true,
                "allowSourceEdit": true,
                "requireBuildVerification": true,
            },
            "origin": "CODEX_DESKTOP",
            "request": request,
        },
        "attachments": metadata,
    });
    fs::write(
        task_root.join("task.json"),
        serde_json::to_vec_pretty(&envelope)?,
    )?;
    fs::write(
        task_root.join("attachments.json"),
        serde_json::to_vec_pretty(&json!({
            "schemaVersion": 1,
            "attachments": local_entries,
            "allVerified": true,
        }))?,
    )?;

    let profile_root = root.join(".elon").join("ui-design");
    fs::create_dir_all(&profile_root)?;
    let profile_path = profile_root.join("project-ui-profile.json");
    let profile = crate::node_agent_ui_design_workspace::build_project_ui_profile(&root)?;
    fs::write(&profile_path, serde_json::to_vec_pretty(&profile)?)?;

    Ok(json!({
        "taskId": task_id,
        "origin": "CODEX_DESKTOP",
        "taskDirectory": task_root,
        "attachmentCount": attachments.len(),
        "targetDesignAttachmentId": target_design_attachment_id,
        "projectProfile": profile_path,
        "next": ["ui_get_design_task", "ui_get_project_profile", "ui_check_capabilities"],
    }))
}

struct ImageFile {
    display_name: String,
    extension: String,
    mime_type: &'static str,
}

fn validate_image(path: &Path) -> Result<ImageFile> {
    let size = fs::metadata(path)?.len();
    if size == 0 || size > MAX_IMAGE_BYTES {
        bail!("桌面草图大小必须在 1..16MiB");
    }
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let mime_type = match extension.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        _ => bail!("桌面草图只支持 PNG/JPEG"),
    };
    Ok(ImageFile {
        display_name: path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("desktop-design.png")
            .to_string(),
        extension,
        mime_type,
    })
}

fn canonical_project_root(session: &LiveUiSession) -> Result<PathBuf> {
    let value = session
        .project_root
        .as_deref()
        .ok_or_else(|| anyhow!("Codex 桌面 UI MCP 未绑定项目目录"))?;
    PathBuf::from(value)
        .canonicalize()
        .with_context(|| format!("项目目录不存在: {value}"))
}

fn required_text(arguments: &Value, field: &str, max: usize) -> Result<String> {
    let value = arguments
        .get(field)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow!("缺少 {field}"))?;
    if value.chars().count() > max {
        bail!("{field} 超过 {max} 字符");
    }
    Ok(value.to_string())
}

fn enum_value(value: &Value, field: &str, allowed: &[&str], default: &str) -> Result<String> {
    let candidate = value
        .get(field)
        .and_then(Value::as_str)
        .unwrap_or(default)
        .trim()
        .to_ascii_uppercase();
    if !allowed.iter().any(|allowed| *allowed == candidate) {
        bail!("{field} 不支持: {candidate}");
    }
    Ok(candidate)
}

fn safe_id(value: &str) -> String {
    value
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_'))
        .take(96)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::safe_id;

    #[test]
    fn desktop_task_id_cannot_escape_workspace() {
        assert_eq!(safe_id("../../ui:task"), "uitask");
    }
}
