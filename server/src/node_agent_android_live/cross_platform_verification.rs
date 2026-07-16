use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail, Context, Result};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use super::broker::LiveUiSession;
use super::fit_run::workspace_fingerprint;

const CAPABILITY: &str = "CROSS_PLATFORM_STYLE_WRITEBACK";

pub(crate) fn write(session: &LiveUiSession, arguments: &Value) -> Result<Value> {
    let task = super::design_bootstrap::design_task(session, arguments)?;
    let task_id = required_text(arguments, "taskId", 128)?;
    let actual_task_id = task
        .pointer("/task/task/taskId")
        .or_else(|| task.pointer("/task/task/task_id"))
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("设计任务缺少 taskId"))?;
    if actual_task_id != task_id {
        bail!("跨端验收 taskId 与当前设计任务不一致")
    }
    let task_directory = task["taskDirectory"]
        .as_str()
        .ok_or_else(|| anyhow!("设计任务缺少 taskDirectory"))?;
    let project_root = session
        .project_root
        .as_deref()
        .ok_or_else(|| anyhow!("跨端验收未绑定项目目录"))?;
    let source_revision = workspace_fingerprint(project_root)?
        .ok_or_else(|| anyhow!("无法读取当前 Git sourceRevision"))?;
    let evidence = write_document(
        Path::new(task_directory),
        &task_id,
        &source_revision,
        arguments,
    )?;
    Ok(json!({
        "capability": CAPABILITY,
        "evidencePath": Path::new(task_directory).join("cross-platform-verification.json"),
        "evidence": evidence,
        "next": "ui_check_workflow_completion",
    }))
}

fn write_document(
    task_directory: &Path,
    task_id: &str,
    source_revision: &str,
    arguments: &Value,
) -> Result<Value> {
    let task_directory = task_directory
        .canonicalize()
        .context("跨端验收任务目录不存在")?;
    let visual_loss = finite_number(arguments, "visualLoss")?;
    let max_visual_loss = finite_number(arguments, "maxVisualLoss")?;
    if max_visual_loss < 0.0 || visual_loss < 0.0 || visual_loss > max_visual_loss {
        bail!("visualLoss 必须是 0..=maxVisualLoss 的有限数值")
    }
    require_true(arguments, "sourceWritebackVerified")?;
    require_true(arguments, "patchFreeBuildVerified")?;

    let android = read_image(arguments, "androidArtifact")?;
    let web = read_image(arguments, "webArtifact")?;
    if android.canonical_path == web.canonical_path {
        bail!("Android 与 Web 必须提供彼此独立的真实截图")
    }
    let evidence_directory = task_directory.join("evidence");
    fs::create_dir_all(&evidence_directory).context("无法创建跨端证据目录")?;
    let android_relative = PathBuf::from(format!("evidence/android.{}", android.extension));
    let web_relative = PathBuf::from(format!("evidence/web.{}", web.extension));
    crate::node_agent_atomic_file::write(&task_directory.join(&android_relative), &android.bytes)?;
    crate::node_agent_atomic_file::write(&task_directory.join(&web_relative), &web.bytes)?;

    let evidence = json!({
        "schemaVersion": 1,
        "taskId": task_id,
        "sourceRevision": source_revision,
        "androidArtifact": slash_path(&android_relative),
        "androidSha256": hex::encode(Sha256::digest(&android.bytes)),
        "webArtifact": slash_path(&web_relative),
        "webSha256": hex::encode(Sha256::digest(&web.bytes)),
        "visualLoss": visual_loss,
        "maxVisualLoss": max_visual_loss,
        "sourceWritebackVerified": true,
        "patchFreeBuildVerified": true,
    });
    let evidence_path = task_directory.join("cross-platform-verification.json");
    crate::node_agent_atomic_file::write(&evidence_path, &serde_json::to_vec_pretty(&evidence)?)?;
    super::task_completion::cross_platform_evidence(&evidence_path, task_id, Some(source_revision))
}

struct ImageArtifact {
    bytes: Vec<u8>,
    canonical_path: PathBuf,
    extension: &'static str,
}

fn read_image(arguments: &Value, field: &str) -> Result<ImageArtifact> {
    let value = required_text(arguments, field, 4_000)?;
    let canonical_path = PathBuf::from(&value)
        .canonicalize()
        .with_context(|| format!("{field} 不存在"))?;
    if !canonical_path.is_file() {
        bail!("{field} 必须是截图文件")
    }
    let bytes = fs::read(&canonical_path).with_context(|| format!("无法读取 {field}"))?;
    let extension = image_extension(&bytes)
        .ok_or_else(|| anyhow!("{field} 必须是非空 PNG、JPEG 或 WebP 截图"))?;
    Ok(ImageArtifact {
        bytes,
        canonical_path,
        extension,
    })
}

fn image_extension(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some("png")
    } else if bytes.starts_with(&[0xff, 0xd8, 0xff]) {
        Some("jpg")
    } else if bytes.len() >= 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        Some("webp")
    } else {
        None
    }
}

fn finite_number(arguments: &Value, field: &str) -> Result<f64> {
    let value = arguments
        .get(field)
        .and_then(Value::as_f64)
        .ok_or_else(|| anyhow!("缺少有限数值 {field}"))?;
    if !value.is_finite() {
        bail!("{field} 必须是有限数值")
    }
    Ok(value)
}

fn require_true(arguments: &Value, field: &str) -> Result<()> {
    if arguments.get(field).and_then(Value::as_bool) != Some(true) {
        bail!("{field} 必须为 true")
    }
    Ok(())
}

fn required_text(arguments: &Value, field: &str, max: usize) -> Result<String> {
    let value = arguments
        .get(field)
        .and_then(Value::as_str)
        .map(str::trim)
        .unwrap_or_default();
    if value.is_empty() || value.chars().count() > max {
        bail!("{field} 为空或超长")
    }
    Ok(value.to_string())
}

fn slash_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use super::write_document;
    use serde_json::json;
    use std::fs;

    #[test]
    fn writes_task_scoped_visual_evidence_for_current_revision() {
        let root = std::env::temp_dir().join(format!(
            "elon-cross-platform-writeback-{}",
            uuid::Uuid::new_v4().simple()
        ));
        let task = root.join("task");
        let source = root.join("source");
        fs::create_dir_all(&task).unwrap();
        fs::create_dir_all(&source).unwrap();
        let android = source.join("android.png");
        let web = source.join("web.png");
        fs::write(&android, b"\x89PNG\r\n\x1a\nandroid").unwrap();
        fs::write(&web, b"\x89PNG\r\n\x1a\nweb").unwrap();

        let evidence = write_document(
            &task,
            "task-1",
            "revision-1",
            &json!({
                "androidArtifact":android,
                "webArtifact":web,
                "visualLoss":0.02,
                "maxVisualLoss":0.03,
                "sourceWritebackVerified":true,
                "patchFreeBuildVerified":true
            }),
        )
        .unwrap();

        assert_eq!(evidence["taskId"], "task-1");
        assert_eq!(evidence["sourceRevision"], "revision-1");
        assert!(task
            .join(evidence["androidArtifact"].as_str().unwrap())
            .is_file());
        assert!(task
            .join(evidence["webArtifact"].as_str().unwrap())
            .is_file());
        assert!(task.join("cross-platform-verification.json").is_file());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn rejects_failed_or_non_visual_verification() {
        let root = std::env::temp_dir().join(format!(
            "elon-cross-platform-reject-{}",
            uuid::Uuid::new_v4().simple()
        ));
        fs::create_dir_all(&root).unwrap();
        let text = root.join("not-image.txt");
        fs::write(&text, "not an image").unwrap();
        let result = write_document(
            &root,
            "task-1",
            "revision-1",
            &json!({
                "androidArtifact":text,
                "webArtifact":root.join("missing.png"),
                "visualLoss":0.04,
                "maxVisualLoss":0.03,
                "sourceWritebackVerified":true,
                "patchFreeBuildVerified":true
            }),
        );
        assert!(result.is_err());
        fs::remove_dir_all(root).unwrap();
    }
}
