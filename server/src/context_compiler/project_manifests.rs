use std::{fs, path::Path};

use serde_json::Value;

use super::{repo_snapshot::relative_path, repo_walk};

const MAX_PROJECT_FILES: usize = 64;
const MAX_MANIFEST_BYTES: u64 = 256 * 1024;
const MAX_ITEMS: usize = 24;

#[derive(Debug, Clone, Default, serde::Serialize)]
pub(crate) struct ProjectManifestReport {
    pub(crate) readmes: Vec<ReadmeSummary>,
    pub(crate) manifests: Vec<ProjectManifestSummary>,
    pub(crate) warnings: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub(crate) struct ReadmeSummary {
    pub(crate) path: String,
    pub(crate) title: Option<String>,
    pub(crate) headings: Vec<String>,
    pub(crate) preview: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub(crate) struct ProjectManifestSummary {
    pub(crate) path: String,
    pub(crate) kind: &'static str,
    pub(crate) name: Option<String>,
    pub(crate) version: Option<String>,
    pub(crate) description: Option<String>,
    pub(crate) scripts: Vec<String>,
    pub(crate) dependencies: Vec<String>,
    pub(crate) features: Vec<String>,
}

pub(crate) fn collect_project_manifest_report(workspace: &Path) -> ProjectManifestReport {
    let files = repo_walk::collect_matching_files(workspace, MAX_PROJECT_FILES, is_manifest_file);
    let mut report = ProjectManifestReport::default();

    for path in files {
        let relative = relative_path(workspace, &path);
        let Some(text) = read_small_text(&path) else {
            report
                .warnings
                .push(format!("skip unreadable or oversized manifest: {relative}"));
            continue;
        };
        match file_name(&path).as_deref() {
            Some("README.md") | Some("README") => {
                report.readmes.push(parse_readme(relative, &text))
            }
            Some("Cargo.toml") => report.manifests.push(parse_cargo(relative, &text)),
            Some("package.json") => report.manifests.push(parse_package_json(relative, &text)),
            Some("pyproject.toml") => report.manifests.push(parse_pyproject(relative, &text)),
            _ => {}
        }
    }

    report
}

fn is_manifest_file(path: &Path) -> bool {
    matches!(
        file_name(path).as_deref(),
        Some("README.md" | "README" | "Cargo.toml" | "package.json" | "pyproject.toml")
    )
}

fn parse_readme(path: String, text: &str) -> ReadmeSummary {
    let headings = text
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            trimmed
                .strip_prefix('#')
                .map(|heading| heading.trim_start_matches('#').trim().to_string())
                .filter(|heading| !heading.is_empty())
        })
        .take(12)
        .collect::<Vec<_>>();
    let title = headings.first().cloned();
    let preview = text
        .split("\n\n")
        .map(|part| part.trim())
        .find(|part| !part.is_empty() && !part.starts_with('#'))
        .map(|part| compact(part, 260));

    ReadmeSummary {
        path,
        title,
        headings,
        preview,
    }
}

fn parse_cargo(path: String, text: &str) -> ProjectManifestSummary {
    ProjectManifestSummary {
        path,
        kind: "cargo",
        name: value_in_section(text, "package", "name"),
        version: value_in_section(text, "package", "version"),
        description: value_in_section(text, "package", "description"),
        scripts: Vec::new(),
        dependencies: dependency_keys(
            text,
            &["dependencies", "dev-dependencies", "build-dependencies"],
        ),
        features: section_keys(text, "features"),
    }
}

fn parse_package_json(path: String, text: &str) -> ProjectManifestSummary {
    let Ok(value) = serde_json::from_str::<Value>(text) else {
        return ProjectManifestSummary {
            path,
            kind: "package_json",
            name: None,
            version: None,
            description: Some("invalid package.json".to_string()),
            scripts: Vec::new(),
            dependencies: Vec::new(),
            features: Vec::new(),
        };
    };

    ProjectManifestSummary {
        path,
        kind: "package_json",
        name: string_field(&value, "name"),
        version: string_field(&value, "version"),
        description: string_field(&value, "description"),
        scripts: object_keys(value.get("scripts")),
        dependencies: package_dependencies(&value),
        features: Vec::new(),
    }
}

fn parse_pyproject(path: String, text: &str) -> ProjectManifestSummary {
    ProjectManifestSummary {
        path,
        kind: "pyproject",
        name: value_in_section(text, "project", "name")
            .or_else(|| value_in_section(text, "tool.poetry", "name")),
        version: value_in_section(text, "project", "version")
            .or_else(|| value_in_section(text, "tool.poetry", "version")),
        description: value_in_section(text, "project", "description")
            .or_else(|| value_in_section(text, "tool.poetry", "description")),
        scripts: section_keys(text, "project.scripts"),
        dependencies: pyproject_dependencies(text),
        features: section_keys(text, "project.optional-dependencies"),
    }
}

fn package_dependencies(value: &Value) -> Vec<String> {
    let mut deps = Vec::new();
    for section in [
        "dependencies",
        "devDependencies",
        "peerDependencies",
        "optionalDependencies",
    ] {
        deps.extend(object_keys(value.get(section)));
    }
    deps.sort();
    deps.dedup();
    deps.truncate(MAX_ITEMS);
    deps
}

fn pyproject_dependencies(text: &str) -> Vec<String> {
    let mut deps = quoted_array_values_in_section(text, "project", "dependencies");
    deps.extend(section_keys(text, "tool.poetry.dependencies"));
    deps.sort();
    deps.dedup();
    deps.truncate(MAX_ITEMS);
    deps
}

fn value_in_section(text: &str, section: &str, key: &str) -> Option<String> {
    let mut in_section = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            in_section = trimmed.trim_matches(&['[', ']'][..]) == section;
            continue;
        }
        if !in_section || !trimmed.starts_with(key) {
            continue;
        }
        let (left, value) = trimmed.split_once('=')?;
        if left.trim() == key {
            return Some(unquote(value.trim()));
        }
    }
    None
}

fn section_keys(text: &str, section: &str) -> Vec<String> {
    let mut in_section = false;
    let mut keys = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            in_section = trimmed.trim_matches(&['[', ']'][..]) == section;
            continue;
        }
        if !in_section || trimmed.starts_with('#') {
            continue;
        }
        if let Some((key, _)) = trimmed.split_once('=') {
            let key = key.trim().trim_matches('"').to_string();
            if !key.is_empty() {
                keys.push(key);
            }
        }
    }
    keys.sort();
    keys.dedup();
    keys.truncate(MAX_ITEMS);
    keys
}

fn dependency_keys(text: &str, sections: &[&str]) -> Vec<String> {
    let mut keys = Vec::new();
    for section in sections {
        keys.extend(section_keys(text, section));
    }
    keys.sort();
    keys.dedup();
    keys.truncate(MAX_ITEMS);
    keys
}

fn quoted_array_values_in_section(text: &str, section: &str, key: &str) -> Vec<String> {
    let mut in_section = false;
    let mut in_array = false;
    let mut values = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            in_section = trimmed.trim_matches(&['[', ']'][..]) == section;
            in_array = false;
            continue;
        }
        if !in_section {
            continue;
        }
        if trimmed.starts_with(key) && trimmed.contains('[') {
            in_array = true;
        }
        if in_array {
            values.extend(quoted_values(trimmed));
            if trimmed.contains(']') {
                break;
            }
        }
    }
    values.truncate(MAX_ITEMS);
    values
}

fn object_keys(value: Option<&Value>) -> Vec<String> {
    let mut keys = value
        .and_then(Value::as_object)
        .map(|object| object.keys().cloned().collect::<Vec<_>>())
        .unwrap_or_default();
    keys.sort();
    keys.truncate(MAX_ITEMS);
    keys
}

fn string_field(value: &Value, field: &str) -> Option<String> {
    value
        .get(field)
        .and_then(Value::as_str)
        .map(|item| item.to_string())
}

fn quoted_values(line: &str) -> Vec<String> {
    let mut values = Vec::new();
    let mut chars = line.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch != '"' && ch != '\'' {
            continue;
        }
        let quote = ch;
        let mut value = String::new();
        for inner in chars.by_ref() {
            if inner == quote {
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

fn unquote(value: &str) -> String {
    value
        .trim()
        .trim_matches(',')
        .trim_matches('"')
        .trim_matches('\'')
        .to_string()
}

fn file_name(path: &Path) -> Option<String> {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.to_string())
}

fn read_small_text(path: &Path) -> Option<String> {
    if fs::metadata(path).ok()?.len() > MAX_MANIFEST_BYTES {
        return None;
    }
    fs::read_to_string(path).ok()
}

fn compact(value: &str, max_chars: usize) -> String {
    let single_line = value.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut out = single_line.chars().take(max_chars).collect::<String>();
    if single_line.chars().count() > max_chars {
        out.push_str("...");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn parses_readme_package_and_pyproject() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "elon_context_project_manifests_{}_{}",
            std::process::id(),
            nonce
        ));
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("README.md"), "# Demo\n\nA useful project.\n").unwrap();
        fs::write(
            dir.join("package.json"),
            r#"{"name":"web","version":"1.0.0","scripts":{"test":"vitest"},"dependencies":{"react":"latest"}}"#,
        )
        .unwrap();
        fs::write(
            dir.join("pyproject.toml"),
            "[project]\nname = \"py\"\nversion = \"0.1.0\"\ndependencies = [\"ruff\"]\n",
        )
        .unwrap();

        let report = collect_project_manifest_report(&dir);

        assert_eq!(report.readmes[0].title.as_deref(), Some("Demo"));
        assert!(report.manifests.iter().any(|item| {
            item.kind == "package_json" && item.scripts.contains(&"test".to_string())
        }));
        assert!(report.manifests.iter().any(|item| {
            item.kind == "pyproject" && item.dependencies.contains(&"ruff".to_string())
        }));

        fs::remove_dir_all(dir).unwrap();
    }
}
