use std::{collections::BTreeMap, path::Path};

use anyhow::{bail, Result};
use serde::Serialize;

use super::{
    symbol_index_chunks::{search_latest_symbol_chunks, SymbolChunkHit, SymbolChunkSearch},
    symbol_index_impact_pack::{build_symbol_impact_pack, normalize_pack_max_chars},
    symbol_index_impact_query::load_latest_symbol_impact,
    symbol_index_impact_types::SymbolImpactQuery,
    symbol_index_query::{search_latest_symbol_index, SymbolIndexSearch},
    symbol_index_query_types::SymbolHit,
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
    pub(crate) pack: String,
    pub(crate) char_count: usize,
    pub(crate) truncated: bool,
    pub(crate) candidate_symbols: Vec<SymbolHit>,
    pub(crate) text_chunks: Vec<SymbolChunkHit>,
    pub(crate) vector_chunks: Vec<SymbolVectorHit>,
    pub(crate) chosen_seed: SymbolHit,
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
    let search = SymbolIndexSearch {
        trace_id: query.trace_id.clone(),
        text: Some(text.clone()),
        kind: query.kind.clone(),
        path: query.path.clone(),
        edge_kind: None,
        include_edges: false,
        limit: search_limit(query.search_limit),
    };
    let search_response = search_latest_symbol_index(data_dir, &search)?;
    let text_chunks = search_latest_symbol_chunks(
        data_dir,
        &SymbolChunkSearch {
            trace_id: query.trace_id.clone(),
            text: Some(text.clone()),
            path: query.path.clone(),
            chunk_type: None,
            limit: chunk_limit(query.chunk_limit),
        },
    )
    .map(|response| response.chunks)
    .unwrap_or_default();
    let vector_chunks = if let Some(vector_model) = clean_filter(query.vector_model.as_deref()) {
        search_latest_symbol_vectors(
            data_dir,
            &SymbolVectorSearchQuery {
                trace_id: query.trace_id.clone(),
                text: Some(text.clone()),
                model: Some(vector_model),
                path: query.path.clone(),
                limit: vector_limit(query.vector_limit),
                ..Default::default()
            },
        )
        .map(|response| response.chunks)
        .unwrap_or_default()
    } else {
        Vec::new()
    };
    let Some(chosen_seed) = search_response.symbols.first().cloned() else {
        bail!("没有找到与任务相关的符号");
    };

    let impact_query = SymbolImpactQuery {
        trace_id: query.trace_id.clone(),
        symbol_id: Some(chosen_seed.id.clone()),
        path: None,
        edge_kind: query.edge_kind.clone(),
        depth: query.depth,
        limit: query.impact_limit,
    };
    let impact = load_latest_symbol_impact(data_dir, &impact_query)?;
    let impact_pack = build_symbol_impact_pack(impact, normalize_pack_max_chars(query.max_chars));
    let candidate_symbols = search_response.symbols;
    let pack = render_task_pack(
        &text,
        &candidate_symbols,
        &text_chunks,
        &vector_chunks,
        &chosen_seed,
        &impact_pack.pack,
    );
    let (pack, truncated) = truncate_pack(pack, normalize_pack_max_chars(query.max_chars));
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
            search_limit: search_limit(query.search_limit),
            chunk_limit: chunk_limit(query.chunk_limit),
            vector_model: query.vector_model.clone(),
            vector_limit: vector_limit(query.vector_limit),
            impact_limit: impact_pack.query.limit,
            max_chars: normalize_pack_max_chars(query.max_chars),
        },
        metadata: impact_pack.metadata,
        pack,
        char_count,
        truncated: truncated || impact_pack.truncated,
        candidate_symbols,
        text_chunks,
        vector_chunks,
        chosen_seed,
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
    chosen_seed: &SymbolHit,
    impact_pack: &str,
) -> String {
    let mut out = String::new();
    out.push_str("<symbol_task_context format=\"xml-wrapped-markdown\">\n");
    out.push_str(&format!("<task>{}</task>\n", xml_escape(task)));
    out.push_str(&format!(
        "<chosen_seed id=\"{}\" name=\"{}\" path=\"{}\" line=\"{}\" />\n",
        xml_escape(&chosen_seed.id),
        xml_escape(&chosen_seed.name),
        xml_escape(&chosen_seed.file_path),
        chosen_seed.start_line
    ));
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

fn search_limit(limit: usize) -> usize {
    if limit == 0 {
        DEFAULT_TASK_SEARCH_LIMIT
    } else {
        limit.min(MAX_TASK_SEARCH_LIMIT)
    }
}

fn chunk_limit(limit: usize) -> usize {
    if limit == 0 {
        DEFAULT_TASK_CHUNK_LIMIT
    } else {
        limit.min(MAX_TASK_CHUNK_LIMIT)
    }
}

fn vector_limit(limit: usize) -> usize {
    chunk_limit(limit)
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
