use std::{
    collections::{BTreeSet, HashSet},
    fs,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, bail, Context, Result};
use ignore::WalkBuilder;
use serde::Serialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use super::{
    broker::LiveUiSession,
    design_session_store::{read_record, read_verified_tree},
};

const TOOL: &str = "ui_suggest_design_source_binding";
const MAX_SCANNED_FILES: usize = 2_000;
const MAX_FILE_BYTES: u64 = 512 * 1024;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct Candidate {
    file: String,
    line: usize,
    byte_range: ByteRange,
    excerpt: String,
    score: u32,
    matched_signals: Vec<String>,
    source_sha256: String,
    suggested_binding: Value,
}

#[derive(Debug, Serialize)]
struct ByteRange {
    start: usize,
    end: usize,
}

struct SearchContext {
    selector: String,
    label: Option<String>,
    role: Option<String>,
    tag: Option<String>,
    route_term: Option<String>,
    tokens: Vec<String>,
}

pub(super) fn tool_definitions() -> Vec<Value> {
    vec![json!({
        "name":TOOL,
        "description":"根据已校验 UI tree、selector 和 route，在 designSession 声明的项目源码根内给出有界源码绑定候选。结果始终是 CANDIDATE，必须由代理或用户显式确认后才能变成 BOUND。",
        "inputSchema":{"type":"object","additionalProperties":false,"required":["draftId"],
            "properties":{"draftId":{"type":"string","pattern":"^draft_[a-f0-9]{32}$"},
                "limit":{"type":"integer","minimum":1,"maximum":20,"default":8}}},
        "annotations":{"readOnlyHint":true,"destructiveHint":false,
            "idempotentHint":true,"openWorldHint":false}
    })]
}

pub(super) fn is_tool(name: &str) -> bool {
    name == TOOL
}

pub(super) fn call(session: &LiveUiSession, arguments: Value) -> Result<Value> {
    let draft_id = required_text(&arguments, "draftId")?;
    let limit = arguments
        .get("limit")
        .and_then(Value::as_u64)
        .unwrap_or(8)
        .clamp(1, 20) as usize;
    let root = canonical_root(session)?;
    let draft_result =
        super::design_drafts::call(session, "ui_get_design_draft", json!({"draftId":draft_id}))?;
    let draft = draft_result
        .get("draft")
        .context("设计草稿响应缺少 draft")?;
    let record = read_record(&root, required_text(draft, "designSessionId")?)?;
    let evidence = record
        .last_evidence
        .as_ref()
        .context("designSession 尚无 UI tree；请先执行后台捕获")?;
    let tree = read_verified_tree(&root, evidence)?;
    let selector = required_text(draft, "selector")?;
    let context = search_context(selector, required_text(draft, "route")?, &tree);
    let (mut candidates, scanned_files, truncated) =
        scan_source_roots(&root, &record.target.source_roots, &context)?;
    candidates.sort_by(|left, right| {
        right
            .score
            .cmp(&left.score)
            .then_with(|| left.file.cmp(&right.file))
            .then_with(|| left.line.cmp(&right.line))
    });
    candidates.truncate(limit);
    Ok(json!({
        "schema":"elon.ui-design-source-binding-candidates.v1",
        "draftId":draft_id,
        "designSessionId":record.design_session_id,
        "selector":selector,
        "uiTreeVerified":true,
        "candidates":candidates,
        "scan":{"filesInspected":scanned_files,"truncated":truncated,
            "sourceRoots":record.target.source_roots,"maxFileBytes":MAX_FILE_BYTES},
        "bindingStatus":"CANDIDATE",
        "autoBound":false,
        "contentEmbedded":false
    }))
}

fn scan_source_roots(
    root: &Path,
    source_roots: &[String],
    context: &SearchContext,
) -> Result<(Vec<Candidate>, usize, bool)> {
    if source_roots.is_empty() {
        bail!("designSession 没有声明 sourceRoots");
    }
    let mut candidates = Vec::new();
    let mut visited = HashSet::new();
    let mut scanned = 0usize;
    let mut truncated = false;
    for source_root in source_roots {
        let directory = canonical_source_root(root, source_root)?;
        for entry in WalkBuilder::new(&directory)
            .hidden(false)
            .git_ignore(true)
            .git_exclude(true)
            .max_depth(Some(12))
            .build()
            .filter_map(|entry| entry.ok())
        {
            if scanned >= MAX_SCANNED_FILES {
                truncated = true;
                break;
            }
            let path = entry.path();
            if !entry.file_type().is_some_and(|kind| kind.is_file())
                || ignored_path(path)
                || !supported_source(path)
            {
                continue;
            }
            let canonical = match path.canonicalize() {
                Ok(value) if value.starts_with(root) => value,
                _ => continue,
            };
            if !visited.insert(canonical.clone()) {
                continue;
            }
            let metadata = match fs::metadata(&canonical) {
                Ok(value) if value.len() <= MAX_FILE_BYTES => value,
                _ => continue,
            };
            let _ = metadata;
            scanned += 1;
            if let Some(candidate) = inspect_file(root, &canonical, context)? {
                candidates.push(candidate);
            }
        }
        if truncated {
            break;
        }
    }
    Ok((candidates, scanned, truncated))
}

fn inspect_file(root: &Path, path: &Path, context: &SearchContext) -> Result<Option<Candidate>> {
    let bytes = fs::read(path)?;
    let text = String::from_utf8_lossy(&bytes);
    let mut best: Option<(u32, usize, usize, usize, String, Vec<String>)> = None;
    let mut offset = 0usize;
    for (index, line_with_newline) in text.split_inclusive('\n').enumerate() {
        let line = line_with_newline.trim_end_matches(['\r', '\n']);
        if line.trim().is_empty() || contains_secret_marker(line) {
            offset += line_with_newline.len();
            continue;
        }
        let (score, signals) = score_line(line, context);
        if score > best.as_ref().map(|value| value.0).unwrap_or(0) {
            let leading = line.len().saturating_sub(line.trim_start().len());
            let start = offset + leading;
            let end = offset + line.len();
            best = Some((score, index + 1, start, end, compact_excerpt(line), signals));
        }
        offset += line_with_newline.len();
    }
    let Some((score, line, start, end, excerpt, signals)) = best.filter(|value| value.0 >= 10)
    else {
        return Ok(None);
    };
    let relative = path
        .strip_prefix(root)?
        .to_string_lossy()
        .replace('\\', "/");
    let sha256 = hex::encode(Sha256::digest(&bytes));
    let confidence = if score >= 100 {
        "HIGH"
    } else if score >= 55 {
        "MEDIUM"
    } else {
        "LOW"
    };
    let kind = binding_kind(path);
    Ok(Some(Candidate {
        file: relative.clone(),
        line,
        byte_range: ByteRange { start, end },
        excerpt,
        score,
        matched_signals: signals.clone(),
        source_sha256: sha256.clone(),
        suggested_binding: json!({
            "status":"CANDIDATE","sourceFile":relative,"symbol":Value::Null,
            "kind":kind,"range":{"start":start,"end":end},
            "sourceRevision":format!("sha256:{sha256}"),"confidence":confidence,
            "reason":format!("有界源码扫描命中：{}",signals.join(", "))
        }),
    }))
}

fn search_context(selector: &str, route: &str, tree: &Value) -> SearchContext {
    let node = tree
        .get("nodes")
        .and_then(Value::as_array)
        .and_then(|nodes| {
            nodes
                .iter()
                .find(|node| node.get("selector").and_then(Value::as_str) == Some(selector))
        });
    let label = node.and_then(|node| text_field(node, "label"));
    let role = node.and_then(|node| text_field(node, "role"));
    let tag = node.and_then(|node| text_field(node, "tag"));
    let route_term = route
        .split(['/', '?', '#'])
        .filter(|value| value.chars().count() >= 3)
        .next_back()
        .map(str::to_ascii_lowercase);
    let mut tokens = BTreeSet::new();
    for value in [
        Some(selector.to_string()),
        label.clone(),
        role.clone(),
        tag.clone(),
    ]
    .into_iter()
    .flatten()
    {
        for token in value
            .split(|ch: char| !ch.is_alphanumeric())
            .map(str::to_ascii_lowercase)
            .filter(|value| value.chars().count() >= 3 && !generic_token(value))
        {
            tokens.insert(token);
        }
    }
    SearchContext {
        selector: selector.to_string(),
        label,
        role,
        tag,
        route_term,
        tokens: tokens.into_iter().collect(),
    }
}

fn score_line(line: &str, context: &SearchContext) -> (u32, Vec<String>) {
    let lower = line.to_ascii_lowercase();
    let mut score = 0u32;
    let mut signals = Vec::new();
    if line.contains(&context.selector) {
        score += 90;
        signals.push("exact-selector".into());
    }
    if let Some(label) = context
        .label
        .as_ref()
        .filter(|value| value.chars().count() >= 2)
    {
        if lower.contains(&label.to_ascii_lowercase()) {
            score += 55;
            signals.push("ui-label".into());
        }
    }
    for token in &context.tokens {
        if lower.contains(token) {
            score += 12;
            signals.push(format!("token:{token}"));
        }
    }
    for (value, name, points) in [
        (context.role.as_ref(), "role", 8),
        (context.tag.as_ref(), "tag", 5),
        (context.route_term.as_ref(), "route", 18),
    ] {
        if value.is_some_and(|value| lower.contains(&value.to_ascii_lowercase())) {
            score += points;
            signals.push(name.into());
        }
    }
    signals.sort();
    signals.dedup();
    (score, signals)
}

fn canonical_source_root(root: &Path, value: &str) -> Result<PathBuf> {
    let relative = Path::new(value);
    if relative.is_absolute() || relative.components().any(|part| part.as_os_str() == "..") {
        bail!("sourceRoot 不是安全相对路径");
    }
    let canonical = root
        .join(relative)
        .canonicalize()
        .with_context(|| format!("sourceRoot 不存在: {value}"))?;
    if !canonical.starts_with(root) || !canonical.is_dir() {
        bail!("sourceRoot 越出项目或不是目录");
    }
    Ok(canonical)
}

fn supported_source(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|value| value.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase()
            .as_str(),
        "tsx"
            | "ts"
            | "jsx"
            | "js"
            | "vue"
            | "svelte"
            | "css"
            | "scss"
            | "less"
            | "html"
            | "rs"
            | "kt"
            | "kts"
            | "java"
            | "xml"
    )
}

fn ignored_path(path: &Path) -> bool {
    path.components().any(|component| {
        matches!(
            component.as_os_str().to_string_lossy().as_ref(),
            ".git" | ".elon" | "node_modules" | "target" | "build" | "dist" | ".gradle"
        )
    })
}

fn contains_secret_marker(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    [
        "password",
        "authorization",
        "private_key",
        "privatekey",
        "api_key",
        "apikey",
        "access_token",
        "refresh_token",
        "client_secret",
    ]
    .iter()
    .any(|marker| lower.contains(marker))
}

fn compact_excerpt(line: &str) -> String {
    line.trim().chars().take(220).collect()
}

fn binding_kind(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase()
        .as_str()
    {
        "css" | "scss" | "less" => "STYLE_RULE",
        "xml" => "XML_LAYOUT",
        "kt" | "kts" | "java" => "ANDROID_SOURCE",
        "rs" => "RUST_SOURCE",
        _ => "UI_COMPONENT",
    }
}

fn text_field(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn generic_token(value: &str) -> bool {
    matches!(
        value,
        "body" | "html" | "div" | "span" | "button" | "input" | "main" | "root"
    )
}

fn required_text<'a>(value: &'a Value, key: &str) -> Result<&'a str> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow!("缺少 {key}"))
}

fn canonical_root(session: &LiveUiSession) -> Result<PathBuf> {
    PathBuf::from(
        session
            .project_root
            .as_deref()
            .context("源码绑定候选需要绑定项目目录")?,
    )
    .canonicalize()
    .context("项目目录不存在")
}
