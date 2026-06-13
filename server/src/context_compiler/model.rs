#[derive(Debug, Clone, Default, serde::Serialize)]
pub(crate) struct RepoContextIndex {
    pub(crate) task: TaskProfile,
    pub(crate) cargo: CargoIndex,
    pub(crate) rust: RustIndex,
    pub(crate) graph: SymbolGraphSummary,
    pub(crate) rust_analyzer: RustAnalyzerReport,
    pub(crate) evidence: ContextEvidence,
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
    pub(crate) warnings: Vec<String>,
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
    pub(crate) warnings: Vec<String>,
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
