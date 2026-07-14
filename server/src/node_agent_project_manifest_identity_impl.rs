use super::*;

pub(super) fn detect_module_manifest_project_identity(
    fallback_name: &str,
    module_root: &Path,
) -> Option<ManifestProjectIdentity> {
    identity_from_json_manifest(
        fallback_name,
        &module_root.join("package.json"),
        &["displayName", "display_name", "name"],
        &["description"],
        "package.json",
    )
    .or_else(|| {
        identity_from_toml_manifest(
            fallback_name,
            &module_root.join("Cargo.toml"),
            "package",
            "Cargo.toml",
        )
    })
    .or_else(|| {
        identity_from_toml_manifest(
            fallback_name,
            &module_root.join("pyproject.toml"),
            "project",
            "pyproject.toml",
        )
    })
    .or_else(|| identity_from_go_mod(fallback_name, &module_root.join("go.mod")))
    .or_else(|| detect_manifest_project_identity(fallback_name, module_root))
}

pub(super) fn identity_from_tauri_config(
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

pub(super) fn identity_from_json_manifest(
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

pub(super) fn read_json_manifest(path: &Path) -> Option<Value> {
    if !path.is_file() {
        return None;
    }
    std::fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_json::from_str::<Value>(&text).ok())
}

pub(super) fn identity_from_toml_manifest(
    fallback_name: &str,
    manifest_path: &Path,
    section: &str,
    source: &str,
) -> Option<ManifestProjectIdentity> {
    if !manifest_path.is_file() {
        return None;
    }
    identity_from_parts(
        fallback_name,
        toml_section_string(manifest_path, section, "name"),
        toml_section_string(manifest_path, section, "description"),
        source,
    )
}

pub(super) fn toml_section_string(path: &Path, section: &str, key: &str) -> Option<String> {
    let text = std::fs::read_to_string(path).ok()?;
    let mut in_section = false;
    for raw_line in text.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            in_section = line.trim_start_matches('[').trim_end_matches(']').trim() == section;
            continue;
        }
        if !in_section {
            continue;
        }
        let Some((left, right)) = line.split_once('=') else {
            continue;
        };
        if left.trim() == key {
            return parse_toml_string(right.trim())
                .and_then(|value| clean_project_text(&value, 240));
        }
    }
    None
}

pub(super) fn parse_toml_string(value: &str) -> Option<String> {
    let value = value.trim();
    if let Some(rest) = value.strip_prefix('"') {
        let mut escaped = false;
        let mut output = String::new();
        for ch in rest.chars() {
            if escaped {
                output.push(ch);
                escaped = false;
                continue;
            }
            if ch == '\\' {
                escaped = true;
                continue;
            }
            if ch == '"' {
                return Some(output);
            }
            output.push(ch);
        }
        return None;
    }
    if let Some(rest) = value.strip_prefix('\'') {
        return rest.split_once('\'').map(|(text, _)| text.to_string());
    }
    None
}

pub(super) fn identity_from_go_mod(
    fallback_name: &str,
    go_mod: &Path,
) -> Option<ManifestProjectIdentity> {
    let module_path = go_module_path(go_mod)?;
    let name = go_module_name(&module_path)?;
    identity_from_parts(fallback_name, Some(name), None, "go.mod")
}

pub(super) fn go_module_path(go_mod: &Path) -> Option<String> {
    let text = std::fs::read_to_string(go_mod).ok()?;
    text.lines().find_map(|raw_line| {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with("//") {
            return None;
        }
        line.strip_prefix("module ")
            .map(str::trim)
            .and_then(|value| clean_project_text(value, 240))
    })
}

pub(super) fn go_module_name(module_path: &str) -> Option<String> {
    let mut parts = module_path
        .trim()
        .trim_end_matches('/')
        .rsplit('/')
        .filter(|part| !part.trim().is_empty());
    let first = parts.next()?.trim();
    let name = if is_go_major_version_suffix(first) {
        parts.next().unwrap_or(first).trim()
    } else {
        first
    };
    clean_project_text(name.trim_end_matches(".git"), 120)
}

pub(super) fn is_go_major_version_suffix(value: &str) -> bool {
    let Some(rest) = value.strip_prefix('v') else {
        return false;
    };
    rest.len() <= 3 && rest.chars().all(|ch| ch.is_ascii_digit())
}

pub(super) fn identity_from_dotnet_solution_or_project(
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

pub(super) fn identity_from_dotnet_project(
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

pub(super) fn first_file_with_extension(
    project_root: &Path,
    extension: &str,
) -> Option<std::path::PathBuf> {
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

pub(super) fn first_xml_tag_text(text: &str, tags: &[&str]) -> Option<String> {
    tags.iter().find_map(|tag| {
        let open = format!("<{tag}>");
        let close = format!("</{tag}>");
        let start = text.find(&open)? + open.len();
        let end = text[start..].find(&close)? + start;
        clean_project_text(&text[start..end], 240)
    })
}

pub(super) fn identity_from_gradle_settings(
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

pub(super) fn gradle_root_project_name(settings_text: &str) -> Option<String> {
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

pub(super) fn quoted_gradle_value(value: &str) -> Option<&str> {
    let quote = value.chars().next()?;
    if quote != '"' && quote != '\'' {
        return None;
    }
    let body = &value[quote.len_utf8()..];
    body.find(quote).map(|end| &body[..end])
}

pub(super) fn first_json_path_string(value: &Value, paths: &[&str]) -> Option<String> {
    paths.iter().find_map(|path| {
        path.split('.')
            .try_fold(value, |current, key| current.get(key))
            .and_then(Value::as_str)
            .and_then(|text| clean_project_text(text, 240))
    })
}

pub(super) fn identity_from_parts(
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

impl ManifestProjectIdentity {
    pub(super) fn with_source_prefix(mut self, prefix: &str) -> Self {
        self.source = format!("{prefix}/{}", self.source);
        self
    }
}

pub(super) fn clean_project_text(value: &str, max_chars: usize) -> Option<String> {
    let value = value.trim();
    if value.is_empty() {
        return None;
    }
    Some(value.chars().take(max_chars).collect())
}

pub(super) fn default_project_description(name: &str) -> String {
    format!("绑定到本 PC 节点的本地项目: {name}")
}

#[cfg(test)]
#[path = "node_agent_project_manifest_identity_tests.rs"]
mod tests;
