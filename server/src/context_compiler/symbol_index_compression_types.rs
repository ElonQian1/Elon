use std::collections::BTreeMap;

use serde::Serialize;

use super::symbol_index_ranker::RerankDecision;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum CompressionLevel {
    Drop,
    RelationOnly,
    SignatureOnly,
    SummaryAndSignature,
    FocusedSnippet,
    FullSymbolBody,
    FullFile,
}

impl CompressionLevel {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            CompressionLevel::Drop => "drop",
            CompressionLevel::RelationOnly => "relation_only",
            CompressionLevel::SignatureOnly => "signature_only",
            CompressionLevel::SummaryAndSignature => "summary_and_signature",
            CompressionLevel::FocusedSnippet => "focused_snippet",
            CompressionLevel::FullSymbolBody => "full_symbol_body",
            CompressionLevel::FullFile => "full_file",
        }
    }

    pub(crate) fn downgrade(self) -> Self {
        match self {
            CompressionLevel::FullFile => CompressionLevel::FullSymbolBody,
            CompressionLevel::FullSymbolBody => CompressionLevel::FocusedSnippet,
            CompressionLevel::FocusedSnippet => CompressionLevel::SummaryAndSignature,
            CompressionLevel::SummaryAndSignature => CompressionLevel::SignatureOnly,
            CompressionLevel::SignatureOnly => CompressionLevel::RelationOnly,
            CompressionLevel::RelationOnly | CompressionLevel::Drop => CompressionLevel::Drop,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SymbolCompressedContext {
    pub(crate) budget_tokens: usize,
    pub(crate) used_tokens: usize,
    pub(crate) original_tokens: usize,
    pub(crate) saved_tokens: usize,
    pub(crate) dropped_count: usize,
    pub(crate) blocks: Vec<CompressedContextBlock>,
    pub(crate) level_counts: BTreeMap<String, usize>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CompressedContextBlock {
    pub(crate) rank: usize,
    pub(crate) id: String,
    pub(crate) title: String,
    pub(crate) file_path: String,
    pub(crate) symbol_id: Option<String>,
    pub(crate) source: String,
    pub(crate) sources: Vec<String>,
    pub(crate) decision: RerankDecision,
    pub(crate) level: CompressionLevel,
    pub(crate) original_tokens: usize,
    pub(crate) compressed_tokens: usize,
    pub(crate) content: String,
    pub(crate) reasons: Vec<String>,
}
