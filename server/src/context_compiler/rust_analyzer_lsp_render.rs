use super::model::RustAnalyzerLspReport;

pub(crate) fn render_rust_analyzer_lsp(out: &mut String, report: Option<&RustAnalyzerLspReport>) {
    let Some(report) = report else {
        return;
    };

    out.push_str("<rust_analyzer_lsp>\n");
    if !report.enabled {
        out.push_str("- status: disabled\n");
        out.push_str("- enable_with: ELON_CONTEXT_COMPILER_RA_LSP=true\n");
        out.push_str("</rust_analyzer_lsp>\n\n");
        return;
    }

    out.push_str(&format!(
        "- workspace_path: {}\n",
        report.workspace_path.as_deref().unwrap_or("unknown")
    ));
    out.push_str(&format!(
        "- summary: attempted={} succeeded={} failed={} timed_out={} skipped={}\n",
        report.attempted, report.succeeded, report.failed, report.timed_out, report.skipped
    ));
    if !report.warnings.is_empty() {
        out.push_str("- warnings:\n");
        for warning in &report.warnings {
            out.push_str(&format!("  - {}\n", markdown_escape(warning)));
        }
    }
    if !report.results.is_empty() {
        out.push_str("- results:\n");
        for result in report.results.iter().take(32) {
            out.push_str(&format!(
                "  - status={} method={} path={} line={} symbol={} duration_ms={}\n",
                result.status.as_str(),
                result.method.as_lsp_method(),
                markdown_escape(&result.path),
                result.line,
                markdown_escape(result.symbol.as_deref().unwrap_or("-")),
                result.duration_ms
            ));
            if let Some(summary) = result.summary.as_deref() {
                out.push_str(&format!("    summary: {}\n", markdown_escape(summary)));
            }
            if let Some(warning) = result.warning.as_deref() {
                out.push_str(&format!("    warning: {}\n", markdown_escape(warning)));
            }
        }
    }
    out.push_str("</rust_analyzer_lsp>\n\n");
}

fn markdown_escape(value: &str) -> String {
    value.replace('<', "&lt;").replace('>', "&gt;")
}
