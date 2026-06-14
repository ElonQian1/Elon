use std::collections::BTreeMap;

use serde::Serialize;

use super::{
    symbol_index_impact_types::{ImpactTestHint, SymbolImpactQueryEcho, SymbolImpactResponse},
    symbol_index_query_types::{SymbolEdgeHit, SymbolHit},
};

const DEFAULT_PACK_MAX_CHARS: usize = 12_000;
const MIN_PACK_MAX_CHARS: usize = 1_000;
const MAX_PACK_MAX_CHARS: usize = 50_000;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SymbolImpactPackResponse {
    pub(crate) db_path: String,
    pub(crate) query: SymbolImpactQueryEcho,
    pub(crate) metadata: BTreeMap<String, String>,
    pub(crate) pack: String,
    pub(crate) char_count: usize,
    pub(crate) truncated: bool,
    pub(crate) seed_symbol_count: usize,
    pub(crate) impacted_symbol_count: usize,
    pub(crate) impacted_file_count: usize,
    pub(crate) edge_count: usize,
    pub(crate) test_hint_count: usize,
}

pub(crate) fn normalize_pack_max_chars(max_chars: usize) -> usize {
    if max_chars == 0 {
        DEFAULT_PACK_MAX_CHARS
    } else {
        max_chars.clamp(MIN_PACK_MAX_CHARS, MAX_PACK_MAX_CHARS)
    }
}

pub(crate) fn build_symbol_impact_pack(
    response: SymbolImpactResponse,
    max_chars: usize,
) -> SymbolImpactPackResponse {
    let limit = normalize_pack_max_chars(max_chars);
    let mut pack = String::new();
    render_pack(&mut pack, &response);
    let (pack, truncated) = truncate_pack(pack, limit);
    let char_count = pack.chars().count();

    SymbolImpactPackResponse {
        db_path: response.db_path,
        query: response.query,
        metadata: response.metadata,
        pack,
        char_count,
        truncated,
        seed_symbol_count: response.seed_symbols.len(),
        impacted_symbol_count: response.impacted_symbols.len(),
        impacted_file_count: response.impacted_files.len(),
        edge_count: response.edges.len(),
        test_hint_count: response.test_hints.len(),
    }
}

fn render_pack(out: &mut String, response: &SymbolImpactResponse) {
    out.push_str("<symbol_impact_context format=\"xml-wrapped-markdown\">\n");
    out.push_str(&format!(
        "<source db_path=\"{}\" schema_version=\"{}\" symbol_count=\"{}\" edge_count=\"{}\" />\n",
        xml_escape(&response.db_path),
        xml_escape(metadata_value(&response.metadata, "schema_version")),
        xml_escape(metadata_value(&response.metadata, "symbol_count")),
        xml_escape(metadata_value(&response.metadata, "edge_count")),
    ));
    render_query(out, &response.query);
    render_symbols(out, "seed_symbols", &response.seed_symbols);
    render_symbols(out, "impacted_symbols", &response.impacted_symbols);
    render_files(out, response);
    render_edges(out, &response.edges);
    render_test_hints(out, &response.test_hints);
    out.push_str("<usage_guidance>\n");
    out.push_str("- Treat this as navigation and impact evidence, not a substitute for reading source files before editing.\n");
    out.push_str("- Prioritize seed files, impacted files with test hints, then high-confidence relationships.\n");
    out.push_str("- Use test_hints as validation starting points after code changes.\n");
    out.push_str("</usage_guidance>\n");
    out.push_str("</symbol_impact_context>\n");
}

fn render_query(out: &mut String, query: &SymbolImpactQueryEcho) {
    out.push_str(&format!(
        "<query depth=\"{}\" limit=\"{}\"",
        query.depth, query.limit
    ));
    if let Some(trace_id) = query.trace_id.as_deref() {
        out.push_str(&format!(" trace_id=\"{}\"", xml_escape(trace_id)));
    }
    if let Some(symbol_id) = query.symbol_id.as_deref() {
        out.push_str(&format!(" symbol_id=\"{}\"", xml_escape(symbol_id)));
    }
    if let Some(path) = query.path.as_deref() {
        out.push_str(&format!(" path=\"{}\"", xml_escape(path)));
    }
    if let Some(edge_kind) = query.edge_kind.as_deref() {
        out.push_str(&format!(" edge_kind=\"{}\"", xml_escape(edge_kind)));
    }
    out.push_str(" />\n");
}

fn render_symbols(out: &mut String, tag: &str, symbols: &[SymbolHit]) {
    out.push_str(&format!("<{tag} count=\"{}\">\n", symbols.len()));
    for symbol in symbols.iter().take(80) {
        out.push_str(&format!(
            "- `{}` kind={} path={}:{}-{} visibility={} score={}\n",
            xml_escape(&symbol.qualified_name),
            xml_escape(&symbol.kind),
            xml_escape(&symbol.file_path),
            symbol.start_line,
            symbol.end_line,
            xml_escape(&symbol.visibility),
            symbol.importance_score.unwrap_or_default()
        ));
        if !symbol.signature.is_empty() {
            out.push_str(&format!(
                "  signature: `{}`\n",
                xml_escape(&symbol.signature)
            ));
        }
        if !symbol.source_providers.is_empty() {
            out.push_str(&format!(
                "  sources: {}\n",
                xml_escape(&symbol.source_providers.join(","))
            ));
        }
    }
    out.push_str(&format!("</{tag}>\n"));
}

fn render_files(out: &mut String, response: &SymbolImpactResponse) {
    out.push_str(&format!(
        "<impacted_files count=\"{}\">\n",
        response.impacted_files.len()
    ));
    for file in response.impacted_files.iter().take(120) {
        out.push_str(&format!(
            "- `{}` seed={} symbols={} edges={} test_hints={}\n",
            xml_escape(&file.path),
            file.seed,
            file.symbol_count,
            file.edge_count,
            file.test_hint_count
        ));
    }
    out.push_str("</impacted_files>\n");
}

fn render_edges(out: &mut String, edges: &[SymbolEdgeHit]) {
    out.push_str(&format!("<relationships count=\"{}\">\n", edges.len()));
    for edge in edges.iter().take(160) {
        out.push_str(&format!(
            "- kind={} source={} confidence={:.2} from={} at {}:{} -> to={} path={} reason=\"{}\"\n",
            xml_escape(&edge.kind),
            xml_escape(&edge.source),
            edge.confidence,
            xml_escape(edge.from_symbol_id.as_deref().unwrap_or("unknown")),
            xml_escape(&edge.from_path),
            edge.line,
            xml_escape(edge.to_symbol_id.as_deref().or(edge.to_symbol_name.as_deref()).unwrap_or("unknown")),
            xml_escape(edge.to_path.as_deref().unwrap_or("unknown")),
            xml_escape(&edge.reason)
        ));
    }
    out.push_str("</relationships>\n");
}

fn render_test_hints(out: &mut String, hints: &[ImpactTestHint]) {
    out.push_str(&format!("<test_hints count=\"{}\">\n", hints.len()));
    for hint in hints.iter().take(80) {
        out.push_str(&format!(
            "- `{}` path={}:{} edge_kind={} target={} reason=\"{}\"\n",
            xml_escape(&hint.symbol_name),
            xml_escape(&hint.path),
            hint.line,
            xml_escape(hint.edge_kind.as_deref().unwrap_or("heuristic")),
            xml_escape(hint.target_symbol_id.as_deref().unwrap_or("unknown")),
            xml_escape(&hint.reason)
        ));
    }
    out.push_str("</test_hints>\n");
}

fn metadata_value<'a>(metadata: &'a BTreeMap<String, String>, key: &str) -> &'a str {
    metadata.get(key).map(String::as_str).unwrap_or("")
}

fn truncate_pack(pack: String, max_chars: usize) -> (String, bool) {
    if pack.chars().count() <= max_chars {
        return (pack, false);
    }
    let mut truncated = pack.chars().take(max_chars).collect::<String>();
    truncated.push_str("\n<!-- symbol impact context truncated by maxChars -->\n");
    (truncated, true)
}

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
