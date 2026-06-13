use super::model::RepoMapTagSummary;

pub(crate) fn render_repo_map_tags(out: &mut String, summary: Option<&RepoMapTagSummary>) {
    let Some(summary) = summary else {
        return;
    };
    if summary.definitions == 0 && summary.references == 0 && summary.edges.is_empty() {
        return;
    }

    out.push_str("<aider_repo_map_tags>\n");
    out.push_str("- source: rust-native def/ref tags\n");
    out.push_str(&format!(
        "- definitions: {}\n- references: {}\n",
        summary.definitions, summary.references
    ));
    if !summary.warnings.is_empty() {
        out.push_str("- warnings:\n");
        for warning in &summary.warnings {
            out.push_str(&format!("  - {}\n", markdown_escape(warning)));
        }
    }
    if !summary.edges.is_empty() {
        out.push_str("- top_edges:\n");
        for edge in summary.edges.iter().take(12) {
            let lines = edge
                .reference_lines
                .iter()
                .take(6)
                .map(|line| line.to_string())
                .collect::<Vec<_>>()
                .join(",");
            out.push_str(&format!(
                "  - {} -> {} symbol={} refs={} def_line={} ref_lines={} score={:.2}\n",
                markdown_escape(&edge.from_path),
                markdown_escape(&edge.to_path),
                markdown_escape(&edge.symbol),
                edge.references,
                edge.definition_line,
                lines,
                edge.score
            ));
        }
    }
    out.push_str("</aider_repo_map_tags>\n\n");
}

fn markdown_escape(value: &str) -> String {
    value.replace('<', "&lt;").replace('>', "&gt;")
}
