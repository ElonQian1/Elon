use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
};

use serde_json::json;

use super::{
    model::{RepoContextIndex, RustAnalyzerLspStatus, SemanticQueryMethod},
    validation::ValidationPlan,
};

const MAX_SYMBOLS_JSONL: usize = 2_000;
const MAX_EDGES_TSV: usize = 2_000;
const MAX_CHUNKS_JSONL: usize = 120;
const MAX_LSP_LOCATIONS_JSONL: usize = 500;
const MAX_SEMANTIC_FACTS_JSONL: usize = 500;
const MAX_TESTS_JSONL: usize = 200;

pub(crate) fn write_context_exports(
    bundle_dir: &Path,
    repo_index: Option<&RepoContextIndex>,
    validation_plan: &ValidationPlan,
    files: &mut Vec<PathBuf>,
) -> Option<usize> {
    let Some(repo_index) = repo_index else {
        return Some(0);
    };
    let mut bytes = 0usize;
    bytes += write_export_text(
        &bundle_dir.join("repo_map.md"),
        &build_repo_map_markdown(repo_index),
        files,
    )?;
    bytes += write_export_text(
        &bundle_dir.join("summaries.md"),
        &build_summaries_markdown(repo_index),
        files,
    )?;
    bytes += write_export_text(
        &bundle_dir.join("symbols.jsonl"),
        &build_symbols_jsonl(repo_index),
        files,
    )?;
    bytes += write_export_text(
        &bundle_dir.join("edges.tsv"),
        &build_edges_tsv(repo_index),
        files,
    )?;
    bytes += write_export_text(
        &bundle_dir.join("chunks.jsonl"),
        &build_chunks_jsonl(repo_index),
        files,
    )?;
    bytes += write_export_text(
        &bundle_dir.join("tests.jsonl"),
        &build_tests_jsonl(repo_index, validation_plan),
        files,
    )?;
    bytes += write_export_text(
        &bundle_dir.join("lsp_locations.jsonl"),
        &build_lsp_locations_jsonl(repo_index),
        files,
    )?;
    bytes += write_export_text(
        &bundle_dir.join("semantic_facts.jsonl"),
        &build_semantic_facts_jsonl(repo_index),
        files,
    )?;
    Some(bytes)
}

fn build_repo_map_markdown(index: &RepoContextIndex) -> String {
    let mut out = String::new();
    out.push_str("# Repo Map\n\n");
    out.push_str(
        "This file is a compact, tool-generated map. Read real source files before editing.\n\n",
    );
    out.push_str("## Ranked Files\n\n");
    out.push_str("| rank | score | role | path | top symbols |\n");
    out.push_str("|---:|---:|---|---|---|\n");
    for (rank, file) in index.graph.ranked_files.iter().take(40).enumerate() {
        out.push_str(&format!(
            "| {} | {:.2} | {} | `{}` | {} |\n",
            rank + 1,
            file.score,
            markdown_table_escape(file.role),
            markdown_table_escape(&file.path),
            markdown_table_escape(&file.top_symbols.join(", "))
        ));
    }

    out.push_str("\n## Ranked Symbols\n\n");
    out.push_str("```text\n");
    for (rank, symbol) in index.graph.ranked_symbols.iter().take(80).enumerate() {
        out.push_str(&format!(
            "{:02}. {} {} {}:{}-{} score={:.2}\n",
            rank + 1,
            symbol.kind.as_str(),
            symbol.name,
            symbol.path,
            symbol.line_start,
            symbol.line_end,
            symbol.score
        ));
        if !symbol.reasons.is_empty() {
            out.push_str(&format!(
                "    reason={}\n",
                compact(&symbol.reasons.join("; "), 220)
            ));
        }
    }
    out.push_str("```\n\n");

    out.push_str("## Relationships\n\n");
    out.push_str("```text\n");
    for edge in index.graph.relationships.iter().take(80) {
        out.push_str(&format!(
            "{}:{} --{}--> {} ({}) reason={}\n",
            edge.from_path,
            edge.line,
            edge.kind.as_str(),
            edge.to_symbol_name,
            edge.to_path,
            compact(&edge.reason, 180)
        ));
    }
    out.push_str("```\n\n");

    if !index.graph.repo_map_tags.edges.is_empty() {
        out.push_str("## Def/Ref Tags\n\n");
        out.push_str("```text\n");
        for edge in index.graph.repo_map_tags.edges.iter().take(80) {
            out.push_str(&format!(
                "{} refs={} {} -> {}:{} reason={}\n",
                edge.symbol,
                edge.references,
                edge.from_path,
                edge.to_path,
                edge.definition_line,
                compact(&edge.reason, 180)
            ));
        }
        out.push_str("```\n\n");
    }

    out
}

fn build_summaries_markdown(index: &RepoContextIndex) -> String {
    let mut out = String::new();
    out.push_str("# Context Summaries\n\n");
    out.push_str("## Task Profile\n\n");
    push_list(&mut out, "keywords", &index.task.keywords);
    push_list(&mut out, "likely_domains", &index.task.likely_domains);
    push_list(&mut out, "suspected_symbols", &index.task.suspected_symbols);
    push_list(&mut out, "suspected_files", &index.task.suspected_files);

    out.push_str("\n## Cargo Workspace\n\n");
    out.push_str(&format!(
        "- manifest_path: {}\n",
        index.cargo.manifest_path.as_deref().unwrap_or("unknown")
    ));
    out.push_str(&format!(
        "- workspace_root: {}\n",
        index.cargo.workspace_root.as_deref().unwrap_or("unknown")
    ));
    for package in index.cargo.packages.iter().take(24) {
        out.push_str(&format!(
            "- package: {} {} manifest={} targets={} features={}\n",
            package.name,
            package.version,
            package.manifest_path,
            package.targets.join(","),
            package
                .features
                .iter()
                .take(12)
                .cloned()
                .collect::<Vec<_>>()
                .join(",")
        ));
    }

    out.push_str("\n## Semantic Coverage\n\n");
    out.push_str(&format!(
        "- rust_symbols: {} files_scanned={}\n",
        index.rust.symbols.len(),
        index.rust.files_scanned
    ));
    out.push_str(&format!(
        "- relationships: {} repo_map_tag_edges={}\n",
        index.graph.relationships.len(),
        index.graph.repo_map_tags.edges.len()
    ));
    out.push_str(&format!(
        "- semantic_queries: {} lsp_enabled={} lsp_attempted={} lsp_succeeded={} lsp_locations={}\n",
        index.semantic_plan.queries.len(),
        index.rust_analyzer.lsp.enabled,
        index.rust_analyzer.lsp.attempted,
        index.rust_analyzer.lsp.succeeded,
        index
            .rust_analyzer
            .lsp
            .results
            .iter()
            .map(|result| result.locations.len())
            .sum::<usize>()
    ));
    out.push_str(&format!(
        "- context_quality: score={} gaps={}\n",
        index.quality.score,
        index.quality.gaps.len()
    ));

    out.push_str("\n## Recommended Actions\n\n");
    for action in &index.quality.recommended_actions {
        out.push_str(&format!("- {}\n", action));
    }
    for action in &index.evidence.recommended_actions {
        out.push_str(&format!("- {}\n", action));
    }
    out
}

fn build_symbols_jsonl(index: &RepoContextIndex) -> String {
    index
        .rust
        .symbols
        .iter()
        .take(MAX_SYMBOLS_JSONL)
        .filter_map(|symbol| serde_json::to_string(symbol).ok())
        .collect::<Vec<_>>()
        .join("\n")
        + "\n"
}

fn build_edges_tsv(index: &RepoContextIndex) -> String {
    let mut out = String::new();
    out.push_str("source\tkind\tfrom_path\tline\tto_path\tto_symbol_id\tto_symbol_name\treason\n");
    let mut rows = 0usize;
    for edge in &index.graph.relationships {
        if rows >= MAX_EDGES_TSV {
            break;
        }
        out.push_str(&format!(
            "symbol_graph\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
            edge.kind.as_str(),
            tsv_escape(&edge.from_path),
            edge.line,
            tsv_escape(&edge.to_path),
            tsv_escape(&edge.to_symbol_id),
            tsv_escape(&edge.to_symbol_name),
            tsv_escape(&edge.reason)
        ));
        rows += 1;
    }
    for edge in &index.graph.repo_map_tags.edges {
        if rows >= MAX_EDGES_TSV {
            break;
        }
        out.push_str(&format!(
            "repo_map_tags\tdef_ref\t{}\t{}\t{}\t{}\t{}\t{}\n",
            tsv_escape(&edge.from_path),
            edge.reference_lines
                .first()
                .copied()
                .unwrap_or(edge.definition_line),
            tsv_escape(&edge.to_path),
            tsv_escape(&edge.target_symbol_id),
            tsv_escape(&edge.symbol),
            tsv_escape(&edge.reason)
        ));
        rows += 1;
    }
    out
}

fn build_chunks_jsonl(index: &RepoContextIndex) -> String {
    index
        .evidence
        .snippets
        .iter()
        .take(MAX_CHUNKS_JSONL)
        .filter_map(|snippet| {
            serde_json::to_string(&json!({
                "id": snippet.id,
                "source": "context_evidence",
                "path": snippet.path,
                "role": snippet.role,
                "symbols": snippet.symbols,
                "line_start": snippet.line_start,
                "line_end": snippet.line_end,
                "sha256": snippet.sha256,
                "reason": snippet.reason,
                "content": snippet.content,
            }))
            .ok()
        })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n"
}

fn build_tests_jsonl(index: &RepoContextIndex, validation_plan: &ValidationPlan) -> String {
    let mut lines = Vec::new();

    for target in &index.evidence.test_targets {
        if lines.len() >= MAX_TESTS_JSONL {
            break;
        }
        if let Ok(line) = serde_json::to_string(&json!({
            "source": "context_evidence",
            "test_kind": "test_target",
            "path": &target.path,
            "reason": &target.reason,
            "confidence": "candidate"
        })) {
            lines.push(line);
        }
    }

    for command in &index.evidence.build_commands {
        if lines.len() >= MAX_TESTS_JSONL {
            break;
        }
        if let Ok(line) = serde_json::to_string(&json!({
            "source": "context_evidence",
            "test_kind": "build_command",
            "command": &command.command,
            "reason": &command.reason,
            "required": false
        })) {
            lines.push(line);
        }
    }

    for command in &validation_plan.commands {
        if lines.len() >= MAX_TESTS_JSONL {
            break;
        }
        if let Ok(line) = serde_json::to_string(&json!({
            "source": "validation_plan",
            "test_kind": "validation_command",
            "command": &command.command,
            "reason": &command.reason,
            "required": command.required
        })) {
            lines.push(line);
        }
    }

    lines.join("\n") + "\n"
}

fn build_lsp_locations_jsonl(index: &RepoContextIndex) -> String {
    let mut lines = Vec::new();
    let mut seen = HashSet::new();
    for result in &index.rust_analyzer.lsp.results {
        if result.status != RustAnalyzerLspStatus::Succeeded {
            continue;
        }
        for location in &result.locations {
            if lines.len() >= MAX_LSP_LOCATIONS_JSONL {
                break;
            }
            let key = format!(
                "{}:{}:{}:{}:{}",
                result.method.as_lsp_method(),
                location.role.as_str(),
                location.path,
                location.line,
                location.symbol.as_deref().unwrap_or("")
            );
            if !seen.insert(key) {
                continue;
            }
            if let Ok(line) = serde_json::to_string(&json!({
                "method": result.method.as_lsp_method(),
                "query_path": result.path,
                "query_line": result.line,
                "query_symbol": result.symbol,
                "role": location.role.as_str(),
                "path": location.path,
                "line": location.line,
                "end_line": location.end_line,
                "symbol": location.symbol,
            })) {
                lines.push(line);
            }
        }
    }
    lines.join("\n") + "\n"
}

fn build_semantic_facts_jsonl(index: &RepoContextIndex) -> String {
    index
        .rust_analyzer
        .lsp
        .results
        .iter()
        .take(MAX_SEMANTIC_FACTS_JSONL)
        .filter_map(|result| {
            serde_json::to_string(&json!({
                "source": "rust_analyzer_lsp",
                "fact_kind": semantic_fact_kind(result.method),
                "method": result.method.as_lsp_method(),
                "status": result.status.as_str(),
                "query_path": result.path,
                "query_line": result.line,
                "query_symbol": result.symbol.as_deref(),
                "summary": result.summary.as_deref(),
                "location_count": result.locations.len(),
                "warning": result.warning.as_deref(),
            }))
            .ok()
        })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n"
}

fn semantic_fact_kind(method: SemanticQueryMethod) -> &'static str {
    match method {
        SemanticQueryMethod::DocumentSymbol => "document_symbols",
        SemanticQueryMethod::WorkspaceSymbol => "workspace_symbols",
        SemanticQueryMethod::Diagnostic => "diagnostic",
        SemanticQueryMethod::Definition => "definitions",
        SemanticQueryMethod::References => "references",
        SemanticQueryMethod::Implementation => "implementations",
        SemanticQueryMethod::Hover => "hover_type",
        SemanticQueryMethod::PrepareCallHierarchy
        | SemanticQueryMethod::IncomingCalls
        | SemanticQueryMethod::OutgoingCalls => "call_hierarchy",
    }
}

fn write_export_text(path: &Path, content: &str, files: &mut Vec<PathBuf>) -> Option<usize> {
    fs::write(path, content.as_bytes()).ok()?;
    files.push(path.to_path_buf());
    Some(content.len())
}

fn push_list(out: &mut String, label: &str, values: &[String]) {
    if values.is_empty() {
        out.push_str(&format!("- {label}: -\n"));
        return;
    }
    out.push_str(&format!("- {label}: {}\n", values.join(", ")));
}

fn markdown_table_escape(value: &str) -> String {
    value.replace('|', "\\|").replace('\n', " ")
}

fn tsv_escape(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .replace('\t', " ")
}

fn compact(value: &str, max_chars: usize) -> String {
    let single_line = value.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut out = single_line.chars().take(max_chars).collect::<String>();
    if single_line.chars().count() > max_chars {
        out.push_str("...");
    }
    out
}
