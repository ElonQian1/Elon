use super::model::SemanticQueryPlan;

pub(crate) fn render_semantic_query_plan(out: &mut String, plan: Option<&SemanticQueryPlan>) {
    let Some(plan) = plan else {
        return;
    };
    if plan.queries.is_empty() && plan.warnings.is_empty() {
        return;
    }

    out.push_str("<semantic_query_plan>\n");
    out.push_str("- source: repo-map Top-K planner for rust-analyzer LSP\n");
    out.push_str(&format!(
        "- coverage: top_files={} top_symbols={} planned_files={} planned_symbols={} queries={}\n",
        plan.coverage.top_files_considered,
        plan.coverage.top_symbols_considered,
        plan.coverage.planned_files,
        plan.coverage.planned_symbols,
        plan.coverage.query_count
    ));
    if !plan.warnings.is_empty() {
        out.push_str("- warnings:\n");
        for warning in &plan.warnings {
            out.push_str(&format!("  - {}\n", markdown_escape(warning)));
        }
    }
    if !plan.queries.is_empty() {
        out.push_str("- lsp_queries:\n");
        for query in plan.queries.iter().take(40) {
            out.push_str(&format!(
                "  - priority={} provider={} method={} path={} line={} symbol={} reason={}\n",
                query.priority,
                query.provider.as_str(),
                query.method.as_lsp_method(),
                markdown_escape(&query.path),
                query.line,
                markdown_escape(query.symbol.as_deref().unwrap_or("-")),
                markdown_escape(&query.reason)
            ));
        }
    }
    out.push_str("</semantic_query_plan>\n\n");
}

fn markdown_escape(value: &str) -> String {
    value.replace('<', "&lt;").replace('>', "&gt;")
}
