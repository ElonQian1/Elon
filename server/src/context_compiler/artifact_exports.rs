use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
};

use serde_json::json;

use super::{
    directory_summary::DirectorySummary,
    model::{RepoContextIndex, RustAnalyzerLspStatus, SemanticQueryMethod},
    project_manifests::ProjectManifestReport,
    symbol_index::SymbolIndex,
    symbol_index_build::build_symbol_index,
    symbol_index_store::{write_symbol_index_sqlite, SYMBOL_INDEX_DB_FILE},
    validation::ValidationPlan,
};

const MAX_SYMBOLS_JSONL: usize = 2_000;
const MAX_EDGES_TSV: usize = 2_000;
const MAX_CHUNKS_JSONL: usize = 120;
const MAX_LSP_LOCATIONS_JSONL: usize = 500;
const MAX_SEMANTIC_FACTS_JSONL: usize = 500;
const MAX_TESTS_JSONL: usize = 200;
const MAX_DIRECTORIES_JSONL: usize = 200;
const MAX_SYMBOL_INDEX_JSONL: usize = 2_000;
const MAX_SYMBOL_EDGES_JSONL: usize = 2_000;

pub(crate) fn write_context_exports(
    bundle_dir: &Path,
    repo_index: Option<&RepoContextIndex>,
    project_manifests: Option<&ProjectManifestReport>,
    directory_summaries: &[DirectorySummary],
    validation_plan: &ValidationPlan,
    files: &mut Vec<PathBuf>,
) -> Option<usize> {
    let mut bytes = 0usize;
    if let Some(project_manifests) = project_manifests {
        bytes += write_export_text(
            &bundle_dir.join("project_manifests.md"),
            &build_project_manifests_markdown(project_manifests),
            files,
        )?;
    }
    if !directory_summaries.is_empty() {
        bytes += write_export_text(
            &bundle_dir.join("directory_summaries.md"),
            &build_directory_summaries_markdown(directory_summaries),
            files,
        )?;
        bytes += write_export_text(
            &bundle_dir.join("directories.jsonl"),
            &build_directories_jsonl(directory_summaries),
            files,
        )?;
    }
    let Some(repo_index) = repo_index else {
        return Some(bytes);
    };
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
    let symbol_index = build_symbol_index(repo_index);
    bytes += write_export_text(
        &bundle_dir.join("symbol_index.jsonl"),
        &build_symbol_index_jsonl(&symbol_index),
        files,
    )?;
    bytes += write_export_text(
        &bundle_dir.join("symbol_edges.jsonl"),
        &build_symbol_edges_jsonl(&symbol_index),
        files,
    )?;
    bytes += write_export_text(
        &bundle_dir.join("symbol_lookup.json"),
        &build_symbol_lookup_json(&symbol_index),
        files,
    )?;
    bytes +=
        write_symbol_index_sqlite(&bundle_dir.join(SYMBOL_INDEX_DB_FILE), &symbol_index, files)?;
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


#[path = "artifact_exports_builders.rs"]
mod builders;
use self::builders::*;
