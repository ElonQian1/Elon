use std::{
    fs,
    path::{Path, PathBuf},
};

use super::repo_walk;

const MAX_MANIFESTS: usize = 16;
const MAX_MANIFEST_BYTES: u64 = 128 * 1024;

#[derive(Debug, Clone, serde::Serialize)]
pub(crate) struct RustProjectSummary {
    pub(crate) root_package: Option<String>,
    pub(crate) workspace: bool,
    pub(crate) workspace_members: Vec<String>,
    pub(crate) manifests: Vec<RustManifestSummary>,
    pub(crate) toolchain: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub(crate) struct RustManifestSummary {
    pub(crate) path: String,
    pub(crate) package_name: Option<String>,
    pub(crate) workspace: bool,
}

pub(crate) fn collect_rust_project_summary(workspace: &Path) -> Option<RustProjectSummary> {
    let mut manifest_paths = Vec::new();
    collect_manifest_paths(workspace, workspace, 0, &mut manifest_paths);
    if manifest_paths.is_empty() {
        return None;
    }

    let mut manifests = Vec::new();
    let mut root_package = None;
    let mut workspace_members = Vec::new();
    let mut workspace_manifest = false;
    for path in manifest_paths {
        let Some(summary) = parse_manifest(workspace, &path) else {
            continue;
        };
        if summary.path == "Cargo.toml" {
            root_package = summary.package_name.clone();
            workspace_manifest = summary.workspace;
            if let Some(members) = read_workspace_members(&path) {
                workspace_members = members;
            }
        }
        manifests.push(summary);
    }

    Some(RustProjectSummary {
        root_package,
        workspace: workspace_manifest || manifests.len() > 1,
        workspace_members,
        manifests,
        toolchain: read_toolchain(workspace),
    })
}

fn collect_manifest_paths(base: &Path, dir: &Path, depth: usize, out: &mut Vec<PathBuf>) {
    let _ = (dir, depth);
    out.extend(repo_walk::collect_matching_files(
        base,
        MAX_MANIFESTS,
        |path| path.file_name().and_then(|name| name.to_str()) == Some("Cargo.toml"),
    ));
    out.sort_by(|left, right| {
        relative_path(base, left)
            .len()
            .cmp(&relative_path(base, right).len())
            .then_with(|| relative_path(base, left).cmp(&relative_path(base, right)))
    });
    out.truncate(MAX_MANIFESTS);
}

fn parse_manifest(base: &Path, path: &Path) -> Option<RustManifestSummary> {
    let text = read_small_text(path)?;
    Some(RustManifestSummary {
        path: relative_path(base, path),
        package_name: package_name(&text),
        workspace: text.lines().any(|line| line.trim() == "[workspace]"),
    })
}

fn package_name(text: &str) -> Option<String> {
    let mut in_package = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            in_package = trimmed == "[package]";
            continue;
        }
        if !in_package || !trimmed.starts_with("name") {
            continue;
        }
        let (_, value) = trimmed.split_once('=')?;
        return Some(value.trim().trim_matches('"').to_string());
    }
    None
}

fn read_workspace_members(path: &Path) -> Option<Vec<String>> {
    let text = read_small_text(path)?;
    let mut members = Vec::new();
    let mut in_members = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("members") && trimmed.contains('[') {
            in_members = true;
        }
        if in_members {
            members.extend(quoted_values(trimmed));
            if trimmed.contains(']') {
                break;
            }
        }
    }
    members.sort();
    members.dedup();
    Some(members)
}

fn quoted_values(line: &str) -> Vec<String> {
    let mut values = Vec::new();
    let mut chars = line.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch != '"' {
            continue;
        }
        let mut value = String::new();
        for inner in chars.by_ref() {
            if inner == '"' {
                break;
            }
            value.push(inner);
        }
        if !value.is_empty() {
            values.push(value);
        }
    }
    values
}

fn read_toolchain(workspace: &Path) -> Option<String> {
    let toml = workspace.join("rust-toolchain.toml");
    if toml.is_file() {
        let text = read_small_text(&toml)?;
        for line in text.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("channel") {
                let (_, value) = trimmed.split_once('=')?;
                return Some(value.trim().trim_matches('"').to_string());
            }
        }
    }
    let plain = workspace.join("rust-toolchain");
    if plain.is_file() {
        return read_small_text(&plain).map(|text| text.trim().to_string());
    }
    None
}

fn read_small_text(path: &Path) -> Option<String> {
    let metadata = fs::metadata(path).ok()?;
    if metadata.len() > MAX_MANIFEST_BYTES {
        return None;
    }
    fs::read_to_string(path).ok()
}

fn relative_path(base: &Path, path: &Path) -> String {
    path.strip_prefix(base)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn collects_workspace_manifests() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "elon_context_rust_project_{}_{}",
            std::process::id(),
            nonce
        ));
        fs::create_dir_all(dir.join("crates/api")).unwrap();
        fs::write(
            dir.join("Cargo.toml"),
            "[workspace]\nmembers = [\"crates/api\"]\n",
        )
        .unwrap();
        fs::write(
            dir.join("crates/api/Cargo.toml"),
            "[package]\nname = \"api\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();
        fs::write(
            dir.join("rust-toolchain.toml"),
            "[toolchain]\nchannel = \"stable\"\n",
        )
        .unwrap();

        let summary = collect_rust_project_summary(&dir).unwrap();

        assert!(summary.workspace);
        assert_eq!(summary.workspace_members, vec!["crates/api"]);
        assert_eq!(summary.manifests.len(), 2);
        assert_eq!(summary.toolchain.as_deref(), Some("stable"));

        fs::remove_dir_all(dir).unwrap();
    }
}
