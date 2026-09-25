//! File manifests are derived from selected, revision-checked messages, never client URLs.
use anyhow::{ensure, Result};
use rusqlite::Connection;
use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Clone, Serialize)]
pub(crate) struct GroupAiAttachment {
    pub message_id: String,
    pub attachment_id: String,
    pub name: String,
    pub mime_type: String,
    pub size_bytes: u64,
    pub download_path: String,
    pub sha256: Option<String>,
}

pub(crate) fn append(
    files: &mut Vec<GroupAiAttachment>,
    message: &str,
    attachments: &Value,
) -> Result<()> {
    let items = attachments
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("附件记录无效，请重新选择"))?;
    for item in items {
        ensure!(files.len() < 9, "一次最多分析 9 个附件，请减少选区");
        let mime = item["mime_type"]
            .as_str()
            .unwrap_or("")
            .to_ascii_lowercase();
        ensure!(
            supported(&mime),
            "所选附件格式尚不支持私有上传，请调整选区；未发送任何消息"
        );
        let size = item["size_bytes"].as_u64().unwrap_or(0);
        ensure!(
            (1..=8 * 1024 * 1024).contains(&size),
            "附件为空或超过 8 MB，请调整选区"
        );
        let id = item["attachment_id"].as_str().unwrap_or("");
        ensure!(!id.is_empty(), "附件缺少标识，请重新上传");
        let path = download_path(item["url"].as_str().unwrap_or(""))?;
        let original = item["display_name"]
            .as_str()
            .or(item["file_name"].as_str())
            .unwrap_or("attachment");
        let safe: String = original
            .chars()
            .filter(|c| !c.is_control() && !matches!(c, '/' | '\\' | ':'))
            .take(90)
            .collect();
        let name = format!("group_{:02}_{}", files.len() + 1, safe);
        let sha256 = item["sha256"]
            .as_str()
            .filter(|s| s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit()))
            .map(str::to_ascii_lowercase);
        files.push(GroupAiAttachment {
            message_id: message.into(),
            attachment_id: id.into(),
            name,
            mime_type: mime,
            size_bytes: size,
            download_path: path,
            sha256,
        });
    }
    Ok(())
}

fn supported(mime: &str) -> bool {
    matches!(
        mime,
        "image/png"
            | "image/jpeg"
            | "image/webp"
            | "text/plain"
            | "application/pdf"
            | "application/msword"
            | "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
            | "application/vnd.ms-excel"
            | "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
            | "application/vnd.ms-powerpoint"
            | "application/vnd.openxmlformats-officedocument.presentationml.presentation"
            | "text/csv"
            | "application/json"
            | "text/markdown"
    )
}

fn download_path(raw: &str) -> Result<String> {
    let url = reqwest::Url::parse(raw)
        .or_else(|_| reqwest::Url::parse("https://attachments.invalid")?.join(raw))?;
    ensure!(
        matches!(url.scheme(), "https" | "http")
            && url.query().is_none()
            && url.fragment().is_none(),
        "附件地址无效"
    );
    let path = url.path();
    let parts: Vec<_> = path.split('/').collect();
    ensure!(
        parts.len() == 7
            && parts[1] == "api"
            && parts[2] == "user"
            && parts[4] == "chat-attachments"
            && parts[3..]
                .iter()
                .all(|p| !p.is_empty() && *p != "." && *p != "..")
            && !["%2f", "%5c", "%00", "%25"]
                .iter()
                .any(|p| path.to_ascii_lowercase().contains(p)),
        "仅支持本平台保存的群聊附件；未发送任何消息"
    );
    // Clients resolve this path against their authenticated platform, never the supplied origin.
    Ok(path.into())
}

pub(crate) fn for_request(conn: &Connection, request: &str) -> Result<Vec<GroupAiAttachment>> {
    let mut stmt = conn.prepare(
        "SELECT m.id,m.attachments_json FROM group_ai_selected_sources s
        JOIN friend_group_messages m ON m.id=s.message_id WHERE s.request_id=?1 ORDER BY m.rowid",
    )?;
    let rows = stmt
        .query_map([request], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, Option<String>>(1)?))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let mut files = Vec::new();
    for (id, raw) in rows {
        append(
            &mut files,
            &id,
            &serde_json::from_str::<Value>(raw.as_deref().unwrap_or("[]"))?,
        )?;
    }
    Ok(files)
}

#[cfg(test)]
#[path = "group_web_ai_attachment_tests.rs"]
mod tests;
