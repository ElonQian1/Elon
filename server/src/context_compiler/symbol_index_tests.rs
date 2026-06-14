use super::{
    model::{
        CodeRelationship, ImpactFact, ImpactKind, RankedSymbol, RelationshipKind, RepoContextIndex,
        RustAnalyzerLspLocation, RustAnalyzerLspLocationRole, RustAnalyzerLspQueryResult,
        RustAnalyzerLspReport, RustAnalyzerLspStatus, RustAnalyzerReport, RustImpactAnalysis,
        RustImport, RustIndex, RustSymbol, SemanticQueryMethod, SymbolGraphSummary, SymbolKind,
        SymbolVisibility,
    },
    symbol_index::SymbolQuery,
    symbol_index_build::build_symbol_index,
};

#[test]
fn builds_queryable_symbol_index_from_repo_map_and_lsp_facts() {
    let service_id = "src/service.rs:1:struct:AuthService";
    let login_id = "src/service.rs:10:function:login";
    let user_id = "src/domain.rs:1:struct:User";
    let repo_trait_id = "src/repo.rs:1:trait:UserRepository";
    let repo_impl_id = "src/repo_pg.rs:4:impl:impl UserRepository for PgRepo";
    let handler_id = "src/api.rs:25:function:login_handler";
    let issue_id = "src/token.rs:12:function:issue_token";
    let test_id = "tests/auth_test.rs:5:function:login_works";
    let index = RepoContextIndex {
        rust: RustIndex {
            files_scanned: 6,
            symbols: vec![
                symbol(user_id, "User", SymbolKind::Struct, "src/domain.rs", 1, 4),
                symbol(
                    repo_trait_id,
                    "UserRepository",
                    SymbolKind::Trait,
                    "src/repo.rs",
                    1,
                    8,
                ),
                symbol(
                    repo_impl_id,
                    "impl UserRepository for PgRepo",
                    SymbolKind::Impl,
                    "src/repo_pg.rs",
                    4,
                    20,
                ),
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
                    handler_id,
                    "login_handler",
                    SymbolKind::Function,
                    "src/api.rs",
                    25,
                    35,
                ),
                symbol(
                    issue_id,
                    "issue_token",
                    SymbolKind::Function,
                    "src/token.rs",
                    12,
                    18,
                ),
                symbol(
                    test_id,
                    "login_works",
                    SymbolKind::Function,
                    "tests/auth_test.rs",
                    5,
                    12,
                ),
            ],
            imports: vec![RustImport {
                path: "src/service.rs".to_string(),
                line: 2,
                imported_path: "crate::domain::User".to_string(),
                alias: None,
                public: true,
                glob: false,
                raw: "pub use crate::domain::User;".to_string(),
            }],
            ..RustIndex::default()
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
                attempted: 5,
                succeeded: 5,
                results: vec![
                    RustAnalyzerLspQueryResult {
                        method: SemanticQueryMethod::Definition,
                        path: "src/api.rs".to_string(),
                        line: 30,
                        symbol: Some("login".to_string()),
                        status: RustAnalyzerLspStatus::Succeeded,
                        duration_ms: 2,
                        summary: Some("1 item(s)".to_string()),
                        locations: vec![RustAnalyzerLspLocation {
                            role: RustAnalyzerLspLocationRole::Definition,
                            path: "src/service.rs".to_string(),
                            line: 10,
                            end_line: None,
                            symbol: Some("login".to_string()),
                        }],
                        warning: None,
                    },
                    RustAnalyzerLspQueryResult {
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
                    },
                    RustAnalyzerLspQueryResult {
                        method: SemanticQueryMethod::Implementation,
                        path: "src/repo.rs".to_string(),
                        line: 1,
                        symbol: Some("UserRepository".to_string()),
                        status: RustAnalyzerLspStatus::Succeeded,
                        duration_ms: 4,
                        summary: Some("1 item(s)".to_string()),
                        locations: vec![RustAnalyzerLspLocation {
                            role: RustAnalyzerLspLocationRole::Implementation,
                            path: "src/repo_pg.rs".to_string(),
                            line: 4,
                            end_line: None,
                            symbol: Some("PgRepo".to_string()),
                        }],
                        warning: None,
                    },
                    RustAnalyzerLspQueryResult {
                        method: SemanticQueryMethod::IncomingCalls,
                        path: "src/service.rs".to_string(),
                        line: 10,
                        symbol: Some("login".to_string()),
                        status: RustAnalyzerLspStatus::Succeeded,
                        duration_ms: 5,
                        summary: Some("1 item(s)".to_string()),
                        locations: vec![RustAnalyzerLspLocation {
                            role: RustAnalyzerLspLocationRole::IncomingCaller,
                            path: "src/api.rs".to_string(),
                            line: 30,
                            end_line: None,
                            symbol: Some("login_handler".to_string()),
                        }],
                        warning: None,
                    },
                    RustAnalyzerLspQueryResult {
                        method: SemanticQueryMethod::OutgoingCalls,
                        path: "src/service.rs".to_string(),
                        line: 10,
                        symbol: Some("login".to_string()),
                        status: RustAnalyzerLspStatus::Succeeded,
                        duration_ms: 6,
                        summary: Some("1 item(s)".to_string()),
                        locations: vec![RustAnalyzerLspLocation {
                            role: RustAnalyzerLspLocationRole::OutgoingCallee,
                            path: "src/token.rs".to_string(),
                            line: 12,
                            end_line: None,
                            symbol: Some("issue_token".to_string()),
                        }],
                        warning: None,
                    },
                ],
                ..RustAnalyzerLspReport::default()
            },
            ..RustAnalyzerReport::default()
        },
        impact: RustImpactAnalysis {
            function_call_sites: vec![ImpactFact {
                subject: "login".to_string(),
                path: "src/api.rs".to_string(),
                line: 30,
                kind: ImpactKind::FunctionCallSite,
                evidence: "service.login().await".to_string(),
                reason: "call-like token hit".to_string(),
            }],
            ..RustImpactAnalysis::default()
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
    assert!(symbol_index
        .definitions_for(handler_id)
        .iter()
        .any(|edge| edge.to_symbol_id.as_deref() == Some(login_id)));
    assert!(symbol_index
        .implementations_for(repo_trait_id)
        .iter()
        .any(|edge| edge.to_symbol_id.as_deref() == Some(repo_impl_id)));
    assert!(symbol_index
        .callers_of(login_id)
        .iter()
        .any(|edge| edge.from_symbol_id.as_deref() == Some(handler_id)));
    assert!(symbol_index
        .callees_of(login_id)
        .iter()
        .any(|edge| edge.to_symbol_id.as_deref() == Some(issue_id)));
    assert!(symbol_index
        .semantic_edges_for(login_id)
        .iter()
        .any(|edge| edge.source == "rust_analyzer_lsp"));
    let login_neighbors = symbol_index.neighbors(login_id);
    assert!(login_neighbors
        .iter()
        .any(|edge| edge.source == "rust_symbols" && edge.kind == "contains"));
    assert!(symbol_index
        .edges
        .iter()
        .any(|edge| edge.source == "rust_imports"
            && edge.kind == "exports"
            && edge.to_symbol_id.as_deref() == Some(user_id)));
    assert!(symbol_index
        .edges
        .iter()
        .any(|edge| edge.source == "impact_analysis"
            && edge.kind == "calls"
            && edge.to_symbol_id.as_deref() == Some(login_id)));
    assert_eq!(symbol_index.tests_for_symbol(login_id).len(), 1);

    let summary = symbol_index.lookup_summary();
    assert_eq!(summary.symbol_count, 8);
    assert_eq!(summary.file_count, 7);
    assert!(summary.lsp_edge_count >= 5);
    assert!(summary.precise_semantic_edge_count >= 5);
    assert!(summary.edge_kind_counts.contains_key("outgoing_call"));
    assert!(summary.edge_source_counts.contains_key("rust_analyzer_lsp"));
    assert!(summary.query_api.contains(&"search_symbols"));
    assert!(summary.query_api.contains(&"callers_of"));
    assert!(summary.query_api.contains(&"implementations_for"));
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
