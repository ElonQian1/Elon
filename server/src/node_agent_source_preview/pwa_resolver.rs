use super::types::{
    PwaExplicitStyleBinding, PwaSourceRange, PwaStyleBindingKind, ResolvePwaStyleBindingRequest,
    ResolvePwaStyleBindingResponse,
};
use anyhow::{anyhow, Context, Result};
use ignore::WalkBuilder;
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs, path::Path};

const MAX_SOURCE_BYTES: u64 = 2 * 1024 * 1024;
const MAX_SCANNED_FILES: usize = 3_000;
const MAX_SELECTORS: usize = 16;
const STYLE_PROPERTIES: &[(&str, &str)] = &[
    ("width", "width"),
    ("height", "height"),
    ("paddingTop", "padding-top"),
    ("paddingRight", "padding-right"),
    ("paddingBottom", "padding-bottom"),
    ("paddingLeft", "padding-left"),
    ("marginTop", "margin-top"),
    ("marginRight", "margin-right"),
    ("marginBottom", "margin-bottom"),
    ("marginLeft", "margin-left"),
    ("borderRadius", "border-radius"),
    ("fontSize", "font-size"),
    ("fontWeight", "font-weight"),
    ("lineHeight", "line-height"),
    ("color", "color"),
    ("backgroundColor", "background-color"),
    ("opacity", "opacity"),
];

pub(crate) fn resolve_pwa_style_binding(
    request: &ResolvePwaStyleBindingRequest,
) -> Result<ResolvePwaStyleBindingResponse> {
    let raw_root = request.project_root.trim();
    let root_path = Path::new(raw_root);
    if raw_root.is_empty() || !root_path.is_dir() {
        return Err(anyhow!("请选择有效的本机 PWA 项目目录"));
    }
    let root = root_path.canonicalize().context("无法解析 PWA 项目目录")?;
    let selectors = normalized_selectors(&request.selectors)?;
    let mut bindings = selectors
        .iter()
        .map(|_| Vec::<PwaExplicitStyleBinding>::new())
        .collect::<Vec<_>>();
    let mut candidate_counts = vec![0usize; selectors.len()];
    let mut scanned_files = 0usize;
    let walker = WalkBuilder::new(&root)
        .hidden(false)
        .git_ignore(true)
        .git_global(false)
        .git_exclude(true)
        .parents(true)
        .filter_entry(|entry| !ignored_directory(entry.path()))
        .build();
    for entry in walker.filter_map(|entry| entry.ok()) {
        if scanned_files >= MAX_SCANNED_FILES {
            break;
        }
        let path = entry.path();
        if !entry.file_type().is_some_and(|kind| kind.is_file()) || !supported_source(path) {
            continue;
        }
        let metadata = match path.metadata() {
            Ok(value) if value.len() <= MAX_SOURCE_BYTES => value,
            _ => continue,
        };
        if !metadata.is_file() {
            continue;
        }
        scanned_files += 1;
        let content = match fs::read_to_string(path) {
            Ok(value) => value,
            Err(_) => continue,
        };
        for (selector_index, selector) in selectors.iter().enumerate() {
            for (start, end, target) in find_rules(&content, selector, is_html(path)) {
                let relative = path
                    .strip_prefix(&root)
                    .context("PWA 样式候选越出项目目录")?
                    .to_string_lossy()
                    .replace('\\', "/");
                candidate_counts[selector_index] += 1;
                if bindings[selector_index].is_empty() {
                    bindings[selector_index].push(PwaExplicitStyleBinding {
                        version: 1,
                        source_file: relative,
                        source_revision: sha256(&content),
                        kind: PwaStyleBindingKind::CssRule,
                        target,
                        range: PwaSourceRange { start, end },
                        property_map: STYLE_PROPERTIES
                            .iter()
                            .map(|(property, source)| {
                                ((*property).to_string(), (*source).to_string())
                            })
                            .collect::<BTreeMap<_, _>>(),
                    });
                }
            }
        }
    }
    let selected_index = candidate_counts.iter().position(|count| *count > 0);
    let candidate_count = selected_index.map_or(0, |index| candidate_counts[index]);
    let binding = selected_index
        .filter(|index| candidate_counts[*index] == 1)
        .and_then(|index| bindings[index].pop());
    let detail = match candidate_count {
        0 => "没有找到与真实 DOM 匹配的静态 CSS 规则，需要 AI 按需建立绑定",
        1 => "已按真实 DOM 的专用选择器找到唯一静态 CSS 规则，可确定性写回",
        _ => "最高优先级选择器对应多个 CSS 规则，为避免误改已停止自动绑定",
    };
    Ok(ResolvePwaStyleBindingResponse {
        ok: true,
        binding,
        candidate_count,
        detail: detail.to_string(),
    })
}

fn normalized_selectors(values: &[String]) -> Result<Vec<String>> {
    if values.is_empty() || values.len() > MAX_SELECTORS {
        return Err(anyhow!(
            "selectors 必须包含 1 到 {MAX_SELECTORS} 个真实 CSS 选择器"
        ));
    }
    let mut result = Vec::new();
    for value in values {
        let selector = value.trim();
        if selector.is_empty()
            || selector.len() > 240
            || selector.chars().any(char::is_control)
            || selector.contains(['{', '}', ';'])
        {
            return Err(anyhow!("CSS 选择器为空、过长或包含不安全字符"));
        }
        if !result.iter().any(|item| item == selector) {
            result.push(selector.to_string());
        }
    }
    Ok(result)
}

fn ignored_directory(path: &Path) -> bool {
    path.file_name()
        .and_then(|value| value.to_str())
        .is_some_and(|name| {
            matches!(
                name.to_ascii_lowercase().as_str(),
                ".git" | "node_modules" | "target" | "build" | "dist" | ".next" | ".gradle"
            )
        })
}

fn supported_source(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|value| value.to_str())
            .unwrap_or("")
            .to_ascii_lowercase()
            .as_str(),
        "css" | "scss" | "sass" | "less" | "html" | "htm"
    )
}

fn is_html(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|value| value.to_str())
            .unwrap_or("")
            .to_ascii_lowercase()
            .as_str(),
        "html" | "htm"
    )
}

fn find_rules(content: &str, selector: &str, html: bool) -> Vec<(usize, usize, String)> {
    let mut result = Vec::new();
    for (match_start, _) in content.match_indices(selector) {
        if html && !inside_style_block(content, match_start) {
            continue;
        }
        let Some(open) = content[match_start + selector.len()..]
            .find('{')
            .map(|offset| match_start + selector.len() + offset)
        else {
            continue;
        };
        if open.saturating_sub(match_start) > 1_000 {
            continue;
        }
        let previous_rule_end = content[..match_start]
            .rfind('}')
            .map_or(0, |index| index + 1);
        let style_content_start = html
            .then(|| content[..match_start].to_ascii_lowercase().rfind("<style"))
            .flatten()
            .and_then(|tag_start| {
                content[tag_start..match_start]
                    .find('>')
                    .map(|offset| tag_start + offset + 1)
            })
            .unwrap_or(0);
        let raw_start = previous_rule_end.max(style_content_start);
        let leading = content[raw_start..open].len() - content[raw_start..open].trim_start().len();
        let start = raw_start + leading;
        let target = content[start..open].trim();
        if !selector_matches_target(target, selector) {
            continue;
        }
        let Some(close) = matching_closing_brace(content, open) else {
            continue;
        };
        if !result.iter().any(|(existing, _, _)| *existing == start) {
            result.push((start, close + 1, target.to_string()));
        }
    }
    result
}

fn selector_matches_target(target: &str, selector: &str) -> bool {
    normalize_selector(target) == normalize_selector(selector)
}

fn normalize_selector(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn inside_style_block(content: &str, position: usize) -> bool {
    let before = content[..position].to_ascii_lowercase();
    let after = content[position..].to_ascii_lowercase();
    let Some(open) = before.rfind("<style") else {
        return false;
    };
    before.rfind("</style>").map_or(true, |close| close < open) && after.contains("</style>")
}

fn matching_closing_brace(content: &str, open: usize) -> Option<usize> {
    let bytes = content.as_bytes();
    let mut depth = 0usize;
    let mut quote = None;
    let mut escaped = false;
    let mut index = open;
    while index < bytes.len() {
        let byte = bytes[index];
        if let Some(active) = quote {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == active {
                quote = None;
            }
        } else if matches!(byte, b'\'' | b'"') {
            quote = Some(byte);
        } else if byte == b'{' {
            depth += 1;
        } else if byte == b'}' {
            depth = depth.checked_sub(1)?;
            if depth == 0 {
                return Some(index);
            }
        }
        index += 1;
    }
    None
}

fn sha256(content: &str) -> String {
    hex::encode(Sha256::digest(content.as_bytes()))
}
