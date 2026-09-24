//! Add a narrowly scoped reader when a Claude task carries explicit conversation links.
use super::ProjectDocsMcpLaunchConfig;
use serde_json::{json, Value};
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};

const ASSETS: [(&str, &str); 4] = [
    (
        "stdio.mjs",
        include_str!("../../../scripts/web-conversations/stdio.mjs"),
    ),
    (
        "service.mjs",
        include_str!("../../../scripts/web-conversations/service.mjs"),
    ),
    (
        "transport.mjs",
        include_str!("../../../scripts/web-conversations/transport.mjs"),
    ),
    (
        "local-rpc.mjs",
        include_str!("../../../scripts/web-conversations/local-rpc.mjs"),
    ),
];

fn references(prompt: &str) -> Vec<String> {
    prompt
        .split(|c: char| c.is_whitespace() || "()[]{}<>\"'`，。,;".contains(c))
        .filter_map(|token| {
            let id = if let Some(id) = token.strip_prefix("chatgpt-conversation://") {
                id.to_string()
            } else {
                let url = reqwest::Url::parse(token).ok()?;
                if url.origin().ascii_serialization() != "https://chatgpt.com"
                    || !url.username().is_empty()
                    || url.password().is_some()
                    || url.query().is_some()
                    || url.fragment().is_some()
                {
                    return None;
                }
                let parts: Vec<_> = url.path().split('/').collect();
                match parts.as_slice() {
                    ["", "c", id] => id.to_string(),
                    ["", "g", group, "c", id] if group.starts_with("g-p-") => id.to_string(),
                    _ => return None,
                }
            };
            if id.len() != 36 {
                return None;
            }
            uuid::Uuid::parse_str(&id)
                .ok()
                .map(|value| value.to_string())
        })
        .collect::<BTreeSet<_>>()
        .into_iter()
        .take(16)
        .collect()
}

fn materialize(root: &Path) -> anyhow::Result<PathBuf> {
    use sha2::{Digest, Sha256};
    let mut hash = Sha256::new();
    for (name, body) in ASSETS {
        hash.update(name);
        hash.update(body);
    }
    let directory = root.join(format!("{:x}", hash.finalize()));
    std::fs::create_dir_all(&directory)?;
    for (name, body) in ASSETS {
        let path = directory.join(name);
        // Immutable, content-addressed code assets. No conversation bodies or tokens on disk.
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(mut file) => {
                use std::io::Write;
                file.write_all(body.as_bytes())?;
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                anyhow::ensure!(
                    std::fs::read(&path)? == body.as_bytes(),
                    "reader_asset_mismatch"
                );
            }
            Err(error) => return Err(error.into()),
        }
    }
    Ok(directory.join("stdio.mjs"))
}

pub(super) fn extend(
    previous: Option<ProjectDocsMcpLaunchConfig>,
    prompt: &str,
    cwd: Option<&str>,
    cli: &str,
    port: u16,
) -> Option<ProjectDocsMcpLaunchConfig> {
    if !cli.trim().eq_ignore_ascii_case("claude") {
        return previous;
    }
    let ids = references(prompt);
    let Some(cwd) = cwd.filter(|value| !value.trim().is_empty()) else {
        return previous;
    };
    if ids.is_empty() {
        return previous;
    }
    let root = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("Elon/web-conversation-reader");
    let Ok(script) = materialize(&root) else {
        tracing::warn!("conversation_reader_assets_unavailable");
        return previous;
    };
    let mut config = previous.unwrap_or(ProjectDocsMcpLaunchConfig {
        args: vec![],
        env: vec![],
    });
    let existing = config.args.iter().position(|arg| arg == "--mcp-config");
    let mut document = if let Some(index) = existing {
        let parsed = config
            .args
            .get(index + 1)
            .and_then(|path| std::fs::read(path).ok())
            .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok());
        let Some(value) = parsed.filter(|v| v.get("mcpServers").is_some_and(Value::is_object))
        else {
            tracing::warn!("conversation_reader_config_merge_failed");
            return Some(config);
        };
        value
    } else {
        json!({"mcpServers":{}})
    };
    document["mcpServers"]["yilong_web_conversations"] = json!({
        "type":"stdio", "command":"node", "args":[script], "env":{
            "ELON_PROJECT_ROOT":cwd, "ELON_NODE_ADMIN_URL":format!("http://127.0.0.1:{port}"),
            "ELON_WEB_CONVERSATION_IDS":ids.join(",")
        }
    });
    let serialized = document.to_string();
    if let Some(index) = existing {
        config.args[index + 1] = serialized;
    } else {
        config.args.extend(["--mcp-config".into(), serialized]);
    }
    Some(config)
}

#[cfg(test)]
mod tests {
    use super::*;
    const ID: &str = "00000000-0000-4000-8000-000000000001";
    #[test]
    fn scope_uses_explicit_supported_links_only() {
        assert!(references(ID).is_empty());
        assert_eq!(
            references(&format!("{{\"url\":\"https://chatgpt.com/c/{ID}\"}}")),
            vec![ID]
        );
        assert_eq!(
            references(&format!("[读取](chatgpt-conversation://{ID})")),
            vec![ID]
        );
        assert_eq!(
            references(&format!("https://chatgpt.com/g/g-p-test/c/{ID}")),
            vec![ID]
        );
        for suffix in ["?share=1", "more", "/", "#fragment"] {
            assert!(references(&format!("https://chatgpt.com/c/{ID}{suffix}")).is_empty());
        }
        assert!(references(&format!("https://elsewhere.example/c/{ID}")).is_empty());
    }
    #[test]
    fn ordinary_claude_tasks_are_unchanged() {
        assert!(extend(None, "Please edit a file", Some("."), "claude", 7799).is_none());
        assert!(extend(
            None,
            &format!("chatgpt-conversation://{ID}"),
            Some("."),
            "copilot",
            7799
        )
        .is_none());
    }
    #[test]
    fn claude_launch_merges_governance_and_grants_only_linked_ids() {
        let root = std::env::temp_dir().join(format!("reader-config-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("mcp.json");
        std::fs::write(&path, r#"{"mcpServers":{"existing":{"type":"http","url":"http://127.0.0.1:7799/synthetic"}}}"#).unwrap();
        let previous = ProjectDocsMcpLaunchConfig {
            args: vec!["--mcp-config".into(), path.to_string_lossy().into()],
            env: vec![],
        };
        let config = extend(
            Some(previous),
            &format!("Read [chat](chatgpt-conversation://{ID})"),
            Some("synthetic-project"),
            "claude",
            7799,
        )
        .unwrap();
        assert_eq!(config.args.len(), 2);
        let value: Value = serde_json::from_str(&config.args[1]).unwrap();
        assert!(value["mcpServers"]["existing"].is_object());
        let reader = &value["mcpServers"]["yilong_web_conversations"];
        assert_eq!(reader["command"], "node");
        assert_eq!(reader["env"]["ELON_WEB_CONVERSATION_IDS"], ID);
        assert_eq!(reader["env"]["ELON_PROJECT_ROOT"], "synthetic-project");
        assert!(!value.to_string().contains("strict-mcp-config"));
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn assets_are_complete_and_immutable() {
        let root = std::env::temp_dir().join(format!("reader-test-{}", uuid::Uuid::new_v4()));
        let entry = materialize(&root).unwrap();
        assert_eq!(materialize(&root).unwrap(), entry);
        std::fs::write(entry, "changed").unwrap();
        assert!(materialize(&root).is_err());
        std::fs::remove_dir_all(root).unwrap();
    }
}
