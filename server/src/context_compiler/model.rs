#[derive(Debug, Clone, Default, serde::Serialize)]
pub(crate) struct RepoContextIndex {
    pub(crate) task: TaskProfile,
    pub(crate) cargo: CargoIndex,
    pub(crate) rust: RustIndex,
    pub(crate) graph: SymbolGraphSummary,
    pub(crate) rust_analyzer: RustAnalyzerReport,
    pub(crate) semantic_plan: SemanticQueryPlan,
    pub(crate) impact: RustImpactAnalysis,
    pub(crate) evidence: ContextEvidence,
    pub(crate) quality: ContextQualityReport,
}

#[derive(Debug, Clone, Default, serde::Serialize)]
pub(crate) struct TaskProfile {
    pub(crate) keywords: Vec<String>,
    pub(crate) likely_domains: Vec<String>,
    pub(crate) suspected_symbols: Vec<String>,
    pub(crate) suspected_files: Vec<String>,
    pub(crate) action_hints: Vec<String>,
}

#[derive(Debug, Clone, Default, serde::Serialize)]
pub(crate) struct CargoIndex {
    pub(crate) manifest_path: Option<String>,
    pub(crate) workspace_root: Option<String>,
    pub(crate) packages: Vec<CargoPackageSummary>,
    pub(crate) warnings: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub(crate) struct CargoPackageSummary {
    pub(crate) name: String,
    pub(crate) version: String,
    pub(crate) manifest_path: String,
    pub(crate) targets: Vec<String>,
    pub(crate) target_paths: Vec<String>,
    pub(crate) features: Vec<String>,
}

#[derive(Debug, Clone, Default, serde::Serialize)]
pub(crate) struct RustIndex {
    pub(crate) files_scanned: usize,
    pub(crate) symbols: Vec<RustSymbol>,
    pub(crate) imports: Vec<RustImport>,
    pub(crate) warnings: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub(crate) struct RustImport {
    pub(crate) path: String,
    pub(crate) line: usize,
    pub(crate) imported_path: String,
    pub(crate) alias: Option<String>,
    pub(crate) public: bool,
    pub(crate) glob: bool,
    pub(crate) raw: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub(crate) struct RustSymbol {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) kind: SymbolKind,
    pub(crate) path: String,
    pub(crate) line_start: usize,
    pub(crate) line_end: usize,
    pub(crate) visibility: SymbolVisibility,
    pub(crate) signature: String,
    pub(crate) parent: Option<String>,
    pub(crate) docs: Option<String>,
    pub(crate) role: &'static str,
    pub(crate) safety_notes: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum SymbolKind {
    Module,
    Struct,
    Enum,
    Trait,
    Impl,
    Function,
    TypeAlias,
    Const,
    Static,
    Macro,
}

impl SymbolKind {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Module => "module",
            Self::Struct => "struct",
            Self::Enum => "enum",
            Self::Trait => "trait",
            Self::Impl => "impl",
            Self::Function => "function",
            Self::TypeAlias => "type_alias",
            Self::Const => "const",
            Self::Static => "static",
            Self::Macro => "macro",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum SymbolVisibility {
    Public,
    Crate,
    Private,
}

impl SymbolVisibility {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Public => "pub",
            Self::Crate => "pub(crate)",
            Self::Private => "private",
        }
    }
}

#[derive(Debug, Clone, Default, serde::Serialize)]
pub(crate) struct SymbolGraphSummary {
    pub(crate) ranked_files: Vec<RankedFile>,
    pub(crate) ranked_symbols: Vec<RankedSymbol>,
    pub(crate) relationships: Vec<CodeRelationship>,
    pub(crate) repo_map_tags: RepoMapTagSummary,
    pub(crate) warnings: Vec<String>,
}

#[derive(Debug, Clone, Default, serde::Serialize)]
pub(crate) struct RepoMapTagSummary {
    pub(crate) definitions: usize,
    pub(crate) references: usize,
    pub(crate) edges: Vec<RepoMapTagEdge>,
    pub(crate) warnings: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub(crate) struct RepoMapTagEdge {
    pub(crate) symbol: String,
    pub(crate) target_symbol_id: String,
    pub(crate) from_path: String,
    pub(crate) to_path: String,
    pub(crate) definition_line: usize,
    pub(crate) reference_lines: Vec<usize>,
    pub(crate) references: usize,
    pub(crate) score: f64,
    pub(crate) reason: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub(crate) struct RankedFile {
    pub(crate) path: String,
    pub(crate) role: &'static str,
    pub(crate) score: f64,
    pub(crate) symbol_count: usize,
    pub(crate) top_symbols: Vec<String>,
    pub(crate) reasons: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub(crate) struct RankedSymbol {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) kind: SymbolKind,
    pub(crate) path: String,
    pub(crate) line_start: usize,
    pub(crate) line_end: usize,
    pub(crate) score: f64,
    pub(crate) reasons: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub(crate) struct CodeRelationship {
    pub(crate) from_path: String,
    pub(crate) to_symbol_id: String,
    pub(crate) to_symbol_name: String,
    pub(crate) to_path: String,
    pub(crate) kind: RelationshipKind,
    pub(crate) line: usize,
    pub(crate) reason: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum RelationshipKind {
    CallsOrMentions,
    TypeReference,
    TraitReference,
    ImplReference,
    TestCovers,
}

impl RelationshipKind {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::CallsOrMentions => "calls_or_mentions",
            Self::TypeReference => "type_reference",
            Self::TraitReference => "trait_reference",
            Self::ImplReference => "impl_reference",
            Self::TestCovers => "test_covers",
        }
    }
}

#[derive(Debug, Clone, Default, serde::Serialize)]
pub(crate) struct RustAnalyzerReport {
    pub(crate) available: bool,
    pub(crate) version: Option<String>,
    pub(crate) files_enhanced: usize,
    pub(crate) symbols: Vec<RustAnalyzerSymbol>,
    pub(crate) enhancement_targets: Vec<String>,
    pub(crate) probes: RustAnalyzerProbeReport,
    pub(crate) lsp: RustAnalyzerLspReport,
    pub(crate) warnings: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub(crate) struct RustAnalyzerSymbol {
    pub(crate) path: String,
    pub(crate) label: String,
    pub(crate) kind: String,
    pub(crate) detail: Option<String>,
    pub(crate) line: usize,
    pub(crate) parent: Option<String>,
}

#[derive(Debug, Clone, Default, serde::Serialize)]
pub(crate) struct RustAnalyzerProbeReport {
    pub(crate) enabled: bool,
    pub(crate) workspace_path: Option<String>,
    pub(crate) commands: Vec<RustAnalyzerCommandProbe>,
    pub(crate) warnings: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub(crate) struct RustAnalyzerCommandProbe {
    pub(crate) name: String,
    pub(crate) command: String,
    pub(crate) status: RustAnalyzerProbeStatus,
    pub(crate) duration_ms: u64,
    pub(crate) exit_code: Option<i32>,
    pub(crate) findings: Vec<RustAnalyzerFinding>,
    pub(crate) stdout_excerpt: Vec<String>,
    pub(crate) stderr_excerpt: Vec<String>,
    pub(crate) warning: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum RustAnalyzerProbeStatus {
    Succeeded,
    Failed,
    TimedOut,
    Skipped,
}

impl RustAnalyzerProbeStatus {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::TimedOut => "timed_out",
            Self::Skipped => "skipped",
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub(crate) struct RustAnalyzerFinding {
    pub(crate) path: Option<String>,
    pub(crate) line: Option<usize>,
    pub(crate) severity: Option<String>,
    pub(crate) message: String,
    pub(crate) evidence: String,
}

#[derive(Debug, Clone, Default, serde::Serialize)]
pub(crate) struct RustAnalyzerLspReport {
    pub(crate) enabled: bool,
    pub(crate) workspace_path: Option<String>,
    pub(crate) attempted: usize,
    pub(crate) succeeded: usize,
    pub(crate) failed: usize,
    pub(crate) skipped: usize,
    pub(crate) timed_out: usize,
    pub(crate) results: Vec<RustAnalyzerLspQueryResult>,
    pub(crate) warnings: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub(crate) struct RustAnalyzerLspQueryResult {
    pub(crate) method: SemanticQueryMethod,
    pub(crate) path: String,
    pub(crate) line: usize,
    pub(crate) symbol: Option<String>,
    pub(crate) status: RustAnalyzerLspStatus,
    pub(crate) duration_ms: u64,
    pub(crate) summary: Option<String>,
    pub(crate) locations: Vec<RustAnalyzerLspLocation>,
    pub(crate) warning: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub(crate) struct RustAnalyzerLspLocation {
    pub(crate) role: RustAnalyzerLspLocationRole,
    pub(crate) path: String,
    pub(crate) line: usize,
    pub(crate) end_line: Option<usize>,
    pub(crate) symbol: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum RustAnalyzerLspLocationRole {
    DocumentSymbol,
    WorkspaceSymbol,
    Definition,
    Reference,
    Implementation,
    IncomingCaller,
    OutgoingCallee,
    CallHierarchyItem,
}

impl RustAnalyzerLspLocationRole {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::DocumentSymbol => "document_symbol",
            Self::WorkspaceSymbol => "workspace_symbol",
            Self::Definition => "definition",
            Self::Reference => "reference",
            Self::Implementation => "implementation",
            Self::IncomingCaller => "incoming_caller",
            Self::OutgoingCallee => "outgoing_callee",
            Self::CallHierarchyItem => "call_hierarchy_item",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum RustAnalyzerLspStatus {
    Succeeded,
    Failed,
    TimedOut,
    Skipped,
}

impl RustAnalyzerLspStatus {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::TimedOut => "timed_out",
            Self::Skipped => "skipped",
        }
    }
}

#[derive(Debug, Clone, Default, serde::Serialize)]
pub(crate) struct SemanticQueryPlan {
    pub(crate) coverage: SemanticQueryCoverage,
    pub(crate) queries: Vec<SemanticQuery>,
    pub(crate) warnings: Vec<String>,
}

#[derive(Debug, Clone, Default, serde::Serialize)]
pub(crate) struct SemanticQueryCoverage {
    pub(crate) top_files_considered: usize,
    pub(crate) top_symbols_considered: usize,
    pub(crate) planned_files: usize,
    pub(crate) planned_symbols: usize,
    pub(crate) query_count: usize,
}

#[derive(Debug, Clone, serde::Serialize)]
pub(crate) struct SemanticQuery {
    pub(crate) provider: SemanticQueryProvider,
    pub(crate) method: SemanticQueryMethod,
    pub(crate) path: String,
    pub(crate) line: usize,
    pub(crate) symbol: Option<String>,
    pub(crate) priority: u8,
    pub(crate) reason: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum SemanticQueryProvider {
    RustAnalyzerLsp,
}

impl SemanticQueryProvider {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::RustAnalyzerLsp => "rust_analyzer_lsp",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum SemanticQueryMethod {
    DocumentSymbol,
    WorkspaceSymbol,
    Diagnostic,
    Definition,
    References,
    Implementation,
    Hover,
    PrepareCallHierarchy,
    IncomingCalls,
    OutgoingCalls,
}

impl SemanticQueryMethod {
    pub(crate) fn as_lsp_method(self) -> &'static str {
        match self {
            Self::DocumentSymbol => "textDocument/documentSymbol",
            Self::WorkspaceSymbol => "workspace/symbol",
            Self::Diagnostic => "textDocument/diagnostic",
            Self::Definition => "textDocument/definition",
            Self::References => "textDocument/references",
            Self::Implementation => "textDocument/implementation",
            Self::Hover => "textDocument/hover",
            Self::PrepareCallHierarchy => "textDocument/prepareCallHierarchy",
            Self::IncomingCalls => "callHierarchy/incomingCalls",
            Self::OutgoingCalls => "callHierarchy/outgoingCalls",
        }
    }
}

#[derive(Debug, Clone, Default, serde::Serialize)]
pub(crate) struct RustImpactAnalysis {
    pub(crate) trait_implementations: Vec<ImpactFact>,
    pub(crate) function_call_sites: Vec<ImpactFact>,
    pub(crate) enum_match_sites: Vec<ImpactFact>,
    pub(crate) field_accesses: Vec<ImpactFact>,
    pub(crate) public_api_references: Vec<ImpactFact>,
    pub(crate) test_links: Vec<ImpactFact>,
    pub(crate) async_boundaries: Vec<ImpactFact>,
    pub(crate) limitations: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub(crate) struct ImpactFact {
    pub(crate) subject: String,
    pub(crate) path: String,
    pub(crate) line: usize,
    pub(crate) kind: ImpactKind,
    pub(crate) evidence: String,
    pub(crate) reason: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ImpactKind {
    TraitImplementation,
    FunctionCallSite,
    EnumMatchSite,
    FieldRead,
    FieldWrite,
    PublicApiReference,
    TestLink,
    AsyncBoundary,
}

impl ImpactKind {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::TraitImplementation => "trait_implementation",
            Self::FunctionCallSite => "function_call_site",
            Self::EnumMatchSite => "enum_match_site",
            Self::FieldRead => "field_read",
            Self::FieldWrite => "field_write",
            Self::PublicApiReference => "public_api_reference",
            Self::TestLink => "test_link",
            Self::AsyncBoundary => "async_boundary",
        }
    }
}

#[derive(Debug, Clone, Default, serde::Serialize)]
pub(crate) struct ContextEvidence {
    pub(crate) snippets: Vec<EvidenceSnippet>,
    pub(crate) neighbor_summaries: Vec<NeighborSummary>,
    pub(crate) test_targets: Vec<TestTarget>,
    pub(crate) build_commands: Vec<BuildCommand>,
    pub(crate) invariants: Vec<ContextFact>,
    pub(crate) public_api_contracts: Vec<ContextFact>,
    pub(crate) unsafe_boundaries: Vec<ContextFact>,
    pub(crate) feature_flags: Vec<FeatureFlagFact>,
    pub(crate) missing_context: Vec<String>,
    pub(crate) recommended_actions: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub(crate) struct EvidenceSnippet {
    pub(crate) id: String,
    pub(crate) path: String,
    pub(crate) role: &'static str,
    pub(crate) symbols: Vec<String>,
    pub(crate) line_start: usize,
    pub(crate) line_end: usize,
    pub(crate) sha256: String,
    pub(crate) reason: String,
    pub(crate) content: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub(crate) struct NeighborSummary {
    pub(crate) path: String,
    pub(crate) relationship: RelationshipKind,
    pub(crate) symbols: Vec<String>,
    pub(crate) reason: String,
    pub(crate) needed_if: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub(crate) struct TestTarget {
    pub(crate) path: String,
    pub(crate) reason: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub(crate) struct BuildCommand {
    pub(crate) command: String,
    pub(crate) reason: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub(crate) struct ContextFact {
    pub(crate) subject: String,
    pub(crate) path: String,
    pub(crate) line_start: usize,
    pub(crate) line_end: usize,
    pub(crate) detail: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub(crate) struct FeatureFlagFact {
    pub(crate) package: String,
    pub(crate) feature: String,
    pub(crate) manifest_path: String,
}

#[derive(Debug, Clone, Default, serde::Serialize)]
pub(crate) struct ContextQualityReport {
    pub(crate) score: u8,
    pub(crate) coverage: ContextQualityCoverage,
    pub(crate) semantic: ContextQualitySemantic,
    pub(crate) gaps: Vec<ContextQualityGap>,
    pub(crate) recommended_actions: Vec<String>,
}

#[derive(Debug, Clone, Default, serde::Serialize)]
pub(crate) struct ContextQualityCoverage {
    pub(crate) top_files_considered: usize,
    pub(crate) top_files_with_snippets: usize,
    pub(crate) top_symbols_considered: usize,
    pub(crate) top_symbols_with_snippets: usize,
    pub(crate) snippet_count: usize,
    pub(crate) relationship_count: usize,
    pub(crate) repo_map_tag_edges: usize,
    pub(crate) impact_fact_count: usize,
    pub(crate) validation_commands: usize,
}

#[derive(Debug, Clone, Default, serde::Serialize)]
pub(crate) struct ContextQualitySemantic {
    pub(crate) rust_analyzer_available: bool,
    pub(crate) rust_analyzer_symbols: usize,
    pub(crate) rust_analyzer_files_enhanced: usize,
    pub(crate) lsp_queries_planned: usize,
    pub(crate) lsp_enabled: bool,
    pub(crate) lsp_attempted: usize,
    pub(crate) lsp_succeeded: usize,
    pub(crate) lsp_locations: usize,
    pub(crate) lsp_failed: usize,
    pub(crate) lsp_timed_out: usize,
    pub(crate) probe_enabled: bool,
    pub(crate) probe_succeeded: usize,
    pub(crate) probe_failed: usize,
    pub(crate) probe_timed_out: usize,
}

#[derive(Debug, Clone, serde::Serialize)]
pub(crate) struct ContextQualityGap {
    pub(crate) severity: ContextQualitySeverity,
    pub(crate) subject: String,
    pub(crate) path: Option<String>,
    pub(crate) line: Option<usize>,
    pub(crate) detail: String,
    pub(crate) action: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ContextQualitySeverity {
    Info,
    Warning,
    Critical,
}

impl ContextQualitySeverity {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Warning => "warning",
            Self::Critical => "critical",
        }
    }
}
