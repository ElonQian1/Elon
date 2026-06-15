use std::{collections::BTreeMap, path::Path};

use anyhow::{bail, Result};
use serde::Serialize;

use super::{
    symbol_index_chunks::{search_latest_symbol_chunks, SymbolChunkHit, SymbolChunkSearch},
    symbol_index_compression::compress_symbol_context,
    symbol_index_compression_render::render_compressed_context,
    symbol_index_compression_types::SymbolCompressedContext,
    symbol_index_impact_pack::{build_symbol_impact_pack, normalize_pack_max_chars},
    symbol_index_impact_query::load_latest_symbol_impact,
    symbol_index_impact_types::SymbolImpactQuery,
    symbol_index_query::{search_latest_symbol_index, SymbolIndexSearch},
    symbol_index_query_types::SymbolHit,
    symbol_index_rank_profile::{infer_rank_profile, HybridRankProfile},
    symbol_index_ranker::{rank_hybrid_context_with_plan, RankedContextItem},
    symbol_index_retrieval_plan::{build_retrieval_plan, render_retrieval_plan, RetrievalPlan},
    symbol_index_vector::{search_latest_symbol_vectors, SymbolVectorSearchQuery},
    symbol_index_vector_types::SymbolVectorHit,
};

const DEFAULT_TASK_SEARCH_LIMIT: usize = 8;
const MAX_TASK_SEARCH_LIMIT: usize = 20;
const DEFAULT_TASK_CHUNK_LIMIT: usize = 8;
const MAX_TASK_CHUNK_LIMIT: usize = 20;

#[derive(Debug, Clone, Default)]
pub(crate) struct SymbolTaskPackQuery {
    pub(crate) trace_id: Option<String>,
    pub(crate) text: Option<String>,
    pub(crate) kind: Option<String>,
    pub(crate) path: Option<String>,
    pub(crate) edge_kind: Option<String>,
    pub(crate) depth: usize,
    pub(crate) search_limit: usize,
    pub(crate) chunk_limit: usize,
    pub(crate) vector_model: Option<String>,
    pub(crate) vector_limit: usize,
    pub(crate) impact_limit: usize,
    pub(crate) max_chars: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SymbolTaskPackResponse {
    pub(crate) db_path: String,
    pub(crate) query: SymbolTaskPackQueryEcho,
    pub(crate) metadata: BTreeMap<String, String>,
    pub(crate) retrieval_plan: RetrievalPlan,
    pub(crate) ranking_profile: HybridRankProfile,
    pub(crate) pack: String,
    pub(crate) char_count: usize,
    pub(crate) truncated: bool,
    pub(crate) candidate_symbols: Vec<SymbolHit>,
    pub(crate) text_chunks: Vec<SymbolChunkHit>,
    pub(crate) vector_chunks: Vec<SymbolVectorHit>,
    pub(crate) ranked_context: Vec<RankedContextItem>,
    pub(crate) compressed_context: SymbolCompressedContext,
    pub(crate) chosen_seed: SymbolHit,
    pub(crate) chosen_seed_source: String,
    pub(crate) impacted_symbol_count: usize,
    pub(crate) impacted_file_count: usize,
    pub(crate) edge_count: usize,
    pub(crate) test_hint_count: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SymbolTaskPackQueryEcho {
    pub(crate) trace_id: Option<String>,
    pub(crate) q: String,
    pub(crate) kind: Option<String>,
    pub(crate) path: Option<String>,
    pub(crate) edge_kind: Option<String>,
    pub(crate) depth: usize,
    pub(crate) search_limit: usize,
    pub(crate) chunk_limit: usize,
    pub(crate) vector_model: Option<String>,
    pub(crate) vector_limit: usize,
    pub(crate) impact_limit: usize,
    pub(crate) max_chars: usize,
}

#[derive(Debug, Clone)]
struct TaskSeedChoice {
    source: &'static str,
    symbol_id: Option<String>,
    path: Option<String>,
    symbol: Option<SymbolHit>,
}

pub(crate) fn build_latest_symbol_task_pack(
    data_dir: &Path,
    query: &SymbolTaskPackQuery,
) -> Result<SymbolTaskPackResponse> {
    let text = query
        .text
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow::anyhow!("q 不能为空"))?
        .to_string();
    let vector_model = clean_filter(query.vector_model.as_deref());
    let retrieval_plan = build_retrieval_plan(&text, vector_model.is_some());
    let planned_search_limit = retrieval_plan.planned_limit(
        query.search_limit,
        DEFAULT_TASK_SEARCH_LIMIT,
        MAX_TASK_SEARCH_LIMIT,
        "symbol",
    );
    let planned_chunk_limit = retrieval_plan.planned_limit(
        query.chunk_limit,
        DEFAULT_TASK_CHUNK_LIMIT,
        MAX_TASK_CHUNK_LIMIT,
        "full_text",
    );
    let planned_vector_limit = retrieval_plan.planned_limit(
        query.vector_limit,
        DEFAULT_TASK_CHUNK_LIMIT,
        MAX_TASK_CHUNK_LIMIT,
        "vector",
    );
    let search = SymbolIndexSearch {
        trace_id: query.trace_id.clone(),
        text: Some(text.clone()),
        kind: query.kind.clone(),
        path: query.path.clone(),
        edge_kind: None,
        include_edges: false,
        limit: planned_search_limit,
    };
    let search_response = search_latest_symbol_index(data_dir, &search)?;
    let text_chunks = search_latest_symbol_chunks(
        data_dir,
        &SymbolChunkSearch {
            trace_id: query.trace_id.clone(),
            text: Some(text.clone()),
            path: query.path.clone(),
            chunk_type: None,
            limit: planned_chunk_limit,
        },
    )
    .map(|response| response.chunks)
    .unwrap_or_default();
    let vector_chunks = if retrieval_plan.retrievers.vector {
        let Some(vector_model) = vector_model.as_deref() else {
            unreachable!("vector retriever requires a requested vector model");
        };
        search_latest_symbol_vectors(
            data_dir,
            &SymbolVectorSearchQuery {
                trace_id: query.trace_id.clone(),
                text: Some(text.clone()),
                model: Some(vector_model.to_string()),
                path: query.path.clone(),
                limit: planned_vector_limit,
                ..Default::default()
            },
        )
        .map(|response| response.chunks)
        .unwrap_or_default()
    } else {
        Vec::new()
    };
    let search_symbols = search_response.symbols;
    let seed_choice = choose_task_seed(&search_symbols, &text_chunks, &vector_chunks)?;

    let impact_query = SymbolImpactQuery {
        trace_id: query.trace_id.clone(),
        symbol_id: seed_choice.symbol_id.clone(),
        path: seed_choice.path.clone(),
        edge_kind: query.edge_kind.clone(),
        depth: retrieval_plan.planned_graph_depth(query.depth),
        limit: query.impact_limit,
    };
    let impact = load_latest_symbol_impact(data_dir, &impact_query)?;
    let chosen_seed = seed_choice
        .symbol
        .clone()
        .or_else(|| choose_impact_seed(&impact.seed_symbols, seed_choice.symbol_id.as_deref()))
        .ok_or_else(|| anyhow::anyhow!("没有找到与任务相关的符号"))?;
    let ranking_profile = infer_rank_profile(&text);
    let ranked_context = rank_hybrid_context_with_plan(
        &search_symbols,
        &text_chunks,
        &vector_chunks,
        Some(&impact),
        &ranking_profile,
        &retrieval_plan,
    );
    let mut candidate_symbols = search_symbols;
    if candidate_symbols.is_empty() {
        candidate_symbols.extend(impact.seed_symbols.iter().cloned());
    }
    let max_chars = normalize_pack_max_chars(query.max_chars);
    let compressed_context = compress_symbol_context(
        &ranked_context,
        &candidate_symbols,
        &impact,
        &text_chunks,
        &vector_chunks,
        &retrieval_plan,
        max_chars,
    );
    let impact_pack = build_symbol_impact_pack(impact, max_chars);
    let pack = render_task_pack(
        &text,
        &candidate_symbols,
        &text_chunks,
        &vector_chunks,
        &ranked_context,
        &compressed_context,
        &retrieval_plan,
        &ranking_profile,
        &chosen_seed,
        seed_choice.source,
        &impact_pack.pack,
    );
    let (pack, truncated) = truncate_pack(pack, max_chars);
    let char_count = pack.chars().count();

    Ok(SymbolTaskPackResponse {
        db_path: impact_pack.db_path,
        query: SymbolTaskPackQueryEcho {
            trace_id: query.trace_id.clone(),
            q: text,
            kind: query.kind.clone(),
            path: query.path.clone(),
            edge_kind: query.edge_kind.clone(),
            depth: impact_pack.query.depth,
            search_limit: planned_search_limit,
            chunk_limit: planned_chunk_limit,
            vector_model: query.vector_model.clone(),
            vector_limit: planned_vector_limit,
            impact_limit: impact_pack.query.limit,
            max_chars,
        },
        metadata: impact_pack.metadata,
        retrieval_plan,
        ranking_profile,
        pack,
        char_count,
        truncated: truncated || impact_pack.truncated,
        candidate_symbols,
        text_chunks,
        vector_chunks,
        ranked_context,
        compressed_context,
        chosen_seed,
        chosen_seed_source: seed_choice.source.to_string(),
        impacted_symbol_count: impact_pack.impacted_symbol_count,
        impacted_file_count: impact_pack.impacted_file_count,
        edge_count: impact_pack.edge_count,
        test_hint_count: impact_pack.test_hint_count,
    })
}

fn render_task_pack(
    task: &str,
    candidates: &[SymbolHit],
    text_chunks: &[SymbolChunkHit],
    vector_chunks: &[SymbolVectorHit],
    ranked_context: &[RankedContextItem],
    compressed_context: &SymbolCompressedContext,
    retrieval_plan: &RetrievalPlan,
    ranking_profile: &HybridRankProfile,
    chosen_seed: &SymbolHit,
    chosen_seed_source: &str,
    impact_pack: &str,
) -> String {
    let mut out = String::new();
    out.push_str("<symbol_task_context format=\"xml-wrapped-markdown\">\n");
    out.push_str(&format!("<task>{}</task>\n", xml_escape(task)));
    out.push_str(&format!(
        "<chosen_seed id=\"{}\" name=\"{}\" path=\"{}\" line=\"{}\" source=\"{}\" />\n",
        xml_escape(&chosen_seed.id),
        xml_escape(&chosen_seed.name),
        xml_escape(&chosen_seed.file_path),
        chosen_seed.start_line,
        xml_escape(chosen_seed_source)
    ));
    out.push_str(&render_retrieval_plan(retrieval_plan));
    out.push_str(&format!(
        "<ranking_profile name=\"{}\" testContextBonus=\"{:.0}\">\n",
        xml_escape(&ranking_profile.name),
        ranking_profile.test_context_bonus
    ));
    out.push_str(&format!(
        "- description: {}\n",
        xml_escape(&ranking_profile.description)
    ));
    for (source, weight) in &ranking_profile.source_weights {
        out.push_str(&format!("- weight {}={:.0}\n", xml_escape(source), weight));
    }
    if !ranking_profile.reasons.is_empty() {
        out.push_str(&format!(
            "- reasons: {}\n",
            xml_escape(&ranking_profile.reasons.join("; "))
        ));
    }
    out.push_str("</ranking_profile>\n");
    out.push_str(&format!(
        "<ranked_context count=\"{}\">\n",
        ranked_context.len()
    ));
    for item in ranked_context.iter().take(20) {
        out.push_str(&format!(
            "- #{} decision={} source={} sources={} `{}` path={}:{} score={:.2} reasons={}\n",
            item.rank,
            item.decision.as_str(),
            xml_escape(&item.source),
            xml_escape(&item.sources.join("+")),
            xml_escape(&item.label),
            xml_escape(&item.file_path),
            item.start_line.unwrap_or_default(),
            item.score,
            xml_escape(&item.reasons.join("; "))
        ));
    }
    out.push_str("</ranked_context>\n");
    out.push_str(&render_compressed_context(compressed_context));
    out.push_str(&format!(
        "<candidate_symbols count=\"{}\">\n",
        candidates.len()
    ));
    for symbol in candidates.iter().take(20) {
        out.push_str(&format!(
            "- `{}` kind={} path={}:{} score={:.1} matched={}\n",
            xml_escape(&symbol.qualified_name),
            xml_escape(&symbol.kind),
            xml_escape(&symbol.file_path),
            symbol.start_line,
            symbol.score,
            xml_escape(&symbol.matched_terms.join(","))
        ));
    }
    out.push_str("</candidate_symbols>\n");
    out.push_str(&format!(
        "<full_text_chunks count=\"{}\">\n",
        text_chunks.len()
    ));
    for chunk in text_chunks.iter().take(20) {
        out.push_str(&format!(
            "- `{}` type={} path={}:{} score={:.4} matched={}\n",
            xml_escape(chunk.qualified_name.as_deref().unwrap_or(chunk.id.as_str())),
            xml_escape(&chunk.chunk_type),
            xml_escape(&chunk.file_path),
            chunk.start_line.unwrap_or_default(),
            chunk.score,
            xml_escape(&chunk.matched_terms.join(","))
        ));
        if let Some(summary) = chunk.summary.as_deref() {
            out.push_str(&format!("  summary: {}\n", xml_escape(summary)));
        }
    }
    out.push_str("</full_text_chunks>\n");
    out.push_str(&format!(
        "<vector_chunks count=\"{}\">\n",
        vector_chunks.len()
    ));
    for chunk in vector_chunks.iter().take(20) {
        out.push_str(&format!(
            "- `{}` type={} path={}:{} score={:.4} matched={}\n",
            xml_escape(chunk.qualified_name.as_deref().unwrap_or(chunk.id.as_str())),
            xml_escape(&chunk.chunk_type),
            xml_escape(&chunk.file_path),
            chunk.start_line.unwrap_or_default(),
            chunk.score,
            xml_escape(&chunk.matched_terms.join(","))
        ));
        if let Some(summary) = chunk.summary.as_deref() {
            out.push_str(&format!("  summary: {}\n", xml_escape(summary)));
        }
    }
    out.push_str("</vector_chunks>\n");
    out.push_str(impact_pack);
    out.push_str("<task_pack_usage>\n");
    out.push_str("- Start by reading the chosen_seed file and the highest-ranked impacted files before editing.\n");
    out.push_str("- If the chosen seed is not the intended target, rerun task-pack with a narrower query, kind, or path filter.\n");
    out.push_str(
        "- Use the test hints and relationship edges as the first validation checklist.\n",
    );
    out.push_str("</task_pack_usage>\n");
    out.push_str("</symbol_task_context>\n");
    out
}

fn choose_task_seed(
    symbols: &[SymbolHit],
    text_chunks: &[SymbolChunkHit],
    vector_chunks: &[SymbolVectorHit],
) -> Result<TaskSeedChoice> {
    if let Some(symbol) = symbols
        .iter()
        .find(|symbol| has_direct_symbol_match(symbol))
    {
        return Ok(TaskSeedChoice {
            source: "symbol",
            symbol_id: Some(symbol.id.clone()),
            path: None,
            symbol: Some(symbol.clone()),
        });
    }

    if let Some(seed) = text_chunks
        .iter()
        .find_map(|chunk| seed_from_chunk("full_text_symbol", chunk.symbol_id.as_deref(), None))
    {
        return Ok(seed);
    }

    if let Some(seed) = vector_chunks
        .iter()
        .find_map(|chunk| seed_from_chunk("vector_symbol", chunk.symbol_id.as_deref(), None))
    {
        return Ok(seed);
    }

    if let Some(seed) = text_chunks
        .iter()
        .find_map(|chunk| seed_from_chunk("full_text_path", None, Some(chunk.file_path.as_str())))
    {
        return Ok(seed);
    }

    if let Some(seed) = vector_chunks
        .iter()
        .find_map(|chunk| seed_from_chunk("vector_path", None, Some(chunk.file_path.as_str())))
    {
        return Ok(seed);
    }

    bail!("没有找到与任务相关的符号或 chunk");
}

fn seed_from_chunk(
    source: &'static str,
    symbol_id: Option<&str>,
    path: Option<&str>,
) -> Option<TaskSeedChoice> {
    let symbol_id = clean_filter(symbol_id);
    let path = clean_filter(path);
    if symbol_id.is_none() && path.is_none() {
        return None;
    }
    Some(TaskSeedChoice {
        source,
        symbol_id,
        path,
        symbol: None,
    })
}

fn choose_impact_seed(seed_symbols: &[SymbolHit], symbol_id: Option<&str>) -> Option<SymbolHit> {
    symbol_id
        .and_then(|id| seed_symbols.iter().find(|symbol| symbol.id == id).cloned())
        .or_else(|| seed_symbols.first().cloned())
}

fn has_direct_symbol_match(symbol: &SymbolHit) -> bool {
    !symbol.matched_terms.is_empty()
}

fn clean_filter(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn truncate_pack(pack: String, max_chars: usize) -> (String, bool) {
    if pack.chars().count() <= max_chars {
        return (pack, false);
    }
    let mut truncated = pack.chars().take(max_chars).collect::<String>();
    truncated.push_str("\n<!-- symbol task context truncated by maxChars -->\n");
    (truncated, true)
}

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
