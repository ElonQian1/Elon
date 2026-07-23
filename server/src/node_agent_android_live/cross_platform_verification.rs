use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail, Context, Result};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use super::broker::LiveUiSession;
use super::fit_run::workspace_fingerprint;

const CAPABILITY: &str = "CROSS_PLATFORM_STYLE_WRITEBACK";

pub(crate) fn tool_input_schema() -> Value {
    json!({
        "type":"object",
        "required":["taskId","androidArtifact","sourceWritebackVerified","patchFreeBuildVerified"],
        "oneOf":[
            {
                "properties":{"verificationMode":{"enum":["VISUAL_PARITY"]}},
                "required":["webArtifact","visualLoss","maxVisualLoss"]
            },
            {
                "properties":{"verificationMode":{"const":"NO_WEB_COUNTERPART"}},
                "required":["verificationMode","repositoryEvidence"]
            }
        ],
        "properties":{
            "taskId":{"type":"string","minLength":1,"maxLength":128},
            "verificationMode":{"enum":["VISUAL_PARITY","NO_WEB_COUNTERPART"],"default":"VISUAL_PARITY"},
            "androidArtifact":{"type":"string","minLength":1,"maxLength":4000},
            "webArtifact":{"type":"string","minLength":1,"maxLength":4000},
            "visualLoss":{"type":"number","minimum":0,"maximum":1},
            "maxVisualLoss":{"type":"number","minimum":0,"maximum":1},
            "sourceWritebackVerified":{"const":true},
            "patchFreeBuildVerified":{"const":true},
            "repositoryEvidence":{
                "type":"object",
                "additionalProperties":false,
                "required":["reason","androidSourceFiles","webRoots","searchTerms"],
                "properties":{
                    "reason":{"type":"string","minLength":1,"maxLength":1000},
                    "androidSourceFiles":{"type":"array","minItems":1,"maxItems":32,"items":{"type":"string","minLength":1,"maxLength":400}},
                    "webRoots":{"type":"array","minItems":1,"maxItems":16,"items":{"type":"string","minLength":1,"maxLength":400}},
                    "searchTerms":{"type":"array","minItems":1,"maxItems":16,"items":{"type":"string","minLength":1,"maxLength":80}}
                }
            }
        }
    })
}

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
        Path::new(project_root),
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
    project_root: &Path,
    task_id: &str,
    source_revision: &str,
    arguments: &Value,
) -> Result<Value> {
    let task_directory = task_directory
        .canonicalize()
        .context("跨端验收任务目录不存在")?;
    require_true(arguments, "sourceWritebackVerified")?;
    require_true(arguments, "patchFreeBuildVerified")?;

    let android = read_image(arguments, "androidArtifact")?;
    let evidence_directory = task_directory.join("evidence");
    fs::create_dir_all(&evidence_directory).context("无法创建跨端证据目录")?;
    let android_relative = PathBuf::from(format!("evidence/android.{}", android.extension));
    crate::node_agent_atomic_file::write(&task_directory.join(&android_relative), &android.bytes)?;

    let verification_mode = arguments
        .get("verificationMode")
        .and_then(Value::as_str)
        .unwrap_or("VISUAL_PARITY");
    let mut evidence = json!({
        "schemaVersion": 2,
        "taskId": task_id,
        "sourceRevision": source_revision,
        "verificationMode": verification_mode,
        "androidArtifact": slash_path(&android_relative),
        "androidSha256": hex::encode(Sha256::digest(&android.bytes)),
        "sourceWritebackVerified": true,
        "patchFreeBuildVerified": true,
    });
    match verification_mode {
        "VISUAL_PARITY" => {
            let visual_loss = finite_number(arguments, "visualLoss")?;
            let max_visual_loss = finite_number(arguments, "maxVisualLoss")?;
            if max_visual_loss < 0.0 || visual_loss < 0.0 || visual_loss > max_visual_loss {
                bail!("visualLoss 必须是 0..=maxVisualLoss 的有限数值")
            }
            let web = read_image(arguments, "webArtifact")?;
            if android.canonical_path == web.canonical_path {
                bail!("Android 与 Web 必须提供彼此独立的真实截图")
            }
            let web_relative = PathBuf::from(format!("evidence/web.{}", web.extension));
            crate::node_agent_atomic_file::write(&task_directory.join(&web_relative), &web.bytes)?;
            evidence["webArtifact"] = json!(slash_path(&web_relative));
            evidence["webSha256"] = json!(hex::encode(Sha256::digest(&web.bytes)));
            evidence["visualLoss"] = json!(visual_loss);
            evidence["maxVisualLoss"] = json!(max_visual_loss);
        }
        "NO_WEB_COUNTERPART" => {
            evidence["repositoryEvidence"] = verify_no_web_counterpart(project_root, arguments)?;
        }
        _ => bail!("verificationMode 只允许 VISUAL_PARITY 或 NO_WEB_COUNTERPART"),
    }
    let evidence_path = task_directory.join("cross-platform-verification.json");
    crate::node_agent_atomic_file::write(&evidence_path, &serde_json::to_vec_pretty(&evidence)?)?;
    super::task_completion::cross_platform_evidence(&evidence_path, task_id, Some(source_revision))
}

fn verify_no_web_counterpart(project_root: &Path, arguments: &Value) -> Result<Value> {
    let root = project_root
        .canonicalize()
        .context("NO_WEB_COUNTERPART 项目目录不存在")?;
    let input = arguments
        .get("repositoryEvidence")
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow!("NO_WEB_COUNTERPART 缺少 repositoryEvidence"))?;
    let reason = input
        .get("reason")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty() && value.chars().count() <= 1_000)
        .ok_or_else(|| anyhow!("repositoryEvidence.reason 为空或超长"))?;
    let android_source_files = text_array(input.get("androidSourceFiles"), 1, 32, 400)?;
    let web_roots = text_array(input.get("webRoots"), 1, 16, 400)?;
    let search_terms = text_array(input.get("searchTerms"), 1, 16, 80)?;

    let mut android_sources = Vec::with_capacity(android_source_files.len());
    let mut android_text = String::new();
    for relative in &android_source_files {
        let path = tracked_file(&root, relative)?;
        let bytes = fs::read(&path)?;
        if bytes.len() > 4 * 1024 * 1024 || bytes.contains(&0) {
            bail!("Android 来源证据必须是小于 4MiB 的文本文件: {relative}");
        }
        android_text.push_str(&String::from_utf8_lossy(&bytes).to_ascii_lowercase());
        android_text.push('\n');
        android_sources.push(json!({
            "path": slash_path(Path::new(relative)),
            "sha256": hex::encode(Sha256::digest(&bytes)),
        }));
    }
    for term in &search_terms {
        if !android_text.contains(&term.to_ascii_lowercase()) {
            bail!("NO_WEB_COUNTERPART 搜索词必须能在 Android 来源证据中核对: {term}");
        }
    }

    let mut inspected_files = Vec::new();
    let mut matches = Vec::new();
    let lowered_terms = search_terms
        .iter()
        .map(|term| term.to_ascii_lowercase())
        .collect::<Vec<_>>();
    for web_root in &web_roots {
        let directory = safe_relative_path(&root, web_root)?;
        if !directory.is_dir() {
            bail!("NO_WEB_COUNTERPART webRoots 必须是存在的目录: {web_root}");
        }
        for relative in git_tracked_files(&root, web_root)? {
            let path = safe_relative_path(&root, &relative)?;
            let bytes = fs::read(&path)?;
            if bytes.len() > 4 * 1024 * 1024 || bytes.contains(&0) {
                continue;
            }
            let text = String::from_utf8_lossy(&bytes).to_ascii_lowercase();
            if lowered_terms.iter().any(|term| text.contains(term)) {
                matches.push(relative.clone());
            }
            inspected_files.push(relative);
            if inspected_files.len() > 20_000 {
                bail!("NO_WEB_COUNTERPART Web 跟踪文件超过 20000，拒绝生成不完整证据");
            }
        }
    }
    inspected_files.sort();
    inspected_files.dedup();
    matches.sort();
    matches.dedup();
    if !matches.is_empty() {
        bail!(
            "NO_WEB_COUNTERPART_REJECTED: Web 跟踪源码存在对应搜索词: {}",
            matches.join(", ")
        );
    }
    if inspected_files.is_empty() {
        bail!("NO_WEB_COUNTERPART 未检查到任何 Web 跟踪源码");
    }
    let repository_git_revision = git_text(&root, &["rev-parse", "--verify", "HEAD"])?;
    Ok(json!({
        "kind": "NO_WEB_COUNTERPART",
        "reason": reason,
        "repositoryGitRevision": repository_git_revision,
        "androidSources": android_sources,
        "webRoots": web_roots,
        "searchTerms": search_terms,
        "inspectedWebFileCount": inspected_files.len(),
        "inspectedWebFilesSha256": hex::encode(Sha256::digest(inspected_files.join("\n").as_bytes())),
        "matchingWebFiles": matches,
    }))
}

fn text_array(
    value: Option<&Value>,
    min: usize,
    max: usize,
    max_chars: usize,
) -> Result<Vec<String>> {
    let values = value
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow!("repositoryEvidence 数组字段缺失"))?;
    if !(min..=max).contains(&values.len()) {
        bail!("repositoryEvidence 数组数量必须为 {min}..{max}");
    }
    values
        .iter()
        .map(|value| {
            value
                .as_str()
                .map(str::trim)
                .filter(|value| !value.is_empty() && value.chars().count() <= max_chars)
                .map(ToOwned::to_owned)
                .ok_or_else(|| anyhow!("repositoryEvidence 包含空值或超长值"))
        })
        .collect()
}

fn tracked_file(root: &Path, relative: &str) -> Result<PathBuf> {
    let path = safe_relative_path(root, relative)?;
    if !path.is_file() {
        bail!("仓库证据文件不存在: {relative}");
    }
    git_text(root, &["ls-files", "--error-unmatch", "--", relative])?;
    Ok(path)
}

fn safe_relative_path(root: &Path, relative: &str) -> Result<PathBuf> {
    let relative_path = Path::new(relative);
    if relative_path.is_absolute()
        || relative_path
            .components()
            .any(|part| matches!(part, std::path::Component::ParentDir))
    {
        bail!("仓库证据路径必须是项目内相对路径: {relative}");
    }
    let path = root.join(relative_path).canonicalize()?;
    if !path.starts_with(root) {
        bail!("仓库证据路径越出项目目录: {relative}");
    }
    Ok(path)
}

fn git_tracked_files(root: &Path, relative: &str) -> Result<Vec<String>> {
    let output = git_bytes(root, &["ls-files", "-z", "--", relative])?;
    output
        .split(|byte| *byte == 0)
        .filter(|value| !value.is_empty())
        .map(|value| {
            std::str::from_utf8(value)
                .map(ToOwned::to_owned)
                .context("Git 返回了非 UTF-8 路径")
        })
        .collect()
}

fn git_text(root: &Path, args: &[&str]) -> Result<String> {
    Ok(String::from_utf8(git_bytes(root, args)?)?
        .trim()
        .to_string())
}

fn git_bytes(root: &Path, args: &[&str]) -> Result<Vec<u8>> {
    let output = crate::git_command_error::git_command()
        .current_dir(root)
        .args(args)
        .output()?;
    if !output.status.success() {
        bail!(
            "git {} 失败: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(output.stdout)
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
    use super::{tool_input_schema, write_document};
    use serde_json::json;
    use std::fs;

    #[test]
    fn tool_schema_exposes_no_web_counterpart_without_web_artifact() {
        let schema = tool_input_schema();
        assert_eq!(
            schema["properties"]["verificationMode"]["enum"][1],
            "NO_WEB_COUNTERPART"
        );
        assert!(schema["oneOf"][1]["required"]
            .as_array()
            .is_some_and(|fields| fields.iter().any(|field| field == "repositoryEvidence")));
        assert!(!schema["oneOf"][1]["required"]
            .as_array()
            .is_some_and(|fields| fields.iter().any(|field| field == "webArtifact")));
    }

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
            &root,
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

    #[test]
    fn writes_no_web_counterpart_from_tracked_repository_evidence() {
        let root = std::env::temp_dir().join(format!(
            "elon-no-web-counterpart-{}",
            uuid::Uuid::new_v4().simple()
        ));
        let task = root.join("task");
        let android_dir = root.join("android");
        let web_dir = root.join("pc-frontend");
        fs::create_dir_all(&task).unwrap();
        fs::create_dir_all(&android_dir).unwrap();
        fs::create_dir_all(&web_dir).unwrap();
        let screenshot = root.join("android.png");
        fs::write(&screenshot, b"\x89PNG\r\n\x1a\nandroid").unwrap();
        fs::write(
            android_dir.join("FriendSidebar.kt"),
            "fun favoriteDateSidebar() = Unit",
        )
        .unwrap();
        fs::write(web_dir.join("App.tsx"), "export const App = () => null;").unwrap();
        git(&root, &["init"]);
        git(&root, &["config", "user.email", "test@elon.local"]);
        git(&root, &["config", "user.name", "Elon Test"]);
        git(&root, &["add", "android", "pc-frontend"]);
        git(&root, &["commit", "-m", "fixture"]);

        let evidence = write_document(
            &task,
            &root,
            "task-1",
            "revision-1",
            &json!({
                "verificationMode":"NO_WEB_COUNTERPART",
                "androidArtifact":screenshot,
                "sourceWritebackVerified":true,
                "patchFreeBuildVerified":true,
                "repositoryEvidence":{
                    "reason":"好友侧栏只存在于 Android 导航。",
                    "androidSourceFiles":["android/FriendSidebar.kt"],
                    "webRoots":["pc-frontend"],
                    "searchTerms":["favoriteDateSidebar"]
                }
            }),
        )
        .unwrap();

        assert_eq!(evidence["verificationMode"], "NO_WEB_COUNTERPART");
        assert_eq!(
            evidence["repositoryEvidence"]["matchingWebFiles"],
            json!([])
        );
        assert!(evidence.get("webArtifact").is_none());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn rejects_no_web_counterpart_when_tracked_web_source_matches() {
        let root = std::env::temp_dir().join(format!(
            "elon-no-web-counterpart-drift-{}",
            uuid::Uuid::new_v4().simple()
        ));
        fs::create_dir_all(root.join("android")).unwrap();
        fs::create_dir_all(root.join("web")).unwrap();
        fs::write(root.join("android/Feature.kt"), "favoriteDateSidebar").unwrap();
        fs::write(root.join("web/Feature.ts"), "favoriteDateSidebar").unwrap();
        fs::write(root.join("android.png"), b"\x89PNG\r\n\x1a\nandroid").unwrap();
        git(&root, &["init"]);
        git(&root, &["config", "user.email", "test@elon.local"]);
        git(&root, &["config", "user.name", "Elon Test"]);
        git(&root, &["add", "android", "web"]);
        git(&root, &["commit", "-m", "fixture"]);
        let result = write_document(
            &root,
            &root,
            "task-1",
            "revision-1",
            &json!({
                "verificationMode":"NO_WEB_COUNTERPART",
                "androidArtifact":root.join("android.png"),
                "sourceWritebackVerified":true,
                "patchFreeBuildVerified":true,
                "repositoryEvidence":{
                    "reason":"应被拒绝",
                    "androidSourceFiles":["android/Feature.kt"],
                    "webRoots":["web"],
                    "searchTerms":["favoriteDateSidebar"]
                }
            }),
        );
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("NO_WEB_COUNTERPART_REJECTED"));
        fs::remove_dir_all(root).unwrap();
    }

    fn git(root: &std::path::Path, args: &[&str]) {
        assert!(crate::git_command_error::git_command()
            .current_dir(root)
            .args(args)
            .status()
            .unwrap()
            .success());
    }
}
