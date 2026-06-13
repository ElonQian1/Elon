use std::{
    fs,
    io::{Read, Write},
    path::Path,
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use super::{
    model::{RustAnalyzerReport, RustAnalyzerSymbol, RustIndex, SymbolGraphSummary},
    rust_analyzer_probe,
};

const COMMAND_TIMEOUT: Duration = Duration::from_secs(4);
const MAX_RA_SYMBOLS: usize = 160;

pub(crate) fn collect_rust_analyzer_report(
    workspace: &Path,
    rust: &RustIndex,
    graph: &SymbolGraphSummary,
    enabled: bool,
    max_files: usize,
    probe_enabled: bool,
    probe_timeout_ms: usize,
) -> RustAnalyzerReport {
    if !enabled {
        return RustAnalyzerReport {
            warnings: vec!["rust-analyzer enhancement disabled by config".to_string()],
            ..RustAnalyzerReport::default()
        };
    }

    let mut report = RustAnalyzerReport::default();
    match run_process(
        workspace,
        "rust-analyzer",
        &["--version"],
        None,
        COMMAND_TIMEOUT,
    ) {
        Ok(output) => {
            report.available = true;
            report.version = Some(compact(output.stdout.trim(), 120));
        }
        Err(error) => {
            report
                .warnings
                .push(format!("rust-analyzer unavailable: {error}"));
            return report;
        }
    }

    report.probes = rust_analyzer_probe::collect_rust_analyzer_probes(
        workspace,
        rust,
        probe_enabled,
        probe_timeout_ms,
    );

    let targets = enhancement_targets(rust, graph, max_files);
    report.enhancement_targets = targets.clone();
    for path in targets {
        if report.symbols.len() >= MAX_RA_SYMBOLS {
            break;
        }
        let full_path = workspace.join(&path);
        let Ok(content) = fs::read_to_string(&full_path) else {
            report.warnings.push(format!(
                "rust-analyzer symbols skipped unreadable file: {path}"
            ));
            continue;
        };
        match run_process(
            workspace,
            "rust-analyzer",
            &["symbols"],
            Some(content.as_str()),
            COMMAND_TIMEOUT,
        ) {
            Ok(output) => {
                let parsed = parse_symbols_output(&path, &content, &output.stdout);
                report.files_enhanced += 1;
                report.symbols.extend(parsed);
            }
            Err(error) => report
                .warnings
                .push(format!("rust-analyzer symbols failed for {path}: {error}")),
        }
    }
    report.symbols.truncate(MAX_RA_SYMBOLS);
    report
}

fn enhancement_targets(
    rust: &RustIndex,
    graph: &SymbolGraphSummary,
    max_files: usize,
) -> Vec<String> {
    let mut targets = graph
        .ranked_files
        .iter()
        .filter(|file| file.path.ends_with(".rs"))
        .map(|file| file.path.clone())
        .collect::<Vec<_>>();
    if targets.is_empty() {
        for symbol in &rust.symbols {
            if !targets.contains(&symbol.path) {
                targets.push(symbol.path.clone());
            }
            if targets.len() >= max_files {
                break;
            }
        }
    }
    targets.truncate(max_files);
    targets
}

struct ProcessOutput {
    stdout: String,
}

fn run_process(
    workspace: &Path,
    program: &str,
    args: &[&str],
    stdin_text: Option<&str>,
    timeout: Duration,
) -> Result<ProcessOutput, String> {
    let mut child = Command::new(program)
        .args(args)
        .current_dir(workspace)
        .stdin(if stdin_text.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| error.to_string())?;

    if let Some(input) = stdin_text {
        if let Some(mut stdin) = child.stdin.take() {
            stdin
                .write_all(input.as_bytes())
                .map_err(|error| error.to_string())?;
        }
    }

    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let mut stdout = String::new();
                let mut stderr = String::new();
                if let Some(mut pipe) = child.stdout.take() {
                    let _ = pipe.read_to_string(&mut stdout);
                }
                if let Some(mut pipe) = child.stderr.take() {
                    let _ = pipe.read_to_string(&mut stderr);
                }
                if !status.success() {
                    return Err(compact(stderr.trim(), 240));
                }
                return Ok(ProcessOutput { stdout });
            }
            Ok(None) => {
                if started.elapsed() >= timeout {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(format!("timed out after {}s", timeout.as_secs()));
                }
                thread::sleep(Duration::from_millis(40));
            }
            Err(error) => return Err(error.to_string()),
        }
    }
}

fn parse_symbols_output(path: &str, content: &str, output: &str) -> Vec<RustAnalyzerSymbol> {
    let mut symbols = Vec::new();
    for line in output.lines() {
        let Some(label) = quoted_field(line, "label: ") else {
            continue;
        };
        let kind = symbol_kind(line).unwrap_or_else(|| "Unknown".to_string());
        let detail = if line.contains("detail: Some(") {
            quoted_field(line, "detail: Some(")
        } else {
            None
        };
        let line_number = navigation_offset(line)
            .map(|offset| byte_offset_to_line(content, offset))
            .unwrap_or(1);
        symbols.push(RustAnalyzerSymbol {
            path: path.to_string(),
            label,
            kind,
            detail,
            line: line_number,
            parent: parent_index(line),
        });
    }
    symbols
}

fn quoted_field(line: &str, marker: &str) -> Option<String> {
    let start = line.find(marker)? + marker.len();
    let after = line.get(start..)?.trim_start();
    let quote_start = after.find('"')? + 1;
    let quoted = after.get(quote_start..)?;
    let quote_end = quoted.find('"')?;
    Some(quoted[..quote_end].to_string())
}

fn symbol_kind(line: &str) -> Option<String> {
    let marker = "kind: SymbolKind(";
    let start = line.find(marker)? + marker.len();
    let rest = line.get(start..)?;
    let end = rest.find(')')?;
    Some(rest[..end].to_string())
}

fn navigation_offset(line: &str) -> Option<usize> {
    let marker = "navigation_range: ";
    let start = line.find(marker)? + marker.len();
    let rest = line.get(start..)?;
    let (from, _) = rest.split_once("..")?;
    from.trim().parse::<usize>().ok()
}

fn parent_index(line: &str) -> Option<String> {
    let marker = "parent: Some(";
    let start = line.find(marker)? + marker.len();
    let rest = line.get(start..)?;
    let end = rest.find(')')?;
    Some(rest[..end].to_string())
}

fn byte_offset_to_line(content: &str, offset: usize) -> usize {
    let safe_offset = offset.min(content.len());
    content
        .as_bytes()
        .iter()
        .take(safe_offset)
        .filter(|byte| **byte == b'\n')
        .count()
        + 1
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

    #[test]
    fn parses_rust_analyzer_symbols_output() {
        let content = "pub struct Foo;\nimpl Foo {\n  pub fn new() {}\n}\n";
        let output = r#"StructureNode { parent: None, label: "Foo", navigation_range: 11..14, node_range: 0..15, kind: SymbolKind(Struct), detail: None, deprecated: false }
StructureNode { parent: Some(1), label: "new", navigation_range: 37..40, node_range: 30..46, kind: SymbolKind(Function), detail: Some("fn()"), deprecated: false }"#;

        let symbols = parse_symbols_output("src/lib.rs", content, output);

        assert_eq!(symbols[0].label, "Foo");
        assert_eq!(symbols[1].kind, "Function");
        assert_eq!(symbols[1].line, 3);
    }
}
