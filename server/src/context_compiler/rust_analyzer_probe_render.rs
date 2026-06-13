use super::model::{RustAnalyzerProbeReport, RustAnalyzerProbeStatus};

pub(crate) fn render_rust_analyzer_probes(
    out: &mut String,
    report: Option<&RustAnalyzerProbeReport>,
) {
    let Some(report) = report else {
        return;
    };

    out.push_str("<rust_analyzer_probes>\n");
    if !report.enabled {
        out.push_str("- status: disabled\n");
        out.push_str("- enable_with: ELON_CONTEXT_COMPILER_RA_PROBE=true\n");
        out.push_str("</rust_analyzer_probes>\n\n");
        return;
    }

    out.push_str(&format!(
        "- workspace_path: {}\n",
        report.workspace_path.as_deref().unwrap_or("unknown")
    ));
    for warning in &report.warnings {
        out.push_str(&format!("- warning: {}\n", markdown_escape(warning)));
    }
    for command in &report.commands {
        out.push_str(&format!(
            "- command: {} status={} duration_ms={} findings={}\n",
            markdown_escape(&command.command),
            command.status.as_str(),
            command.duration_ms,
            command.findings.len()
        ));
        if let Some(code) = command.exit_code {
            out.push_str(&format!("  exit_code: {code}\n"));
        }
        if let Some(warning) = command.warning.as_deref() {
            out.push_str(&format!("  warning: {}\n", markdown_escape(warning)));
        }
        render_findings(
            out,
            command.name.as_str(),
            command.status,
            &command.findings,
        );
        render_excerpt(out, "stdout", &command.stdout_excerpt);
        render_excerpt(out, "stderr", &command.stderr_excerpt);
    }
    out.push_str("</rust_analyzer_probes>\n\n");
}

fn render_findings(
    out: &mut String,
    command_name: &str,
    status: RustAnalyzerProbeStatus,
    findings: &[super::model::RustAnalyzerFinding],
) {
    if findings.is_empty() {
        if status == RustAnalyzerProbeStatus::Succeeded {
            out.push_str("  findings: none\n");
        }
        return;
    }
    out.push_str("  findings:\n");
    for finding in findings.iter().take(12) {
        let location = match (finding.path.as_deref(), finding.line) {
            (Some(path), Some(line)) => format!("{path}:{line}"),
            (Some(path), None) => path.to_string(),
            _ => "unknown".to_string(),
        };
        out.push_str(&format!(
            "    - source={} severity={} location={} message={}\n",
            command_name,
            finding.severity.as_deref().unwrap_or("unknown"),
            markdown_escape(&location),
            markdown_escape(&finding.message)
        ));
    }
}

fn render_excerpt(out: &mut String, label: &str, lines: &[String]) {
    if lines.is_empty() {
        return;
    }
    out.push_str(&format!("  {label}_excerpt:\n"));
    for line in lines.iter().take(4) {
        out.push_str(&format!("    - {}\n", markdown_escape(line)));
    }
}

fn markdown_escape(value: &str) -> String {
    value.replace('<', "&lt;").replace('>', "&gt;")
}
