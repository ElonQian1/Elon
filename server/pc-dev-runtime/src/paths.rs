use std::path::PathBuf;

use crate::node_data_paths::{NodeDataPaths, NODE_DATA_ROOT_ENV};

pub fn configured_node_data_root() -> Option<PathBuf> {
    nonempty_env(NODE_DATA_ROOT_ENV).map(PathBuf::from)
}

pub fn legacy_workspace_root_override() -> Option<PathBuf> {
    [
        "ELON_NODE_WORKSPACE_ROOT",
        "ELON_PC_WORKSPACE_ROOT",
        "NODE_WORKSPACE_ROOT",
    ]
    .into_iter()
    .find_map(nonempty_env)
    .map(PathBuf::from)
}

pub fn workspace_root() -> PathBuf {
    if let Some(root) = legacy_workspace_root_override() {
        return root;
    }
    if let Some(root) = configured_node_data_root() {
        return NodeDataPaths::new(root).workspaces();
    }

    legacy_default_workspace_root()
}

pub fn legacy_default_workspace_root() -> PathBuf {
    #[cfg(windows)]
    {
        if let Ok(profile) = std::env::var("USERPROFILE") {
            return PathBuf::from(profile).join("Elon").join("workspaces");
        }
    }

    #[cfg(not(windows))]
    {
        if let Ok(home) = std::env::var("HOME") {
            return PathBuf::from(&home).join(".elon").join("workspaces");
        }
    }

    std::env::temp_dir().join("elon").join("workspaces")
}

fn nonempty_env(key: &str) -> Option<String> {
    std::env::var(key)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

pub fn safe_path_part(value: &str, fallback: &str, max_len: usize) -> String {
    let cleaned: String = value
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .take(max_len)
        .collect();
    if cleaned.is_empty() {
        fallback.to_string()
    } else {
        cleaned
    }
}

#[cfg(test)]
mod tests {
    use super::safe_path_part;

    #[test]
    fn safe_path_part_removes_path_separators() {
        assert_eq!(
            safe_path_part("../usr:abc\\project", "fallback", 80),
            "usrabcproject"
        );
    }

    #[test]
    fn safe_path_part_uses_fallback_when_empty() {
        assert_eq!(safe_path_part("///", "fallback", 80), "fallback");
    }
}
