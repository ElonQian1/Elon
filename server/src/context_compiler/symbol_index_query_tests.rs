use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use rusqlite::{params, Connection};

use super::{
    symbol_index::{SymbolEdge, SymbolIndex, SymbolRecord},
    symbol_index_chunks::{search_symbol_chunks_db, SymbolChunkSearch},
    symbol_index_embeddings::{load_symbol_embedding_status_db, SymbolEmbeddingStatus},
    symbol_index_eval::{evaluate_latest_symbol_retrieval, RetrievalEvalQuery},
    symbol_index_eval_runs::{
        evaluate_latest_symbol_retrieval_batch, list_latest_retrieval_runs,
        load_latest_retrieval_run,
    },
    symbol_index_eval_types::{
        SymbolRetrievalEvalBatchCaseQuery, SymbolRetrievalEvalBatchQuery,
        SymbolRetrievalRunHistoryQuery, SymbolRetrievalRunLookupQuery,
    },
    symbol_index_graph_query::{load_symbol_graph_db, SymbolGraphQuery, SymbolRelationDirection},
    symbol_index_impact_pack::{build_symbol_impact_pack, normalize_pack_max_chars},
    symbol_index_impact_query::load_symbol_impact_db,
    symbol_index_impact_types::SymbolImpactQuery,
    symbol_index_patch_check::{check_symbol_patch_diff, PatchDiffCheckStatus},
    symbol_index_patch_generation_types::{PatchApplyReadinessLevel, PatchGenerationMode},
    symbol_index_query::{find_symbol_index_db, search_symbol_index_db, SymbolIndexSearch},
    symbol_index_retrieval_plan::QueryIntent,
    symbol_index_store::{write_symbol_index_sqlite, SYMBOL_INDEX_DB_FILE},
    symbol_index_task_pack::{build_latest_symbol_task_pack, SymbolTaskPackQuery},
    symbol_index_vector::{
        backfill_symbol_vectors_db, search_symbol_vectors_db, SymbolVectorBackfill,
        SymbolVectorSearchQuery,
    },
    symbol_index_vector_types::LOCAL_HASH_VECTOR_MODEL,
};

#[test]
fn searches_sqlite_symbol_index_by_text_and_returns_edges() {
    let dir = temp_dir("elon_symbol_query_text");
    let db = write_bundle(&dir, "20260614", "213000-trace-alpha-user", sample_index());

    let response = search_symbol_index_db(
        &db,
        &SymbolIndexSearch {
            text: Some("compile preflight".to_string()),
            include_edges: true,
            limit: 5,
            ..Default::default()
        },
    )
    .unwrap();

    assert_eq!(
        response.symbols[0].id,
        "server/src/context_compiler/mod.rs::compile_preflight_note"
    );
    assert!(response.symbols[0]
        .matched_terms
        .iter()
        .any(|term| term == "compile"));
    assert!(response
        .edges
        .iter()
        .any(|edge| edge.kind == "calls"
            && edge.to_symbol_name.as_deref() == Some("build_context_pack")));

    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn filters_symbols_by_kind_and_path() {
    let dir = temp_dir("elon_symbol_query_filter");
    let db = write_bundle(&dir, "20260614", "213001-trace-filter-user", sample_index());

    let response = search_symbol_index_db(
        &db,
        &SymbolIndexSearch {
            kind: Some("struct".to_string()),
            path: Some("context_pack.rs".to_string()),
            limit: 10,
            ..Default::default()
        },
    )
    .unwrap();

    assert_eq!(response.symbols.len(), 1);
    assert_eq!(response.symbols[0].name, "ContextPackArtifact");

    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn symbol_search_ignores_importance_only_matches_when_query_has_terms() {
    let dir = temp_dir("elon_symbol_query_no_weak_match");
    let db = write_bundle(
        &dir,
        "20260614",
        "213001-trace-no-weak-match-user",
        sample_index(),
    );

    let response = search_symbol_index_db(
        &db,
        &SymbolIndexSearch {
            text: Some("symbol_count".to_string()),
            path: Some("context_pack.rs".to_string()),
            limit: 10,
            ..Default::default()
        },
    )
    .unwrap();

    assert!(response.symbols.is_empty());

    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn finds_latest_or_trace_specific_symbol_index_db() {
    let dir = temp_dir("elon_symbol_query_latest");
    let first = write_bundle(&dir, "20260614", "213002-trace-one-user", sample_index());
    let second = write_bundle(&dir, "20260614", "213003-trace-two-user", sample_index());

    assert_eq!(
        find_symbol_index_db(&dir, Some("trace-one")).unwrap(),
        first
    );
    assert_eq!(
        find_symbol_index_db(&dir, Some("trace-two")).unwrap(),
        second
    );
    assert_eq!(find_symbol_index_db(&dir, None).unwrap(), second);

    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn edge_kind_limits_returned_relations() {
    let dir = temp_dir("elon_symbol_query_edges");
    let db = write_bundle(&dir, "20260614", "213004-trace-edges-user", sample_index());

    let response = search_symbol_index_db(
        &db,
        &SymbolIndexSearch {
            text: Some("compile_preflight_note".to_string()),
            edge_kind: Some("references".to_string()),
            include_edges: true,
            limit: 5,
            ..Default::default()
        },
    )
    .unwrap();

    assert!(response.edges.iter().all(|edge| edge.kind == "references"));
    assert_eq!(response.edges.len(), 1);

    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn loads_symbol_graph_with_related_symbols() {
    let dir = temp_dir("elon_symbol_graph");
    let db = write_bundle(&dir, "20260614", "213005-trace-graph-user", sample_index());

    let response = load_symbol_graph_db(
        &db,
        &SymbolGraphQuery {
            trace_id: None,
            symbol_id: "server/src/context_compiler/mod.rs::compile_preflight_note".to_string(),
            edge_kind: None,
            direction: SymbolRelationDirection::Both,
            limit: 20,
        },
    )
    .unwrap();

    assert_eq!(response.symbol.name, "compile_preflight_note");
    assert_eq!(response.edges.len(), 2);
    assert!(response
        .related_symbols
        .iter()
        .any(|symbol| symbol.name == "build_context_pack"));

    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn graph_query_filters_direction_and_edge_kind() {
    let dir = temp_dir("elon_symbol_graph_direction");
    let db = write_bundle(&dir, "20260614", "213006-trace-graph-user", sample_index());

    let response = load_symbol_graph_db(
        &db,
        &SymbolGraphQuery {
            trace_id: None,
            symbol_id: "server/src/context_compiler/mod.rs::compile_preflight_note".to_string(),
            edge_kind: Some("references".to_string()),
            direction: SymbolRelationDirection::Incoming,
            limit: 20,
        },
    )
    .unwrap();

    assert_eq!(response.edges.len(), 1);
    assert_eq!(response.edges[0].kind, "references");
    assert_eq!(
        response.edges[0].to_symbol_id.as_deref(),
        Some("server/src/context_compiler/mod.rs::compile_preflight_note")
    );

    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn graph_query_reports_missing_symbol_id() {
    let dir = temp_dir("elon_symbol_graph_missing");
    let db = write_bundle(&dir, "20260614", "213007-trace-graph-user", sample_index());

    let error = load_symbol_graph_db(
        &db,
        &SymbolGraphQuery {
            trace_id: None,
            symbol_id: "missing".to_string(),
            edge_kind: None,
            direction: SymbolRelationDirection::Both,
            limit: 20,
        },
    )
    .unwrap_err()
    .to_string();

    assert!(error.contains("symbol_id 不存在"));

    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn impact_query_returns_impacted_files_and_test_hints() {
    let dir = temp_dir("elon_symbol_impact");
    let db = write_bundle(&dir, "20260614", "213008-trace-impact-user", sample_index());

    let response = load_symbol_impact_db(
        &db,
        &SymbolImpactQuery {
            symbol_id: Some(
                "server/src/context_compiler/context_pack.rs::build_context_pack".to_string(),
            ),
            depth: 1,
            limit: 20,
            ..Default::default()
        },
    )
    .unwrap();

    assert_eq!(response.seed_symbols.len(), 1);
    assert_eq!(response.seed_symbols[0].name, "build_context_pack");
    assert!(response
        .impacted_symbols
        .iter()
        .any(|symbol| symbol.name == "compile_preflight_note"));
    assert!(response.impacted_files.iter().any(|file| file.path
        == "server/src/context_compiler/context_pack_tests.rs"
        && file.test_hint_count > 0));
    assert!(response
        .test_hints
        .iter()
        .any(|hint| hint.symbol_name == "build_context_pack_test"
            && hint.edge_kind.as_deref() == Some("test_covers")));

    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn impact_query_can_start_from_path_and_filter_edges() {
    let dir = temp_dir("elon_symbol_impact_path");
    let db = write_bundle(
        &dir,
        "20260614",
        "213009-trace-impact-path-user",
        sample_index(),
    );

    let response = load_symbol_impact_db(
        &db,
        &SymbolImpactQuery {
            path: Some("context_pack.rs".to_string()),
            edge_kind: Some("test_covers".to_string()),
            depth: 1,
            limit: 20,
            ..Default::default()
        },
    )
    .unwrap();

    assert_eq!(response.seed_symbols.len(), 2);
    assert!(response.edges.iter().all(|edge| edge.kind == "test_covers"));
    assert!(response
        .test_hints
        .iter()
        .any(|hint| hint.symbol_name == "build_context_pack_test"));

    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn impact_query_reports_missing_seed() {
    let dir = temp_dir("elon_symbol_impact_missing");
    let db = write_bundle(
        &dir,
        "20260614",
        "213010-trace-impact-missing-user",
        sample_index(),
    );

    let error = load_symbol_impact_db(
        &db,
        &SymbolImpactQuery {
            symbol_id: Some("missing".to_string()),
            depth: 1,
            limit: 20,
            ..Default::default()
        },
    )
    .unwrap_err()
    .to_string();

    assert!(error.contains("没有找到影响面查询种子"));

    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn impact_pack_renders_model_friendly_context_and_truncates() {
    let dir = temp_dir("elon_symbol_impact_pack");
    let db = write_bundle(
        &dir,
        "20260614",
        "213011-trace-impact-pack-user",
        sample_index(),
    );
    let impact = load_symbol_impact_db(
        &db,
        &SymbolImpactQuery {
            symbol_id: Some(
                "server/src/context_compiler/context_pack.rs::build_context_pack".to_string(),
            ),
            depth: 1,
            limit: 20,
            ..Default::default()
        },
    )
    .unwrap();

    let full = build_symbol_impact_pack(impact.clone(), 12_000);
    assert!(!full.truncated);
    assert!(full.pack.contains("<symbol_impact_context"));
    assert!(full.pack.contains("<seed_symbols count=\"1\">"));
    assert!(full.pack.contains("build_context_pack_test"));
    assert!(full.pack.contains("<usage_guidance>"));

    let short = build_symbol_impact_pack(impact, normalize_pack_max_chars(1));
    assert!(short.truncated);
    assert!(short.pack.contains("symbol impact context truncated"));

    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn task_pack_searches_task_and_expands_top_symbol_impact() {
    let dir = temp_dir("elon_symbol_task_pack");
    let db = write_bundle(
        &dir,
        "20260614",
        "213012-trace-task-pack-user",
        sample_index(),
    );
    backfill_symbol_vectors_db(
        &db,
        &SymbolVectorBackfill {
            model: Some(LOCAL_HASH_VECTOR_MODEL.to_string()),
            ..Default::default()
        },
        None,
    )
    .unwrap();

    let response = build_latest_symbol_task_pack(
        &dir,
        &SymbolTaskPackQuery {
            text: Some("build context pack".to_string()),
            path: Some("context_pack.rs".to_string()),
            vector_model: Some(LOCAL_HASH_VECTOR_MODEL.to_string()),
            depth: 1,
            search_limit: 5,
            impact_limit: 20,
            max_chars: 24_000,
            ..Default::default()
        },
    )
    .unwrap();

    assert_eq!(response.chosen_seed.name, "build_context_pack");
    assert_eq!(response.chosen_seed_source, "symbol");
    assert!(response
        .candidate_symbols
        .iter()
        .any(|symbol| symbol.name == "build_context_pack"));
    assert!(response.pack.contains("<symbol_task_context"));
    assert!(response.pack.contains("<ranked_context"));
    assert!(response.pack.contains("decision="));
    assert!(response.pack.contains("sources="));
    assert!(response.pack.contains("<compressed_context"));
    assert!(response.pack.contains("Compression:"));
    assert!(response.pack.contains("<patch_plan"));
    assert!(response.pack.contains("# Patch Plan"));
    assert!(response.pack.contains("<candidate_symbols"));
    assert!(response.pack.contains("<full_text_chunks"));
    assert!(response.pack.contains("<vector_chunks"));
    assert!(response.pack.contains("<symbol_impact_context"));
    assert!(response.pack.contains("build_context_pack_test"));
    assert!(response
        .text_chunks
        .iter()
        .any(|chunk| chunk.chunk_type == "symbol" && chunk.id.contains("build_context_pack")));
    assert!(response
        .vector_chunks
        .iter()
        .any(|chunk| chunk.id.contains("build_context_pack")));
    assert!(response
        .ranked_context
        .iter()
        .any(|item| item.source == "symbol" && item.label.contains("build_context_pack")));
    assert!(!response.compressed_context.blocks.is_empty());
    assert!(
        response.compressed_context.used_tokens <= response.compressed_context.budget_tokens,
        "compressed context should fit its token budget"
    );
    assert!(response
        .compressed_context
        .level_counts
        .keys()
        .any(|level| level == "full_symbol_body" || level == "focused_snippet"));
    assert_eq!(response.patch_plan.plan_kind, "context_only");
    assert!(!response.patch_plan.should_inspect.is_empty());
    assert_eq!(response.patch_generation.mode, PatchGenerationMode::NoPatch);
    assert!(!response.patch_generation.ready_to_generate);
    assert_eq!(
        response.patch_generation.apply_readiness.level,
        PatchApplyReadinessLevel::NotApplicable
    );
    assert!(
        !response
            .patch_generation
            .apply_readiness
            .can_run_apply_check
    );
    assert!(response
        .patch_generation
        .blocked_reasons
        .contains(&"patch_plan_says_context_only".to_string()));
    assert!(response
        .pack
        .contains("<patch_generation mode=\"no_patch\""));
    assert!(response.pack.contains("## Apply Readiness"));
    assert!(response.test_hint_count > 0);

    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn task_pack_generates_patch_plan_for_status_change_tasks() {
    let dir = temp_dir("elon_symbol_task_pack_patch_plan");
    let _db = write_bundle(
        &dir,
        "20260614",
        "213012-trace-task-pack-patch-plan-user",
        sample_index(),
    );

    let response = build_latest_symbol_task_pack(
        &dir,
        &SymbolTaskPackQuery {
            text: Some("把 build_context_pack 报错 500 改成 401".to_string()),
            path: Some("context_pack.rs".to_string()),
            depth: 1,
            search_limit: 5,
            chunk_limit: 10,
            impact_limit: 20,
            max_chars: 24_000,
            ..Default::default()
        },
    )
    .unwrap();

    assert_eq!(response.patch_plan.plan_kind, "debug_fix");
    assert!(response.patch_plan.patch_required);
    assert!(response
        .patch_plan
        .must_edit
        .iter()
        .any(|target| target.file_path == "server/src/context_compiler/context_pack.rs"));
    assert!(response
        .patch_plan
        .must_edit
        .iter()
        .any(|target| target.file_path == "server/src/context_compiler/context_pack_tests.rs"));
    assert!(response
        .patch_plan
        .test_plan
        .commands
        .iter()
        .any(|command| command.contains("build_context_pack_test")));
    assert_eq!(
        response.patch_generation.mode,
        PatchGenerationMode::GenerateDiff
    );
    assert!(response.patch_generation.ready_to_generate);
    assert!(response
        .patch_generation
        .diff_contract
        .allowed_files
        .iter()
        .any(|path| path == "server/src/context_compiler/context_pack.rs"));
    assert!(response
        .patch_generation
        .diff_contract
        .allowed_files
        .iter()
        .any(|path| path == "server/src/context_compiler/context_pack_tests.rs"));
    assert!(response
        .patch_generation
        .prompt
        .contains("Generate a unified diff only"));
    assert_eq!(
        response.patch_generation.apply_readiness.level,
        PatchApplyReadinessLevel::ReadyAfterDiff
    );
    assert!(
        response
            .patch_generation
            .apply_readiness
            .can_run_apply_check
    );
    assert!(response
        .patch_generation
        .apply_readiness
        .source_requirements
        .iter()
        .any(|requirement| requirement.contains("context_pack.rs")));
    assert!(response
        .patch_generation
        .apply_readiness
        .pre_apply_checks
        .iter()
        .any(|check| check.contains("git apply --check")));
    assert!(response.pack.contains("<patch_plan intent=\"debug_error\""));
    assert!(response
        .pack
        .contains("<patch_generation mode=\"generate_diff\" ready=\"true\""));
    assert!(response.pack.contains("# Patch Generation Contract"));
    assert!(response.pack.contains("## Apply Readiness"));
    assert!(response.pack.contains("## Test Plan"));
    assert!(response.pack.contains("Planning Trace"));

    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn patch_check_accepts_diff_that_matches_generation_contract() {
    let dir = temp_dir("elon_symbol_patch_check_accepts");
    let _db = write_bundle(
        &dir,
        "20260614",
        "213012-trace-patch-check-accepts-user",
        sample_index(),
    );

    let response = build_latest_symbol_task_pack(
        &dir,
        &SymbolTaskPackQuery {
            text: Some("把 build_context_pack 报错 500 改成 401".to_string()),
            path: Some("context_pack.rs".to_string()),
            depth: 1,
            search_limit: 5,
            chunk_limit: 10,
            impact_limit: 20,
            max_chars: 24_000,
            ..Default::default()
        },
    )
    .unwrap();
    let diff = r#"diff --git a/server/src/context_compiler/context_pack.rs b/server/src/context_compiler/context_pack.rs
--- a/server/src/context_compiler/context_pack.rs
+++ b/server/src/context_compiler/context_pack.rs
@@ -10,7 +10,7 @@
-old status mapping
+new status mapping
diff --git a/server/src/context_compiler/context_pack_tests.rs b/server/src/context_compiler/context_pack_tests.rs
--- a/server/src/context_compiler/context_pack_tests.rs
+++ b/server/src/context_compiler/context_pack_tests.rs
@@ -3,7 +3,7 @@
-assert old
+assert new
"#;

    let check = check_symbol_patch_diff(&response.patch_generation, diff);

    assert_eq!(check.status, PatchDiffCheckStatus::AcceptedForApplyCheck);
    assert!(check.accepted_for_apply_check);
    assert!(check.violations.is_empty());
    assert_eq!(check.touched_files.len(), 2);
    assert_eq!(
        check.apply_check_command.as_deref(),
        Some("git apply --check <generated.patch>")
    );

    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn patch_check_rejects_diff_outside_allowed_files() {
    let dir = temp_dir("elon_symbol_patch_check_rejects");
    let _db = write_bundle(
        &dir,
        "20260614",
        "213012-trace-patch-check-rejects-user",
        sample_index(),
    );

    let response = build_latest_symbol_task_pack(
        &dir,
        &SymbolTaskPackQuery {
            text: Some("把 build_context_pack 报错 500 改成 401".to_string()),
            path: Some("context_pack.rs".to_string()),
            depth: 1,
            search_limit: 5,
            chunk_limit: 10,
            impact_limit: 20,
            max_chars: 24_000,
            ..Default::default()
        },
    )
    .unwrap();
    let diff = r#"diff --git a/server/src/context_compiler/unrelated.rs b/server/src/context_compiler/unrelated.rs
--- a/server/src/context_compiler/unrelated.rs
+++ b/server/src/context_compiler/unrelated.rs
@@ -1,3 +1,3 @@
-old
+new
"#;

    let check = check_symbol_patch_diff(&response.patch_generation, diff);

    assert_eq!(check.status, PatchDiffCheckStatus::Rejected);
    assert!(!check.accepted_for_apply_check);
    assert!(check
        .violations
        .iter()
        .any(|violation| violation.code == "file_not_allowed"));

    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn task_pack_falls_back_to_text_chunk_path_when_symbol_search_misses() {
    let dir = temp_dir("elon_symbol_task_pack_chunk_seed");
    let _db = write_bundle(
        &dir,
        "20260614",
        "213012-trace-task-pack-chunk-seed-user",
        sample_index(),
    );

    let response = build_latest_symbol_task_pack(
        &dir,
        &SymbolTaskPackQuery {
            text: Some("symbol_count".to_string()),
            path: Some("context_pack.rs".to_string()),
            depth: 1,
            chunk_limit: 5,
            impact_limit: 20,
            max_chars: 12_000,
            ..Default::default()
        },
    )
    .unwrap();

    assert_eq!(response.chosen_seed.name, "build_context_pack");
    assert_eq!(response.chosen_seed_source, "full_text_path");
    assert!(response
        .candidate_symbols
        .iter()
        .any(|symbol| symbol.name == "build_context_pack"));
    assert!(response
        .text_chunks
        .iter()
        .any(|chunk| chunk.chunk_type == "module"
            && chunk.file_path == "server/src/context_compiler/context_pack.rs"));
    assert!(response
        .ranked_context
        .iter()
        .any(|item| item.source == "full_text"
            && item.reasons.iter().any(|reason| reason == "fts_bm25")));
    assert!(response.pack.contains("source=\"full_text_path\""));
    assert!(response.pack.contains("<ranked_context"));
    assert!(response.pack.contains("<symbol_impact_context"));

    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn task_pack_uses_error_rank_profile_for_error_queries() {
    let dir = temp_dir("elon_symbol_task_pack_error_profile");
    let _db = write_bundle(
        &dir,
        "20260614",
        "213012-trace-task-pack-error-profile-user",
        sample_index(),
    );

    let response = build_latest_symbol_task_pack(
        &dir,
        &SymbolTaskPackQuery {
            text: Some("报错 symbol_count duplicate key".to_string()),
            path: Some("context_pack.rs".to_string()),
            depth: 1,
            chunk_limit: 5,
            impact_limit: 20,
            max_chars: 12_000,
            ..Default::default()
        },
    )
    .unwrap();

    assert_eq!(response.ranking_profile.name, "error");
    assert_eq!(response.retrieval_plan.intent, QueryIntent::DebugError);
    assert!(response
        .pack
        .contains("<retrieval_plan intent=\"debug_error\""));
    assert!(response.retrieval_plan.graph_policy.include_error_mappers);
    assert!(response.retrieval_plan.pack_policy.include_error_mapping);
    assert!(response.pack.contains("<ranking_profile name=\"error\""));
    assert!(response.ranked_context.iter().any(|item| item
        .reasons
        .iter()
        .any(|reason| reason.contains("rank_profile=error"))));
    assert!(response.ranked_context.iter().any(|item| item
        .reasons
        .iter()
        .any(|reason| reason.contains("retrieval_plan=debug_error"))));
    assert_eq!(
        response
            .ranked_context
            .first()
            .map(|item| item.source.as_str()),
        Some("full_text")
    );

    fs::remove_dir_all(dir).unwrap();
}


// --- helpers ---
pub(super) fn write_bundle(data_dir: &Path, day: &str, stem: &str, index: SymbolIndex) -> PathBuf {
    let bundle = data_dir.join("context-compiler").join(day).join(stem);
    fs::create_dir_all(&bundle).unwrap();
    let db = bundle.join(SYMBOL_INDEX_DB_FILE);
    let mut files = Vec::new();
    write_symbol_index_sqlite(&db, &index, &mut files).unwrap();
    assert!(db.is_file());
    db
}

pub(super) fn load_first_embedding_test_chunks(conn: &Connection) -> Vec<(String, String)> {
    let mut stmt = conn
        .prepare(
            r#"
            SELECT id, hash
            FROM chunks
            ORDER BY CASE chunk_type WHEN 'symbol' THEN 0 WHEN 'module' THEN 1 ELSE 2 END,
                file_path, start_line
            LIMIT 2
            "#,
        )
        .unwrap();
    let rows = stmt
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .unwrap();
    rows.map(|row| row.unwrap()).collect()
}

pub(super) fn sample_index() -> SymbolIndex {
    SymbolIndex {
        records: vec![
            symbol(
                "server/src/context_compiler/mod.rs::compile_preflight_note",
                "compile_preflight_note",
                "fn",
                "server/src/context_compiler/mod.rs",
                78,
                "pub(crate) async fn compile_preflight_note(...) -> Option<String>",
                Some(9.4),
                vec!["rust_symbols", "rust_analyzer_lsp"],
            ),
            symbol(
                "server/src/context_compiler/context_pack.rs::build_context_pack",
                "build_context_pack",
                "fn",
                "server/src/context_compiler/context_pack.rs",
                10,
                "pub(crate) fn build_context_pack(...) -> String",
                Some(8.2),
                vec!["rust_symbols"],
            ),
            symbol(
                "server/src/context_compiler/context_pack.rs::ContextPackArtifact",
                "ContextPackArtifact",
                "struct",
                "server/src/context_compiler/context_pack.rs",
                18,
                "pub(crate) struct ContextPackArtifact",
                Some(4.0),
                vec!["rust_symbols"],
            ),
            symbol(
                "server/src/context_compiler/context_pack_tests.rs::build_context_pack_test",
                "build_context_pack_test",
                "fn",
                "server/src/context_compiler/context_pack_tests.rs",
                22,
                "#[test] fn build_context_pack_test()",
                Some(3.0),
                vec!["rust_symbols"],
            ),
        ],
        edges: vec![
            SymbolEdge {
                id: "edge-calls".to_string(),
                source: "rust_analyzer_lsp",
                kind: "calls".to_string(),
                from_symbol_id: Some(
                    "server/src/context_compiler/mod.rs::compile_preflight_note".to_string(),
                ),
                from_path: "server/src/context_compiler/mod.rs".to_string(),
                line: 132,
                to_symbol_id: Some(
                    "server/src/context_compiler/context_pack.rs::build_context_pack".to_string(),
                ),
                to_symbol_name: Some("build_context_pack".to_string()),
                to_path: Some("server/src/context_compiler/context_pack.rs".to_string()),
                confidence: 0.95,
                reason: "call hierarchy".to_string(),
            },
            SymbolEdge {
                id: "edge-ref".to_string(),
                source: "rust_analyzer_lsp",
                kind: "references".to_string(),
                from_symbol_id: Some(
                    "server/src/context_compiler/context_pack.rs::build_context_pack".to_string(),
                ),
                from_path: "server/src/context_compiler/context_pack.rs".to_string(),
                line: 44,
                to_symbol_id: Some(
                    "server/src/context_compiler/mod.rs::compile_preflight_note".to_string(),
                ),
                to_symbol_name: Some("compile_preflight_note".to_string()),
                to_path: Some("server/src/context_compiler/mod.rs".to_string()),
                confidence: 0.8,
                reason: "reference lookup".to_string(),
            },
            SymbolEdge {
                id: "edge-test".to_string(),
                source: "rust_analyzer_lsp",
                kind: "test_covers".to_string(),
                from_symbol_id: Some(
                    "server/src/context_compiler/context_pack_tests.rs::build_context_pack_test"
                        .to_string(),
                ),
                from_path: "server/src/context_compiler/context_pack_tests.rs".to_string(),
                line: 24,
                to_symbol_id: Some(
                    "server/src/context_compiler/context_pack.rs::build_context_pack".to_string(),
                ),
                to_symbol_name: Some("build_context_pack".to_string()),
                to_path: Some("server/src/context_compiler/context_pack.rs".to_string()),
                confidence: 0.9,
                reason: "test covers symbol".to_string(),
            },
        ],
        ..Default::default()
    }
}

pub(super) fn symbol(
    id: &str,
    name: &str,
    kind: &str,
    file_path: &str,
    start_line: usize,
    signature: &str,
    importance_score: Option<f64>,
    source_providers: Vec<&str>,
) -> SymbolRecord {
    SymbolRecord {
        id: id.to_string(),
        name: name.to_string(),
        qualified_name: id.to_string(),
        kind: kind.to_string(),
        language: "rust",
        file_path: file_path.to_string(),
        start_line,
        end_line: start_line + 10,
        signature: signature.to_string(),
        visibility: "pub".to_string(),
        parent_symbol_id: None,
        module_path: file_path.replace('/', "::"),
        doc_summary: None,
        role: "definition",
        importance_score,
        signature_hash: format!("{name}-hash"),
        source_providers: source_providers
            .into_iter()
            .map(ToOwned::to_owned)
            .collect(),
    }
}

pub(super) fn temp_dir(prefix: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("{prefix}_{}_{}", std::process::id(), nonce))
}
