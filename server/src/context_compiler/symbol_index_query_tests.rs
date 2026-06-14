use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use super::{
    symbol_index::{SymbolEdge, SymbolIndex, SymbolRecord},
    symbol_index_graph_query::{load_symbol_graph_db, SymbolGraphQuery, SymbolRelationDirection},
    symbol_index_impact_query::load_symbol_impact_db,
    symbol_index_impact_types::SymbolImpactQuery,
    symbol_index_query::{find_symbol_index_db, search_symbol_index_db, SymbolIndexSearch},
    symbol_index_store::{write_symbol_index_sqlite, SYMBOL_INDEX_DB_FILE},
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
