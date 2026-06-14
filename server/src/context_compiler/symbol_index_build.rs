use std::collections::{BTreeMap, HashSet};

use super::{
    model::{
        CodeRelationship, RelationshipKind, RepoContextIndex, RepoMapTagEdge,
        RustAnalyzerLspLocationRole, RustAnalyzerLspStatus, RustSymbol, SemanticQueryMethod,
    },
    symbol_index::{
        normalize_path, push_source, stable_hash, SymbolEdge, SymbolIndex, SymbolRecord,
    },
};

pub(crate) fn build_symbol_index(index: &RepoContextIndex) -> SymbolIndex {
    let mut symbol_index = SymbolIndex::default();
    let parent_names = index
        .rust
        .symbols
        .iter()
        .map(|symbol| (symbol.id.clone(), symbol.name.clone()))
        .collect::<BTreeMap<_, _>>();

    for symbol in &index.rust.symbols {
        symbol_index
            .records
            .push(symbol_record(symbol, &parent_names));
    }
    symbol_index.rebuild_lookups();
    symbol_index.apply_ranked_symbols(index);
    symbol_index.add_symbol_graph_edges(&index.graph.relationships);
    symbol_index.add_repo_map_tag_edges(&index.graph.repo_map_tags.edges);
    symbol_index.add_lsp_facts(index);
    symbol_index.rebuild_edge_lookups();
    symbol_index
}

impl SymbolIndex {
    fn apply_ranked_symbols(&mut self, index: &RepoContextIndex) {
        for ranked in &index.graph.ranked_symbols {
            if let Some(record_index) = self.by_id.get(&ranked.id).copied() {
                let record = &mut self.records[record_index];
                record.importance_score = Some(ranked.score);
                push_source(record, "symbol_graph_rank");
            }
        }
    }

    fn add_symbol_graph_edges(&mut self, relationships: &[CodeRelationship]) {
        let mut seen = HashSet::new();
        for relationship in relationships {
            let from_symbol_id = self
                .find_symbol_index_at(&relationship.from_path, relationship.line, None)
                .map(|index| self.records[index].id.clone());
            self.push_edge(
                &mut seen,
                SymbolEdge {
                    id: String::new(),
                    source: "symbol_graph",
                    kind: relationship.kind.as_str().to_string(),
                    from_symbol_id,
                    from_path: normalize_path(&relationship.from_path),
                    line: relationship.line,
                    to_symbol_id: Some(relationship.to_symbol_id.clone()),
                    to_symbol_name: Some(relationship.to_symbol_name.clone()),
                    to_path: Some(normalize_path(&relationship.to_path)),
                    confidence: relationship_confidence(relationship.kind),
                    reason: relationship.reason.clone(),
                },
            );
        }
    }

    fn add_repo_map_tag_edges(&mut self, edges: &[RepoMapTagEdge]) {
        let mut seen = HashSet::new();
        for edge in edges {
            let line = edge
                .reference_lines
                .first()
                .copied()
                .unwrap_or(edge.definition_line);
            let from_symbol_id = self
                .find_symbol_index_at(&edge.from_path, line, None)
                .map(|index| self.records[index].id.clone());
            self.push_edge(
                &mut seen,
                SymbolEdge {
                    id: String::new(),
                    source: "repo_map_tags",
                    kind: "def_ref".to_string(),
                    from_symbol_id,
                    from_path: normalize_path(&edge.from_path),
                    line,
                    to_symbol_id: Some(edge.target_symbol_id.clone()),
                    to_symbol_name: Some(edge.symbol.clone()),
                    to_path: Some(normalize_path(&edge.to_path)),
                    confidence: 0.75,
                    reason: edge.reason.clone(),
                },
            );
        }
    }

    fn add_lsp_facts(&mut self, index: &RepoContextIndex) {
        let mut seen = HashSet::new();
        for result in &index.rust_analyzer.lsp.results {
            if result.status != RustAnalyzerLspStatus::Succeeded {
                continue;
            }
            let query_symbol_index =
                self.find_symbol_index_at(&result.path, result.line, result.symbol.as_deref());
            if let Some(record_index) = query_symbol_index {
                push_source(&mut self.records[record_index], lsp_source(result.method));
            }
            for location in &result.locations {
                let location_symbol_index = self.find_symbol_index_at(
                    &location.path,
                    location.line,
                    location.symbol.as_deref(),
                );
                if let Some(record_index) = location_symbol_index {
                    push_source(
                        &mut self.records[record_index],
                        lsp_location_source(location.role),
                    );
                }
                match location.role {
                    RustAnalyzerLspLocationRole::Definition => {
                        self.push_lsp_edge(
                            &mut seen,
                            "definition",
                            query_symbol_index,
                            location_symbol_index,
                            &result.path,
                            result.line,
                            &location.path,
                            location.symbol.as_deref().or(result.symbol.as_deref()),
                        );
                    }
                    RustAnalyzerLspLocationRole::Reference => {
                        self.push_lsp_edge(
                            &mut seen,
                            "reference",
                            location_symbol_index,
                            query_symbol_index,
                            &location.path,
                            location.line,
                            &result.path,
                            result.symbol.as_deref(),
                        );
                    }
                    RustAnalyzerLspLocationRole::Implementation => {
                        self.push_lsp_edge(
                            &mut seen,
                            "implementation",
                            query_symbol_index,
                            location_symbol_index,
                            &result.path,
                            result.line,
                            &location.path,
                            location.symbol.as_deref().or(result.symbol.as_deref()),
                        );
                    }
                    RustAnalyzerLspLocationRole::IncomingCaller => {
                        self.push_lsp_edge(
                            &mut seen,
                            "incoming_call",
                            location_symbol_index,
                            query_symbol_index,
                            &location.path,
                            location.line,
                            &result.path,
                            result.symbol.as_deref(),
                        );
                    }
                    RustAnalyzerLspLocationRole::OutgoingCallee => {
                        self.push_lsp_edge(
                            &mut seen,
                            "outgoing_call",
                            query_symbol_index,
                            location_symbol_index,
                            &result.path,
                            result.line,
                            &location.path,
                            location.symbol.as_deref().or(result.symbol.as_deref()),
                        );
                    }
                    RustAnalyzerLspLocationRole::DocumentSymbol
                    | RustAnalyzerLspLocationRole::WorkspaceSymbol
                    | RustAnalyzerLspLocationRole::CallHierarchyItem => {}
                }
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn push_lsp_edge(
        &mut self,
        seen: &mut HashSet<String>,
        kind: &str,
        from_index: Option<usize>,
        to_index: Option<usize>,
        from_path: &str,
        line: usize,
        to_path: &str,
        to_name: Option<&str>,
    ) {
        self.push_edge(
            seen,
            SymbolEdge {
                id: String::new(),
                source: "rust_analyzer_lsp",
                kind: kind.to_string(),
                from_symbol_id: from_index.map(|index| self.records[index].id.clone()),
                from_path: normalize_path(from_path),
                line,
                to_symbol_id: to_index.map(|index| self.records[index].id.clone()),
                to_symbol_name: to_index
                    .map(|index| self.records[index].name.clone())
                    .or_else(|| to_name.map(ToString::to_string)),
                to_path: Some(normalize_path(to_path)),
                confidence: 0.95,
                reason: "rust-analyzer LSP fact".to_string(),
            },
        );
    }
}

fn symbol_record(symbol: &RustSymbol, parent_names: &BTreeMap<String, String>) -> SymbolRecord {
    let module_path = module_path(&symbol.path);
    let parent_name = symbol
        .parent
        .as_ref()
        .and_then(|parent| parent_names.get(parent))
        .map(|value| value.as_str());
    let qualified_name = match parent_name {
        Some(parent) => format!("{module_path}::{parent}::{}", symbol.name),
        None => format!("{module_path}::{}", symbol.name),
    };
    SymbolRecord {
        id: symbol.id.clone(),
        name: symbol.name.clone(),
        qualified_name,
        kind: symbol.kind.as_str().to_string(),
        language: "rust",
        file_path: normalize_path(&symbol.path),
        start_line: symbol.line_start,
        end_line: symbol.line_end,
        signature: symbol.signature.clone(),
        visibility: symbol.visibility.as_str().to_string(),
        parent_symbol_id: symbol.parent.clone(),
        module_path,
        doc_summary: symbol.docs.clone(),
        role: symbol.role,
        importance_score: None,
        signature_hash: stable_hash(&symbol.signature),
        source_providers: vec!["rust_symbols".to_string()],
    }
}

fn module_path(path: &str) -> String {
    let path = normalize_path(path);
    let mut module = path
        .split_once("/src/")
        .map(|(_, rest)| rest)
        .or_else(|| path.strip_prefix("src/"))
        .unwrap_or(&path)
        .trim_end_matches(".rs")
        .replace('/', "::");
    if module == "lib" || module == "main" {
        module.clear();
    }
    if let Some(stripped) = module.strip_suffix("::mod") {
        module = stripped.to_string();
    }
    if module.is_empty() {
        "crate".to_string()
    } else {
        format!("crate::{module}")
    }
}

fn relationship_confidence(kind: RelationshipKind) -> f32 {
    match kind {
        RelationshipKind::TestCovers => 0.8,
        RelationshipKind::ImplReference | RelationshipKind::TraitReference => 0.7,
        RelationshipKind::TypeReference => 0.65,
        RelationshipKind::CallsOrMentions => 0.55,
    }
}

fn lsp_source(method: SemanticQueryMethod) -> &'static str {
    match method {
        SemanticQueryMethod::DocumentSymbol => "rust_analyzer_lsp:document_symbol",
        SemanticQueryMethod::WorkspaceSymbol => "rust_analyzer_lsp:workspace_symbol",
        SemanticQueryMethod::Definition => "rust_analyzer_lsp:definition",
        SemanticQueryMethod::References => "rust_analyzer_lsp:references",
        SemanticQueryMethod::Implementation => "rust_analyzer_lsp:implementation",
        SemanticQueryMethod::Hover => "rust_analyzer_lsp:hover",
        SemanticQueryMethod::PrepareCallHierarchy
        | SemanticQueryMethod::IncomingCalls
        | SemanticQueryMethod::OutgoingCalls => "rust_analyzer_lsp:call_hierarchy",
        SemanticQueryMethod::Diagnostic => "rust_analyzer_lsp:diagnostic",
    }
}

fn lsp_location_source(role: RustAnalyzerLspLocationRole) -> &'static str {
    match role {
        RustAnalyzerLspLocationRole::DocumentSymbol => "rust_analyzer_lsp:document_symbol",
        RustAnalyzerLspLocationRole::WorkspaceSymbol => "rust_analyzer_lsp:workspace_symbol",
        RustAnalyzerLspLocationRole::Definition => "rust_analyzer_lsp:definition",
        RustAnalyzerLspLocationRole::Reference => "rust_analyzer_lsp:references",
        RustAnalyzerLspLocationRole::Implementation => "rust_analyzer_lsp:implementation",
        RustAnalyzerLspLocationRole::IncomingCaller
        | RustAnalyzerLspLocationRole::OutgoingCallee
        | RustAnalyzerLspLocationRole::CallHierarchyItem => "rust_analyzer_lsp:call_hierarchy",
    }
}
