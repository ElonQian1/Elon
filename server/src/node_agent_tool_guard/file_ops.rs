use anyhow::{anyhow, bail, Context, Result};
use serde_json::Value;
use std::{
    collections::VecDeque,
    path::{Path, PathBuf},
    time::Duration,
};
use tokio::process::Command;
use super::{
    MAX_FILE_CHARS, MAX_SEARCH_FILE_BYTES, MAX_SEARCH_FILES_SCANNED,
    MAX_SEARCH_QUERY_CHARS, SEARCH_SKIP_DIRS,
};

// ── Search ────────────────────────────────────────────────────────────────────

pub(crate) fn search_files_blocking(
    workspace: &Path,
    root: &Path,
    query: &str,
    max_results: usize,
) -> Result<String> {
    let metadata = std::fs::metadata(root)
        .with_context(|| format!("search_files failed: {}", root.display()))?;
    if !metadata.is_dir() {
        bail!("search_files path must be a directory");
    }

    let query_lower = query.to_ascii_lowercase();
    let mut queue = VecDeque::from([root.to_path_buf()]);
    let mut results = Vec::new();
    let mut files_scanned = 0usize;
    let mut truncated = false;

    while let Some(dir) = queue.pop_front() {
        let entries = match std::fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            if file_type.is_symlink() {
                continue;
            }
            if path.is_dir() {
                if should_skip_search_dir(&name) {
                    continue;
                }
                queue.push_back(path);
                continue;
            }
            if !path.is_file() {
                continue;
            }
            files_scanned += 1;
            if files_scanned > MAX_SEARCH_FILES_SCANNED {
                truncated = true;
                break;
            }

            let relative = display_relative_path(workspace, &path);
            if relative.to_ascii_lowercase().contains(&query_lower) {
                results.push(format!("{relative}: path match"));
            }
            if results.len() >= max_results {
                truncated = true;
                break;
            }

            let Ok(metadata) = std::fs::metadata(&path) else {
                continue;
            };
            if metadata.len() > MAX_SEARCH_FILE_BYTES {
                continue;
            }
            let Ok(text) = std::fs::read_to_string(&path) else {
                continue;
            };
            for (index, line) in text.lines().enumerate() {
                if !line.to_ascii_lowercase().contains(&query_lower) {
                    continue;
                }
                let snippet = truncate_chars(line.trim(), 240).replace('\t', " ");
                results.push(format!("{}:{}: {}", relative, index + 1, snippet));
                if results.len() >= max_results {
                    truncated = true;
                    break;
                }
            }
            if results.len() >= max_results {
                break;
            }
        }
        if truncated || results.len() >= max_results {
            break;
        }
    }

    if results.is_empty() {
        return Ok(format!("no matches for query: {query}"));
    }
    if truncated {
        results.push("[truncated]".to_string());
    }
    Ok(results.join("\n"))
}

pub(crate) fn should_skip_search_dir(name: &str) -> bool {
    SEARCH_SKIP_DIRS
        .iter()
        .any(|skip| name.eq_ignore_ascii_case(skip))
}

pub(crate) fn display_relative_path(workspace: &Path, path: &Path) -> String {
    path.strip_prefix(workspace)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
        .trim_start_matches('/')
        .to_string()
}

pub(crate) fn truncate_chars(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.to_string();
    }
    let mut truncated: String = value.chars().take(max_chars).collect();
    truncated.push_str("\n[truncated]");
    truncated
}

// ── Shell execution ───────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub(crate) struct RuntimeCommandRequest {
    pub(crate) program: String,
    pub(crate) args: Vec<String>,
    pub(crate) legacy_command: Option<String>,
    pub(crate) shell: Option<String>,
    pub(crate) cwd: Option<String>,
}

impl RuntimeCommandRequest {
    pub(crate) fn from_action(action: &Value) -> Result<Self> {
        let shell = optional_trimmed_string(action, "shell")?;
        let cwd = optional_trimmed_string(action, "cwd")?;
        if let Some(program) = action
            .get("program")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            let args = action
                .get("args")
                .and_then(Value::as_array)
                .map(|values| {
                    values
                        .iter()
                        .map(|value| {
                            value
                                .as_str()
                                .map(|item| item.to_string())
                                .ok_or_else(|| anyhow!("args must be strings"))
                        })
                        .collect::<Result<Vec<_>>>()
                })
                .transpose()?
                .unwrap_or_default();
            return Ok(Self {
                program: program.to_string(),
                args,
                legacy_command: None,
                shell,
                cwd,
            });
        }

        let command = required_str(action, "command")?.trim().to_string();
        Ok(Self {
            program: String::new(),
            args: Vec::new(),
            legacy_command: Some(command),
            shell,
            cwd,
        })
    }

    pub(crate) fn display_command(&self) -> String {
        if let Some(command) = self.legacy_command.as_deref() {
            return command.to_string();
        }
        std::iter::once(self.program.as_str())
            .chain(self.args.iter().map(String::as_str))
            .collect::<Vec<_>>()
            .join(" ")
    }
}

pub(crate) fn shell_command_runner(shell: Option<&str>, command: &str) -> Result<Command> {
    let shell = shell
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(default_shell());
    let normalized = shell.to_ascii_lowercase();
    let mut command_runner = match normalized.as_str() {
        "cmd" | "cmd.exe" => {
            let mut runner = Command::new("cmd");
            runner.args(["/C", command]);
            runner
        }
        "powershell" | "powershell.exe" => {
            let mut runner = Command::new("powershell");
            runner.args([
                "-NoProfile",
                "-ExecutionPolicy",
                "Bypass",
                "-Command",
                command,
            ]);
            runner
        }
        "pwsh" | "pwsh.exe" => {
            let mut runner = Command::new("pwsh");
            runner.args(["-NoProfile", "-Command", command]);
            runner
        }
        "sh" => {
            let mut runner = Command::new("sh");
            runner.args(["-lc", command]);
            runner
        }
        "bash" => {
            let mut runner = Command::new("bash");
            runner.args(["-lc", command]);
            runner
        }
        _ => bail!("unsupported shell for run_command: {shell}"),
    };
    hide_command_window(&mut command_runner);
    Ok(command_runner)
}

#[cfg(windows)]
pub(crate) fn default_shell() -> &'static str {
    "powershell"
}

#[cfg(not(windows))]
pub(crate) fn default_shell() -> &'static str {
    "sh"
}

pub(crate) fn required_str<'a>(action: &'a Value, key: &str) -> Result<&'a str> {
    action
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| anyhow!("{key} is required"))
}

pub(crate) fn optional_trimmed_string(action: &Value, key: &str) -> Result<Option<String>> {
    let Some(value) = action.get(key) else {
        return Ok(None);
    };
    if value.is_null() {
        return Ok(None);
    }
    let Some(text) = value.as_str() else {
        bail!("{key} must be a string");
    };
    let clean = text.trim();
    if clean.is_empty() {
        Ok(None)
    } else {
        Ok(Some(clean.to_string()))
    }
}

pub(crate) fn optional_positive_usize(action: &Value, key: &str) -> Result<Option<usize>> {
    let Some(value) = action.get(key) else {
        return Ok(None);
    };
    let Some(value) = value.as_u64() else {
        bail!("{key} must be a positive integer");
    };
    if value == 0 {
        bail!("{key} must be >= 1");
    }
    usize::try_from(value)
        .map(Some)
        .map_err(|_| anyhow!("{key} is too large"))
}

pub(crate) fn safe_git_revision(revision: &str) -> Result<String> {
    let revision = revision.trim();
    if revision.is_empty() {
        bail!("revision cannot be empty");
    }
    if revision.len() > 120 {
        bail!("revision is too long");
    }
    if revision.starts_with('-') {
        bail!("revision cannot start with '-': {revision}");
    }
    if revision.contains("..") || revision.contains('\\') {
        bail!("revision contains unsupported syntax: {revision}");
    }
    if !revision.chars().all(|ch| {
        ch.is_ascii_alphanumeric()
            || matches!(
                ch,
                '_' | '-' | '.' | '/' | ':' | '@' | '{' | '}' | '~' | '^'
            )
    }) {
        bail!("revision contains unsupported characters: {revision}");
    }
    Ok(revision.to_string())
}

pub(crate) fn normalize_path(path: PathBuf) -> Result<PathBuf> {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::Prefix(prefix) => normalized.push(prefix.as_os_str()),
            std::path::Component::RootDir => normalized.push(component.as_os_str()),
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                normalized.pop();
            }
            std::path::Component::Normal(part) => normalized.push(part),
        }
    }
    Ok(normalized)
}

pub(crate) fn reject_unsafe_path_components(path: &Path, raw: &str) -> Result<()> {
    for component in path.components() {
        match component {
            std::path::Component::ParentDir => {
                bail!("parent path segments are not allowed: {raw}");
            }
            std::path::Component::Normal(part) => {
                if part.to_string_lossy().eq_ignore_ascii_case(".git") {
                    bail!("path cannot target .git: {raw}");
                }
            }
            _ => {}
        }
    }
    Ok(())
}

pub(crate) fn hide_command_window(_command: &mut Command) {
    #[cfg(windows)]
    {
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
        _command.creation_flags(CREATE_NO_WINDOW | CREATE_NEW_PROCESS_GROUP);
    }
}
