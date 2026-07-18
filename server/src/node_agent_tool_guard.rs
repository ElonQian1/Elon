use crate::node_agent_program_resolver::resolve_structured_program;
use anyhow::{anyhow, bail, Context, Result};
use serde_json::Value;
use std::{
    path::{Path, PathBuf},
    time::Duration,
};
use tokio::process::Command;

const MAX_FILE_CHARS: usize = 40_000;
const MAX_TOOL_RESULT_CHARS: usize = 24_000;
const MAX_SEARCH_FILE_BYTES: u64 = 1024 * 1024;
const MAX_SEARCH_FILES_SCANNED: usize = 2_000;
const MAX_SEARCH_QUERY_CHARS: usize = 200;
const SEARCH_SKIP_DIRS: &[&str] = &[
    ".git",
    ".hg",
    ".svn",
    "node_modules",
    "target",
    "dist",
    "build",
    ".gradle",
    ".idea",
    ".vscode",
    ".elon",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RuntimeToolMode {
    ReadOnly,
    ProjectWrite,
    FullAccess,
    DangerFullAccess,
}

impl RuntimeToolMode {
    pub(crate) fn from_runtime_permission(runtime_permission: Option<&str>) -> Self {
        match runtime_permission.map(str::trim) {
            Some("project_write") => Self::ProjectWrite,
            Some("full_access") => Self::FullAccess,
            Some("danger_full_access") => Self::DangerFullAccess,
            _ => Self::ReadOnly,
        }
    }

    pub(crate) fn read_only(self) -> bool {
        self == Self::ReadOnly
    }

    pub(crate) fn danger_full_access(self) -> bool {
        self == Self::DangerFullAccess
    }
}

pub(crate) struct ToolGuard {
    workspace: PathBuf,
    workspace_prefix: PathBuf,
    mode: RuntimeToolMode,
}

impl ToolGuard {
    pub(crate) fn new(workspace: PathBuf, runtime_permission: Option<&str>) -> Self {
        let mut workspace_prefix = workspace.clone();
        workspace_prefix.push("");
        Self {
            workspace,
            workspace_prefix,
            mode: RuntimeToolMode::from_runtime_permission(runtime_permission),
        }
    }

    pub(crate) fn read_only(&self) -> bool {
        self.mode.read_only()
    }

    pub(crate) fn danger_full_access(&self) -> bool {
        self.mode.danger_full_access()
    }

    pub(crate) async fn invoke_action(&mut self, action: &Value) -> String {
        match self.invoke_action_result(action).await {
            Ok(value) => value,
            Err(error) => format!("error: {error}"),
        }
    }

    async fn invoke_action_result(&mut self, action: &Value) -> Result<String> {
        let tool = action
            .get("tool")
            .and_then(Value::as_str)
            .unwrap_or_default();
        match tool {
            "list_dir" => {
                let path = action.get("path").and_then(Value::as_str).unwrap_or(".");
                self.list_dir(path).await
            }
            "search_files" => {
                let query = action
                    .get("query")
                    .and_then(Value::as_str)
                    .ok_or_else(|| anyhow!("query is required"))?;
                let path = action.get("path").and_then(Value::as_str).unwrap_or(".");
                let max_results = optional_positive_usize(action, "max_results")?.unwrap_or(50);
                self.search_files(path, query, max_results).await
            }
            "file_info" => {
                let path = required_str(action, "path")?;
                self.file_info(path).await
            }
            "read_file" => {
                let path = required_str(action, "path")?;
                self.read_file(path).await
            }
            "read_file_range" => {
                let path = required_str(action, "path")?;
                let start_line = optional_positive_usize(action, "start_line")?.unwrap_or(1);
                let line_count = optional_positive_usize(action, "line_count")?.unwrap_or(120);
                self.read_file_range(path, start_line, line_count).await
            }
            "git_status" => self.git_status().await,
            "git_diff" => {
                let path = action.get("path").and_then(Value::as_str);
                let cached = action
                    .get("cached")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                let stat = action.get("stat").and_then(Value::as_bool).unwrap_or(false);
                self.git_diff(path, cached, stat).await
            }
            "git_log" => {
                let path = action.get("path").and_then(Value::as_str);
                let limit = optional_positive_usize(action, "limit")?.unwrap_or(20);
                self.git_log(path, limit).await
            }
            "git_show" => {
                let revision = action
                    .get("revision")
                    .and_then(Value::as_str)
                    .unwrap_or("HEAD");
                let path = action.get("path").and_then(Value::as_str);
                let stat = action.get("stat").and_then(Value::as_bool).unwrap_or(false);
                self.git_show(revision, path, stat).await
            }
            "write_file" => {
                if self.read_only() {
                    bail!("write_file denied: read-only planning mode");
                }
                let path = required_str(action, "path")?;
                let content = required_str(action, "content")?;
                self.write_file(path, content).await
            }
            "apply_patch" => {
                if self.read_only() {
                    bail!("apply_patch denied: read-only planning mode");
                }
                let patch = required_str(action, "patch")?;
                let check_only = action
                    .get("check_only")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                self.apply_patch(patch, check_only).await
            }
            "run_command" => {
                if self.read_only() {
                    bail!("run_command denied: read-only planning mode");
                }
                let request = RuntimeCommandRequest::from_action(action)?;
                self.run_command(&request).await
            }
            _ => bail!("unknown tool: {tool}"),
        }
    }

    async fn list_dir(&self, path: &str) -> Result<String> {
        let full = self.resolve_safe_path(path)?;
        let mut entries = tokio::fs::read_dir(&full).await?;
        let mut rows = Vec::new();
        while let Some(entry) = entries.next_entry().await? {
            let metadata = entry.metadata().await.ok();
            let kind = if metadata.as_ref().is_some_and(|item| item.is_dir()) {
                "dir"
            } else {
                "file"
            };
            let len = metadata
                .as_ref()
                .filter(|item| item.is_file())
                .map(|item| item.len())
                .unwrap_or(0);
            rows.push(format!(
                "{}\t{}\t{}",
                kind,
                len,
                entry.file_name().to_string_lossy()
            ));
            if rows.len() >= 200 {
                rows.push("[truncated]".to_string());
                break;
            }
        }
        rows.sort();
        Ok(rows.join("\n"))
    }

    async fn search_files(&self, path: &str, query: &str, max_results: usize) -> Result<String> {
        let query = query.trim();
        if query.is_empty() {
            bail!("query cannot be empty");
        }
        if query.chars().count() > MAX_SEARCH_QUERY_CHARS {
            bail!("query is too long");
        }
        let root = self.resolve_safe_path(path)?;
        let workspace = self.workspace.clone();
        let max_results = max_results.clamp(1, 200);
        let query = query.to_string();
        tokio::task::spawn_blocking(move || {
            search_files_blocking(&workspace, &root, &query, max_results)
        })
        .await
        .map_err(|error| anyhow!("search_files task failed: {error}"))?
    }

    async fn file_info(&self, path: &str) -> Result<String> {
        let full = self.resolve_safe_path(path)?;
        crate::node_agent_file_info::file_info(&full, path).await
    }

    async fn read_file(&self, path: &str) -> Result<String> {
        let full = self.resolve_safe_path(path)?;
        let text = tokio::fs::read_to_string(&full)
            .await
            .with_context(|| format!("read_file failed: {path}"))?;
        Ok(truncate_chars(&text, MAX_FILE_CHARS))
    }

    async fn read_file_range(
        &self,
        path: &str,
        start_line: usize,
        line_count: usize,
    ) -> Result<String> {
        let full = self.resolve_safe_path(path)?;
        crate::node_agent_file_range::read_file_range(&full, path, start_line, line_count).await
    }

    async fn write_file(&self, path: &str, content: &str) -> Result<String> {
        let full = self.resolve_safe_path(path)?;
        if let Some(parent) = full.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        tokio::fs::write(&full, content).await?;
        Ok(format!("write_file ok: {path} ({} chars)", content.len()))
    }

    async fn git_status(&self) -> Result<String> {
        self.run_git(vec![
            "-c".to_string(),
            "core.quotepath=false".to_string(),
            "status".to_string(),
            "--short".to_string(),
            "--branch".to_string(),
        ])
        .await
    }

    async fn git_diff(&self, path: Option<&str>, cached: bool, stat: bool) -> Result<String> {
        let mut args = vec![
            "-c".to_string(),
            "core.quotepath=false".to_string(),
            "diff".to_string(),
            "--no-ext-diff".to_string(),
        ];
        if cached {
            args.push("--cached".to_string());
        }
        if stat {
            args.push("--stat".to_string());
        }
        if let Some(path) = path.map(str::trim).filter(|value| !value.is_empty()) {
            if path != "." {
                let full = self.resolve_safe_path(path)?;
                let relative = display_relative_path(&self.workspace, &full);
                args.push("--".to_string());
                args.push(relative);
            }
        }
        self.run_git(args).await
    }

    async fn git_log(&self, path: Option<&str>, limit: usize) -> Result<String> {
        let count = limit.clamp(1, 100);
        let mut args = vec![
            "-c".to_string(),
            "core.quotepath=false".to_string(),
            "log".to_string(),
            "--oneline".to_string(),
            "--decorate".to_string(),
            format!("--max-count={count}"),
        ];
        if let Some(path) = path.map(str::trim).filter(|value| !value.is_empty()) {
            if path != "." {
                let full = self.resolve_safe_path(path)?;
                let relative = display_relative_path(&self.workspace, &full);
                args.push("--".to_string());
                args.push(relative);
            }
        }
        self.run_git(args).await
    }

    async fn git_show(&self, revision: &str, path: Option<&str>, stat: bool) -> Result<String> {
        let revision = safe_git_revision(revision)?;
        let mut args = vec![
            "-c".to_string(),
            "core.quotepath=false".to_string(),
            "show".to_string(),
            "--no-ext-diff".to_string(),
            "--decorate".to_string(),
        ];
        if stat {
            args.push("--stat".to_string());
        }
        args.push(revision);
        if let Some(path) = path.map(str::trim).filter(|value| !value.is_empty()) {
            if path != "." {
                let full = self.resolve_safe_path(path)?;
                let relative = display_relative_path(&self.workspace, &full);
                args.push("--".to_string());
                args.push(relative);
            }
        }
        self.run_git(args).await
    }

    async fn run_git(&self, args: Vec<String>) -> Result<String> {
        let mut child_command = crate::git_command_error::tokio_git_command();
        child_command.args(&args);
        child_command.current_dir(&self.workspace);
        hide_command_window(&mut child_command);
        let output = tokio::time::timeout(Duration::from_secs(30), child_command.output())
            .await
            .map_err(|_| anyhow!("git tool timed out after 30s"))??;
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = if stdout.trim().is_empty() {
            "[no output]".to_string()
        } else {
            stdout.to_string()
        };
        let combined = format!(
            "git {} exit={}\nstdout:\n{}\nstderr:\n{}",
            args.join(" "),
            output.status.code().unwrap_or(-1),
            stdout,
            stderr
        );
        Ok(truncate_chars(&combined, MAX_TOOL_RESULT_CHARS))
    }

    pub(crate) async fn write_file_diff_preview(&self, action: &Value) -> Result<Option<Value>> {
        if action.get("tool").and_then(Value::as_str) != Some("write_file") {
            return Ok(None);
        }
        let path = required_str(action, "path")?;
        let content = required_str(action, "content")?;
        let full = self.resolve_safe_path(path)?;
        // 预览必须走同一套路径守卫，避免审批卡展示的是安全路径、
        // 实际执行却写到另一个位置。
        crate::node_agent_write_preview::write_file_diff_preview(&full, path, content)
            .await
            .map(Some)
    }

    pub(crate) async fn verify_write_file_preview_unchanged(
        &self,
        action: &Value,
        diff: &Value,
    ) -> Result<()> {
        if action.get("tool").and_then(Value::as_str) != Some("write_file") {
            return Ok(());
        }
        let path = required_str(action, "path")?;
        let content = required_str(action, "content")?;
        let full = self.resolve_safe_path(path)?;
        let current_content = match tokio::fs::read_to_string(&full).await {
            Ok(content) => Some(content),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => {
                return Err(error).with_context(|| format!("write_file base check failed: {path}"))
            }
        };

        let expected_old = diff.get("old_sha256").and_then(Value::as_str);
        let actual_old = current_content
            .as_deref()
            .map(crate::node_agent_write_preview::sha256_hex);
        if expected_old != actual_old.as_deref() {
            bail!("write_file base changed since approval preview: {path}");
        }

        let expected_new = diff
            .get("new_sha256")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow!("write_file approval preview missing new_sha256"))?;
        let actual_new = crate::node_agent_write_preview::sha256_hex(content);
        if expected_new != actual_new {
            bail!("write_file content changed since approval preview: {path}");
        }

        Ok(())
    }

    pub(crate) async fn apply_patch_diff_preview(&self, action: &Value) -> Result<Option<Value>> {
        if action.get("tool").and_then(Value::as_str) != Some("apply_patch") {
            return Ok(None);
        }
        let patch = required_str(action, "patch")?.to_string();
        let workspace = self.workspace.clone();
        tokio::task::spawn_blocking(move || {
            crate::tools_patch::apply_patch_diff_preview(&workspace, &patch)
        })
        .await
        .map_err(|error| anyhow!("apply_patch diff preview task failed: {error}"))?
        .map(Some)
    }

    pub(crate) async fn verify_apply_patch_preview_unchanged(
        &self,
        action: &Value,
        diff: &Value,
    ) -> Result<()> {
        if action.get("tool").and_then(Value::as_str) != Some("apply_patch") {
            return Ok(());
        }
        let patch = required_str(action, "patch")?.to_string();
        let diff = diff.clone();
        let workspace = self.workspace.clone();
        tokio::task::spawn_blocking(move || {
            crate::tools_patch::verify_apply_patch_preview_unchanged(&workspace, &patch, &diff)
        })
        .await
        .map_err(|error| anyhow!("apply_patch preview verification task failed: {error}"))?
    }

    async fn apply_patch(&self, patch: &str, check_only: bool) -> Result<String> {
        let workspace = self.workspace.clone();
        let patch = patch.to_string();
        tokio::task::spawn_blocking(move || {
            crate::tools_patch::apply_patch(&workspace, &patch, check_only)
        })
        .await
        .map_err(|error| anyhow!("apply_patch task failed: {error}"))?
    }

    async fn run_command(&self, request: &RuntimeCommandRequest) -> Result<String> {
        let danger_full_access = self.danger_full_access();
        let mut child_command = if let Some(command) = request.legacy_command.as_deref() {
            if !danger_full_access && !command_allowed(command) {
                bail!("run_command denied by policy: {command}");
            }
            shell_command_runner(request.shell.as_deref(), command)?
        } else {
            if !danger_full_access && !structured_command_allowed(&request.program, &request.args) {
                bail!(
                    "run_command denied by policy: {}",
                    request.display_command()
                );
            }
            let mut command_runner = Command::new(resolve_structured_program(&request.program));
            command_runner.args(&request.args);
            command_runner
        };
        let cwd = self.resolve_command_cwd(request.cwd.as_deref())?;
        child_command.current_dir(cwd);
        hide_command_window(&mut child_command);
        let output = tokio::time::timeout(Duration::from_secs(300), child_command.output())
            .await
            .map_err(|_| anyhow!("run_command timed out after 300s"))??;
        let combined = format!(
            "exit={}\nstdout:\n{}\nstderr:\n{}",
            output.status.code().unwrap_or(-1),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        Ok(truncate_chars(&combined, MAX_TOOL_RESULT_CHARS))
    }

    fn resolve_safe_path(&self, path: &str) -> Result<PathBuf> {
        let raw = path.trim();
        if raw.is_empty() {
            bail!("path cannot be empty");
        }
        if self.danger_full_access() {
            return self.resolve_unrestricted_path(raw);
        }
        let candidate = Path::new(raw);
        if candidate.is_absolute() {
            bail!("absolute paths are not allowed: {raw}");
        }
        reject_unsafe_path_components(candidate, raw)?;
        let full = normalize_path(self.workspace.join(candidate))?;
        if full != self.workspace && !full.starts_with(&self.workspace_prefix) {
            bail!("path escapes project workspace: {raw}");
        }
        self.reject_unsafe_existing_ancestors(&full, raw)?;
        Ok(full)
    }

    fn resolve_command_cwd(&self, cwd: Option<&str>) -> Result<PathBuf> {
        let Some(raw) = cwd.map(str::trim).filter(|value| !value.is_empty()) else {
            return Ok(self.workspace.clone());
        };
        if self.danger_full_access() {
            return self.resolve_unrestricted_path(raw);
        }
        self.resolve_safe_path(raw)
    }

    fn resolve_unrestricted_path(&self, raw: &str) -> Result<PathBuf> {
        let candidate = Path::new(raw);
        let full = if candidate.is_absolute() {
            candidate.to_path_buf()
        } else {
            self.workspace.join(candidate)
        };
        normalize_path(full)
    }

    fn reject_unsafe_existing_ancestors(&self, full: &Path, raw: &str) -> Result<()> {
        let relative = full
            .strip_prefix(&self.workspace)
            .map_err(|_| anyhow!("path escapes project workspace: {raw}"))?;
        let mut current = self.workspace.clone();
        for component in relative.components() {
            let std::path::Component::Normal(part) = component else {
                continue;
            };
            current.push(part);
            let Ok(metadata) = std::fs::symlink_metadata(&current) else {
                continue;
            };
            if metadata.file_type().is_symlink() {
                bail!("path crosses a symlink or junction: {raw}");
            }
        }
        Ok(())
    }
}

pub(crate) mod command_policy;
pub(crate) mod file_ops;
#[cfg(test)]
mod tool_guard_tests;

use self::command_policy::command_allowed;
use self::file_ops::{
    display_relative_path, hide_command_window, normalize_path, optional_positive_usize,
    reject_unsafe_path_components, required_str, safe_git_revision, search_files_blocking,
    shell_command_runner, RuntimeCommandRequest,
};
pub(crate) use command_policy::structured_command_allowed;
pub(crate) use file_ops::truncate_chars;
