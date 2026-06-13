use super::model::{ImpactFact, RustImpactAnalysis};

pub(crate) fn render_impact_analysis(out: &mut String, impact: Option<&RustImpactAnalysis>) {
    let Some(impact) = impact else {
        return;
    };
    if is_empty(impact) {
        return;
    }

    out.push_str("<impact_analysis>\n");
    out.push_str("Rust 重构影响面由本地索引器生成，作为 caller/reference/test 查询的优先导航。\n");
    render_fact_group(out, "trait_implementations", &impact.trait_implementations);
    render_fact_group(out, "function_call_sites", &impact.function_call_sites);
    render_fact_group(out, "enum_match_sites", &impact.enum_match_sites);
    render_fact_group(out, "field_accesses", &impact.field_accesses);
    render_fact_group(out, "public_api_references", &impact.public_api_references);
    render_fact_group(out, "test_links", &impact.test_links);
    render_fact_group(out, "async_and_safety_boundaries", &impact.async_boundaries);
    if !impact.limitations.is_empty() {
        out.push_str("<impact_limitations>\n");
        for limitation in &impact.limitations {
            out.push_str(&format!("- {}\n", markdown_escape(limitation)));
        }
        out.push_str("</impact_limitations>\n");
    }
    out.push_str("</impact_analysis>\n\n");
}

fn render_fact_group(out: &mut String, tag: &str, facts: &[ImpactFact]) {
    if facts.is_empty() {
        return;
    }
    out.push_str(&format!("<{tag}>\n"));
    for fact in facts {
        out.push_str(&format!(
            "- kind={} subject={} at {}:{} reason={} evidence={}\n",
            fact.kind.as_str(),
            markdown_escape(&fact.subject),
            markdown_escape(&fact.path),
            fact.line,
            markdown_escape(&fact.reason),
            markdown_escape(&fact.evidence)
        ));
    }
    out.push_str(&format!("</{tag}>\n"));
}

fn is_empty(impact: &RustImpactAnalysis) -> bool {
    impact.trait_implementations.is_empty()
        && impact.function_call_sites.is_empty()
        && impact.enum_match_sites.is_empty()
        && impact.field_accesses.is_empty()
        && impact.public_api_references.is_empty()
        && impact.test_links.is_empty()
        && impact.async_boundaries.is_empty()
        && impact.limitations.is_empty()
}

fn markdown_escape(value: &str) -> String {
    value.replace('<', "&lt;").replace('>', "&gt;")
}
