// server/src/node_agent_write_preview.rs

use anyhow::{bail, Context, Result};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{io::ErrorKind, path::Path};

const MAX_WRITE_DIFF_PREVIEW_CHARS: usize = 4_000;
const MAX_WRITE_PREVIEW_FILE_BYTES: u64 = 256 * 1024;
const MAX_WRITE_CONTENT_BYTES: usize = 256 * 1024;

pub(crate) async fn write_file_diff_preview(
    full_path: &Path,
    display_path: &str,
    new_content: &str,
) -> Result<Value> {
    reject_sensitive_path(display_path)?;
    reject_sensitive_content("new content", new_content)?;
    if new_content.len() > MAX_WRITE_CONTENT_BYTES {
        bail!("write_file diff preview refused: new content is too large");
    }
    if let Ok(metadata) = tokio::fs::metadata(full_path).await {
        if metadata.len() > MAX_WRITE_PREVIEW_FILE_BYTES {
            bail!("write_file diff preview refused: existing file is too large");
        }
    }

    let old_content = match tokio::fs::read_to_string(full_path).await {
        Ok(content) => Some(content),
        Err(error) if error.kind() == ErrorKind::NotFound => None,
        Err(error) => {
            // 既有文件如果无法按 UTF-8 读取，就不能给用户可信 diff。
            // 这里 fail-closed，避免用户在看不到旧内容时批准覆盖文件。
            return Err(error)
                .with_context(|| format!("write_file diff preview failed: {display_path}"));
        }
    };
    if let Some(old_content) = old_content.as_deref() {
        reject_sensitive_content("existing content", old_content)?;
    }

    render_write_file_diff(display_path, old_content.as_deref(), new_content)
}

fn render_write_file_diff(
    display_path: &str,
    old_content: Option<&str>,
    new_content: &str,
) -> Result<Value> {
    let clean_path = display_path.trim();
    let full_preview = match old_content {
        Some(old) => replacement_diff(clean_path, old, new_content),
        None => new_file_diff(clean_path, new_content),
    };
    if full_preview.chars().count() > MAX_WRITE_DIFF_PREVIEW_CHARS {
        // 审批卡必须完整展示 write_file 的整文件替换效果。预览过大时
        // 不让用户批准，要求 agent 改用更小的 apply_patch。
        bail!("write_file diff preview refused: diff preview is too large");
    }

    Ok(json!({
        "format": "unified",
        "source": "write_file",
        "kind": if old_content.is_some() { "replace" } else { "create" },
        "preview": full_preview,
        "truncated": false,
        "files": [clean_path],
        "old_sha256": old_content.map(sha256_hex),
        "new_sha256": sha256_hex(new_content)
    }))
}

fn replacement_diff(path: &str, old_content: &str, new_content: &str) -> String {
    let old_line_count = line_count(old_content);
    let new_line_count = line_count(new_content);
    let mut out = format!(
        "diff --git a/{path} b/{path}\n--- a/{path}\n+++ b/{path}\n@@ -1,{old_line_count} +1,{new_line_count} @@\n"
    );
    append_prefixed_lines(&mut out, '-', old_content);
    append_prefixed_lines(&mut out, '+', new_content);
    out
}

fn new_file_diff(path: &str, new_content: &str) -> String {
    let new_line_count = line_count(new_content);
    let mut out = format!(
        "diff --git a/{path} b/{path}\nnew file mode 100644\n--- /dev/null\n+++ b/{path}\n@@ -0,0 +1,{new_line_count} @@\n"
    );
    append_prefixed_lines(&mut out, '+', new_content);
    out
}

fn append_prefixed_lines(out: &mut String, prefix: char, content: &str) {
    for line in content.lines() {
        out.push(prefix);
        out.push_str(line);
        out.push('\n');
    }
    if content.is_empty() {
        return;
    }
    if !content.ends_with('\n') {
        out.push_str("\\ No newline at end of file\n");
    }
}

fn line_count(content: &str) -> usize {
    if content.is_empty() {
        return 0;
    }
    content.lines().count()
}

pub(crate) fn sha256_hex(text: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    hex::encode(hasher.finalize())
}

fn reject_sensitive_path(display_path: &str) -> Result<()> {
    let path = display_path.trim().to_ascii_lowercase().replace('\\', "/");
    let sensitive_names = [".npmrc", ".pypirc", "id_rsa", "id_ed25519", "credentials"];
    if path.split('/').any(|part| {
        part == ".env"
            || part.starts_with(".env.")
            || part.starts_with(".env-")
            || sensitive_names.contains(&part)
    }) || path.ends_with(".pem")
        || path.ends_with(".key")
        || path.ends_with(".p12")
        || path.ends_with(".pfx")
    {
        bail!("write_file diff preview refused: sensitive path");
    }
    Ok(())
}

fn reject_sensitive_content(label: &str, content: &str) -> Result<()> {
    if content
        .chars()
        .any(|ch| ch == '\0' || (ch.is_control() && !matches!(ch, '\n' | '\r' | '\t')))
    {
        bail!("write_file diff preview refused: binary {label}");
    }

    let lower = content.to_ascii_lowercase();
    let compact: String = lower
        .chars()
        .filter(|ch| !ch.is_whitespace() && *ch != '"' && *ch != '\'')
        .collect();
    let secret_markers = [
        "-----begin private key-----",
        "-----begin rsa private key-----",
        "-----begin openssh private key-----",
    ];
    let secret_keys = [
        "api_key",
        "apikey",
        "password",
        "access_token",
        "refresh_token",
        "secret_key",
        "client_secret",
        "private_key",
    ];
    if secret_markers.iter().any(|marker| lower.contains(marker))
        || secret_keys
            .iter()
            .any(|key| compact.contains(&format!("{key}=")) || compact.contains(&format!("{key}:")))
    {
        bail!("write_file diff preview refused: sensitive {label}");
    }
    Ok(())
}


#[cfg(test)]
#[path = "node_agent_write_preview_tests.rs"]
mod tests;
