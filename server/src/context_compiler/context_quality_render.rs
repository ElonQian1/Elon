use super::model::ContextQualityReport;

pub(crate) fn render_context_quality(out: &mut String, report: Option<&ContextQualityReport>) {
    let Some(report) = report else {
        return;
    };
    if report.score == 0 && report.gaps.is_empty() && report.recommended_actions.is_empty() {
        return;
    }

    out.push_str("<context_quality_report>\n");
    out.push_str(&format!("- score: {}/100\n", report.score));
    out.push_str(&format!(
        "- coverage: top_files={}/{} top_symbols={}/{} snippets={} relationships={} tag_edges={} impact_facts={} validation_commands={}\n",
        report.coverage.top_files_with_snippets,
        report.coverage.top_files_considered,
        report.coverage.top_symbols_with_snippets,
        report.coverage.top_symbols_considered,
        report.coverage.snippet_count,
        report.coverage.relationship_count,
        report.coverage.repo_map_tag_edges,
        report.coverage.impact_fact_count,
        report.coverage.validation_commands
    ));
    out.push_str(&format!(
        "- semantic: rust_analyzer_available={} ra_symbols={} ra_files={} lsp_queries={} lsp_enabled={} lsp_attempted={} lsp_ok={} lsp_locations={} lsp_failed={} lsp_timed_out={} probe_enabled={} probe_ok={} probe_failed={} probe_timed_out={}\n",
        report.semantic.rust_analyzer_available,
        report.semantic.rust_analyzer_symbols,
        report.semantic.rust_analyzer_files_enhanced,
        report.semantic.lsp_queries_planned,
        report.semantic.lsp_enabled,
        report.semantic.lsp_attempted,
        report.semantic.lsp_succeeded,
        report.semantic.lsp_locations,
        report.semantic.lsp_failed,
        report.semantic.lsp_timed_out,
        report.semantic.probe_enabled,
        report.semantic.probe_succeeded,
        report.semantic.probe_failed,
        report.semantic.probe_timed_out
    ));
    if !report.gaps.is_empty() {
        out.push_str("- gaps:\n");
        for gap in report.gaps.iter().take(12) {
            out.push_str(&format!(
                "  - severity={} subject={} path={} line={} detail={} action={}\n",
                gap.severity.as_str(),
                markdown_escape(&gap.subject),
                markdown_escape(gap.path.as_deref().unwrap_or("-")),
                gap.line
                    .map(|line| line.to_string())
                    .unwrap_or_else(|| "-".to_string()),
                markdown_escape(&gap.detail),
                markdown_escape(&gap.action)
            ));
        }
    }
    if !report.recommended_actions.is_empty() {
        out.push_str("- recommended_actions:\n");
        for action in report.recommended_actions.iter().take(8) {
            out.push_str(&format!("  - {}\n", markdown_escape(action)));
        }
    }
    out.push_str("</context_quality_report>\n\n");
}

fn markdown_escape(value: &str) -> String {
    value.replace('<', "&lt;").replace('>', "&gt;")
}
