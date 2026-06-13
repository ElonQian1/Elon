use super::{
    context_quality::build_context_quality_report,
    model::{
        ContextEvidence, EvidenceSnippet, RankedFile, RankedSymbol, RepoContextIndex,
        RustAnalyzerReport, SemanticQuery, SemanticQueryCoverage, SemanticQueryMethod,
        SemanticQueryPlan, SemanticQueryProvider, SymbolGraphSummary, SymbolKind,
    },
    validation::ValidationPlan,
};

#[test]
fn scores_context_higher_when_top_symbols_have_evidence_and_semantic_plan() {
    let index = RepoContextIndex {
        graph: SymbolGraphSummary {
            ranked_files: vec![RankedFile {
                path: "src/lib.rs".to_string(),
                role: "source",
                score: 10.0,
                symbol_count: 1,
                top_symbols: vec!["build".to_string()],
                reasons: vec!["task term hit".to_string()],
            }],
            ranked_symbols: vec![ranked("src/lib.rs:1:function:build", "build")],
            ..SymbolGraphSummary::default()
        },
        rust_analyzer: RustAnalyzerReport {
            available: true,
            files_enhanced: 1,
            ..RustAnalyzerReport::default()
        },
        semantic_plan: SemanticQueryPlan {
            coverage: SemanticQueryCoverage {
                query_count: 1,
                ..SemanticQueryCoverage::default()
            },
            queries: vec![SemanticQuery {
                provider: SemanticQueryProvider::RustAnalyzerLsp,
                method: SemanticQueryMethod::References,
                path: "src/lib.rs".to_string(),
                line: 1,
                symbol: Some("build".to_string()),
                priority: 1,
                reason: "test".to_string(),
            }],
            warnings: Vec::new(),
        },
        evidence: ContextEvidence {
            snippets: vec![snippet("src/lib.rs:1:function:build")],
            ..ContextEvidence::default()
        },
        ..RepoContextIndex::default()
    };
    let validation = ValidationPlan {
        commands: vec![super::validation::ValidationCommand {
            command: "cargo check".to_string(),
            reason: "test fixture".to_string(),
            required: true,
        }],
        notes: Vec::new(),
    };

    let report = build_context_quality_report(&index, &[], &validation);

    assert!(report.score >= 80);
    assert_eq!(report.coverage.top_symbols_with_snippets, 1);
    assert!(report
        .recommended_actions
        .iter()
        .any(|action| action.contains("semantic_query_plan")));
}

#[test]
fn records_quality_gaps_for_uncovered_symbols_and_missing_rust_analyzer() {
    let index = RepoContextIndex {
        graph: SymbolGraphSummary {
            ranked_symbols: vec![ranked("src/lib.rs:1:function:build", "build")],
            ..SymbolGraphSummary::default()
        },
        ..RepoContextIndex::default()
    };
    let validation = ValidationPlan {
        commands: Vec::new(),
        notes: Vec::new(),
    };

    let report = build_context_quality_report(&index, &[], &validation);

    assert!(report.score < 80);
    assert!(report
        .gaps
        .iter()
        .any(|gap| gap.subject.contains("symbol build")));
    assert!(report.gaps.iter().any(|gap| gap.subject == "rust_analyzer"));
}

fn ranked(id: &str, name: &str) -> RankedSymbol {
    RankedSymbol {
        id: id.to_string(),
        name: name.to_string(),
        kind: SymbolKind::Function,
        path: "src/lib.rs".to_string(),
        line_start: 1,
        line_end: 3,
        score: 10.0,
        reasons: vec!["test fixture".to_string()],
    }
}

fn snippet(symbol_id: &str) -> EvidenceSnippet {
    EvidenceSnippet {
        id: "snippet-1".to_string(),
        path: "src/lib.rs".to_string(),
        role: "edit_target",
        symbols: vec![symbol_id.to_string()],
        line_start: 1,
        line_end: 3,
        sha256: "hash".to_string(),
        reason: "test fixture".to_string(),
        content: "fn build() {}".to_string(),
    }
}
