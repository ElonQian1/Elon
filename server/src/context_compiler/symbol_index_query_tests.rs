use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use rusqlite::Connection;

use super::{
    symbol_index::{SymbolEdge, SymbolIndex, SymbolRecord},
    symbol_index_chunks::{search_symbol_chunks_db, SymbolChunkSearch},
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
    symbol_index_query::{find_symbol_index_db, search_symbol_index_db, SymbolIndexSearch},
    symbol_index_store::{write_symbol_index_sqlite, SYMBOL_INDEX_DB_FILE},
    symbol_index_task_pack::{build_latest_symbol_task_pack, SymbolTaskPackQuery},
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
    let _db = write_bundle(
        &dir,
        "20260614",
        "213012-trace-task-pack-user",
        sample_index(),
    );

    let response = build_latest_symbol_task_pack(
        &dir,
        &SymbolTaskPackQuery {
            text: Some("build context pack".to_string()),
            path: Some("context_pack.rs".to_string()),
            depth: 1,
            search_limit: 5,
            impact_limit: 20,
            max_chars: 12_000,
            ..Default::default()
        },
    )
    .unwrap();

    assert_eq!(response.chosen_seed.name, "build_context_pack");
    assert!(response
        .candidate_symbols
        .iter()
        .any(|symbol| symbol.name == "build_context_pack"));
    assert!(response.pack.contains("<symbol_task_context"));
    assert!(response.pack.contains("<candidate_symbols"));
    assert!(response.pack.contains("<full_text_chunks"));
    assert!(response.pack.contains("<symbol_impact_context"));
    assert!(response.pack.contains("build_context_pack_test"));
    assert!(response
        .text_chunks
        .iter()
        .any(|chunk| chunk.chunk_type == "symbol" && chunk.id.contains("build_context_pack")));
    assert!(response.test_hint_count > 0);

    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn chunk_search_uses_fts_for_symbol_module_and_test_chunks() {
    let dir = temp_dir("elon_symbol_chunks");
    let db = write_bundle(&dir, "20260614", "213013-trace-chunks-user", sample_index());

    let response = search_symbol_chunks_db(
        &db,
        &SymbolChunkSearch {
            text: Some("context pack test".to_string()),
            limit: 10,
            ..Default::default()
        },
    )
    .unwrap();

    assert!(response
        .metadata
        .get("schema_version")
        .is_some_and(|version| version == "3"));
    assert!(response
        .metadata
        .get("chunk_count")
        .and_then(|value| value.parse::<usize>().ok())
        .is_some_and(|count| count >= 5));
    assert!(response
        .chunks
        .iter()
        .any(|chunk| chunk.chunk_type == "symbol"
            && chunk
                .qualified_name
                .as_deref()
                .is_some_and(|name| name.contains("build_context_pack"))));
    assert!(response
        .chunks
        .iter()
        .any(|chunk| chunk.chunk_type == "module"
            && chunk.file_path == "server/src/context_compiler/context_pack.rs"));
    assert!(response
        .chunks
        .iter()
        .any(|chunk| chunk.chunk_type == "test"
            && chunk.file_path == "server/src/context_compiler/context_pack_tests.rs"));

    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn eval_reports_recall_mrr_and_missing_context_requirements() {
    let dir = temp_dir("elon_symbol_eval");
    let _db = write_bundle(&dir, "20260614", "213014-trace-eval-user", sample_index());

    let response = evaluate_latest_symbol_retrieval(
        &dir,
        &RetrievalEvalQuery {
            text: Some("build context pack".to_string()),
            must_include: vec![
                "context_pack.rs".to_string(),
                "context_pack_tests.rs".to_string(),
                "build_context_pack".to_string(),
            ],
            k: 10,
            symbol_limit: 5,
            chunk_limit: 10,
            depth: 1,
            impact_limit: 20,
            ..Default::default()
        },
    )
    .unwrap();

    assert_eq!(response.metrics.requirement_count, 3);
    assert_eq!(response.metrics.recall_at_k, 1.0);
    assert!(response.metrics.mean_reciprocal_rank > 0.0);
    assert!(response.metrics.has_test_context_at_k);
    assert!(response.missing_requirements.is_empty());
    assert!(response
        .candidates
        .iter()
        .any(|candidate| candidate.source == "symbol"));
    assert!(response
        .candidates
        .iter()
        .any(|candidate| candidate.source == "full_text"));
    assert!(response
        .candidates
        .iter()
        .any(|candidate| candidate.source.starts_with("graph_")));

    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn eval_batch_aggregates_cases_and_records_retrieval_run() {
    let dir = temp_dir("elon_symbol_eval_batch");
    let db = write_bundle(
        &dir,
        "20260614",
        "213015-trace-eval-batch-user",
        sample_index(),
    );

    let response = evaluate_latest_symbol_retrieval_batch(
        &dir,
        &SymbolRetrievalEvalBatchQuery {
            trace_id: Some("trace-eval-batch".to_string()),
            record_runs: true,
            cases: vec![
                SymbolRetrievalEvalBatchCaseQuery {
                    id: "context-pack".to_string(),
                    query: RetrievalEvalQuery {
                        text: Some("build context pack".to_string()),
                        must_include: vec![
                            "context_pack.rs".to_string(),
                            "context_pack_tests.rs".to_string(),
                            "build_context_pack".to_string(),
                        ],
                        k: 10,
                        symbol_limit: 5,
                        chunk_limit: 10,
                        depth: 1,
                        impact_limit: 20,
                        ..Default::default()
                    },
                },
                SymbolRetrievalEvalBatchCaseQuery {
                    id: "compile-preflight".to_string(),
                    query: RetrievalEvalQuery {
                        text: Some("compile preflight".to_string()),
                        must_include: vec![
                            "mod.rs".to_string(),
                            "compile_preflight_note".to_string(),
                        ],
                        k: 10,
                        symbol_limit: 5,
                        chunk_limit: 10,
                        depth: 1,
                        impact_limit: 20,
                        ..Default::default()
                    },
                },
            ],
        },
    )
    .unwrap();

    assert_eq!(response.case_count, 2);
    assert_eq!(response.evaluated_count, 2);
    assert_eq!(response.failed_count, 0);
    assert!(response.recorded);
    assert!(response.record_error.is_none());
    assert_eq!(response.aggregate.requirement_count, 5);
    assert_eq!(response.aggregate.mean_recall_at_k, 1.0);
    assert!(response.aggregate.mean_reciprocal_rank > 0.0);
    assert!(response.aggregate.has_test_context_rate > 0.0);
    assert!(response
        .cases
        .iter()
        .all(|case| case.ok && case.result.is_some()));

    let conn = Connection::open(&db).unwrap();
    let selected_chunks_json: String = conn
        .query_row(
            "SELECT selected_chunks_json FROM retrieval_runs WHERE id = ?1",
            [response.run_id.as_str()],
            |row| row.get(0),
        )
        .unwrap();
    assert!(selected_chunks_json.contains("build_context_pack"));
    assert!(selected_chunks_json.contains("compile_preflight_note"));

    let history = list_latest_retrieval_runs(
        &dir,
        &SymbolRetrievalRunHistoryQuery {
            trace_id: Some("trace-eval-batch".to_string()),
            limit: 10,
        },
    )
    .unwrap();
    assert_eq!(history.runs.len(), 1);
    assert_eq!(history.runs[0].id.as_str(), response.run_id.as_str());
    assert_eq!(history.runs[0].scores["caseCount"].as_u64(), Some(2));

    let detail = load_latest_retrieval_run(
        &dir,
        &SymbolRetrievalRunLookupQuery {
            trace_id: Some("trace-eval-batch".to_string()),
            id: response.run_id.clone(),
        },
    )
    .unwrap();
    assert_eq!(detail.run.id.as_str(), response.run_id.as_str());
    assert_eq!(detail.run.scores["evaluatedCount"].as_u64(), Some(2));
    assert!(detail
        .run
        .selected_chunks
        .to_string()
        .contains("context_pack_tests.rs"));

    drop(conn);
    fs::remove_dir_all(dir).unwrap();
}

fn write_bundle(data_dir: &Path, day: &str, stem: &str, index: SymbolIndex) -> PathBuf {
    let bundle = data_dir.join("context-compiler").join(day).join(stem);
    fs::create_dir_all(&bundle).unwrap();
    let db = bundle.join(SYMBOL_INDEX_DB_FILE);
    let mut files = Vec::new();
    write_symbol_index_sqlite(&db, &index, &mut files).unwrap();
    assert!(db.is_file());
    db
}

fn sample_index() -> SymbolIndex {
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

fn symbol(
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

fn temp_dir(prefix: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("{prefix}_{}_{}", std::process::id(), nonce))
}
