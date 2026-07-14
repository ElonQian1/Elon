use serde_json::Value;
use std::path::{Path, PathBuf};

use crate::node_agent_project_manifest_identity::{
    detect_manifest_project_identity, detect_shallow_manifest_project_identity,
};

use super::helpers::{clean_project_text, default_project_description, project_name};

pub(super) struct ProjectIdentity {
    pub(super) name: String,
    pub(super) description: Option<String>,
    pub(super) source: Option<String>,
}

pub(super) fn detect_project_identity(
    path: &Path,
    landing: Option<&Value>,
    git_remote_origin: Option<&str>,
) -> ProjectIdentity {
    let fallback_name = project_name(path);
    if let Some(identity) = identity_from_landing(&fallback_name, landing) {
        return identity;
    }
    if let Some(identity) = identity_from_package_json(&fallback_name, &path.join("package.json")) {
        return identity;
    }
    if let Some(identity) = detect_manifest_project_identity(&fallback_name, path) {
        return ProjectIdentity {
            name: identity.name,
            description: identity.description,
            source: Some(identity.source),
        };
    }
    if let Some(identity) = identity_from_toml_manifest(
        &fallback_name,
        &path.join("Cargo.toml"),
        "package",
        "Cargo.toml",
    ) {
        return identity;
    }
    if let Some(identity) = identity_from_toml_manifest(
        &fallback_name,
        &path.join("pyproject.toml"),
        "project",
        "pyproject.toml",
    ) {
        return identity;
    }
    if let Some(identity) = identity_from_go_mod(&fallback_name, &path.join("go.mod")) {
        return identity;
    }
    if let Some(identity) = detect_shallow_manifest_project_identity(&fallback_name, path) {
        return ProjectIdentity {
            name: identity.name,
            description: identity.description,
            source: Some(identity.source),
        };
    }
    if let Some(identity) = identity_from_readme(&fallback_name, path) {
        return identity;
    }
    if let Some(identity) = identity_from_git_remote(git_remote_origin) {
        return identity;
    }
    ProjectIdentity {
        description: Some(default_project_description(&fallback_name)),
        name: fallback_name,
        source: Some("目录名".to_string()),
    }
}

pub(super) fn identity_from_landing(
    fallback_name: &str,
    landing: Option<&Value>,
) -> Option<ProjectIdentity> {
    let object = landing?.as_object()?;
    let name = first_json_string(object, &["title"]);
    let description = first_json_string(object, &["tagline", "summary", "description"]);
    identity_from_parts(
        fallback_name,
        name,
        description,
        ".elon/project-landing.json",
    )
}

pub(super) fn identity_from_package_json(
    fallback_name: &str,
    package_json: &Path,
) -> Option<ProjectIdentity> {
    let object = std::fs::read_to_string(package_json)
        .ok()
        .and_then(|text| serde_json::from_str::<Value>(&text).ok())
        .and_then(|value| value.as_object().cloned())?;
    identity_from_parts(
        fallback_name,
        first_json_string(&object, &["displayName", "display_name", "name"]),
        first_json_string(&object, &["description"]),
        "package.json",
    )
}

pub(super) fn identity_from_toml_manifest(
    fallback_name: &str,
    manifest_path: &Path,
    section: &str,
    source: &str,
) -> Option<ProjectIdentity> {
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

pub(super) fn identity_from_go_mod(fallback_name: &str, go_mod: &Path) -> Option<ProjectIdentity> {
    let module_path = go_module_path(go_mod)?;
    let name = go_module_name(&module_path)?;
    identity_from_parts(fallback_name, Some(name), None, "go.mod")
}

pub(super) fn identity_from_readme(fallback_name: &str, path: &Path) -> Option<ProjectIdentity> {
    let (readme_path, source) = ["README.md", "README.MD", "Readme.md", "README"]
        .into_iter()
        .map(|file| (path.join(file), file))
        .find(|(candidate, _)| candidate.is_file())?;
    let text = std::fs::read_to_string(readme_path).ok()?;
    let mut title = None;
    let mut description_lines = Vec::new();
    let mut in_code_fence = false;
    let mut seen_heading = false;

    for raw_line in text.lines().take(120) {
        let line = raw_line.trim();
        if line.starts_with("```") || line.starts_with("~~~") {
            in_code_fence = !in_code_fence;
            continue;
        }
        if in_code_fence || line.is_empty() {
            if !description_lines.is_empty() {
                break;
            }
            continue;
        }
        if line.starts_with("<!--") || line.starts_with("[!") || line.starts_with("![") {
            continue;
        }
        if title.is_none() {
            if let Some(heading) = markdown_heading_text(line) {
                title = Some(heading);
                seen_heading = true;
                continue;
            }
        } else if markdown_heading_text(line).is_some() {
            if !description_lines.is_empty() {
                break;
            }
            continue;
        }
        if seen_heading || title.is_none() {
            if let Some(text) = clean_readme_line(line) {
                description_lines.push(text);
            }
        }
    }

    let description = clean_project_text(&description_lines.join(" "), 240);
    identity_from_parts(fallback_name, title, description, source)
}

pub(super) fn identity_from_git_remote(remote: Option<&str>) -> Option<ProjectIdentity> {
    let remote = remote.map(str::trim).filter(|value| !value.is_empty())?;
    let trimmed = remote.trim_end_matches('/');
    let mut name_part = trimmed.rsplit(['/', ':']).next().unwrap_or(trimmed).trim();
    if let Some(stripped) = name_part.strip_suffix(".git") {
        name_part = stripped;
    }
    let name = clean_project_text(name_part, 120)?;
    if name == "." || name == ".." {
        return None;
    }
    Some(ProjectIdentity {
        description: Some(default_project_description(&name)),
        name,
        source: Some("Git 远端".to_string()),
    })
}

pub(super) fn identity_from_parts(
    fallback_name: &str,
    name: Option<String>,
    description: Option<String>,
    source: &str,
) -> Option<ProjectIdentity> {
    if name.is_none() && description.is_none() {
        return None;
    }
    let name = name.unwrap_or_else(|| fallback_name.to_string());
    let description = description.or_else(|| Some(default_project_description(&name)));
    Some(ProjectIdentity {
        name,
        description,
        source: Some(source.to_string()),
    })
}

pub(super) fn markdown_heading_text(line: &str) -> Option<String> {
    let text = line.strip_prefix('#')?;
    if !text.starts_with('#') && !text.starts_with(' ') {
        return None;
    }
    let text = line.trim_start_matches('#').trim();
    clean_project_text(&strip_markdown_inline(text), 120)
}

pub(super) fn clean_readme_line(line: &str) -> Option<String> {
    if line.starts_with('#') || line.starts_with('|') || line.starts_with('>') {
        return None;
    }
    let text = strip_markdown_inline(line);
    clean_project_text(&text, 240)
}

pub(super) fn strip_markdown_inline(value: &str) -> String {
    let mut output = String::new();
    let mut chars = value.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '`' | '*' | '_' => {}
            '[' => {
                let mut label = String::new();
                for next in chars.by_ref() {
                    if next == ']' {
                        break;
                    }
                    label.push(next);
                }
                if chars.peek() == Some(&'(') {
                    for next in chars.by_ref() {
                        if next == ')' {
                            break;
                        }
                    }
                }
                output.push_str(&label);
            }
            _ => output.push(ch),
        }
    }
    output.trim().to_string()
}

pub(super) fn first_json_string(
    object: &serde_json::Map<String, Value>,
    keys: &[&str],
) -> Option<String> {
    keys.iter()
        .find_map(|key| object.get(*key).and_then(|value| value.as_str()))
        .and_then(|value| clean_project_text(value, 240))
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
