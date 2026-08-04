//! Privacy-bounded Codex lifecycle hook for native project-memory receipts.
//!
//! PostToolUse persists only normalized repository-relative paths and a coarse
//! access kind. It intentionally ignores tool responses, prompts, commands,
//! transcripts, assistant messages, and source bodies. Stop may request one
//! bounded continuation when the turn observed enough distinct read evidence.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    io::{Read, Write},
    path::{Component, Path, PathBuf},
    time::SystemTime,
};

const MAX_INPUT_BYTES: u64 = 2 * 1024 * 1024;
const MAX_PATHS_PER_EVENT: usize = 24;
const MAX_OBSERVATIONS_PER_TURN: usize = 64;
const MAX_PROMPT_PATHS: usize = 6;
const MAX_PROMPT_PATH_CHARS: usize = 360;
const MAX_SESSION_PROMPTS: usize = 3;
const SESSION_RETENTION_SECS: u64 = 24 * 60 * 60;

#[derive(Debug, Default, Deserialize)]
struct HookInput {
    #[serde(default)]
    session_id: String,
    #[serde(default)]
    turn_id: String,
    #[serde(default)]
    cwd: String,
    #[serde(default)]
    hook_event_name: String,
    #[serde(default)]
    tool_name: String,
    #[serde(default)]
    tool_input: Value,
    #[serde(default)]
    stop_hook_active: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct PathObservation {
    schema: String,
    path: String,
    kind: String,
}

pub(crate) fn is_hook_invocation() -> bool {
    std::env::args()
        .skip(1)
        .any(|arg| arg == "--project-memory-hook")
}

pub(crate) fn run_stdio() -> Result<()> {
    let Some(input) = read_input() else {
        return Ok(());
    };
    let Some(workspace) = find_git_root(Path::new(input.cwd.trim())) else {
        return write_stop_continue_if_needed(&input);
    };
    let Some(session_dir) = session_directory(&workspace, &input.session_id) else {
        return write_stop_continue_if_needed(&input);
    };
    cleanup_expired_sessions();
    match input.hook_event_name.as_str() {
        "PostToolUse" => record_paths(&workspace, &session_dir, &input),
        "Stop" => handle_stop(&session_dir, &input),
        "SessionEnd" => {
            let _ = fs::remove_dir_all(session_dir);
            Ok(())
        }
        _ => Ok(()),
    }
}

fn read_input() -> Option<HookInput> {
    let mut bytes = Vec::new();
    std::io::stdin()
        .take(MAX_INPUT_BYTES + 1)
        .read_to_end(&mut bytes)
        .ok()?;
    if bytes.len() as u64 > MAX_INPUT_BYTES {
        return None;
    }
    serde_json::from_slice(&bytes).ok()
}

fn find_git_root(start: &Path) -> Option<PathBuf> {
    let mut current = start.canonicalize().ok()?;
    loop {
        if current.join(".git").exists() {
            return Some(current);
        }
        if !current.pop() {
            return None;
        }
    }
}

fn session_directory(workspace: &Path, session_id: &str) -> Option<PathBuf> {
    if session_id.trim().is_empty() {
        return None;
    }
    Some(
        hook_root()
            .join(short_hash(workspace.to_string_lossy().as_bytes()))
            .join(short_hash(session_id.as_bytes())),
    )
}

fn record_paths(workspace: &Path, session_dir: &Path, input: &HookInput) -> Result<()> {
    let turn_hash = short_hash(input.turn_id.as_bytes());
    if input.turn_id.trim().is_empty() {
        return Ok(());
    }
    let mut raw_paths = Vec::new();
    collect_structured_paths(&input.tool_input, None, 0, &mut raw_paths);
    collect_command_paths(&input.tool_name, &input.tool_input, &mut raw_paths);
    let kind = observation_kind(&input.tool_name, &input.tool_input);
    let mut normalized = BTreeMap::new();
    for raw in raw_paths.into_iter().take(MAX_PATHS_PER_EVENT * 3) {
        if let Some(path) = normalize_workspace_path(workspace, &raw) {
            normalized.insert(path, kind.clone());
            if normalized.len() >= MAX_PATHS_PER_EVENT {
                break;
            }
        }
    }
    let directory = session_dir.join("observations").join(turn_hash);
    fs::create_dir_all(&directory)?;
    for (path, kind) in normalized {
        let observation = PathObservation {
            schema: "elon.project_memory_path_observation.v1".to_string(),
            path: path.clone(),
            kind,
        };
        let target = directory.join(format!("{}.json", short_hash(path.as_bytes())));
        crate::node_agent_atomic_file::write(&target, &serde_json::to_vec(&observation)?)?;
    }
    Ok(())
}

fn collect_structured_paths(
    value: &Value,
    key: Option<&str>,
    depth: usize,
    output: &mut Vec<String>,
) {
    if depth > 4 || output.len() >= MAX_PATHS_PER_EVENT * 2 {
        return;
    }
    let key = key.unwrap_or_default().to_ascii_lowercase();
    let path_key = matches!(
        key.as_str(),
        "path" | "paths" | "file" | "files" | "file_path" | "file_paths" | "filepath" | "filename"
    );
    match value {
        Value::String(value) if path_key => output.push(value.clone()),
        Value::Array(values) if path_key => {
            output.extend(values.iter().filter_map(Value::as_str).map(str::to_string));
        }
        Value::Array(values) => {
            for value in values {
                collect_structured_paths(value, None, depth + 1, output);
            }
        }
        Value::Object(values) => {
            for (child_key, child) in values {
                if matches!(
                    child_key.to_ascii_lowercase().as_str(),
                    "tool_response" | "response" | "content" | "body" | "prompt" | "transcript"
                ) {
                    continue;
                }
                collect_structured_paths(child, Some(child_key), depth + 1, output);
            }
        }
        _ => {}
    }
}

fn collect_command_paths(tool_name: &str, input: &Value, output: &mut Vec<String>) {
    let Some(command) = input.get("command").and_then(Value::as_str) else {
        return;
    };
    if tool_name.eq_ignore_ascii_case("apply_patch") {
        for line in command.lines() {
            if let Some((_, path)) = line.split_once(" File: ") {
                output.push(path.trim().to_string());
            }
        }
        return;
    }
    if !tool_name.eq_ignore_ascii_case("Bash") {
        return;
    }
    for token in command.split_whitespace().take(160) {
        let candidate = token.trim_matches(|ch: char| {
            matches!(
                ch,
                '\'' | '"' | '`' | '(' | ')' | '[' | ']' | '{' | '}' | ',' | ';'
            )
        });
        if looks_like_path_token(candidate) {
            output.push(candidate.to_string());
        }
    }
}

fn looks_like_path_token(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 320
        && !value.starts_with('-')
        && !value
            .chars()
            .any(|ch| matches!(ch, '$' | '*' | '?' | '|' | '<' | '>' | '='))
        && (value.contains('/') || value.contains('\\') || allowed_file_name(value))
}

fn normalize_workspace_path(workspace: &Path, value: &str) -> Option<String> {
    let value = value
        .trim()
        .trim_matches(|ch: char| matches!(ch, '\'' | '"' | '`' | ',' | ';'));
    let value = value
        .rsplit_once(':')
        .filter(|(_, line)| line.parse::<u32>().is_ok())
        .map(|(path, _)| path)
        .unwrap_or(value);
    let candidate = PathBuf::from(value);
    let relative = if candidate.is_absolute() {
        candidate
            .canonicalize()
            .ok()?
            .strip_prefix(workspace)
            .ok()?
            .to_path_buf()
    } else {
        let mut safe = PathBuf::new();
        for component in candidate.components() {
            match component {
                Component::Normal(value) => safe.push(value),
                Component::CurDir => {}
                _ => return None,
            }
        }
        safe
    };
    if relative.as_os_str().is_empty() {
        return None;
    }
    let joined = workspace.join(&relative);
    if joined.exists() {
        let canonical = joined.canonicalize().ok()?;
        if !canonical.starts_with(workspace) || !canonical.is_file() {
            return None;
        }
    }
    let normalized = relative.to_string_lossy().replace('\\', "/");
    if ignored_path(&normalized) || !allowed_path(&normalized) {
        return None;
    }
    Some(normalized)
}

fn ignored_path(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    lower.starts_with(".git/")
        || lower.starts_with("target/")
        || lower.contains("/target/")
        || lower.starts_with("node_modules/")
        || lower.contains("/node_modules/")
        || lower.starts_with(".ai-tmp/")
        || lower.ends_with("cargo.lock")
        || lower.ends_with("package-lock.json")
        || lower.ends_with("pnpm-lock.yaml")
        || lower.ends_with("yarn.lock")
        || lower == ".env"
        || lower.contains("credential")
        || lower.contains("secret")
        || lower.contains("private_key")
}

fn allowed_path(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    allowed_file_name(&lower)
        || [
            ".rs", ".toml", ".md", ".json", ".yaml", ".yml", ".ts", ".tsx", ".js", ".jsx", ".mjs",
            ".cjs", ".kt", ".kts", ".java", ".py", ".go", ".cs", ".cpp", ".c", ".h", ".html",
            ".css", ".scss", ".sql", ".proto", ".ps1", ".sh",
        ]
        .iter()
        .any(|extension| lower.ends_with(extension))
}

fn allowed_file_name(path: &str) -> bool {
    let name = path
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(path)
        .to_ascii_lowercase();
    matches!(
        name.as_str(),
        "agents.md"
            | "codex.md"
            | "readme"
            | "readme.md"
            | "makefile"
            | "dockerfile"
            | "cargo.toml"
            | "package.json"
            | "tsconfig.json"
    )
}

fn observation_kind(tool_name: &str, input: &Value) -> String {
    let lower = tool_name.to_ascii_lowercase();
    if ["write", "edit", "create", "delete", "move", "copy", "patch"]
        .iter()
        .any(|marker| lower.contains(marker))
    {
        return "write".to_string();
    }
    let command = input
        .get("command")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_ascii_lowercase();
    if lower == "bash"
        && [
            "rg ",
            "get-content",
            "select-string",
            "git show",
            "git diff",
            "type ",
            "cat ",
        ]
        .iter()
        .any(|marker| command.contains(marker))
    {
        return "read".to_string();
    }
    if lower.contains("read") || lower.contains("view") || lower.contains("search") {
        return "read".to_string();
    }
    "access".to_string()
}

fn handle_stop(session_dir: &Path, input: &HookInput) -> Result<()> {
    if input.stop_hook_active || input.turn_id.trim().is_empty() {
        return write_json(&json!({"continue": true}));
    }
    let turn_hash = short_hash(input.turn_id.as_bytes());
    let prompted = session_dir.join("prompted").join(&turn_hash);
    if prompted.exists() {
        return write_json(&json!({"continue": true}));
    }
    let observations = read_observations(&session_dir.join("observations").join(&turn_hash));
    let read_count = observations
        .values()
        .filter(|observation| observation.kind == "read")
        .count();
    if observations.len() < 2 || read_count == 0 {
        return write_json(&json!({"continue": true}));
    }
    if session_prompt_count(session_dir) >= MAX_SESSION_PROMPTS
        || !contains_session_novel_path(session_dir, &turn_hash, &observations)
    {
        return write_json(&json!({"continue": true}));
    }
    if let Some(parent) = prompted.parent() {
        fs::create_dir_all(parent)?;
    }
    crate::node_agent_atomic_file::write(&prompted, b"prompted")?;
    let mut path_chars = 0usize;
    let paths = observations
        .values()
        .filter_map(|observation| {
            let next = path_chars.saturating_add(observation.path.chars().count());
            if next > MAX_PROMPT_PATH_CHARS {
                return None;
            }
            path_chars = next;
            Some(observation.path.as_str())
        })
        .take(MAX_PROMPT_PATHS)
        .collect::<Vec<_>>();
    let reason = format!(
        "Project-memory receipt gate: this turn inspected {} distinct repository paths ({}). Only if those native reads established a reusable navigation fact that is not already present, stale, task-local, speculative, or conflicting, call project_docs_record_native_context_receipt from yilong_project_receipt with 1-8 concise candidates. Use evidence paths/locators only; never include source bodies, commands, outputs, prompts, chat, or Codex private memories. If nothing is genuinely novel, finish now without calling it.",
        observations.len(),
        paths.join(", ")
    );
    write_json(&json!({"decision": "block", "reason": reason}))
}

fn session_prompt_count(session_dir: &Path) -> usize {
    fs::read_dir(session_dir.join("prompted"))
        .map(|entries| entries.flatten().take(MAX_SESSION_PROMPTS).count())
        .unwrap_or_default()
}

fn contains_session_novel_path(
    session_dir: &Path,
    current_turn_hash: &str,
    current: &BTreeMap<String, PathObservation>,
) -> bool {
    let root = session_dir.join("observations");
    let Ok(turns) = fs::read_dir(root) else {
        return true;
    };
    let mut prior_paths = BTreeMap::new();
    for turn in turns.flatten().take(8) {
        if turn.file_name().to_string_lossy() == current_turn_hash {
            continue;
        }
        let observations = read_observations(&turn.path());
        for path in observations.keys() {
            prior_paths.insert(path.clone(), ());
        }
    }
    current.keys().any(|path| !prior_paths.contains_key(path))
}

fn read_observations(directory: &Path) -> BTreeMap<String, PathObservation> {
    let Ok(entries) = fs::read_dir(directory) else {
        return BTreeMap::new();
    };
    entries
        .flatten()
        .take(MAX_OBSERVATIONS_PER_TURN)
        .filter_map(|entry| fs::read(entry.path()).ok())
        .filter_map(|bytes| serde_json::from_slice::<PathObservation>(&bytes).ok())
        .filter(|observation| observation.schema == "elon.project_memory_path_observation.v1")
        .map(|observation| (observation.path.clone(), observation))
        .collect()
}

fn write_stop_continue_if_needed(input: &HookInput) -> Result<()> {
    if input.hook_event_name == "Stop" {
        write_json(&json!({"continue": true}))?;
    }
    Ok(())
}

fn write_json(value: &Value) -> Result<()> {
    std::io::stdout().write_all(&serde_json::to_vec(value)?)?;
    Ok(())
}

fn cleanup_expired_sessions() {
    let Ok(workspaces) = fs::read_dir(hook_root()) else {
        return;
    };
    for workspace in workspaces.flatten().take(16) {
        let Ok(sessions) = fs::read_dir(workspace.path()) else {
            continue;
        };
        for session in sessions.flatten().take(32) {
            let age = session
                .metadata()
                .and_then(|metadata| metadata.modified())
                .ok()
                .and_then(|modified| SystemTime::now().duration_since(modified).ok())
                .map(|duration| duration.as_secs())
                .unwrap_or_default();
            if age > SESSION_RETENTION_SECS {
                let _ = fs::remove_dir_all(session.path());
            }
        }
    }
}

fn hook_root() -> PathBuf {
    std::env::temp_dir().join("elon-project-memory-hooks")
}

fn short_hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
        .chars()
        .take(24)
        .collect()
}
