use std::{
    fs,
    path::{Path, PathBuf},
};

use chrono::{DateTime, Utc};
use serde_json::{json, Value};

use super::{
    model::RepoContextIndex, relevance::RelevantFile, repo_snapshot::RepoSnapshot,
    validation::ValidationPlan,
};

pub(crate) struct TaskContextExportsInput<'a> {
    pub(crate) bundle_dir: &'a Path,
    pub(crate) created_at: &'a DateTime<Utc>,
    pub(crate) trace_id: Option<&'a str>,
    pub(crate) user_id: &'a str,
    pub(crate) user_message: &'a str,
    pub(crate) pack: &'a str,
    pub(crate) snapshot: &'a RepoSnapshot,
    pub(crate) repo_index: Option<&'a RepoContextIndex>,
    pub(crate) relevant_files: &'a [RelevantFile],
    pub(crate) validation_plan: &'a ValidationPlan,
}

pub(crate) fn write_task_context_exports(
    input: TaskContextExportsInput<'_>,
    files: &mut Vec<PathBuf>,
) -> Option<usize> {
    let mut bytes = 0usize;
    bytes += write_text(
        &input.bundle_dir.join("task_context_pack.md"),
        input.pack,
        files,
    )?;

    let current_task_dir = input.bundle_dir.join(".ai").join("context");
    fs::create_dir_all(&current_task_dir).ok()?;
    bytes += write_text(&current_task_dir.join("current-task.md"), input.pack, files)?;
    bytes += write_text(
        &current_task_dir.join("current-task.json"),
        &build_current_task_json(&input)?,
        files,
    )?;
    Some(bytes)
}

fn build_current_task_json(input: &TaskContextExportsInput<'_>) -> Option<String> {
    serde_json::to_string_pretty(&json!({
        "version": 1,
        "source": "elon-context-compiler",
        "format": "task_context_pack.v1",
        "generated_at": input.created_at.to_rfc3339(),
        "trace_id": input.trace_id,
        "user_id": input.user_id,
        "user_request": input.user_message.trim(),
        "context_files": {
            "model_input": "task_context_pack.md",
            "harness_markdown": ".ai/context/current-task.md",
            "harness_json": ".ai/context/current-task.json",
            "repo_index": "repo_context_index.json",
            "repo_map": "repo_map.md",
            "symbols": "symbols.jsonl",
            "edges": "edges.tsv",
            "chunks": "chunks.jsonl",
            "lsp_locations": "lsp_locations.jsonl"
        },
        "contract": {
            "model_input": "XML-wrapped Markdown with fenced source snippets",
            "tool_input": "JSON/JSONL/TSV sidecar files",
            "ground_truth_rule": "Read real source files before editing; summaries are navigation aids only."
        },
        "git": {
            "head": input.snapshot.git_head.as_deref(),
            "branch": input.snapshot.git_branch.as_deref(),
            "dirty": input.snapshot.git_dirty,
            "status_short": &input.snapshot.git_status_short
        },
        "task_profile": task_profile_json(input.repo_index),
        "analysis_summary": analysis_summary_json(input.repo_index),
        "relevant_files": input.relevant_files,
        "validation_commands": &input.validation_plan.commands,
        "validation_notes": &input.validation_plan.notes,
        "missing_context": missing_context(input.repo_index),
        "recommended_actions": recommended_actions(input.repo_index),
        "pack_chars": input.pack.chars().count()
    }))
    .ok()
}

fn task_profile_json(repo_index: Option<&RepoContextIndex>) -> Option<Value> {
    let index = repo_index?;
    Some(json!({
        "keywords": &index.task.keywords,
        "likely_domains": &index.task.likely_domains,
        "suspected_symbols": &index.task.suspected_symbols,
        "suspected_files": &index.task.suspected_files,
        "action_hints": &index.task.action_hints
    }))
}

fn analysis_summary_json(repo_index: Option<&RepoContextIndex>) -> Option<Value> {
    let index = repo_index?;
    Some(json!({
        "cargo_packages": index.cargo.packages.len(),
        "rust_files_scanned": index.rust.files_scanned,
        "rust_symbols": index.rust.symbols.len(),
        "ranked_files": index.graph.ranked_files.len(),
        "ranked_symbols": index.graph.ranked_symbols.len(),
        "relationships": index.graph.relationships.len(),
        "repo_map_tag_edges": index.graph.repo_map_tags.edges.len(),
        "semantic_queries": index.semantic_plan.queries.len(),
        "evidence_snippets": index.evidence.snippets.len(),
        "quality_score": index.quality.score,
        "quality_gaps": index.quality.gaps.len(),
        "rust_analyzer": {
            "available": index.rust_analyzer.available,
            "files_enhanced": index.rust_analyzer.files_enhanced,
            "probe_enabled": index.rust_analyzer.probes.enabled,
            "lsp_enabled": index.rust_analyzer.lsp.enabled,
            "lsp_attempted": index.rust_analyzer.lsp.attempted,
            "lsp_succeeded": index.rust_analyzer.lsp.succeeded,
            "lsp_locations": index
                .rust_analyzer
                .lsp
                .results
                .iter()
                .map(|result| result.locations.len())
                .sum::<usize>()
        }
    }))
}

fn missing_context(repo_index: Option<&RepoContextIndex>) -> Vec<String> {
    repo_index
        .map(|index| index.evidence.missing_context.clone())
        .unwrap_or_default()
}

fn recommended_actions(repo_index: Option<&RepoContextIndex>) -> Vec<String> {
    let Some(index) = repo_index else {
        return Vec::new();
    };
    let mut actions = index.quality.recommended_actions.clone();
    actions.extend(index.evidence.recommended_actions.iter().cloned());
    actions
}

fn write_text(path: &Path, content: &str, files: &mut Vec<PathBuf>) -> Option<usize> {
    fs::write(path, content.as_bytes()).ok()?;
    files.push(path.to_path_buf());
    Some(content.len())
}
