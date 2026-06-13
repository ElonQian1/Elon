use std::collections::{HashMap, HashSet};

use super::model::{
    RankedFile, RankedSymbol, RepoContextIndex, SemanticQuery, SemanticQueryCoverage,
    SemanticQueryMethod, SemanticQueryPlan, SemanticQueryProvider, SymbolKind,
};

pub(crate) fn build_semantic_query_plan(
    index: &RepoContextIndex,
    max_files: usize,
    max_symbols: usize,
) -> SemanticQueryPlan {
    let file_limit = max_files.max(1).min(16);
    let symbol_limit = max_symbols.max(1).min(64);
    let mut plan = SemanticQueryPlan::default();
    let mut seen = HashSet::new();
    let mut planned_files = HashSet::new();
    let mut planned_symbols = HashSet::new();
    let ranked_symbol_by_id = index
        .graph
        .ranked_symbols
        .iter()
        .map(|symbol| (symbol.id.as_str(), symbol))
        .collect::<HashMap<_, _>>();

    for (rank, file) in index.graph.ranked_files.iter().take(file_limit).enumerate() {
        planned_files.insert(file.path.clone());
        push_file_queries(&mut plan, &mut seen, file, rank);
    }

    for (rank, symbol) in index
        .graph
        .ranked_symbols
        .iter()
        .take(symbol_limit)
        .enumerate()
    {
        planned_files.insert(symbol.path.clone());
        planned_symbols.insert(symbol.id.clone());
        push_symbol_queries(&mut plan, &mut seen, symbol, rank);
    }

    add_relationship_target_queries(
        &mut plan,
        &mut seen,
        &mut planned_files,
        &mut planned_symbols,
        &ranked_symbol_by_id,
        index,
    );

    plan.coverage = SemanticQueryCoverage {
        top_files_considered: index.graph.ranked_files.len().min(file_limit),
        top_symbols_considered: index.graph.ranked_symbols.len().min(symbol_limit),
        planned_files: planned_files.len(),
        planned_symbols: planned_symbols.len(),
        query_count: plan.queries.len(),
    };

    if !index.rust_analyzer.available {
        plan.warnings.push(
            "rust-analyzer is not available; this records intended Top-K LSP queries and keeps repo_map_tags/context_evidence as fallback"
                .to_string(),
        );
    }
    if plan.queries.is_empty() {
        plan.warnings.push(
            "no ranked files or symbols were available for semantic LSP query planning".to_string(),
        );
    }

    plan
}

fn push_file_queries(
    plan: &mut SemanticQueryPlan,
    seen: &mut HashSet<String>,
    file: &RankedFile,
    rank: usize,
) {
    let priority = priority_for_rank(rank);
    let reason = format!(
        "top ranked repo-map file score={:.2}; {}",
        file.score,
        join_reasons(&file.reasons)
    );
    push_query(
        plan,
        seen,
        SemanticQuery {
            provider: SemanticQueryProvider::RustAnalyzerLsp,
            method: SemanticQueryMethod::DocumentSymbol,
            path: file.path.clone(),
            line: 1,
            symbol: None,
            priority,
            reason: reason.clone(),
        },
    );
    push_query(
        plan,
        seen,
        SemanticQuery {
            provider: SemanticQueryProvider::RustAnalyzerLsp,
            method: SemanticQueryMethod::Diagnostic,
            path: file.path.clone(),
            line: 1,
            symbol: None,
            priority,
            reason: format!("collect diagnostics before editing {}", file.path),
        },
    );
}

fn push_symbol_queries(
    plan: &mut SemanticQueryPlan,
    seen: &mut HashSet<String>,
    symbol: &RankedSymbol,
    rank: usize,
) {
    let priority = priority_for_rank(rank);
    let reason = format!(
        "top ranked {} `{}` score={:.2}; {}",
        symbol.kind.as_str(),
        symbol.name,
        symbol.score,
        join_reasons(&symbol.reasons)
    );
    push_symbol_query(
        plan,
        seen,
        symbol,
        SemanticQueryMethod::Hover,
        priority,
        format!("confirm signature and type details for {}", reason),
    );

    if should_query_references(symbol.kind) {
        push_symbol_query(
            plan,
            seen,
            symbol,
            SemanticQueryMethod::References,
            priority,
            format!("find edit blast radius for {}", reason),
        );
    }

    if should_query_implementation(symbol.kind) {
        push_symbol_query(
            plan,
            seen,
            symbol,
            SemanticQueryMethod::Implementation,
            priority,
            format!(
                "find impl blocks and trait/type realization sites for {}",
                reason
            ),
        );
    }

    if should_query_call_hierarchy(symbol.kind) {
        push_symbol_query(
            plan,
            seen,
            symbol,
            SemanticQueryMethod::PrepareCallHierarchy,
            priority,
            format!("prepare call hierarchy item for {}", reason),
        );
        push_symbol_query(
            plan,
            seen,
            symbol,
            SemanticQueryMethod::IncomingCalls,
            priority,
            format!("find callers before modifying {}", reason),
        );
        push_symbol_query(
            plan,
            seen,
            symbol,
            SemanticQueryMethod::OutgoingCalls,
            priority,
            format!("find callees before modifying {}", reason),
        );
    }
}

fn add_relationship_target_queries(
    plan: &mut SemanticQueryPlan,
    seen: &mut HashSet<String>,
    planned_files: &mut HashSet<String>,
    planned_symbols: &mut HashSet<String>,
    ranked_symbol_by_id: &HashMap<&str, &RankedSymbol>,
    index: &RepoContextIndex,
) {
    let mut added = 0usize;
    for relationship in &index.graph.relationships {
        if added >= 12 {
            break;
        }
        let Some(symbol) = ranked_symbol_by_id.get(relationship.to_symbol_id.as_str()) else {
            continue;
        };
        if !planned_symbols.insert(symbol.id.clone()) {
            continue;
        }
        planned_files.insert(symbol.path.clone());
        push_symbol_query(
            plan,
            seen,
            symbol,
            SemanticQueryMethod::References,
            3,
            format!(
                "relationship target from {} line {}: {}",
                relationship.from_path, relationship.line, relationship.reason
            ),
        );
        added += 1;
    }
}

fn push_symbol_query(
    plan: &mut SemanticQueryPlan,
    seen: &mut HashSet<String>,
    symbol: &RankedSymbol,
    method: SemanticQueryMethod,
    priority: u8,
    reason: String,
) {
    push_query(
        plan,
        seen,
        SemanticQuery {
            provider: SemanticQueryProvider::RustAnalyzerLsp,
            method,
            path: symbol.path.clone(),
            line: symbol.line_start.max(1),
            symbol: Some(symbol.name.clone()),
            priority,
            reason,
        },
    );
}

fn push_query(plan: &mut SemanticQueryPlan, seen: &mut HashSet<String>, query: SemanticQuery) {
    let key = format!(
        "{}:{}:{}:{}:{}",
        query.provider.as_str(),
        query.method.as_lsp_method(),
        query.path,
        query.line,
        query.symbol.as_deref().unwrap_or("")
    );
    if seen.insert(key) {
        plan.queries.push(query);
    }
}

fn should_query_references(kind: SymbolKind) -> bool {
    !matches!(kind, SymbolKind::Impl | SymbolKind::Module)
}

fn should_query_implementation(kind: SymbolKind) -> bool {
    matches!(
        kind,
        SymbolKind::Trait | SymbolKind::Struct | SymbolKind::Enum | SymbolKind::TypeAlias
    )
}

fn should_query_call_hierarchy(kind: SymbolKind) -> bool {
    matches!(kind, SymbolKind::Function)
}

fn priority_for_rank(rank: usize) -> u8 {
    match rank {
        0..=2 => 1,
        3..=9 => 2,
        _ => 3,
    }
}

fn join_reasons(reasons: &[String]) -> String {
    if reasons.is_empty() {
        return "ranked by symbol graph".to_string();
    }
    reasons
        .iter()
        .take(3)
        .map(String::as_str)
        .collect::<Vec<_>>()
        .join("; ")
}
