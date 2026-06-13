use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use serde::Serialize;

const TOKEN_CHARS: usize = 4;

#[derive(Debug, Clone, Serialize)]
struct ContextBudgetReport {
    version: u8,
    estimation: &'static str,
    total_chars: usize,
    estimated_tokens: usize,
    truncated: bool,
    sections: Vec<ContextBudgetSection>,
    groups: Vec<ContextBudgetGroup>,
    warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
struct ContextBudgetSection {
    name: String,
    group: &'static str,
    chars: usize,
    estimated_tokens: usize,
    share_pct: f64,
}

#[derive(Debug, Clone, Serialize)]
struct ContextBudgetGroup {
    name: &'static str,
    chars: usize,
    estimated_tokens: usize,
    share_pct: f64,
    doc_budget_hint: &'static str,
}

pub(crate) fn write_context_budget_exports(
    bundle_dir: &Path,
    pack: &str,
    files: &mut Vec<PathBuf>,
) -> Option<usize> {
    let report = build_context_budget_report(pack);
    let mut bytes = 0usize;
    bytes += write_text(
        &bundle_dir.join("context_budget.json"),
        &serde_json::to_string_pretty(&report).ok()?,
        files,
    )?;
    bytes += write_text(
        &bundle_dir.join("context_budget.md"),
        &render_context_budget_markdown(&report),
        files,
    )?;
    Some(bytes)
}

fn build_context_budget_report(pack: &str) -> ContextBudgetReport {
    let sections = scan_sections(pack);
    let total_chars = pack.chars().count();
    let estimated_tokens = estimate_tokens(total_chars);
    let mut groups = build_groups(&sections, total_chars);
    groups.sort_by(|left, right| right.estimated_tokens.cmp(&left.estimated_tokens));
    let truncated = pack.contains("context pack truncated by ELON_CONTEXT_COMPILER_MAX_CHARS");
    let warnings = build_warnings(&sections, &groups, truncated);

    ContextBudgetReport {
        version: 1,
        estimation: "rough estimate: Unicode scalar count / 4 chars per token",
        total_chars,
        estimated_tokens,
        truncated,
        sections,
        groups,
        warnings,
    }
}

fn scan_sections(pack: &str) -> Vec<ContextBudgetSection> {
    let total_chars = pack.chars().count().max(1);
    let mut sections = Vec::new();
    let mut depth = 0usize;
    let mut in_fence = false;
    let mut current: Option<(String, String)> = None;

    for line in pack.lines() {
        let line_with_newline = format!("{line}\n");
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") {
            in_fence = !in_fence;
        }

        if !in_fence {
            if let Some(close) = closing_tag(trimmed) {
                if let Some((name, content)) = current.as_mut() {
                    content.push_str(&line_with_newline);
                    if close == *name && depth == 2 {
                        let chars = content.chars().count();
                        sections.push(ContextBudgetSection {
                            name: name.clone(),
                            group: group_for_section(name),
                            chars,
                            estimated_tokens: estimate_tokens(chars),
                            share_pct: pct(chars, total_chars),
                        });
                        current = None;
                    }
                }
                depth = depth.saturating_sub(1);
                continue;
            }

            if let Some(open) = opening_tag(trimmed) {
                if depth == 1 && open != "task_context_pack" {
                    current = Some((open.clone(), String::new()));
                }
                if let Some((_, content)) = current.as_mut() {
                    content.push_str(&line_with_newline);
                }
                depth += 1;
                continue;
            }
        }

        if let Some((_, content)) = current.as_mut() {
            content.push_str(&line_with_newline);
        }
    }

    sections
}

fn build_groups(sections: &[ContextBudgetSection], total_chars: usize) -> Vec<ContextBudgetGroup> {
    let mut grouped = BTreeMap::<&'static str, usize>::new();
    for section in sections {
        *grouped.entry(section.group).or_default() += section.chars;
    }
    grouped
        .into_iter()
        .map(|(name, chars)| ContextBudgetGroup {
            name,
            chars,
            estimated_tokens: estimate_tokens(chars),
            share_pct: pct(chars, total_chars.max(1)),
            doc_budget_hint: doc_budget_hint(name),
        })
        .collect()
}

fn build_warnings(
    sections: &[ContextBudgetSection],
    groups: &[ContextBudgetGroup],
    truncated: bool,
) -> Vec<String> {
    let mut warnings = Vec::new();
    if truncated {
        warnings.push("context pack was truncated by ELON_CONTEXT_COMPILER_MAX_CHARS".to_string());
    }
    if !sections
        .iter()
        .any(|section| section.name == "relevant_files")
    {
        warnings.push("no relevant_files full-source section was present".to_string());
    }
    if !sections
        .iter()
        .any(|section| section.name == "symbol_graph")
    {
        warnings.push("no symbol_graph section was present".to_string());
    }
    if let Some(group) = groups.iter().find(|group| group.name == "full_source") {
        if group.share_pct < 15.0 {
            warnings.push(
                "full_source share is low for refactor tasks; read real files before editing"
                    .to_string(),
            );
        }
    }
    warnings
}

fn render_context_budget_markdown(report: &ContextBudgetReport) -> String {
    let mut out = String::new();
    out.push_str("# Context Budget\n\n");
    out.push_str("- estimation: rough chars / 4 token estimate\n");
    out.push_str(&format!(
        "- total_chars: {}\n- estimated_tokens: {}\n- truncated: {}\n\n",
        report.total_chars, report.estimated_tokens, report.truncated
    ));

    out.push_str("## Groups\n\n");
    out.push_str("| group | est_tokens | share | doc_budget_hint |\n");
    out.push_str("|---|---:|---:|---|\n");
    for group in &report.groups {
        out.push_str(&format!(
            "| {} | {} | {:.1}% | {} |\n",
            group.name, group.estimated_tokens, group.share_pct, group.doc_budget_hint
        ));
    }

    out.push_str("\n## Sections\n\n");
    out.push_str("| section | group | est_tokens | share |\n");
    out.push_str("|---|---|---:|---:|\n");
    for section in &report.sections {
        out.push_str(&format!(
            "| {} | {} | {} | {:.1}% |\n",
            section.name, section.group, section.estimated_tokens, section.share_pct
        ));
    }

    if !report.warnings.is_empty() {
        out.push_str("\n## Warnings\n\n");
        for warning in &report.warnings {
            out.push_str(&format!("- {warning}\n"));
        }
    }
    out
}

fn opening_tag(line: &str) -> Option<String> {
    let rest = line.strip_prefix('<')?;
    if rest.starts_with('/') || rest.starts_with('!') || rest.starts_with('?') {
        return None;
    }
    let end = rest
        .find(|ch: char| ch == '>' || ch.is_ascii_whitespace())
        .unwrap_or(rest.len());
    let name = &rest[..end];
    is_section_name(name).then(|| name.to_string())
}

fn closing_tag(line: &str) -> Option<String> {
    let rest = line.strip_prefix("</")?;
    let end = rest.find('>').unwrap_or(rest.len());
    let name = &rest[..end];
    is_section_name(name).then(|| name.to_string())
}

fn is_section_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|ch| ch.is_ascii_lowercase() || ch == '_' || ch.is_ascii_digit())
}

fn group_for_section(name: &str) -> &'static str {
    match name {
        "instructions" | "output_contract" | "final_instructions" => "instructions_output",
        "task" | "task_understanding" | "llm_brief" => "task",
        "repo_snapshot" | "rust_project" | "cargo_workspace" => "project_brief",
        "repo_map" | "repo_map_tags" => "repo_map",
        "symbol_graph"
        | "rust_analyzer"
        | "rust_analyzer_probe"
        | "semantic_query_plan"
        | "rust_analyzer_lsp"
        | "impact_analysis" => "symbol_graph",
        "rust_safety_context"
        | "source_size_risks"
        | "invariants"
        | "public_api_contracts"
        | "unsafe_boundaries"
        | "feature_flags"
        | "missing_context_policy" => "constraints",
        "relevant_files" => "full_source",
        "neighbor_summaries" => "neighbor_summaries",
        "tests" | "build_commands" | "validation_guidance" => "tests_validation",
        "retrieval_evidence" | "recommended_agent_actions" | "context_quality" => {
            "retrieval_evidence"
        }
        _ => "other",
    }
}

fn doc_budget_hint(group: &str) -> &'static str {
    match group {
        "instructions_output" => "1k-3k",
        "project_brief" => "1k-3k",
        "repo_map" => "3k-10k",
        "symbol_graph" => "5k-20k",
        "constraints" => "1k-5k",
        "full_source" => "30k-80k",
        "neighbor_summaries" => "10k-40k",
        "tests_validation" => "10k-40k",
        "retrieval_evidence" => "3k-10k",
        _ => "task-dependent",
    }
}

fn estimate_tokens(chars: usize) -> usize {
    chars.div_ceil(TOKEN_CHARS)
}

fn pct(part: usize, total: usize) -> f64 {
    if total == 0 {
        return 0.0;
    }
    (part as f64 / total as f64) * 100.0
}

fn write_text(path: &Path, content: &str, files: &mut Vec<PathBuf>) -> Option<usize> {
    fs::write(path, content.as_bytes()).ok()?;
    files.push(path.to_path_buf());
    Some(content.len())
}
