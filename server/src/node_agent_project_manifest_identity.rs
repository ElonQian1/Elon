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
    .or_else(|| {
        identity_from_gradle_settings(
            fallback_name,
            &project_root.join("settings.gradle.kts"),
            "settings.gradle.kts",
        )
    })
    .or_else(|| {
        identity_from_gradle_settings(
            fallback_name,
            &project_root.join("settings.gradle"),
            "settings.gradle",
        )
    })
    .or_else(|| identity_from_dotnet_solution_or_project(fallback_name, project_root))
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

fn identity_from_dotnet_solution_or_project(
    fallback_name: &str,
    project_root: &Path,
) -> Option<ManifestProjectIdentity> {
    if let Some(solution) = first_file_with_extension(project_root, "sln") {
        let name = solution
            .file_stem()
            .and_then(|value| value.to_str())
            .and_then(|value| clean_project_text(value, 120));
        let source = solution
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("*.sln")
            .to_string();
        if let Some(identity) = identity_from_parts(fallback_name, name, None, &source) {
            return Some(identity);
        }
    }
    identity_from_dotnet_project(fallback_name, project_root)
}

fn identity_from_dotnet_project(
    fallback_name: &str,
    project_root: &Path,
) -> Option<ManifestProjectIdentity> {
    let project = first_file_with_extension(project_root, "csproj")?;
    let source = project
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("*.csproj")
        .to_string();
    let text = std::fs::read_to_string(&project).ok();
    let name = text
        .as_deref()
        .and_then(|text| first_xml_tag_text(text, &["AssemblyName", "PackageId"]))
        .or_else(|| {
            project
                .file_stem()
                .and_then(|value| value.to_str())
                .and_then(|value| clean_project_text(value, 120))
        });
    let description = text
        .as_deref()
        .and_then(|text| first_xml_tag_text(text, &["Description"]));
    identity_from_parts(fallback_name, name, description, &source)
}

fn first_file_with_extension(project_root: &Path, extension: &str) -> Option<std::path::PathBuf> {
    let mut files = std::fs::read_dir(project_root)
        .ok()?
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let file_type = entry.file_type().ok()?;
            if !file_type.is_file() {
                return None;
            }
            let path = entry.path();
            path.extension()
                .and_then(|value| value.to_str())
                .map(str::to_ascii_lowercase)
                .filter(|value| value == extension)
                .map(|_| path)
        })
        .collect::<Vec<_>>();
    files.sort_by_key(|path| {
        path.file_name()
            .map(|value| value.to_string_lossy().to_ascii_lowercase())
            .unwrap_or_default()
    });
    files.into_iter().next()
}

fn first_xml_tag_text(text: &str, tags: &[&str]) -> Option<String> {
    tags.iter().find_map(|tag| {
        let open = format!("<{tag}>");
        let close = format!("</{tag}>");
        let start = text.find(&open)? + open.len();
        let end = text[start..].find(&close)? + start;
        clean_project_text(&text[start..end], 240)
    })
}

fn identity_from_gradle_settings(
    fallback_name: &str,
    settings_path: &Path,
    source: &str,
) -> Option<ManifestProjectIdentity> {
    if !settings_path.is_file() {
        return None;
    }
    let text = std::fs::read_to_string(settings_path).ok()?;
    identity_from_parts(fallback_name, gradle_root_project_name(&text), None, source)
}

fn gradle_root_project_name(settings_text: &str) -> Option<String> {
    settings_text.lines().take(400).find_map(|raw_line| {
        let line = raw_line.trim();
        if line.starts_with("//") || line.starts_with("/*") || line.starts_with('*') {
            return None;
        }
        let value = line
            .strip_prefix("rootProject.name")?
            .trim_start()
            .strip_prefix('=')?
            .trim_start();
        quoted_gradle_value(value).and_then(|text| clean_project_text(text, 120))
    })
}

fn quoted_gradle_value(value: &str) -> Option<&str> {
    let quote = value.chars().next()?;
    if quote != '"' && quote != '\'' {
        return None;
    }
    let body = &value[quote.len_utf8()..];
    body.find(quote).map(|end| &body[..end])
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

    #[test]
    fn detects_gradle_root_project_name() {
        let dir = temp_project("gradle");
        std::fs::write(
            dir.join("settings.gradle"),
            "pluginManagement {}\nrootProject.name = 'AndroidWorkbench'\n",
        )
        .unwrap();

        let identity = detect_manifest_project_identity("folder-name", &dir).unwrap();
        let _ = std::fs::remove_dir_all(&dir);

        assert_eq!(identity.name, "AndroidWorkbench");
        assert_eq!(
            identity.description.as_deref(),
            Some("绑定到本 PC 节点的本地项目: AndroidWorkbench")
        );
        assert_eq!(identity.source, "settings.gradle");
    }

    #[test]
    fn detects_gradle_kts_root_project_name() {
        let dir = temp_project("gradle-kts");
        std::fs::write(
            dir.join("settings.gradle.kts"),
            "dependencyResolutionManagement {}\nrootProject.name = \"ComposeDesk\"\n",
        )
        .unwrap();

        let identity = detect_manifest_project_identity("folder-name", &dir).unwrap();
        let _ = std::fs::remove_dir_all(&dir);

        assert_eq!(identity.name, "ComposeDesk");
        assert_eq!(identity.source, "settings.gradle.kts");
    }

    #[test]
    fn ignores_commented_gradle_root_project_name() {
        let dir = temp_project("gradle-commented");
        std::fs::write(
            dir.join("settings.gradle"),
            "// rootProject.name = 'IgnoredName'\nrootProject.name = 'RealName'\n",
        )
        .unwrap();

        let identity = detect_manifest_project_identity("folder-name", &dir).unwrap();
        let _ = std::fs::remove_dir_all(&dir);

        assert_eq!(identity.name, "RealName");
    }

    #[test]
    fn detects_dotnet_solution_name() {
        let dir = temp_project("dotnet-sln");
        std::fs::write(dir.join("OpsDesk.sln"), "").unwrap();

        let identity = detect_manifest_project_identity("folder-name", &dir).unwrap();
        let _ = std::fs::remove_dir_all(&dir);

        assert_eq!(identity.name, "OpsDesk");
        assert_eq!(
            identity.description.as_deref(),
            Some("绑定到本 PC 节点的本地项目: OpsDesk")
        );
        assert_eq!(identity.source, "OpsDesk.sln");
    }

    #[test]
    fn detects_dotnet_project_identity_when_no_solution_exists() {
        let dir = temp_project("dotnet-csproj");
        std::fs::write(
            dir.join("Worker.Host.csproj"),
            r#"<Project>
  <PropertyGroup>
    <AssemblyName>WorkerHost</AssemblyName>
    <Description>后台任务服务</Description>
  </PropertyGroup>
</Project>"#,
        )
        .unwrap();

        let identity = detect_manifest_project_identity("folder-name", &dir).unwrap();
        let _ = std::fs::remove_dir_all(&dir);

        assert_eq!(identity.name, "WorkerHost");
        assert_eq!(identity.description.as_deref(), Some("后台任务服务"));
        assert_eq!(identity.source, "Worker.Host.csproj");
    }
}
