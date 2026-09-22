//! One-shot JSON snapshot of a research session for tools that read files instead of MCP.
//! Bodies are not copied: the snapshot points at the credential-filtered content files already
//! stored under the session, so exporting never creates a second copy of page material.
use super::{files, model::*};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

pub const SCHEMA: &str = "yilong.browser-research.export.v1";
const DIRECTORY: &str = "exports";
const MAX_EXPORTS: usize = 16;

pub fn write(root: &Path, session: &Session) -> Result<Value, String> {
    if !digest_id(&session.id) {
        return Err("invalid_session".into());
    }
    let directory = root.join(&session.id).join(DIRECTORY);
    files::ensure_directory(&directory)?;
    prune(&directory);
    let content = root.join(&session.id).join("content");
    let snapshot = json!({
        "schema": SCHEMA,
        "exported_at_ms": now_ms(),
        "session": session.summary(),
        "site": session.site,
        "content_directory": content,
        "resources": session.resources.iter().map(|resource| {
            let mut value = serde_json::to_value(resource).unwrap_or(Value::Null);
            value["body_path"] = json!(body_path(&content, &resource.sha256));
            value
        }).collect::<Vec<_>>(),
        "requests": session.requests,
        "gaps": session.gaps,
        "trading_enabled": false,
        "page_material_is_untrusted": true,
    });
    let path = directory.join(format!("snapshot-{}.json", now_ms()));
    let bytes = serde_json::to_vec_pretty(&snapshot).map_err(|_| "invalid_session")?;
    files::write_export(&path, &bytes)?;
    Ok(
        json!({"schema":RESULT_SCHEMA,"kind":"export","session_id":session.id,
        "path":path,"bytes":bytes.len(),"resource_count":session.resources.len(),
        "request_count":session.requests.len()}),
    )
}

fn body_path(content: &Path, sha256: &str) -> PathBuf {
    content.join(format!("{sha256}.txt"))
}

// Snapshots are cheap to regenerate; keep the directory from growing without bound.
fn prune(directory: &Path) {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return;
    };
    let mut snapshots: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("snapshot-") && name.ends_with(".json"))
        })
        .collect();
    snapshots.sort();
    while snapshots.len() >= MAX_EXPORTS {
        let _ = std::fs::remove_file(snapshots.remove(0));
    }
}
