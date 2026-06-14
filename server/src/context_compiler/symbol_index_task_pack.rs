use std::{collections::BTreeMap, path::Path};

use anyhow::{bail, Result};
use serde::Serialize;

use super::{
    symbol_index_impact_pack::{build_symbol_impact_pack, normalize_pack_max_chars},
    symbol_index_impact_query::load_latest_symbol_impact,
    symbol_index_impact_types::SymbolImpactQuery,
    symbol_index_query::{search_latest_symbol_index, SymbolIndexSearch},
    symbol_index_query_types::SymbolHit,
};

const DEFAULT_TASK_SEARCH_LIMIT: usize = 8;
const MAX_TASK_SEARCH_LIMIT: usize = 20;

#[derive(Debug, Clone, Default)]
pub(crate) struct SymbolTaskPackQuery {
    pub(crate) trace_id: Option<String>,
    pub(crate) text: Option<String>,
    pub(crate) kind: Option<String>,
    pub(crate) path: Option<String>,
    pub(crate) edge_kind: Option<String>,
    pub(crate) depth: usize,
    pub(crate) search_limit: usize,
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
    let pack = render_task_pack(&text, &candidate_symbols, &chosen_seed, &impact_pack.pack);
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
            impact_limit: impact_pack.query.limit,
            max_chars: normalize_pack_max_chars(query.max_chars),
        },
        metadata: impact_pack.metadata,
        pack,
        char_count,
        truncated: truncated || impact_pack.truncated,
        candidate_symbols,
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
