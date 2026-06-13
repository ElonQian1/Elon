use std::{
    collections::HashSet,
    io::Read,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use super::model::{
    RustAnalyzerCommandProbe, RustAnalyzerFinding, RustAnalyzerProbeReport,
    RustAnalyzerProbeStatus, RustIndex,
};

const MAX_FINDINGS: usize = 24;
const MAX_EXCERPT_LINES: usize = 10;

pub(crate) fn collect_rust_analyzer_probes(
    workspace: &Path,
    rust: &RustIndex,
    enabled: bool,
    timeout_ms: usize,
) -> RustAnalyzerProbeReport {
    if !enabled {
        return RustAnalyzerProbeReport::default();
    }

    let Some(probe_root) = find_probe_root(workspace, rust) else {
        return RustAnalyzerProbeReport {
            enabled: true,
            warnings: vec![
                "rust-analyzer probes skipped: no Cargo.toml found near scanned Rust files"
                    .to_string(),
            ],
            ..RustAnalyzerProbeReport::default()
        };
    };

    let timeout = Duration::from_millis(timeout_ms as u64);
    let commands = vec![
        run_probe_command(
            &probe_root,
            "diagnostics",
            &[
                "diagnostics",
                ".",
                "--disable-build-scripts",
                "--disable-proc-macros",
            ],
            timeout,
        ),
        run_probe_command(
            &probe_root,
            "unresolved_references",
            &[
                "unresolved-references",
                ".",
                "--disable-build-scripts",
                "--disable-proc-macros",
            ],
            timeout,
        ),
        run_probe_command(
            &probe_root,
            "analysis_stats",
            &[
                "analysis-stats",
                ".",
                "--disable-build-scripts",
                "--disable-proc-macros",
                "--skip-inference",
                "--skip-mir-stats",
                "--skip-data-layout",
                "--skip-const-eval",
            ],
            timeout,
        ),
    ];

    RustAnalyzerProbeReport {
        enabled: true,
        workspace_path: Some(display_path(workspace, &probe_root)),
        commands,
        warnings: Vec::new(),
    }
}

fn find_probe_root(workspace: &Path, rust: &RustIndex) -> Option<PathBuf> {
    let root_manifest = workspace.join("Cargo.toml");
    if root_manifest.is_file() {
        return Some(workspace.to_path_buf());
    }

    let mut candidates = rust
        .symbols
        .iter()
        .map(|symbol| symbol.path.as_str())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    candidates.sort_unstable();

    for path in candidates {
        let full_path = workspace.join(path);
        if let Some(root) = find_manifest_parent(workspace, &full_path) {
            return Some(root);
        }
    }
    None
}

fn find_manifest_parent(workspace: &Path, file: &Path) -> Option<PathBuf> {
    let mut current = file.parent()?;
    loop {
        if current.join("Cargo.toml").is_file() {
            return Some(current.to_path_buf());
        }
        if current == workspace {
            break;
        }
        current = current.parent()?;
    }
    None
}

fn run_probe_command(
    root: &Path,
    name: &str,
    args: &[&str],
    timeout: Duration,
) -> RustAnalyzerCommandProbe {
    let command_text = format!("rust-analyzer {}", args.join(" "));
    let started = Instant::now();
    let mut child = match Command::new("rust-analyzer")
        .args(args)
        .current_dir(root)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(error) => {
            return skipped_probe(name, command_text, started.elapsed(), error.to_string());
        }
    };

    let stdout_handle = child
        .stdout
        .take()
        .map(|stdout| thread::spawn(move || read_pipe(stdout)));
    let stderr_handle = child
        .stderr
        .take()
        .map(|stderr| thread::spawn(move || read_pipe(stderr)));

    let mut timed_out = false;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) if started.elapsed() >= timeout => {
                timed_out = true;
                let _ = child.kill();
                break child.wait().ok();
            }
            Ok(None) => thread::sleep(Duration::from_millis(40)),
            Err(_) => break None,
        }
    };

    let stdout = join_output(stdout_handle);
    let stderr = join_output(stderr_handle);
    let duration_ms = started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64;
    let findings = parse_cli_findings(&stdout, &stderr);
    let stdout_excerpt = excerpt_lines(&stdout);
    let stderr_excerpt = excerpt_lines(&stderr);
    let exit_code = status.and_then(|status| status.code());
    let status = if timed_out {
        RustAnalyzerProbeStatus::TimedOut
    } else if matches!(status, Some(status) if status.success()) {
        RustAnalyzerProbeStatus::Succeeded
    } else {
        RustAnalyzerProbeStatus::Failed
    };
    let warning = match status {
        RustAnalyzerProbeStatus::TimedOut => {
            Some(format!("timed out after {}ms", timeout.as_millis()))
        }
        RustAnalyzerProbeStatus::Failed => {
            Some("rust-analyzer command exited with failure".to_string())
        }
        _ => None,
    };

    RustAnalyzerCommandProbe {
        name: name.to_string(),
        command: command_text,
        status,
        duration_ms,
        exit_code,
        findings,
        stdout_excerpt,
        stderr_excerpt,
        warning,
    }
}

fn skipped_probe(
    name: &str,
    command: String,
    elapsed: Duration,
    warning: String,
) -> RustAnalyzerCommandProbe {
    RustAnalyzerCommandProbe {
        name: name.to_string(),
        command,
        status: RustAnalyzerProbeStatus::Skipped,
        duration_ms: elapsed.as_millis().min(u128::from(u64::MAX)) as u64,
        exit_code: None,
        findings: Vec::new(),
        stdout_excerpt: Vec::new(),
        stderr_excerpt: Vec::new(),
        warning: Some(warning),
    }
}

fn read_pipe<R: Read>(mut pipe: R) -> String {
    let mut output = String::new();
    let _ = pipe.read_to_string(&mut output);
    output
}

fn join_output(handle: Option<thread::JoinHandle<String>>) -> String {
    handle
        .and_then(|handle| handle.join().ok())
        .unwrap_or_default()
}

pub(super) fn parse_cli_findings(stdout: &str, stderr: &str) -> Vec<RustAnalyzerFinding> {
    let combined = format!("{stdout}\n{stderr}");
    let mut findings = Vec::new();
    let mut pending_index: Option<usize> = None;

    for raw_line in combined.lines() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        if let Some((severity, message)) = parse_severity_message(line) {
            findings.push(RustAnalyzerFinding {
                path: None,
                line: None,
                severity: Some(severity),
                message: compact(message, 180),
                evidence: compact(line, 220),
            });
            pending_index = Some(findings.len() - 1);
            if findings.len() >= MAX_FINDINGS {
                break;
            }
            continue;
        }
        if let Some((path, line_number)) = parse_arrow_location(line) {
            if let Some(index) = pending_index.and_then(|idx| findings.get_mut(idx).map(|_| idx)) {
                findings[index].path = Some(path);
                findings[index].line = Some(line_number);
                findings[index].evidence = compact(line, 220);
            }
            continue;
        }
        if let Some((path, line_number, message)) = parse_inline_location(line) {
            findings.push(RustAnalyzerFinding {
                path: Some(path),
                line: Some(line_number),
                severity: infer_severity(line),
                message: compact(message, 180),
                evidence: compact(line, 220),
            });
            pending_index = Some(findings.len() - 1);
            if findings.len() >= MAX_FINDINGS {
                break;
            }
        }
    }
    findings
}

fn parse_severity_message(line: &str) -> Option<(String, &str)> {
    for severity in ["error", "warning", "info", "hint"] {
        let prefix = format!("{severity}:");
        if let Some(message) = line.strip_prefix(&prefix) {
            return Some((severity.to_string(), message.trim()));
        }
    }
    None
}

fn parse_arrow_location(line: &str) -> Option<(String, usize)> {
    if !line.contains("-->") {
        return None;
    }
    let location = line.trim_start_matches('-').trim_start_matches('>').trim();
    parse_path_line(location).map(|(path, line_number, _)| (path, line_number))
}

fn parse_inline_location(line: &str) -> Option<(String, usize, &str)> {
    let (path, line_number, rest) = parse_path_line(line)?;
    let message = rest.trim_start_matches(':').trim();
    if message.is_empty() {
        return None;
    }
    Some((path, line_number, message))
}

fn parse_path_line(line: &str) -> Option<(String, usize, &str)> {
    let marker = ".rs:";
    let marker_index = line.find(marker)?;
    let path_end = marker_index + ".rs".len();
    let path = line[..path_end].trim().trim_start_matches("-->").trim();
    let after_path = line[path_end + 1..].trim();
    let (line_text, rest) = after_path.split_once(':').unwrap_or((after_path, ""));
    let line_number = line_text.trim().parse::<usize>().ok()?;
    Some((normalize_path(path), line_number, rest))
}

fn infer_severity(line: &str) -> Option<String> {
    let lower = line.to_ascii_lowercase();
    for severity in ["error", "warning", "info", "hint"] {
        if lower.contains(severity) {
            return Some(severity.to_string());
        }
    }
    None
}

fn excerpt_lines(output: &str) -> Vec<String> {
    output
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .take(MAX_EXCERPT_LINES)
        .map(|line| compact(line, 220))
        .collect()
}

fn display_path(workspace: &Path, path: &Path) -> String {
    path.strip_prefix(workspace)
        .ok()
        .filter(|relative| !relative.as_os_str().is_empty())
        .map(|relative| relative.to_string_lossy().replace('\\', "/"))
        .unwrap_or_else(|| ".".to_string())
}

fn normalize_path(path: &str) -> String {
    path.trim_matches('"').replace('\\', "/")
}

fn compact(value: &str, max_chars: usize) -> String {
    let single_line = value.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut out = single_line.chars().take(max_chars).collect::<String>();
    if single_line.chars().count() > max_chars {
        out.push_str("...");
    }
    out
}
