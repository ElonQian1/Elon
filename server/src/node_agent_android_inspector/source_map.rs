use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;
use sha2::{Digest, Sha256};

use super::types::{RuntimeUiNode, SourceMapEntry};

const MAX_SOURCE_FILES: usize = 2_500;
const MAX_SOURCE_BYTES: u64 = 1024 * 1024;
const MAX_CANDIDATES: usize = 6;

pub(crate) struct SourceMapAttachment {
    pub root: Option<String>,
    pub fingerprint: Option<String>,
    pub bindings_path: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct BindingRegistry<'a> {
    version: u32,
    project_root: &'a str,
    project_fingerprint: &'a str,
    bindings: Vec<NodeBinding<'a>>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct NodeBinding<'a> {
    node_id: &'a str,
    resource_id: Option<&'a str>,
    xpath: &'a str,
    candidates: &'a [SourceMapEntry],
}

pub(crate) fn attach_source_map(
    nodes: &mut [RuntimeUiNode],
    explicit_root: Option<&str>,
) -> SourceMapAttachment {
    let Some(root) = resolve_source_root(explicit_root) else {
        return SourceMapAttachment {
            root: None,
            fingerprint: None,
            bindings_path: None,
        };
    };
    let files = collect_project_source_files(&root);
    let index = build_source_index(&root, &files, nodes);
    for node in nodes.iter_mut() {
        let candidates = index.candidates_for(node);
        node.source = candidates.first().cloned();
        node.source_candidates = candidates;
    }
    let root_text = root.display().to_string();
    let fingerprint = project_fingerprint(&root_text);
    let bindings_path = persist_bindings(&root_text, &fingerprint, nodes);
    SourceMapAttachment {
        root: Some(root_text),
        fingerprint: Some(fingerprint),
        bindings_path,
    }
}

#[derive(Default)]
struct SourceCandidateIndex {
    by_resource: HashMap<String, Vec<SourceMapEntry>>,
    by_literal: HashMap<String, Vec<SourceMapEntry>>,
}

impl SourceCandidateIndex {
    fn candidates_for(&self, node: &RuntimeUiNode) -> Vec<SourceMapEntry> {
        let mut candidates = node
            .resource_id
            .as_deref()
            .and_then(resource_id_name)
            .and_then(|id| self.by_resource.get(id))
            .cloned()
            .unwrap_or_default();
        if candidates.is_empty() {
            for literal in [&node.content_desc, &node.text] {
                if is_useful_literal(literal) {
                    candidates.extend(self.by_literal.get(literal).cloned().unwrap_or_default());
                }
            }
        }
        candidates.sort_by(|left, right| {
            right
                .confidence
                .partial_cmp(&left.confidence)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let mut seen = HashSet::new();
        candidates.retain(|candidate| seen.insert((candidate.file.clone(), candidate.line)));
        candidates.truncate(MAX_CANDIDATES);
        candidates
    }
}

fn build_source_index(
    root: &Path,
    files: &[PathBuf],
    nodes: &[RuntimeUiNode],
) -> SourceCandidateIndex {
    let resource_ids = nodes
        .iter()
        .filter_map(|node| node.resource_id.as_deref().and_then(resource_id_name))
        .map(str::to_string)
        .collect::<HashSet<_>>();
    let literals = nodes
        .iter()
        .flat_map(|node| [&node.content_desc, &node.text])
        .filter(|literal| is_useful_literal(literal))
        .cloned()
        .collect::<HashSet<_>>();
    let mut index = SourceCandidateIndex::default();
    for file in files {
        let Some(content) = read_small_source(file) else {
            continue;
        };
        let extension = file
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or("");
        for (line_index, line) in content.lines().enumerate() {
            let line_number = line_index + 1;
            if extension == "xml" {
                for marker in ["@+id/", "@id/"] {
                    for id in identifiers_after(line, marker) {
                        if resource_ids.contains(id) {
                            index.by_resource.entry(id.to_string()).or_default().push(
                                source_entry(
                                    root,
                                    file,
                                    line_number,
                                    format!("{marker}{id}"),
                                    0.98,
                                    "resource_id_xml",
                                    "resource-id 精确匹配 Android XML",
                                ),
                            );
                        }
                    }
                }
            } else if matches!(extension, "kt" | "java") {
                for marker in ["R.id.", "binding."] {
                    for id in identifiers_after(line, marker) {
                        if resource_ids.contains(id) {
                            index.by_resource.entry(id.to_string()).or_default().push(
                                source_entry(
                                    root,
                                    file,
                                    line_number,
                                    format!("{marker}{id}"),
                                    0.86,
                                    "resource_id_code",
                                    "resource-id 在 Kotlin/Java 中被引用",
                                ),
                            );
                        }
                    }
                }
                for (marker, confidence, reason) in [
                    ("testTag(", 0.9, "Compose testTag 与运行时语义精确匹配"),
                    (
                        "contentDescription =",
                        0.68,
                        "运行时文本/内容描述匹配 Compose 源码候选",
                    ),
                    ("Text(", 0.68, "运行时文本/内容描述匹配 Compose 源码候选"),
                    ("text =", 0.68, "运行时文本/内容描述匹配 Compose 源码候选"),
                ] {
                    let Some(literal) = quoted_value_after(line, marker) else {
                        continue;
                    };
                    if literals.contains(literal) {
                        index
                            .by_literal
                            .entry(literal.to_string())
                            .or_default()
                            .push(source_entry(
                                root,
                                file,
                                line_number,
                                format!("{marker}\"{literal}\""),
                                confidence,
                                "compose_semantics",
                                reason,
                            ));
                    }
                }
            }
        }
    }
    index
}

fn source_entry(
    root: &Path,
    file: &Path,
    line: usize,
    token: String,
    confidence: f32,
    match_kind: &str,
    reason: &str,
) -> SourceMapEntry {
    let relative = file
        .strip_prefix(root)
        .unwrap_or(file)
        .display()
        .to_string();
    let (component_key, scope) = component_identity(file, &relative);
    SourceMapEntry {
        file: relative,
        line: Some(line),
        token,
        confidence,
        reason: reason.to_string(),
        match_kind: match_kind.to_string(),
        component_key,
        scope,
    }
}

fn component_identity(path: &Path, relative: &str) -> (String, String) {
    let stem = path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("unknown")
        .to_ascii_lowercase();
    let normalized = relative.replace('\\', "/");
    let is_layout = normalized.contains("/res/layout");
    let repeated = ["item", "row", "cell", "card", "tile", "list"]
        .iter()
        .any(|part| stem.contains(part));
    let prefix = if is_layout { "layout" } else { "compose" };
    let scope = if repeated {
        "repeated_component"
    } else {
        "component"
    };
    (format!("{prefix}:{stem}"), scope.to_string())
}

fn identifiers_after<'a>(line: &'a str, marker: &str) -> Vec<&'a str> {
    let mut values = Vec::new();
    let mut remaining = line;
    while let Some(offset) = remaining.find(marker) {
        let value = &remaining[offset + marker.len()..];
        let length = value
            .chars()
            .take_while(|ch| ch.is_ascii_alphanumeric() || *ch == '_')
            .map(char::len_utf8)
            .sum::<usize>();
        if length > 0 {
            values.push(&value[..length]);
        }
        remaining = &value[length..];
    }
    values
}

fn quoted_value_after<'a>(line: &'a str, marker: &str) -> Option<&'a str> {
    let remainder = line.split_once(marker)?.1;
    let value = remainder.split_once('"')?.1;
    value.split_once('"').map(|(literal, _)| literal)
}

fn read_small_source(path: &Path) -> Option<String> {
    let metadata = fs::metadata(path).ok()?;
    if metadata.len() > MAX_SOURCE_BYTES {
        return None;
    }
    fs::read_to_string(path).ok()
}

fn resource_id_name(resource_id: &str) -> Option<&str> {
    let value = resource_id.trim();
    value
        .rsplit_once(":id/")
        .or_else(|| value.rsplit_once("/id/"))
        .map(|(_, id)| id.trim())
        .filter(|id| !id.is_empty())
}

fn is_useful_literal(value: &str) -> bool {
    let size = value.trim().chars().count();
    (3..=80).contains(&size)
}

fn resolve_source_root(explicit_root: Option<&str>) -> Option<PathBuf> {
    if let Some(root) = canonical_android_project(explicit_root.map(PathBuf::from)) {
        return Some(root);
    }
    if let Some(root) =
        canonical_android_project(std::env::var_os("ELON_ANDROID_SOURCE_ROOT").map(PathBuf::from))
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
            if has_android_sources(ancestor) {
                return ancestor.canonicalize().ok();
            }
        }
    }
    None
}

fn canonical_android_project(candidate: Option<PathBuf>) -> Option<PathBuf> {
    candidate
        .and_then(|path| path.canonicalize().ok())
        .filter(|path| has_android_sources(path))
}

fn has_android_sources(path: &Path) -> bool {
    find_android_manifest(path, 0)
}

fn find_android_manifest(root: &Path, depth: usize) -> bool {
    if depth > 12 {
        return false;
    }
    let Ok(entries) = fs::read_dir(root) else {
        return false;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let name = path
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or("");
            if matches!(
                name,
                ".git" | "build" | "target" | "node_modules" | ".gradle"
            ) {
                continue;
            }
            if find_android_manifest(&path, depth + 1) {
                return true;
            }
        } else if path.file_name().and_then(|name| name.to_str()) == Some("AndroidManifest.xml")
            && path
                .to_string_lossy()
                .replace('\\', "/")
                .contains("/src/main/")
        {
            return true;
        }
    }
    false
}

fn collect_project_source_files(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    visit_dirs(root, 0, &mut |path| {
        if files.len() >= MAX_SOURCE_FILES {
            return;
        }
        let extension = path.extension().and_then(|value| value.to_str());
        if matches!(extension, Some("xml") | Some("kt") | Some("java"))
            && path
                .to_string_lossy()
                .replace('\\', "/")
                .contains("/src/main/")
        {
            files.push(path.to_path_buf());
        }
    });
    files
}

fn visit_dirs(root: &Path, depth: usize, visit: &mut impl FnMut(&Path)) {
    if depth > 12 {
        return;
    }
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let name = path
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or("");
            if matches!(
                name,
                ".git" | "build" | "target" | "node_modules" | ".gradle"
            ) {
                continue;
            }
            visit_dirs(&path, depth + 1, visit);
        } else {
            visit(&path);
        }
    }
}

fn project_fingerprint(root: &str) -> String {
    let canonical = root.replace('\\', "/").to_ascii_lowercase();
    let digest = Sha256::digest(canonical.as_bytes());
    hex::encode(&digest[..12])
}

fn persist_bindings(root: &str, fingerprint: &str, nodes: &[RuntimeUiNode]) -> Option<String> {
    let bindings = nodes
        .iter()
        .filter(|node| !node.source_candidates.is_empty())
        .map(|node| NodeBinding {
            node_id: &node.id,
            resource_id: node.resource_id.as_deref(),
            xpath: &node.xpath,
            candidates: &node.source_candidates,
        })
        .collect::<Vec<_>>();
    let path = crate::state_path()
        .with_file_name("android-inspector-bindings")
        .join(format!("{fingerprint}.json"));
    fs::create_dir_all(path.parent()?).ok()?;
    let payload = BindingRegistry {
        version: 1,
        project_root: root,
        project_fingerprint: fingerprint,
        bindings,
    };
    let bytes = serde_json::to_vec_pretty(&payload).ok()?;
    fs::write(&path, bytes).ok()?;
    Some(path.display().to_string())
}

#[cfg(test)]
mod tests {
    use super::{build_source_index, component_identity, resource_id_name};
    use crate::node_agent_android_inspector::types::{BoundsRect, RuntimeUiNode};
    use std::{fs, path::Path};

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

    #[test]
    fn recognizes_repeated_layout_component_files() {
        assert_eq!(
            component_identity(
                Path::new("android/app/src/main/res/layout/item_project_card.xml"),
                "android/app/src/main/res/layout/item_project_card.xml"
            ),
            (
                "layout:item_project_card".to_string(),
                "repeated_component".to_string()
            )
        );
    }

    #[test]
    fn maps_runtime_resource_to_xml_and_code_candidates() {
        let root = std::env::temp_dir().join(format!(
            "elon-ui-source-map-{}-resource",
            std::process::id()
        ));
        let layout = root.join("app/src/main/res/layout/item_project_card.xml");
        let code = root.join("app/src/main/java/com/elon/ProjectAdapter.kt");
        fs::create_dir_all(layout.parent().unwrap()).unwrap();
        fs::create_dir_all(code.parent().unwrap()).unwrap();
        fs::write(&layout, "<View android:id=\"@+id/projectCard\" />").unwrap();
        fs::write(&code, "val card = view.findViewById(R.id.projectCard)").unwrap();
        let node = RuntimeUiNode {
            id: "node-1".to_string(),
            depth: 2,
            index_path: vec![0, 1],
            xpath: "/node[1]/node[2]".to_string(),
            text: "项目".to_string(),
            content_desc: String::new(),
            resource_id: Some("com.elon.app:id/projectCard".to_string()),
            package_name: Some("com.elon.app".to_string()),
            class_name: Some("android.view.View".to_string()),
            bounds: BoundsRect {
                left: 0,
                top: 0,
                right: 100,
                bottom: 80,
                width: 100,
                height: 80,
            },
            clickable: true,
            enabled: true,
            focusable: false,
            focused: false,
            scrollable: false,
            checkable: false,
            checked: false,
            selected: false,
            password: false,
            visible: true,
            source: None,
            source_candidates: Vec::new(),
        };
        let index = build_source_index(&root, &[layout, code], std::slice::from_ref(&node));
        let candidates = index.candidates_for(&node);
        assert_eq!(candidates.len(), 2);
        assert_eq!(candidates[0].match_kind, "resource_id_xml");
        assert_eq!(candidates[0].scope, "repeated_component");
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn maps_compose_test_tag_without_rescanning_files_per_node() {
        let root =
            std::env::temp_dir().join(format!("elon-ui-source-map-{}-compose", std::process::id()));
        let code = root.join("app/src/main/java/com/elon/ProjectCard.kt");
        fs::create_dir_all(code.parent().unwrap()).unwrap();
        fs::write(
            &code,
            "@Composable fun ProjectCard() { Modifier.testTag(\"project-card\") }",
        )
        .unwrap();
        let node = RuntimeUiNode {
            id: "node-compose".to_string(),
            depth: 2,
            index_path: vec![0, 1],
            xpath: "/node[1]/node[2]".to_string(),
            text: String::new(),
            content_desc: "project-card".to_string(),
            resource_id: None,
            package_name: Some("com.elon.app".to_string()),
            class_name: Some("android.view.View".to_string()),
            bounds: BoundsRect {
                left: 0,
                top: 0,
                right: 100,
                bottom: 80,
                width: 100,
                height: 80,
            },
            clickable: true,
            enabled: true,
            focusable: false,
            focused: false,
            scrollable: false,
            checkable: false,
            checked: false,
            selected: false,
            password: false,
            visible: true,
            source: None,
            source_candidates: Vec::new(),
        };
        let index = build_source_index(&root, &[code], std::slice::from_ref(&node));
        let candidates = index.candidates_for(&node);
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].match_kind, "compose_semantics");
        assert_eq!(candidates[0].confidence, 0.9);
        let _ = fs::remove_dir_all(root);
    }
}
