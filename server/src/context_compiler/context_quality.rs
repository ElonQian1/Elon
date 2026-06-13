use std::collections::HashSet;

use super::{
    model::{
        ContextQualityCoverage, ContextQualityGap, ContextQualityReport, ContextQualitySemantic,
        ContextQualitySeverity, RepoContextIndex, RustAnalyzerLspStatus, RustAnalyzerProbeStatus,
    },
    relevance::RelevantFile,
    validation::ValidationPlan,
};

const TOP_FILE_SAMPLE: usize = 12;
const TOP_SYMBOL_SAMPLE: usize = 20;
const MAX_GAPS: usize = 18;

pub(crate) fn build_context_quality_report(
    index: &RepoContextIndex,
    relevant_files: &[RelevantFile],
    validation_plan: &ValidationPlan,
) -> ContextQualityReport {
    let coverage = build_coverage(index, validation_plan);
    let semantic = build_semantic(index);
    let mut gaps = collect_gaps(index, relevant_files, &coverage, &semantic, validation_plan);
    gaps.truncate(MAX_GAPS);
    let score = score_quality(&coverage, &semantic, &gaps);
    let recommended_actions = recommended_actions(index, &coverage, &semantic, &gaps);

    ContextQualityReport {
        score,
        coverage,
        semantic,
        gaps,
        recommended_actions,
    }
}

fn build_coverage(
    index: &RepoContextIndex,
    validation_plan: &ValidationPlan,
) -> ContextQualityCoverage {
    let snippet_paths = index
        .evidence
        .snippets
        .iter()
        .map(|snippet| snippet.path.as_str())
        .collect::<HashSet<_>>();
    let top_files = index
        .graph
        .ranked_files
        .iter()
        .take(TOP_FILE_SAMPLE)
        .collect::<Vec<_>>();
    let top_symbols = index
        .graph
        .ranked_symbols
        .iter()
        .take(TOP_SYMBOL_SAMPLE)
        .collect::<Vec<_>>();

    let top_files_with_snippets = top_files
        .iter()
        .filter(|file| snippet_paths.contains(file.path.as_str()))
        .count();
    let top_symbols_with_snippets = top_symbols
        .iter()
        .filter(|symbol| symbol_has_snippet(index, symbol.id.as_str()))
        .count();

    ContextQualityCoverage {
        top_files_considered: top_files.len(),
        top_files_with_snippets,
        top_symbols_considered: top_symbols.len(),
        top_symbols_with_snippets,
        snippet_count: index.evidence.snippets.len(),
        relationship_count: index.graph.relationships.len(),
        repo_map_tag_edges: index.graph.repo_map_tags.edges.len(),
        impact_fact_count: count_impact_facts(index),
        validation_commands: validation_plan.commands.len(),
    }
}

fn build_semantic(index: &RepoContextIndex) -> ContextQualitySemantic {
    let mut probe_succeeded = 0usize;
    let mut probe_failed = 0usize;
    let mut probe_timed_out = 0usize;
    for command in &index.rust_analyzer.probes.commands {
        match command.status {
            RustAnalyzerProbeStatus::Succeeded => probe_succeeded += 1,
            RustAnalyzerProbeStatus::Failed | RustAnalyzerProbeStatus::Skipped => {
                probe_failed += 1;
            }
            RustAnalyzerProbeStatus::TimedOut => probe_timed_out += 1,
        }
    }
    let mut lsp_succeeded = 0usize;
    let mut lsp_failed = 0usize;
    let mut lsp_timed_out = 0usize;
    for result in &index.rust_analyzer.lsp.results {
        match result.status {
            RustAnalyzerLspStatus::Succeeded => lsp_succeeded += 1,
            RustAnalyzerLspStatus::Failed => lsp_failed += 1,
            RustAnalyzerLspStatus::TimedOut => lsp_timed_out += 1,
            RustAnalyzerLspStatus::Skipped => {}
        }
    }

    ContextQualitySemantic {
        rust_analyzer_available: index.rust_analyzer.available,
        rust_analyzer_symbols: index.rust_analyzer.symbols.len(),
        rust_analyzer_files_enhanced: index.rust_analyzer.files_enhanced,
        lsp_queries_planned: index.semantic_plan.queries.len(),
        lsp_enabled: index.rust_analyzer.lsp.enabled,
        lsp_attempted: index.rust_analyzer.lsp.attempted,
        lsp_succeeded,
        lsp_failed,
        lsp_timed_out,
        probe_enabled: index.rust_analyzer.probes.enabled,
        probe_succeeded,
        probe_failed,
        probe_timed_out,
    }
}

fn collect_gaps(
    index: &RepoContextIndex,
    relevant_files: &[RelevantFile],
    coverage: &ContextQualityCoverage,
    semantic: &ContextQualitySemantic,
    validation_plan: &ValidationPlan,
) -> Vec<ContextQualityGap> {
    let mut gaps = Vec::new();

    if coverage.snippet_count == 0 && !index.graph.ranked_symbols.is_empty() {
        gaps.push(gap(
            ContextQualitySeverity::Critical,
            "context_evidence",
            None,
            None,
            "no source snippets were selected for ranked symbols",
            "open the top ranked files before editing or increase evidence budget",
        ));
    }

    for symbol in index.graph.ranked_symbols.iter().take(8) {
        if symbol_has_snippet(index, symbol.id.as_str()) {
            continue;
        }
        gaps.push(gap(
            ContextQualitySeverity::Warning,
            format!("symbol {}", symbol.name),
            Some(symbol.path.clone()),
            Some(symbol.line_start),
            "top ranked symbol has no exact evidence snippet",
            "read the real file around this symbol before changing its API",
        ));
    }

    let snippet_paths = index
        .evidence
        .snippets
        .iter()
        .map(|snippet| snippet.path.as_str())
        .collect::<HashSet<_>>();
    for file in index.graph.ranked_files.iter().take(5) {
        if snippet_paths.contains(file.path.as_str()) {
            continue;
        }
        gaps.push(gap(
            ContextQualitySeverity::Info,
            "ranked_file",
            Some(file.path.clone()),
            None,
            "top ranked file is represented in the repo map but not as a source snippet",
            "open this file if the task touches its symbols",
        ));
    }

    if !semantic.rust_analyzer_available {
        gaps.push(gap(
            ContextQualitySeverity::Warning,
            "rust_analyzer",
            None,
            None,
            "rust-analyzer is unavailable; semantic facts are based on Rust-native fallback indexing",
            "install or expose rust-analyzer for hover/references/implementation confirmation",
        ));
    }
    if semantic.lsp_queries_planned == 0 && !index.graph.ranked_symbols.is_empty() {
        gaps.push(gap(
            ContextQualitySeverity::Warning,
            "semantic_query_plan",
            None,
            None,
            "no Top-K rust-analyzer LSP queries were planned",
            "inspect semantic plan generation before relying on semantic coverage",
        ));
    }
    if semantic.lsp_enabled && semantic.lsp_queries_planned > 0 && semantic.lsp_attempted == 0 {
        gaps.push(gap(
            ContextQualitySeverity::Warning,
            "rust_analyzer_lsp",
            None,
            None,
            "semantic query plan existed but no executable rust-analyzer LSP request was attempted",
            "inspect rust_analyzer_lsp warnings and confirm the Top-K source files are in the Cargo workspace",
        ));
    }
    if semantic.lsp_enabled && (semantic.lsp_failed > 0 || semantic.lsp_timed_out > 0) {
        gaps.push(gap(
            ContextQualitySeverity::Warning,
            "rust_analyzer_lsp",
            None,
            None,
            "one or more Top-K rust-analyzer LSP requests failed or timed out",
            "use successful LSP results plus repo_map_tags fallback before changing public APIs",
        ));
    }
    if semantic.lsp_enabled && semantic.lsp_attempted > 0 && semantic.lsp_succeeded == 0 {
        gaps.push(gap(
            ContextQualitySeverity::Warning,
            "rust_analyzer_lsp",
            None,
            None,
            "Top-K rust-analyzer LSP execution produced no successful semantic result",
            "check rust-analyzer availability, workspace root, and source URI conversion",
        ));
    }
    if semantic.probe_enabled && (semantic.probe_failed > 0 || semantic.probe_timed_out > 0) {
        gaps.push(gap(
            ContextQualitySeverity::Warning,
            "rust_analyzer_probe",
            None,
            None,
            "one or more rust-analyzer probe commands failed or timed out",
            "use probe stderr excerpts and fallback repo_map_tags before editing",
        ));
    }
    if validation_plan.commands.is_empty() {
        gaps.push(gap(
            ContextQualitySeverity::Warning,
            "validation_plan",
            None,
            None,
            "no validation commands were identified for this task",
            "derive the smallest cargo check/test command before changing code",
        ));
    }
    if relevant_files.is_empty() && index.graph.ranked_files.is_empty() {
        gaps.push(gap(
            ContextQualitySeverity::Info,
            "retrieval",
            None,
            None,
            "no relevant files were found by retrieval or repo-map ranking",
            "broaden task keywords or inspect the repository tree manually",
        ));
    }
    for missing in &index.evidence.missing_context {
        gaps.push(gap(
            ContextQualitySeverity::Info,
            "missing_context",
            None,
            None,
            missing,
            "collect the missing context before making broad edits",
        ));
    }

    gaps
}

fn score_quality(
    coverage: &ContextQualityCoverage,
    semantic: &ContextQualitySemantic,
    gaps: &[ContextQualityGap],
) -> u8 {
    let mut score = 100i32;
    if coverage.snippet_count == 0 {
        score -= 30;
    }
    score -= coverage_penalty(
        coverage.top_symbols_with_snippets,
        coverage.top_symbols_considered,
        18,
    );
    score -= coverage_penalty(
        coverage.top_files_with_snippets,
        coverage.top_files_considered,
        12,
    );
    if !semantic.rust_analyzer_available {
        score -= 12;
    }
    if semantic.lsp_queries_planned == 0 {
        score -= 8;
    }
    if semantic.lsp_enabled && semantic.lsp_queries_planned > 0 && semantic.lsp_succeeded == 0 {
        score -= 6;
    }
    if semantic.lsp_enabled {
        score -= ((semantic.lsp_failed + semantic.lsp_timed_out) as i32 * 2).min(8);
    }
    if coverage.validation_commands == 0 {
        score -= 8;
    }
    score -= gaps
        .iter()
        .map(|gap| match gap.severity {
            ContextQualitySeverity::Critical => 10,
            ContextQualitySeverity::Warning => 4,
            ContextQualitySeverity::Info => 1,
        })
        .sum::<i32>()
        .min(20);
    score.clamp(0, 100) as u8
}

fn coverage_penalty(covered: usize, total: usize, max_penalty: i32) -> i32 {
    if total == 0 {
        return 0;
    }
    let missing = total.saturating_sub(covered);
    ((missing as f64 / total as f64) * f64::from(max_penalty)).round() as i32
}

fn recommended_actions(
    index: &RepoContextIndex,
    coverage: &ContextQualityCoverage,
    semantic: &ContextQualitySemantic,
    gaps: &[ContextQualityGap],
) -> Vec<String> {
    let mut actions = Vec::new();
    if coverage.top_symbols_with_snippets < coverage.top_symbols_considered.min(10) {
        actions.push(
            "read uncovered top ranked symbols from real files before applying edits".to_string(),
        );
    }
    if semantic.lsp_queries_planned > 0 && !semantic.lsp_enabled {
        actions.push(
            "enable ELON_CONTEXT_COMPILER_RA_LSP=true to execute the semantic_query_plan Top-K rust-analyzer LSP requests"
                .to_string(),
        );
    }
    if semantic.lsp_enabled && semantic.lsp_succeeded > 0 {
        actions.push(
            "prefer successful rust_analyzer_lsp reference/implementation facts when estimating edit blast radius"
                .to_string(),
        );
    }
    if semantic.lsp_enabled && (semantic.lsp_failed > 0 || semantic.lsp_timed_out > 0) {
        actions.push(
            "inspect rust_analyzer_lsp warnings before trusting missing references or implementations"
                .to_string(),
        );
    }
    if !semantic.rust_analyzer_available {
        actions.push("treat repo_map_tags and impact_analysis as fallback evidence until rust-analyzer is available".to_string());
    }
    if coverage.validation_commands > 0 {
        actions.push("run the listed validation commands after edits".to_string());
    }
    if index.graph.repo_map_tags.edges.is_empty() && !index.rust.symbols.is_empty() {
        actions.push("inspect why def/ref tag edges are empty for this Rust workspace".to_string());
    }
    for gap in gaps.iter().take(3) {
        actions.push(gap.action.clone());
    }
    dedupe(actions)
}

fn symbol_has_snippet(index: &RepoContextIndex, symbol_id: &str) -> bool {
    index
        .evidence
        .snippets
        .iter()
        .any(|snippet| snippet.symbols.iter().any(|symbol| symbol == symbol_id))
}

fn gap(
    severity: ContextQualitySeverity,
    subject: impl Into<String>,
    path: Option<String>,
    line: Option<usize>,
    detail: impl Into<String>,
    action: impl Into<String>,
) -> ContextQualityGap {
    ContextQualityGap {
        severity,
        subject: subject.into(),
        path,
        line,
        detail: detail.into(),
        action: action.into(),
    }
}

fn count_impact_facts(index: &RepoContextIndex) -> usize {
    index.impact.trait_implementations.len()
        + index.impact.function_call_sites.len()
        + index.impact.enum_match_sites.len()
        + index.impact.field_accesses.len()
        + index.impact.public_api_references.len()
        + index.impact.test_links.len()
        + index.impact.async_boundaries.len()
}

fn dedupe(values: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    values
        .into_iter()
        .filter(|value| seen.insert(value.clone()))
        .collect()
}
