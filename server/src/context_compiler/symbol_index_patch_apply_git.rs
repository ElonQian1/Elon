use std::{
    path::{Path, PathBuf},
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use sha2::{Digest, Sha256};

use super::symbol_index_patch_apply_types::{PatchApplyCommandReport, PatchApplyOptions};

const COMMAND_OUTPUT_LIMIT: usize = 4_000;
pub(crate) const DEFAULT_TIMEOUT_SECS: u64 = 60;

pub(crate) fn run_git(
    workdir: &Path,
    args: &[String],
    timeout_seconds: u64,
) -> PatchApplyCommandReport {
    run_process(
        workdir,
        "git",
        args,
        &format!("git -C {} {}", quote_path(workdir), display_args(args)),
        timeout_seconds,
    )
}

pub(crate) fn cleanup_worktree(
    source_git_root: &Path,
    run_workspace: &Path,
) -> PatchApplyCommandReport {
    let temp_dir = std::env::temp_dir();
    if !run_workspace.starts_with(&temp_dir) {
        return PatchApplyCommandReport::not_attempted(
            format!("git worktree remove --force {}", quote_path(run_workspace)),
            "refuse_cleanup_outside_temp_dir",
        );
    }
    run_git(
        source_git_root,
        &[
            "worktree".to_string(),
            "remove".to_string(),
            "--force".to_string(),
            run_workspace.display().to_string(),
        ],
        DEFAULT_TIMEOUT_SECS,
    )
}

pub(crate) fn inspect_git_source(workspace: &Path) -> GitSource {
    let mut source = GitSource {
        git_root: None,
        head: None,
        branch: None,
        clean: false,
        blockers: Vec::new(),
        warnings: Vec::new(),
    };
    if !workspace.exists() {
        source.blockers.push("workspace_not_found".to_string());
        return source;
    }
    let root = run_git(
        workspace,
        &["rev-parse".to_string(), "--show-toplevel".to_string()],
        20,
    );
    if !root.success {
        source
            .blockers
            .push("workspace_git_root_unavailable".to_string());
        source.warnings.push(first_output(&root));
        return source;
    }
    let Some(root_text) = first_non_empty(&root.stdout).map(PathBuf::from) else {
        source.blockers.push("workspace_git_root_empty".to_string());
        return source;
    };
    source.git_root = Some(root_text.clone());
    source.head = git_first_line(&root_text, &["rev-parse", "--short", "HEAD"]);
    source.branch = git_first_line(&root_text, &["rev-parse", "--abbrev-ref", "HEAD"]);
    let status = run_git(
        &root_text,
        &["status".to_string(), "--short".to_string()],
        20,
    );
    if status.success {
        source.clean = status.stdout.trim().is_empty();
    } else {
        source
            .blockers
            .push("workspace_status_unavailable".to_string());
        source.warnings.push(first_output(&status));
    }
    source
}

pub(crate) fn git_first_line(workdir: &Path, args: &[&str]) -> Option<String> {
    let args = args
        .iter()
        .map(|arg| (*arg).to_string())
        .collect::<Vec<_>>();
    let output = run_git(workdir, &args, 20);
    if output.success {
        first_non_empty(&output.stdout).map(ToOwned::to_owned)
    } else {
        None
    }
}

pub(crate) fn commit_message(options: &PatchApplyOptions, task: &str) -> String {
    options
        .commit_message
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| {
            format!(
                "fix(repoctx): apply reviewed patch for {}",
                short_task(task)
            )
        })
}

pub(crate) fn normalize_branch_name(value: Option<&str>) -> Option<String> {
    let mut out = String::new();
    for ch in value?.trim().chars() {
        if ch.is_ascii_alphanumeric() || matches!(ch, '/' | '-' | '_' | '.') {
            out.push(ch.to_ascii_lowercase());
        } else if ch.is_whitespace() {
            out.push('-');
        }
    }
    while out.contains("//") {
        out = out.replace("//", "/");
    }
    let out = out
        .trim_matches('/')
        .trim_matches('.')
        .trim_matches('-')
        .to_string();
    if out.is_empty() || out.contains("..") || out.contains("@{") || out.ends_with(".lock") {
        None
    } else {
        Some(out)
    }
}

pub(crate) fn default_branch_name(task: &str, patch_sha256: &str) -> String {
    let slug = normalize_branch_name(Some(task))
        .unwrap_or_else(|| "patch".to_string())
        .chars()
        .take(32)
        .collect::<String>()
        .trim_matches('-')
        .to_string();
    let short_sha = patch_sha256.chars().take(10).collect::<String>();
    format!("repoctx/{}-{}-{}", slug, short_sha, nonce_tail())
}

pub(crate) fn temp_patch_file(kind: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "elon-symbol-patch-{kind}-{}-{}.patch",
        std::process::id(),
        nonce_tail()
    ))
}

pub(crate) fn temp_run_dir(kind: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "elon-symbol-patch-{kind}-{}-{}",
        std::process::id(),
        nonce_tail()
    ))
}

pub(crate) fn sha256_hex(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    hex::encode(hasher.finalize())
}

pub(crate) fn quote_path(path: &Path) -> String {
    format!("\"{}\"", path.display())
}

pub(crate) fn dedupe(values: Vec<String>) -> Vec<String> {
    let mut out = Vec::new();
    for value in values {
        if !out.contains(&value) {
            out.push(value);
        }
    }
    out
}

fn run_process(
    workdir: &Path,
    program: &str,
    args: &[String],
    display_command: &str,
    timeout_seconds: u64,
) -> PatchApplyCommandReport {
    let started = Instant::now();
    let mut child = match Command::new(program)
        .current_dir(workdir)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(error) => {
            return process_error(display_command, started, false, error.to_string());
        }
    };

    let deadline = Instant::now() + Duration::from_secs(timeout_seconds.max(1));
    loop {
        match child.try_wait() {
            Ok(Some(_status)) => match child.wait_with_output() {
                Ok(output) => {
                    return PatchApplyCommandReport {
                        attempted: true,
                        command: display_command.to_string(),
                        success: output.status.success(),
                        exit_code: output.status.code(),
                        stdout: output_text(&output.stdout),
                        stderr: output_text(&output.stderr),
                        elapsed_ms: started.elapsed().as_millis() as u64,
                        timed_out: false,
                        error: None,
                    };
                }
                Err(error) => {
                    return process_error(display_command, started, false, error.to_string());
                }
            },
            Ok(None) => {
                if Instant::now() >= deadline {
                    let _ = child.kill();
                    let output = child.wait_with_output();
                    let (stdout, stderr, exit_code) = match output {
                        Ok(output) => (
                            output_text(&output.stdout),
                            output_text(&output.stderr),
                            output.status.code(),
                        ),
                        Err(error) => (String::new(), error.to_string(), None),
                    };
                    return PatchApplyCommandReport {
                        attempted: true,
                        command: display_command.to_string(),
                        success: false,
                        exit_code,
                        stdout,
                        stderr,
                        elapsed_ms: started.elapsed().as_millis() as u64,
                        timed_out: true,
                        error: Some("command_timed_out".to_string()),
                    };
                }
                thread::sleep(Duration::from_millis(25));
            }
            Err(error) => return process_error(display_command, started, false, error.to_string()),
        }
    }
}

fn process_error(
    display_command: &str,
    started: Instant,
    timed_out: bool,
    error: String,
) -> PatchApplyCommandReport {
    PatchApplyCommandReport {
        attempted: true,
        command: display_command.to_string(),
        success: false,
        exit_code: None,
        stdout: String::new(),
        stderr: String::new(),
        elapsed_ms: started.elapsed().as_millis() as u64,
        timed_out,
        error: Some(error),
    }
}

fn short_task(task: &str) -> String {
    let mut out = String::new();
    for ch in task.chars().take(40) {
        out.push(ch);
    }
    if out.is_empty() {
        "symbol task".to_string()
    } else {
        out
    }
}

fn nonce_tail() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("{:x}", nanos & 0xffff_ffff)
}

fn output_text(bytes: &[u8]) -> String {
    truncate_text(String::from_utf8_lossy(bytes).to_string())
}

fn truncate_text(text: String) -> String {
    let mut out = String::new();
    for (index, ch) in text.chars().enumerate() {
        if index >= COMMAND_OUTPUT_LIMIT {
            out.push_str("...<truncated>");
            return out;
        }
        out.push(ch);
    }
    out
}

fn first_non_empty(text: &str) -> Option<&str> {
    text.lines().map(str::trim).find(|line| !line.is_empty())
}

fn first_output(output: &PatchApplyCommandReport) -> String {
    first_non_empty(&output.stderr)
        .or_else(|| first_non_empty(&output.stdout))
        .unwrap_or("git command failed")
        .to_string()
}

fn display_args(args: &[String]) -> String {
    args.iter()
        .map(|arg| {
            if arg.contains(char::is_whitespace) {
                quote_path(Path::new(arg))
            } else {
                arg.clone()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub(crate) struct GitSource {
    pub(crate) git_root: Option<PathBuf>,
    pub(crate) head: Option<String>,
    pub(crate) branch: Option<String>,
    pub(crate) clean: bool,
    pub(crate) blockers: Vec<String>,
    pub(crate) warnings: Vec<String>,
}
