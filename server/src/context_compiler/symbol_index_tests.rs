use super::{
    model::{
        CodeRelationship, RankedSymbol, RelationshipKind, RepoContextIndex,
        RustAnalyzerLspLocation, RustAnalyzerLspLocationRole, RustAnalyzerLspQueryResult,
        RustAnalyzerLspReport, RustAnalyzerLspStatus, RustAnalyzerReport, RustIndex, RustSymbol,
        SemanticQueryMethod, SymbolGraphSummary, SymbolKind, SymbolVisibility,
    },
    symbol_index::SymbolQuery,
    symbol_index_build::build_symbol_index,
};

#[test]
fn builds_queryable_symbol_index_from_repo_map_and_lsp_facts() {
    let service_id = "src/service.rs:1:struct:AuthService";
    let login_id = "src/service.rs:10:function:login";
    let test_id = "tests/auth_test.rs:5:function:login_works";
    let index = RepoContextIndex {
        rust: RustIndex {
            files_scanned: 2,
            symbols: vec![
                symbol(
                    service_id,
                    "AuthService",
                    SymbolKind::Struct,
                    "src/service.rs",
                    1,
                    4,
                ),
                RustSymbol {
                    parent: Some(service_id.to_string()),
                    ..symbol(
                        login_id,
                        "login",
                        SymbolKind::Function,
                        "src/service.rs",
                        10,
                        20,
                    )
                },
                symbol(
                    test_id,
                    "login_works",
                    SymbolKind::Function,
                    "tests/auth_test.rs",
                    5,
                    12,
                ),
            ],
            warnings: Vec::new(),
        },
        graph: SymbolGraphSummary {
            ranked_symbols: vec![RankedSymbol {
                id: login_id.to_string(),
                name: "login".to_string(),
                kind: SymbolKind::Function,
                path: "src/service.rs".to_string(),
                line_start: 10,
                line_end: 20,
                score: 9.7,
                reasons: vec!["task mentions login".to_string()],
            }],
            relationships: vec![
                CodeRelationship {
                    from_path: "src/api.rs".to_string(),
                    to_symbol_id: login_id.to_string(),
                    to_symbol_name: "login".to_string(),
                    to_path: "src/service.rs".to_string(),
                    kind: RelationshipKind::CallsOrMentions,
                    line: 30,
                    reason: "handler mentions login".to_string(),
                },
                CodeRelationship {
                    from_path: "tests/auth_test.rs".to_string(),
                    to_symbol_id: login_id.to_string(),
                    to_symbol_name: "login".to_string(),
                    to_path: "src/service.rs".to_string(),
                    kind: RelationshipKind::TestCovers,
                    line: 8,
                    reason: "test calls login".to_string(),
                },
            ],
            ..SymbolGraphSummary::default()
        },
        rust_analyzer: RustAnalyzerReport {
            lsp: RustAnalyzerLspReport {
                enabled: true,
                attempted: 1,
                succeeded: 1,
                results: vec![RustAnalyzerLspQueryResult {
                    method: SemanticQueryMethod::References,
                    path: "src/service.rs".to_string(),
                    line: 10,
                    symbol: Some("login".to_string()),
                    status: RustAnalyzerLspStatus::Succeeded,
                    duration_ms: 3,
                    summary: Some("1 item(s)".to_string()),
                    locations: vec![RustAnalyzerLspLocation {
                        role: RustAnalyzerLspLocationRole::Reference,
                        path: "tests/auth_test.rs".to_string(),
                        line: 8,
                        end_line: None,
                        symbol: Some("login".to_string()),
                    }],
                    warning: None,
                }],
                ..RustAnalyzerLspReport::default()
            },
            ..RustAnalyzerReport::default()
        },
        ..RepoContextIndex::default()
    };

    let symbol_index = build_symbol_index(&index);

    let login = symbol_index.get_symbol(login_id).unwrap();
    assert_eq!(login.qualified_name, "crate::service::AuthService::login");
    assert_eq!(login.importance_score, Some(9.7));
    assert!(login
        .source_providers
        .contains(&"rust_analyzer_lsp:references".to_string()));

    let matches = symbol_index.search_symbols(SymbolQuery {
        text: "auth login".to_string(),
        limit: 3,
        kind: Some("function".to_string()),
    });
    assert_eq!(matches.first().unwrap().id, login_id);
    assert_eq!(symbol_index.symbols_in_file("src\\service.rs").len(), 2);

    let references = symbol_index.references_to(login_id);
    assert!(references
        .iter()
        .any(|edge| edge.source == "rust_analyzer_lsp" && edge.kind == "reference"));
    assert!(!symbol_index.neighbors(login_id).is_empty());
    assert_eq!(symbol_index.tests_for_symbol(login_id).len(), 1);

    let summary = symbol_index.lookup_summary();
    assert_eq!(summary.symbol_count, 3);
    assert_eq!(summary.file_count, 2);
    assert!(summary.lsp_edge_count >= 1);
    assert!(summary.query_api.contains(&"search_symbols"));
}

fn symbol(
    id: &str,
    name: &str,
    kind: SymbolKind,
    path: &str,
    line_start: usize,
    line_end: usize,
) -> RustSymbol {
    RustSymbol {
        id: id.to_string(),
        name: name.to_string(),
        kind,
        path: path.to_string(),
        line_start,
        line_end,
        visibility: SymbolVisibility::Public,
        signature: format!("pub fn {name}()"),
        parent: None,
        docs: Some(format!("{name} docs")),
        role: "source",
        safety_notes: Vec::new(),
    }
}
