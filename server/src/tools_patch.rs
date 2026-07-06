// server/src/tools_patch.rs
//! Workspace-scoped patch application tool for API agents.

use anyhow::{anyhow, Result};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    io::ErrorKind,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

const MAX_APPLY_PATCH_PREVIEW_CHARS: usize = 24_000;
const MAX_PATCH_BASE_FILE_BYTES: u64 = 2 * 1024 * 1024;

pub fn apply_patch(project_root: &Path, patch: &str, check_only: bool) -> Result<String> {
    let normalized_patch = normalize_patch(patch)?;
    let touched_files = touched_files(&normalized_patch)?;
    validate_touched_files(project_root, &touched_files)?;

    let patch_file = temp_patch_file("agent-apply");
    std::fs::write(&patch_file, &normalized_patch)?;
    let cleanup = PatchFileCleanup(patch_file.clone());

    run_git_apply(project_root, &patch_file, true)?;
    if check_only {
        return Ok(format!(
            "补丁检查通过，未应用。涉及文件: {}",
            touched_files.join(", ")
        ));
    }

    run_git_apply(project_root, &patch_file, false)?;
    drop(cleanup);
    Ok(format!(
        "补丁已应用。涉及文件: {}",
        touched_files.join(", ")
    ))
}

pub fn apply_patch_diff_preview(project_root: &Path, patch: &str) -> Result<Value> {
    let normalized_patch = normalize_patch(patch)?;
    let touched_files = touched_files(&normalized_patch)?;
    validate_touched_files(project_root, &touched_files)?;
    if normalized_patch.chars().count() > MAX_APPLY_PATCH_PREVIEW_CHARS {
        return Err(anyhow!(
            "apply_patch diff preview refused: patch is too large; split it into smaller patches"
        ));
    }
    check_normalized_patch(project_root, &normalized_patch)?;
    let base_files = base_file_fingerprints(project_root, &touched_files)?;

    Ok(json!({
        "format": "unified",
        "source": "apply_patch",
        "kind": "patch",
        "preview": normalized_patch,
        "truncated": false,
        "files": touched_files,
        "patch_sha256": sha256_hex(normalized_patch.as_bytes()),
        "base_files": base_files
    }))
}

pub fn verify_apply_patch_preview_unchanged(
    project_root: &Path,
    patch: &str,
    diff: &Value,
) -> Result<()> {
    let normalized_patch = normalize_patch(patch)?;
    let expected_hash = diff
        .get("patch_sha256")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("apply_patch approval preview missing patch_sha256"))?;
    let actual_hash = sha256_hex(normalized_patch.as_bytes());
    if expected_hash != actual_hash {
        return Err(anyhow!(
            "apply_patch content changed since approval preview"
        ));
    }

    let touched_files = touched_files(&normalized_patch)?;
    validate_touched_files(project_root, &touched_files)?;
    let expected_files = string_array_field(diff, "files")?;
    if expected_files != touched_files {
        return Err(anyhow!(
            "apply_patch file list changed since approval preview"
        ));
    }

    let expected_base_files = diff
        .get("base_files")
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow!("apply_patch approval preview missing base_files"))?;
    let current_base_files = base_file_fingerprints(project_root, &touched_files)?;
    if expected_base_files.as_slice() != current_base_files.as_slice() {
        return Err(anyhow!("apply_patch base changed since approval preview"));
    }

    check_normalized_patch(project_root, &normalized_patch)
}

fn normalize_patch(patch: &str) -> Result<String> {
    let stripped = strip_fenced_diff(patch);
    let trimmed = stripped.trim();
    if trimmed.is_empty() {
        return Err(anyhow!("补丁不能为空"));
    }
    if !trimmed.contains("@@") || !(trimmed.contains("diff --git ") || trimmed.contains("--- ")) {
        return Err(anyhow!("补丁必须是 unified diff 格式"));
    }
    if trimmed
        .lines()
        .any(|line| line.starts_with("Binary files "))
    {
        return Err(anyhow!("不支持二进制补丁"));
    }
    Ok(format!("{}\n", trimmed.replace("\r\n", "\n")))
}

fn check_normalized_patch(project_root: &Path, normalized_patch: &str) -> Result<()> {
    let patch_file = temp_patch_file("agent-preview");
    std::fs::write(&patch_file, normalized_patch)?;
    let _cleanup = PatchFileCleanup(patch_file.clone());
    run_git_apply(project_root, &patch_file, true)
}

fn strip_fenced_diff(value: &str) -> String {
    let trimmed = value.trim();
    if !trimmed.starts_with("```") {
        return trimmed.to_string();
    }
    let mut lines = trimmed.lines();
    let _fence = lines.next();
    let mut body = Vec::new();
    for line in lines {
        if line.trim() == "```" {
            break;
        }
        body.push(line);
    }
    if body.is_empty() {
        trimmed.to_string()
    } else {
        body.join("\n")
    }
}

fn touched_files(patch: &str) -> Result<Vec<String>> {
    let mut files = Vec::new();
    for line in patch.lines() {
        if let Some((old_path, new_path)) = parse_diff_git_line(line) {
            push_patch_path(&mut files, old_path);
            push_patch_path(&mut files, new_path);
            continue;
        }
        if let Some(path) = line.strip_prefix("--- ").and_then(parse_marker_path) {
            push_patch_path(&mut files, Some(path));
            continue;
        }
        if let Some(path) = line.strip_prefix("+++ ").and_then(parse_marker_path) {
            push_patch_path(&mut files, Some(path));
        }
    }
    files.sort();
    files.dedup();
    if files.is_empty() {
        return Err(anyhow!("无法从补丁中识别目标文件"));
    }
    Ok(files)
}

fn parse_diff_git_line(line: &str) -> Option<(Option<String>, Option<String>)> {
    let rest = line.strip_prefix("diff --git ")?;
    let mut parts = rest.split_whitespace();
    let old_path = parts.next().and_then(normalize_patch_path);
    let new_path = parts.next().and_then(normalize_patch_path);
    Some((old_path, new_path))
}

fn parse_marker_path(raw: &str) -> Option<String> {
    let path = raw.split_whitespace().next()?;
    normalize_patch_path(path)
}

fn normalize_patch_path(raw: &str) -> Option<String> {
    let path = raw.trim().trim_matches('"');
    if path == "/dev/null" {
        return None;
    }
    path.strip_prefix("a/")
        .or_else(|| path.strip_prefix("b/"))
        .or(Some(path))
        .map(str::to_string)
}

fn push_patch_path(files: &mut Vec<String>, path: Option<String>) {
    if let Some(path) = path {
        if !path.trim().is_empty() {
            files.push(path);
        }
    }
}

fn validate_touched_files(project_root: &Path, files: &[String]) -> Result<()> {
    let canonical_root = project_root
        .canonicalize()
        .unwrap_or_else(|_| project_root.to_path_buf());
    for file in files {
        if file.contains("..") || Path::new(file).is_absolute() {
            return Err(anyhow!("补丁目标路径不安全: {}", file));
        }
        if has_git_path_component(file) {
            return Err(anyhow!("补丁不允许修改 .git 目录"));
        }

        let full = canonical_root.join(file);
        let resolved = resolve_existing_parent(&full)?;
        if !resolved.starts_with(&canonical_root) {
            return Err(anyhow!("补丁目标越界: {}", file));
        }
    }
    Ok(())
}

fn base_file_fingerprints(project_root: &Path, files: &[String]) -> Result<Vec<Value>> {
    let canonical_root = project_root
        .canonicalize()
        .unwrap_or_else(|_| project_root.to_path_buf());
    let mut out = Vec::with_capacity(files.len());
    for file in files {
        let full = canonical_root.join(file);
        let fingerprint = match std::fs::metadata(&full) {
            Ok(metadata) if metadata.is_file() => {
                if metadata.len() > MAX_PATCH_BASE_FILE_BYTES {
                    return Err(anyhow!(
                        "apply_patch diff preview refused: base file is too large: {}",
                        file
                    ));
                }
                let bytes = std::fs::read(&full)?;
                json!({
                    "path": file,
                    "exists": true,
                    "len": bytes.len(),
                    "sha256": sha256_hex(&bytes)
                })
            }
            Ok(metadata) if metadata.is_dir() => {
                return Err(anyhow!(
                    "apply_patch diff preview refused: target is a directory: {}",
                    file
                ));
            }
            Ok(_) => json!({
                "path": file,
                "exists": true,
                "len": 0,
                "sha256": null
            }),
            Err(error) if error.kind() == ErrorKind::NotFound => json!({
                "path": file,
                "exists": false,
                "len": 0,
                "sha256": null
            }),
            Err(error) => {
                return Err(anyhow!(
                    "apply_patch diff preview failed to inspect {}: {}",
                    file,
                    error
                ));
            }
        };
        out.push(fingerprint);
    }
    Ok(out)
}

fn string_array_field(value: &Value, key: &str) -> Result<Vec<String>> {
    value
        .get(key)
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow!("apply_patch approval preview missing {key}"))?
        .iter()
        .map(|item| {
            item.as_str()
                .map(str::to_string)
                .ok_or_else(|| anyhow!("apply_patch approval preview has invalid {key}"))
        })
        .collect()
}

fn has_git_path_component(path: &str) -> bool {
    path.split(['/', '\\'])
        .any(|part| part.eq_ignore_ascii_case(".git"))
}

fn resolve_existing_parent(path: &Path) -> Result<PathBuf> {
    if path.exists() {
        return path
            .canonicalize()
            .map_err(|error| anyhow!("补丁目标路径规范化失败: {}", error));
    }

    let mut current = path;
    while let Some(parent) = current.parent() {
        if parent.exists() {
            return parent
                .canonicalize()
                .map_err(|error| anyhow!("补丁目标父目录规范化失败: {}", error));
        }
        current = parent;
    }
    Err(anyhow!("补丁目标路径无有效父目录"))
}

fn run_git_apply(project_root: &Path, patch_file: &Path, check: bool) -> Result<()> {
    let mut args = vec!["apply", "--whitespace=nowarn"];
    if check {
        args.push("--check");
    }
    let patch_arg = patch_file
        .to_str()
        .ok_or_else(|| anyhow!("补丁临时文件路径不是有效 UTF-8"))?;
    args.push(patch_arg);

    let output = Command::new("git")
        .args(args)
        .current_dir(project_root)
        .output()?;
    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let phase = if check { "检查" } else { "应用" };
    Err(anyhow!(
        "补丁{}失败: {}{}",
        phase,
        stdout.trim(),
        stderr.trim()
    ))
}

fn temp_patch_file(kind: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    std::env::temp_dir().join(format!("elon-{kind}-{nanos}.patch"))
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

struct PatchFileCleanup(PathBuf);

impl Drop for PatchFileCleanup {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}


#[cfg(test)]
#[path = "tools_patch_tests.rs"]
mod tests;
