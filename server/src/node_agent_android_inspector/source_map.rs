use std::fs;
use std::path::{Path, PathBuf};

use super::types::{RuntimeUiNode, SourceMapEntry};

pub(crate) fn attach_source_map(
    nodes: &mut [RuntimeUiNode],
    explicit_root: Option<&str>,
) -> Option<String> {
    let root = resolve_source_root(explicit_root)?;
    for node in nodes {
        if let Some(resource_id) = node.resource_id.as_deref() {
            node.source = find_resource_id_source(&root, resource_id);
        }
    }
    Some(root.display().to_string())
}

fn find_resource_id_source(root: &Path, resource_id: &str) -> Option<SourceMapEntry> {
    let id_name = resource_id_name(resource_id)?;
    if id_name.is_empty() {
        return None;
    }
    let token_plus = format!("@+id/{id_name}");
    let token_ref = format!("@id/{id_name}");
    let res_root = root
        .join("android")
        .join("app")
        .join("src")
        .join("main")
        .join("res");
    let mut xml_files = Vec::new();
    collect_xml_files(&res_root, &mut xml_files);
    for file in xml_files {
        let Ok(content) = fs::read_to_string(&file) else {
            continue;
        };
        if content.len() > 768 * 1024 {
            continue;
        }
        if content.contains(&token_plus) || content.contains(&token_ref) {
            let matched_token = if content.contains(&token_plus) {
                token_plus.clone()
            } else {
                token_ref.clone()
            };
            let line = content
                .lines()
                .position(|line| line.contains(&token_plus) || line.contains(&token_ref))
                .map(|index| index + 1);
            let file = file
                .strip_prefix(root)
                .unwrap_or(&file)
                .display()
                .to_string();
            return Some(SourceMapEntry {
                file,
                line,
                token: matched_token,
                confidence: 0.92,
                reason: "resource-id 精确匹配 Android res XML".to_string(),
            });
        }
    }
    None
}

fn resource_id_name(resource_id: &str) -> Option<&str> {
    let value = resource_id.trim();
    value
        .rsplit_once(":id/")
        .or_else(|| value.rsplit_once("/id/"))
        .map(|(_, id)| id.trim())
        .filter(|id| !id.is_empty())
}

fn resolve_source_root(explicit_root: Option<&str>) -> Option<PathBuf> {
    if let Some(root) = explicit_root
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .and_then(|path| path.canonicalize().ok())
        .filter(|path| has_android_res(path))
    {
        return Some(root);
    }
    if let Some(root) = std::env::var_os("ELON_ANDROID_SOURCE_ROOT")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .and_then(|path| path.canonicalize().ok())
        .filter(|path| has_android_res(path))
    {
        return Some(root);
    }
    let mut candidates = Vec::new();
    if let Ok(current) = std::env::current_dir() {
        candidates.push(current);
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            candidates.push(parent.to_path_buf());
        }
    }
    for candidate in candidates {
        for ancestor in candidate.ancestors() {
            if has_android_res(ancestor) {
                return Some(ancestor.to_path_buf());
            }
        }
    }
    None
}

fn has_android_res(path: &Path) -> bool {
    path.join("android")
        .join("app")
        .join("src")
        .join("main")
        .join("res")
        .is_dir()
}

fn collect_xml_files(root: &Path, output: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_xml_files(&path, output);
        } else if path.extension().and_then(|value| value.to_str()) == Some("xml") {
            output.push(path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::resource_id_name;

    #[test]
    fn extracts_runtime_resource_id_names() {
        assert_eq!(
            resource_id_name("com.elon.app:id/topTitleText"),
            Some("topTitleText")
        );
        assert_eq!(
            resource_id_name("android.view.View/id/title"),
            Some("title")
        );
        assert_eq!(resource_id_name("topTitleText"), None);
    }
}
