mod artifact;
mod artifact_exports;
#[cfg(test)]
mod artifact_exports_tests;
mod cargo_index;
mod config;
mod context_budget;
#[cfg(test)]
mod context_budget_tests;
mod context_evidence;
#[cfg(test)]
mod context_evidence_tests;
mod context_pack;
mod context_pack_render;
mod context_quality;
mod context_quality_render;
#[cfg(test)]
mod context_quality_tests;
mod directory_summary;
mod hunyuan_brief;
mod impact_analysis;
#[cfg(test)]
mod impact_analysis_tests;
mod impact_render;
mod model;
mod project_manifests;
mod relevance;
mod repo_map_tags;
mod repo_map_tags_render;
#[cfg(test)]
mod repo_map_tags_tests;
mod repo_snapshot;
mod repo_walk;
mod rust_analyzer;
mod rust_analyzer_lsp;
mod rust_analyzer_lsp_locations;
mod rust_analyzer_lsp_protocol;
mod rust_analyzer_lsp_render;
#[cfg(test)]
mod rust_analyzer_lsp_tests;
mod rust_analyzer_probe;
mod rust_analyzer_probe_render;
#[cfg(test)]
mod rust_analyzer_probe_tests;
mod rust_imports;
mod rust_project;
mod rust_symbols;
mod semantic_query_plan;
mod semantic_query_plan_render;
#[cfg(test)]
mod semantic_query_plan_tests;
mod symbol_graph;
mod symbol_index;
pub(crate) mod symbol_index_api;
mod symbol_index_build;
mod symbol_index_chunk_types;
mod symbol_index_chunks;
mod symbol_index_compression;
mod symbol_index_compression_render;
mod symbol_index_compression_types;
mod symbol_index_embedding_types;
mod symbol_index_embeddings;
mod symbol_index_eval;
mod symbol_index_eval_compare;
pub(crate) mod symbol_index_eval_compare_api;
#[cfg(test)]
mod symbol_index_eval_compare_tests;
mod symbol_index_eval_runs;
mod symbol_index_eval_types;
mod symbol_index_graph_query;
mod symbol_index_impact_edges;
mod symbol_index_impact_pack;
mod symbol_index_impact_query;
mod symbol_index_impact_types;
pub(crate) mod symbol_index_patch_api;
mod symbol_index_patch_apply;
mod symbol_index_patch_apply_git;
mod symbol_index_patch_apply_policy;
mod symbol_index_patch_apply_rollback;
#[cfg(test)]
mod symbol_index_patch_apply_tests;
mod symbol_index_patch_apply_types;
mod symbol_index_patch_check;
mod symbol_index_patch_dry_run;
#[cfg(test)]
mod symbol_index_patch_dry_run_tests;
mod symbol_index_patch_generation;
mod symbol_index_patch_generation_render;
mod symbol_index_patch_generation_types;
mod symbol_index_patch_plan;
mod symbol_index_patch_plan_guidance;
mod symbol_index_patch_plan_render;
mod symbol_index_patch_plan_rules;
mod symbol_index_patch_plan_types;
mod symbol_index_patch_repair;
mod symbol_index_patch_repair_attempt;
mod symbol_index_patch_repair_generate;
#[cfg(test)]
mod symbol_index_patch_repair_generate_tests;
mod symbol_index_patch_review;
mod symbol_index_patch_review_analysis;
mod symbol_index_patch_review_findings;
mod symbol_index_patch_review_render;
#[cfg(test)]
mod symbol_index_patch_review_tests;
mod symbol_index_patch_review_types;
mod symbol_index_patch_verification;
mod symbol_index_patch_verification_repair;
mod symbol_index_patch_verification_run;
mod symbol_index_patch_verification_run_types;
mod symbol_index_query;
mod symbol_index_query_features;
#[cfg(test)]
mod symbol_index_query_tests;
mod symbol_index_query_types;
mod symbol_index_rank_profile;
mod symbol_index_ranker;
mod symbol_index_retrieval_plan;
mod symbol_index_semantic;
mod symbol_index_store;
mod symbol_index_task_pack;
#[cfg(test)]
mod symbol_index_tests;
mod symbol_index_vector;
mod symbol_index_vector_types;
mod task_context_exports;
#[cfg(test)]
mod task_context_exports_tests;
mod task_profile;
mod validation;

use std::{path::Path, sync::Arc};

use crate::types::AppState;

use self::config::{ContextCompilerConfig, ContextCompilerMode};

pub(crate) async fn compile_preflight_note(
    state: &Arc<AppState>,
    workspace: &Path,
    user_id: &str,
    user_message: &str,
    trace_id: Option<&str>,
) -> Option<String> {
    let config = ContextCompilerConfig::from_env();
    if !config.enabled {
        return None;
    }

    let snapshot = repo_snapshot::collect_repo_snapshot(workspace);
    let task = task_profile::analyze_task(user_message);
    let project_manifests = project_manifests::collect_project_manifest_report(workspace);
    let directory_summaries = directory_summary::collect_directory_summaries(workspace);
    let rust_project = config
        .rust_probe_enabled
        .then(|| rust_project::collect_rust_project_summary(workspace))
        .flatten();
    let relevant_files =
        relevance::find_relevant_files(workspace, user_message, config.max_relevant_files);
    let mut repo_index = if config.rust_analysis_enabled {
        let cargo = cargo_index::collect_cargo_index(workspace);
        let rust = rust_symbols::collect_rust_index(workspace, config.max_rust_files);
        let graph = symbol_graph::build_symbol_graph(
            workspace,
            &rust,
            user_message,
            config.max_symbols,
            config.max_relationships,
        );
        let rust_analyzer = rust_analyzer::collect_rust_analyzer_report(
            workspace,
            &rust,
            &graph,
            config.rust_analyzer_enabled,
            config.max_rust_analyzer_files,
            config.rust_analyzer_probe_enabled,
            config.rust_analyzer_probe_timeout_ms,
        );
        let mut index = model::RepoContextIndex {
            task,
            cargo,
            rust,
            graph,
            rust_analyzer,
            semantic_plan: model::SemanticQueryPlan::default(),
            impact: model::RustImpactAnalysis::default(),
            evidence: model::ContextEvidence::default(),
            quality: model::ContextQualityReport::default(),
        };
        index.semantic_plan = semantic_query_plan::build_semantic_query_plan(
            &index,
            config.max_rust_analyzer_files,
            config.max_symbols,
        );
        index.rust_analyzer.lsp = rust_analyzer_lsp::execute_semantic_query_plan(
            workspace,
            &index,
            config.rust_analyzer_lsp_enabled,
            config.rust_analyzer_lsp_timeout_ms,
            config.rust_analyzer_lsp_max_queries,
        );
        index.impact = impact_analysis::build_rust_impact_analysis(workspace, &index);
        index.evidence =
            context_evidence::build_context_evidence(workspace, &index, &relevant_files);
        Some(index)
    } else {
        None
    };
    let validation_plan =
        validation::build_validation_plan(&snapshot, rust_project.as_ref(), &relevant_files);
    if let Some(index) = repo_index.as_mut() {
        index.quality =
            context_quality::build_context_quality_report(index, &relevant_files, &validation_plan);
    }
    let deterministic_pack = context_pack::build_context_pack(
        &config,
        user_message,
        &snapshot,
        rust_project.as_ref(),
        Some(&project_manifests),
        &directory_summaries,
        repo_index.as_ref(),
        &relevant_files,
        &validation_plan,
        None,
    );
    let llm_brief =
        hunyuan_brief::build_llm_brief(state, &config, user_id, user_message, &deterministic_pack)
            .await;
    let final_pack = context_pack::build_context_pack(
        &config,
        user_message,
        &snapshot,
        rust_project.as_ref(),
        Some(&project_manifests),
        &directory_summaries,
        repo_index.as_ref(),
        &relevant_files,
        &validation_plan,
        llm_brief.as_deref(),
    );
    let artifact = artifact::save_context_artifacts(artifact::ContextArtifactsInput {
        data_dir: &state.data_dir,
        config: &config,
        trace_id,
        user_id,
        user_message,
        pack: &final_pack,
        llm_brief: llm_brief.as_deref(),
        snapshot: &snapshot,
        rust_project: rust_project.as_ref(),
        project_manifests: Some(&project_manifests),
        directory_summaries: &directory_summaries,
        repo_index: repo_index.as_ref(),
        relevant_files: &relevant_files,
        validation_plan: &validation_plan,
    });

    if let Some(trace_id) = trace_id {
        state.server_traces.record(
            trace_id,
            "server_context_compiler_done",
            serde_json::json!({
                "mode": config.mode.as_str(),
                "injected": config.mode == ContextCompilerMode::Inject,
                "agent": config.agent_name,
                "llm_brief": llm_brief.is_some(),
                "relevant_files": relevant_files.len(),
                "project_manifests": project_manifests.manifests.len(),
                "readmes": project_manifests.readmes.len(),
                "directory_summaries": directory_summaries.len(),
                "rust_project": rust_project.is_some(),
                "rust_analysis": repo_index.as_ref().map(|index| serde_json::json!({
                    "cargo_packages": index.cargo.packages.len(),
                    "rust_files": index.rust.files_scanned,
                    "rust_symbols": index.rust.symbols.len(),
                    "relationships": index.graph.relationships.len(),
                    "ra_available": index.rust_analyzer.available,
                    "ra_files": index.rust_analyzer.files_enhanced,
                    "ra_probe_enabled": index.rust_analyzer.probes.enabled,
                    "ra_probe_findings": count_ra_probe_findings(&index.rust_analyzer.probes),
                    "ra_lsp_enabled": index.rust_analyzer.lsp.enabled,
                    "ra_lsp_attempted": index.rust_analyzer.lsp.attempted,
                    "ra_lsp_succeeded": index.rust_analyzer.lsp.succeeded,
                    "semantic_queries": index.semantic_plan.queries.len(),
                    "context_quality_score": index.quality.score,
                    "context_quality_gaps": index.quality.gaps.len(),
                    "snippets": index.evidence.snippets.len(),
                    "test_targets": index.evidence.test_targets.len(),
                    "build_commands": index.evidence.build_commands.len(),
                    "impact_facts": count_impact_facts(&index.impact),
                })),
                "pack_chars": final_pack.chars().count(),
                "artifact_path": artifact.as_ref().map(|item| item.path.display().to_string()),
                "artifact_bundle_dir": artifact.as_ref().map(|item| item.bundle_dir.display().to_string()),
                "artifact_file_count": artifact.as_ref().map(|item| item.files.len()),
                "artifact_bytes": artifact.as_ref().map(|item| item.bytes),
                "validation_commands": validation_plan.commands.len(),
            }),
        );
    }
    tracing::info!(
        mode = config.mode.as_str(),
        llm_brief = llm_brief.is_some(),
        relevant_files = relevant_files.len(),
        project_manifests = project_manifests.manifests.len(),
        readmes = project_manifests.readmes.len(),
        directory_summaries = directory_summaries.len(),
        rust_project = rust_project.is_some(),
        rust_symbols = repo_index
            .as_ref()
            .map(|index| index.rust.symbols.len())
            .unwrap_or_default(),
        rust_analyzer = repo_index
            .as_ref()
            .map(|index| index.rust_analyzer.available)
            .unwrap_or_default(),
        snippets = repo_index
            .as_ref()
            .map(|index| index.evidence.snippets.len())
            .unwrap_or_default(),
        impact_facts = repo_index
            .as_ref()
            .map(|index| count_impact_facts(&index.impact))
            .unwrap_or_default(),
        semantic_queries = repo_index
            .as_ref()
            .map(|index| index.semantic_plan.queries.len())
            .unwrap_or_default(),
        ra_lsp_enabled = repo_index
            .as_ref()
            .map(|index| index.rust_analyzer.lsp.enabled)
            .unwrap_or_default(),
        ra_lsp_succeeded = repo_index
            .as_ref()
            .map(|index| index.rust_analyzer.lsp.succeeded)
            .unwrap_or_default(),
        context_quality_score = repo_index
            .as_ref()
            .map(|index| index.quality.score)
            .unwrap_or_default(),
        artifact_path = artifact
            .as_ref()
            .map(|item| item.path.display().to_string())
            .unwrap_or_default(),
        artifact_bundle_dir = artifact
            .as_ref()
            .map(|item| item.bundle_dir.display().to_string())
            .unwrap_or_default(),
        validation_commands = validation_plan.commands.len(),
        "context compiler completed"
    );

    if config.mode == ContextCompilerMode::Inject {
        Some(final_pack)
    } else {
        None
    }
}

fn count_impact_facts(impact: &model::RustImpactAnalysis) -> usize {
    impact.trait_implementations.len()
        + impact.function_call_sites.len()
        + impact.enum_match_sites.len()
        + impact.field_accesses.len()
        + impact.public_api_references.len()
        + impact.test_links.len()
        + impact.async_boundaries.len()
}

fn count_ra_probe_findings(probes: &model::RustAnalyzerProbeReport) -> usize {
    probes
        .commands
        .iter()
        .map(|command| command.findings.len())
        .sum()
}
