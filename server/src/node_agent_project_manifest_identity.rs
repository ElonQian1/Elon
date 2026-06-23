// server/src/node_agent_project_manifest_identity.rs

use serde_json::Value;
use std::path::Path;

#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) struct ManifestProjectIdentity {
    pub(crate) name: String,
    pub(crate) description: Option<String>,
    pub(crate) source: String,
}

pub(crate) fn detect_manifest_project_identity(
    fallback_name: &str,
    project_root: &Path,
) -> Option<ManifestProjectIdentity> {
    identity_from_json_manifest(
        fallback_name,
        &project_root.join("deno.json"),
        &["name"],
        &["description"],
        "deno.json",
    )
    .or_else(|| identity_from_tauri_config(fallback_name, &project_root.join("tauri.conf.json")))
    .or_else(|| {
        identity_from_tauri_config(
            fallback_name,
            &project_root.join("src-tauri/tauri.conf.json"),
        )
    })
}

fn identity_from_tauri_config(
    fallback_name: &str,
    manifest_path: &Path,
) -> Option<ManifestProjectIdentity> {
    identity_from_json_manifest(
        fallback_name,
        manifest_path,
        &["productName", "package.productName", "package.name", "name"],
        &["description", "package.description"],
        manifest_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("tauri.conf.json"),
    )
}

fn identity_from_json_manifest(
    fallback_name: &str,
    manifest_path: &Path,
    name_paths: &[&str],
    description_paths: &[&str],
    source: &str,
) -> Option<ManifestProjectIdentity> {
    let manifest = read_json_manifest(manifest_path)?;
    identity_from_parts(
        fallback_name,
        first_json_path_string(&manifest, name_paths),
        first_json_path_string(&manifest, description_paths),
        source,
    )
}

fn read_json_manifest(path: &Path) -> Option<Value> {
    if !path.is_file() {
        return None;
    }
    std::fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_json::from_str::<Value>(&text).ok())
}

fn first_json_path_string(value: &Value, paths: &[&str]) -> Option<String> {
    paths.iter().find_map(|path| {
        path.split('.')
            .try_fold(value, |current, key| current.get(key))
            .and_then(Value::as_str)
            .and_then(|text| clean_project_text(text, 240))
    })
}

fn identity_from_parts(
    fallback_name: &str,
    name: Option<String>,
    description: Option<String>,
    source: &str,
) -> Option<ManifestProjectIdentity> {
    if name.is_none() && description.is_none() {
        return None;
    }
    let name = name.unwrap_or_else(|| fallback_name.to_string());
    let description = description.or_else(|| Some(default_project_description(&name)));
    Some(ManifestProjectIdentity {
        name,
        description,
        source: source.to_string(),
    })
}

fn clean_project_text(value: &str, max_chars: usize) -> Option<String> {
    let value = value.trim();
    if value.is_empty() {
        return None;
    }
    Some(value.chars().take(max_chars).collect())
}

fn default_project_description(name: &str) -> String {
    format!("绑定到本 PC 节点的本地项目: {name}")
}

#[cfg(test)]
mod tests {
    use super::detect_manifest_project_identity;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_project(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "elon-project-manifest-identity-{label}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn detects_deno_project_identity() {
        let dir = temp_project("deno");
        std::fs::write(
            dir.join("deno.json"),
            r#"{"name":"edge-script-kit","description":"Deno 自动化项目"}"#,
        )
        .unwrap();

        let identity = detect_manifest_project_identity("fallback", &dir).unwrap();
        let _ = std::fs::remove_dir_all(&dir);

        assert_eq!(identity.name, "edge-script-kit");
        assert_eq!(identity.description.as_deref(), Some("Deno 自动化项目"));
        assert_eq!(identity.source, "deno.json");
    }

    #[test]
    fn detects_tauri_product_name_from_root_config() {
        let dir = temp_project("tauri-root");
        std::fs::write(
            dir.join("tauri.conf.json"),
            r#"{"productName":"一龙桌面工作台","package":{"description":"本机开发入口"}}"#,
        )
        .unwrap();

        let identity = detect_manifest_project_identity("fallback", &dir).unwrap();
        let _ = std::fs::remove_dir_all(&dir);

        assert_eq!(identity.name, "一龙桌面工作台");
        assert_eq!(identity.description.as_deref(), Some("本机开发入口"));
        assert_eq!(identity.source, "tauri.conf.json");
    }

    #[test]
    fn detects_tauri_product_name_from_src_tauri_config() {
        let dir = temp_project("tauri-nested");
        std::fs::create_dir_all(dir.join("src-tauri")).unwrap();
        std::fs::write(
            dir.join("src-tauri").join("tauri.conf.json"),
            r#"{"package":{"productName":"Desktop Agent"}}"#,
        )
        .unwrap();

        let identity = detect_manifest_project_identity("folder-name", &dir).unwrap();
        let _ = std::fs::remove_dir_all(&dir);

        assert_eq!(identity.name, "Desktop Agent");
        assert_eq!(
            identity.description.as_deref(),
            Some("绑定到本 PC 节点的本地项目: Desktop Agent")
        );
    }
}
