// server/src/node_agent_file_info.rs

use anyhow::{Context, Result};
use std::{
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

const MAX_LINE_PROBE_BYTES: u64 = 1024 * 1024;
const MAX_DIR_ENTRIES_SAMPLED: usize = 200;

pub(crate) async fn file_info(full_path: &Path, display_path: &str) -> Result<String> {
    let metadata = tokio::fs::metadata(full_path)
        .await
        .with_context(|| format!("file_info failed: {display_path}"))?;
    let kind = if metadata.is_dir() {
        "dir"
    } else if metadata.is_file() {
        "file"
    } else {
        "other"
    };

    let mut rows = vec![
        format!("file_info ok: {}", display_path.trim()),
        format!("kind={kind}"),
    ];

    if metadata.is_file() {
        rows.push(format!("bytes={}", metadata.len()));
        rows.push(modified_row(&metadata));
        rows.extend(file_text_probe(full_path, metadata.len()).await);
    } else if metadata.is_dir() {
        rows.push(modified_row(&metadata));
        rows.extend(dir_probe(full_path).await?);
    } else {
        rows.push(modified_row(&metadata));
    }

    Ok(rows.join("\n"))
}

async fn file_text_probe(full_path: &Path, len: u64) -> Vec<String> {
    if len > MAX_LINE_PROBE_BYTES {
        return vec![
            "line_probe=skipped_large_file".to_string(),
            format!("line_probe_max_bytes={MAX_LINE_PROBE_BYTES}"),
            "advice=use read_file_range only if you know the needed line span".to_string(),
        ];
    }

    match tokio::fs::read_to_string(full_path).await {
        Ok(text) => vec![
            "line_probe=text".to_string(),
            format!("line_count={}", text.lines().count()),
            "advice=use read_file_range before broad reads on large files".to_string(),
        ],
        Err(_) => vec![
            "line_probe=non_utf8_or_unreadable".to_string(),
            "advice=do not use read_file on binary or non-UTF8 files".to_string(),
        ],
    }
}

async fn dir_probe(full_path: &Path) -> Result<Vec<String>> {
    let mut entries = tokio::fs::read_dir(full_path).await?;
    let mut sampled = 0usize;
    let mut truncated = false;
    while entries.next_entry().await?.is_some() {
        sampled += 1;
        if sampled >= MAX_DIR_ENTRIES_SAMPLED {
            truncated = true;
            break;
        }
    }
    Ok(vec![
        format!("entries_sampled={sampled}"),
        format!("entries_truncated={truncated}"),
        "advice=use list_dir to inspect names in this directory".to_string(),
    ])
}

fn modified_row(metadata: &std::fs::Metadata) -> String {
    let modified = metadata
        .modified()
        .ok()
        .and_then(system_time_millis)
        .map(|millis| millis.to_string())
        .unwrap_or_else(|| "unknown".to_string());
    format!("modified_unix_ms={modified}")
}

fn system_time_millis(value: SystemTime) -> Option<u128> {
    value
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|duration| duration.as_millis())
}

#[cfg(test)]
mod tests {
    use super::file_info;

    #[tokio::test]
    async fn file_info_reports_text_file_shape() {
        let temp =
            std::env::temp_dir().join(format!("elon-file-info-text-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp).unwrap();
        let path = temp.join("note.txt");
        std::fs::write(&path, "one\ntwo\n").unwrap();

        let rendered = file_info(&path, "note.txt").await.expect("file info");
        let _ = std::fs::remove_dir_all(&temp);

        assert!(rendered.contains("file_info ok: note.txt"));
        assert!(rendered.contains("kind=file"));
        assert!(rendered.contains("bytes=8"));
        assert!(rendered.contains("line_probe=text"));
        assert!(rendered.contains("line_count=2"));
    }

    #[tokio::test]
    async fn file_info_reports_directory_shape() {
        let temp =
            std::env::temp_dir().join(format!("elon-file-info-dir-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(temp.join("src")).unwrap();
        std::fs::write(temp.join("src").join("main.rs"), "fn main() {}\n").unwrap();

        let rendered = file_info(&temp.join("src"), "src")
            .await
            .expect("file info");
        let _ = std::fs::remove_dir_all(&temp);

        assert!(rendered.contains("file_info ok: src"));
        assert!(rendered.contains("kind=dir"));
        assert!(rendered.contains("entries_sampled=1"));
        assert!(rendered.contains("entries_truncated=false"));
    }
}
