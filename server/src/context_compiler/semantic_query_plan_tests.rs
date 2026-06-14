use super::{
    model::{
        CodeRelationship, RankedFile, RankedSymbol, RelationshipKind, RepoContextIndex,
        RustAnalyzerReport, SemanticQueryMethod, SymbolGraphSummary, SymbolKind,
    },
    semantic_query_plan::build_semantic_query_plan,
};

#[test]
fn builds_top_k_lsp_queries_for_ranked_files_and_symbols() {
    let index = RepoContextIndex {
        graph: SymbolGraphSummary {
            ranked_files: vec![RankedFile {
                path: "src/lib.rs".to_string(),
                role: "source",
                score: 9.5,
                symbol_count: 3,
                top_symbols: vec!["Runner".to_string(), "run".to_string()],
                reasons: vec!["task term hits".to_string()],
            }],
            ranked_symbols: vec![
                ranked("src/lib.rs:1:trait:Runner", "Runner", SymbolKind::Trait, 1),
                ranked("src/lib.rs:5:function:run", "run", SymbolKind::Function, 5),
                ranked("src/lib.rs:9:struct:Job", "Job", SymbolKind::Struct, 9),
            ],
            relationships: vec![CodeRelationship {
                from_path: "src/caller.rs".to_string(),
                to_symbol_id: "src/lib.rs:5:function:run".to_string(),
                to_symbol_name: "run".to_string(),
                to_path: "src/lib.rs".to_string(),
                kind: RelationshipKind::CallsOrMentions,
                line: 12,
                reason: "caller mentions run".to_string(),
            }],
            ..SymbolGraphSummary::default()
        },
        rust_analyzer: RustAnalyzerReport {
            available: true,
            ..RustAnalyzerReport::default()
        },
        ..RepoContextIndex::default()
    };

    let plan = build_semantic_query_plan(&index, 2, 3);

    assert_eq!(plan.coverage.planned_files, 1);
    assert!(has_method(&plan, None, SemanticQueryMethod::DocumentSymbol));
    assert!(has_method(
        &plan,
        Some("Runner"),
        SemanticQueryMethod::WorkspaceSymbol
    ));
    assert!(has_method(
        &plan,
        Some("Runner"),
        SemanticQueryMethod::Definition
    ));
    assert!(has_method(
        &plan,
        Some("Runner"),
        SemanticQueryMethod::Implementation
    ));
    assert!(has_method(
        &plan,
        Some("run"),
        SemanticQueryMethod::References
    ));
    assert!(has_method(
        &plan,
        Some("run"),
        SemanticQueryMethod::IncomingCalls
    ));
    assert!(has_method(
        &plan,
        Some("run"),
        SemanticQueryMethod::OutgoingCalls
    ));
    assert!(plan.warnings.is_empty());
}

#[test]
fn records_fallback_warning_when_rust_analyzer_is_unavailable() {
    let index = RepoContextIndex {
        graph: SymbolGraphSummary {
            ranked_symbols: vec![ranked(
                "src/lib.rs:1:function:build",
                "build",
                SymbolKind::Function,
                1,
            )],
            ..SymbolGraphSummary::default()
        },
        ..RepoContextIndex::default()
    };

    let plan = build_semantic_query_plan(&index, 1, 1);

    assert!(plan
        .warnings
        .iter()
        .any(|warning| warning.contains("rust-analyzer is not available")));
    assert!(has_method(&plan, Some("build"), SemanticQueryMethod::Hover));
    assert!(has_method(
        &plan,
        Some("build"),
        SemanticQueryMethod::WorkspaceSymbol
    ));
    assert!(has_method(
        &plan,
        Some("build"),
        SemanticQueryMethod::Definition
    ));
}

fn ranked(id: &str, name: &str, kind: SymbolKind, line_start: usize) -> RankedSymbol {
    RankedSymbol {
        id: id.to_string(),
        name: name.to_string(),
        kind,
        path: "src/lib.rs".to_string(),
        line_start,
        line_end: line_start,
        score: 10.0,
        reasons: vec!["test fixture".to_string()],
    }
}

fn has_method(
    plan: &super::model::SemanticQueryPlan,
    symbol: Option<&str>,
    method: SemanticQueryMethod,
) -> bool {
    plan.queries
        .iter()
        .any(|query| query.symbol.as_deref() == symbol && query.method == method)
}
