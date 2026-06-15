use std::collections::{BTreeMap, BTreeSet};

use super::{
    symbol_index_chunks::SymbolChunkHit,
    symbol_index_compression_types::{
        CompressedContextBlock, CompressionLevel, SymbolCompressedContext,
    },
    symbol_index_impact_types::SymbolImpactResponse,
    symbol_index_query_types::SymbolHit,
    symbol_index_ranker::{RankedContextItem, RerankDecision},
    symbol_index_retrieval_plan::{QueryIntent, RetrievalPlan},
    symbol_index_vector_types::SymbolVectorHit,
};

const MIN_COMPRESSION_BUDGET_TOKENS: usize = 250;
const CHARS_PER_TOKEN: usize = 4;

struct CompressionFacts {
    symbols: BTreeMap<String, SymbolHit>,
    chunks: BTreeMap<String, SymbolChunkHit>,
    vectors: BTreeMap<String, SymbolVectorHit>,
}

pub(crate) fn compression_budget_tokens(max_chars: usize) -> usize {
    (max_chars / CHARS_PER_TOKEN).max(MIN_COMPRESSION_BUDGET_TOKENS)
}

pub(crate) fn compress_symbol_context(
    ranked: &[RankedContextItem],
    symbols: &[SymbolHit],
    impact: &SymbolImpactResponse,
    text_chunks: &[SymbolChunkHit],
    vector_chunks: &[SymbolVectorHit],
    plan: &RetrievalPlan,
    max_chars: usize,
) -> SymbolCompressedContext {
    let budget_tokens = compression_budget_tokens(max_chars);
    let facts = CompressionFacts::new(symbols, impact, text_chunks, vector_chunks);
    let mut blocks = ranked
        .iter()
        .take(40)
        .map(|item| build_block(item, &facts, plan))
        .collect::<Vec<_>>();
    degrade_to_budget(&mut blocks, &facts, plan, budget_tokens);

    let used_tokens = blocks
        .iter()
        .filter(|block| block.level != CompressionLevel::Drop)
        .map(|block| block.compressed_tokens)
        .sum::<usize>();
    let original_tokens = blocks
        .iter()
        .map(|block| block.original_tokens.max(block.compressed_tokens))
        .sum::<usize>();
    let dropped_count = blocks
        .iter()
        .filter(|block| block.level == CompressionLevel::Drop)
        .count();
    let level_counts = level_counts(&blocks);

    SymbolCompressedContext {
        budget_tokens,
        used_tokens,
        original_tokens,
        saved_tokens: original_tokens.saturating_sub(used_tokens),
        dropped_count,
        blocks,
        level_counts,
    }
}

impl CompressionFacts {
    fn new(
        symbols: &[SymbolHit],
        impact: &SymbolImpactResponse,
        text_chunks: &[SymbolChunkHit],
        vector_chunks: &[SymbolVectorHit],
    ) -> Self {
        let mut symbol_map = BTreeMap::new();
        for symbol in symbols
            .iter()
            .chain(impact.seed_symbols.iter())
            .chain(impact.impacted_symbols.iter())
        {
            symbol_map
                .entry(symbol.id.clone())
                .or_insert(symbol.clone());
        }
        Self {
            symbols: symbol_map,
            chunks: text_chunks
                .iter()
                .map(|chunk| (chunk.id.clone(), chunk.clone()))
                .collect(),
            vectors: vector_chunks
                .iter()
                .map(|chunk| (chunk.id.clone(), chunk.clone()))
                .collect(),
        }
    }

    fn symbol(&self, item: &RankedContextItem) -> Option<&SymbolHit> {
        item.symbol_id
            .as_deref()
            .and_then(|symbol_id| self.symbols.get(symbol_id))
    }

    fn chunk(&self, item: &RankedContextItem) -> Option<&SymbolChunkHit> {
        item.id
            .strip_prefix("chunk:")
            .and_then(|id| self.chunks.get(id))
            .or_else(|| {
                item.symbol_id.as_deref().and_then(|symbol_id| {
                    self.chunks
                        .values()
                        .find(|chunk| chunk.symbol_id.as_deref() == Some(symbol_id))
                })
            })
    }

    fn vector(&self, item: &RankedContextItem) -> Option<&SymbolVectorHit> {
        item.id
            .strip_prefix("vector:")
            .and_then(|id| self.vectors.get(id))
            .or_else(|| {
                item.symbol_id.as_deref().and_then(|symbol_id| {
                    self.vectors
                        .values()
                        .find(|chunk| chunk.symbol_id.as_deref() == Some(symbol_id))
                })
            })
    }
}

fn build_block(
    item: &RankedContextItem,
    facts: &CompressionFacts,
    plan: &RetrievalPlan,
) -> CompressedContextBlock {
    let mut reasons = vec![format!("rank={}", item.rank)];
    reasons.extend(item.reasons.iter().take(4).cloned());
    let level = choose_level(item, facts, plan, &mut reasons);
    block_with_level(item, facts, level, reasons)
}

fn block_with_level(
    item: &RankedContextItem,
    facts: &CompressionFacts,
    level: CompressionLevel,
    mut reasons: Vec<String>,
) -> CompressedContextBlock {
    if level == CompressionLevel::Drop {
        reasons.push("compression_budget_or_rerank_drop".to_string());
    }
    let content = content_for_level(item, facts, level);
    let compressed_tokens = estimate_tokens(&content);
    CompressedContextBlock {
        rank: item.rank,
        id: item.id.clone(),
        title: item.label.clone(),
        file_path: item.file_path.clone(),
        symbol_id: item.symbol_id.clone(),
        source: item.source.clone(),
        sources: item.sources.clone(),
        decision: item.decision,
        level,
        original_tokens: item.token_count.max(compressed_tokens),
        compressed_tokens,
        content,
        reasons: compact_strings(reasons),
    }
}

fn choose_level(
    item: &RankedContextItem,
    facts: &CompressionFacts,
    plan: &RetrievalPlan,
    reasons: &mut Vec<String>,
) -> CompressionLevel {
    if item.decision == RerankDecision::Drop {
        reasons.push("reranker_drop".to_string());
        return CompressionLevel::Drop;
    }
    let has_chunk = facts.chunk(item).is_some() || facts.vector(item).is_some();
    let has_symbol = facts.symbol(item).is_some() || item.symbol_id.is_some();
    match item.decision {
        RerankDecision::MustInclude => {
            reasons.push(format!("intent={}", plan.intent.as_str()));
            if facts
                .chunk(item)
                .is_some_and(|chunk| chunk.chunk_type == "module")
            {
                CompressionLevel::FullFile
            } else if has_chunk || has_symbol {
                CompressionLevel::FullSymbolBody
            } else {
                CompressionLevel::SummaryAndSignature
            }
        }
        RerankDecision::Include => include_level(item, has_chunk, has_symbol, plan.intent, reasons),
        RerankDecision::Summarize => {
            reasons.push("reranker_summarize".to_string());
            if has_symbol {
                CompressionLevel::SummaryAndSignature
            } else {
                CompressionLevel::RelationOnly
            }
        }
        RerankDecision::Drop => CompressionLevel::Drop,
    }
}

fn include_level(
    item: &RankedContextItem,
    has_chunk: bool,
    has_symbol: bool,
    intent: QueryIntent,
    reasons: &mut Vec<String>,
) -> CompressionLevel {
    if item.is_test_context && matches!(intent, QueryIntent::DebugError | QueryIntent::Test) {
        reasons.push("intent_keeps_test_context".to_string());
        return CompressionLevel::FocusedSnippet;
    }
    if has_chunk {
        reasons.push("chunk_available_focused_window".to_string());
        return CompressionLevel::FocusedSnippet;
    }
    if has_symbol {
        reasons.push("symbol_signature_summary".to_string());
        return CompressionLevel::SummaryAndSignature;
    }
    CompressionLevel::RelationOnly
}

fn degrade_to_budget(
    blocks: &mut [CompressedContextBlock],
    facts: &CompressionFacts,
    plan: &RetrievalPlan,
    budget_tokens: usize,
) {
    while total_tokens(blocks) > budget_tokens {
        let Some(index) = blocks
            .iter()
            .enumerate()
            .rev()
            .filter(|(_, block)| can_downgrade(block))
            .map(|(index, _)| index)
            .next()
        else {
            break;
        };
        let next_level = blocks[index].level.downgrade();
        let mut reasons = blocks[index].reasons.clone();
        reasons.push(format!(
            "downgraded_for_budget={} intent={}",
            budget_tokens,
            plan.intent.as_str()
        ));
        blocks[index] =
            block_with_level(&ranked_shadow(&blocks[index]), facts, next_level, reasons);
    }
}

fn can_downgrade(block: &CompressedContextBlock) -> bool {
    block.level != CompressionLevel::Drop
        && (block.level > CompressionLevel::RelationOnly
            || block.decision != RerankDecision::MustInclude)
}

fn ranked_shadow(block: &CompressedContextBlock) -> RankedContextItem {
    RankedContextItem {
        rank: block.rank,
        source: block.source.clone(),
        sources: block.sources.clone(),
        id: block.id.clone(),
        label: block.title.clone(),
        file_path: block.file_path.clone(),
        symbol_id: block.symbol_id.clone(),
        start_line: None,
        end_line: None,
        score: 0.0,
        token_count: block.original_tokens,
        matched_terms: Vec::new(),
        reasons: Vec::new(),
        is_test_context: false,
        decision: block.decision,
    }
}

fn content_for_level(
    item: &RankedContextItem,
    facts: &CompressionFacts,
    level: CompressionLevel,
) -> String {
    match level {
        CompressionLevel::Drop => String::new(),
        CompressionLevel::RelationOnly => relation_content(item),
        CompressionLevel::SignatureOnly => signature_content(item, facts),
        CompressionLevel::SummaryAndSignature => summary_and_signature_content(item, facts),
        CompressionLevel::FocusedSnippet => focused_snippet_content(item, facts),
        CompressionLevel::FullSymbolBody | CompressionLevel::FullFile => full_content(item, facts),
    }
}

fn relation_content(item: &RankedContextItem) -> String {
    format!(
        "related: {}\nfile: {}\nsource: {}\nrank: {}\n",
        item.label, item.file_path, item.source, item.rank
    )
}

fn signature_content(item: &RankedContextItem, facts: &CompressionFacts) -> String {
    if let Some(symbol) = facts.symbol(item) {
        return compact_lines([
            Some(format!("symbol: {}", symbol.qualified_name)),
            Some(format!("kind: {}", symbol.kind)),
            Some(format!("signature: {}", symbol.signature)),
            Some(format!("file: {}:{}", symbol.file_path, symbol.start_line)),
        ]);
    }
    relation_content(item)
}

fn summary_and_signature_content(item: &RankedContextItem, facts: &CompressionFacts) -> String {
    if let Some(symbol) = facts.symbol(item) {
        return compact_lines([
            Some(format!("symbol: {}", symbol.qualified_name)),
            Some(format!("kind: {}", symbol.kind)),
            Some(format!("signature: {}", symbol.signature)),
            symbol
                .doc_summary
                .as_deref()
                .map(|summary| format!("summary: {summary}")),
            Some(format!("file: {}:{}", symbol.file_path, symbol.start_line)),
        ]);
    }
    if let Some(chunk) = facts.chunk(item) {
        return compact_lines([
            Some(format!(
                "chunk: {}",
                chunk.qualified_name.as_deref().unwrap_or(&chunk.id)
            )),
            Some(format!("type: {}", chunk.chunk_type)),
            chunk
                .summary
                .as_deref()
                .map(|summary| format!("summary: {summary}")),
            Some(format!(
                "file: {}:{}",
                chunk.file_path,
                chunk.start_line.unwrap_or_default()
            )),
        ]);
    }
    signature_content(item, facts)
}

fn focused_snippet_content(item: &RankedContextItem, facts: &CompressionFacts) -> String {
    if let Some(chunk) = facts.chunk(item) {
        return focused_window(&chunk.content, &chunk.matched_terms);
    }
    if let Some(chunk) = facts.vector(item) {
        return focused_window(&chunk.content, &chunk.matched_terms);
    }
    summary_and_signature_content(item, facts)
}

fn full_content(item: &RankedContextItem, facts: &CompressionFacts) -> String {
    if let Some(chunk) = facts.chunk(item) {
        return chunk.content.clone();
    }
    if let Some(chunk) = facts.vector(item) {
        return chunk.content.clone();
    }
    summary_and_signature_content(item, facts)
}

fn focused_window(content: &str, matched_terms: &[String]) -> String {
    let lines = content.lines().collect::<Vec<_>>();
    if lines.len() <= 10 {
        return content.to_string();
    }
    let lower_terms = matched_terms
        .iter()
        .map(|term| term.to_ascii_lowercase())
        .collect::<Vec<_>>();
    let hit = lines
        .iter()
        .position(|line| {
            let line = line.to_ascii_lowercase();
            lower_terms
                .iter()
                .any(|term| !term.is_empty() && line.contains(term))
        })
        .unwrap_or(0);
    let start = hit.saturating_sub(3);
    let end = (hit + 7).min(lines.len());
    lines[start..end].join("\n")
}

fn total_tokens(blocks: &[CompressedContextBlock]) -> usize {
    blocks
        .iter()
        .filter(|block| block.level != CompressionLevel::Drop)
        .map(|block| block.compressed_tokens)
        .sum()
}

fn level_counts(blocks: &[CompressedContextBlock]) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for block in blocks {
        *counts.entry(block.level.as_str().to_string()).or_default() += 1;
    }
    counts
}

fn estimate_tokens(value: &str) -> usize {
    value
        .split_whitespace()
        .map(|part| (part.len() / CHARS_PER_TOKEN).max(1))
        .sum()
}

fn compact_lines(lines: impl IntoIterator<Item = Option<String>>) -> String {
    lines
        .into_iter()
        .flatten()
        .map(|line| line.trim().to_string())
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

fn compact_strings(values: impl IntoIterator<Item = String>) -> Vec<String> {
    values
        .into_iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}
