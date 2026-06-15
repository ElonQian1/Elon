use std::{
    cmp::Ordering,
    collections::{BTreeMap, BTreeSet, HashMap},
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result};
use rusqlite::{params_from_iter, types::Value, Connection, OpenFlags};

pub(crate) use super::symbol_index_query_types::{
    SymbolEdgeHit, SymbolHit, SymbolIndexQueryEcho, SymbolIndexSearch, SymbolIndexSearchResponse,
    MAX_EDGE_LIMIT,
};
use super::{symbol_index::normalize_path, symbol_index_store::SYMBOL_INDEX_DB_FILE};

pub(crate) fn search_latest_symbol_index(
    data_dir: &Path,
    search: &SymbolIndexSearch,
) -> Result<SymbolIndexSearchResponse> {
    let db_path = find_symbol_index_db(data_dir, search.trace_id.as_deref())
        .context("没有找到可查询的 symbol_index.sqlite，请先运行一次 context compiler")?;
    search_symbol_index_db(&db_path, search)
}

pub(crate) fn find_symbol_index_db(data_dir: &Path, trace_id: Option<&str>) -> Option<PathBuf> {
    let root = data_dir.join("context-compiler");
    let trace = trace_id
        .map(safe_component)
        .filter(|value| !value.is_empty());
    let mut latest: Option<(SystemTime, String, PathBuf)> = None;
    collect_symbol_index_dbs(&root, trace.as_deref(), &mut latest);
    latest.map(|(_, _, path)| path)
}

pub(crate) fn search_symbol_index_db(
    db_path: &Path,
    search: &SymbolIndexSearch,
) -> Result<SymbolIndexSearchResponse> {
    let conn = Connection::open_with_flags(db_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .with_context(|| format!("打开符号索引数据库失败: {}", db_path.display()))?;
    let metadata = load_metadata(&conn)?;
    let terms = tokenize(search.text.as_deref().unwrap_or_default());
    let term_scores = load_term_scores(&conn, &terms)?;
    let mut symbols = load_symbols(&conn, search.kind.as_deref(), search.path.as_deref())?;
    score_symbols(&mut symbols, &terms, &term_scores);
    if !terms.is_empty() {
        symbols.retain(|symbol| symbol.score > 0.0);
    }
    symbols.sort_by(compare_symbols);
    symbols.truncate(search.limit());

    let symbol_ids = symbols
        .iter()
        .map(|symbol| symbol.id.clone())
        .collect::<Vec<_>>();
    let edges = if search.include_edges || search.edge_kind.is_some() {
        load_edges(
            &conn,
            &symbol_ids,
            search.edge_kind.as_deref(),
            search.limit().saturating_mul(4).min(MAX_EDGE_LIMIT),
        )?
    } else {
        Vec::new()
    };

    Ok(SymbolIndexSearchResponse {
        db_path: db_path.to_string_lossy().replace('\\', "/"),
        query: SymbolIndexQueryEcho {
            trace_id: search.trace_id.clone(),
            q: search.text.clone(),
            kind: search.kind.clone(),
            path: search.path.clone(),
            edge_kind: search.edge_kind.clone(),
            include_edges: search.include_edges,
            limit: search.limit(),
        },
        metadata,
        symbols,
        edges,
    })
}

fn collect_symbol_index_dbs(
    root: &Path,
    trace: Option<&str>,
    latest: &mut Option<(SystemTime, String, PathBuf)>,
) {
    let entries = match fs::read_dir(root) {
        Ok(entries) => entries,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_symbol_index_dbs(&path, trace, latest);
            continue;
        }
        if path.file_name().and_then(|name| name.to_str()) != Some(SYMBOL_INDEX_DB_FILE) {
            continue;
        }
        if !matches_trace(&path, trace) {
            continue;
        }
        let modified = entry
            .metadata()
            .and_then(|metadata| metadata.modified())
            .unwrap_or(UNIX_EPOCH);
        let sort_key = path.to_string_lossy().to_string();
        let replace = latest
            .as_ref()
            .map(|(current_modified, current_key, _)| {
                modified > *current_modified
                    || (modified == *current_modified && sort_key > *current_key)
            })
            .unwrap_or(true);
        if replace {
            *latest = Some((modified, sort_key, path));
        }
    }
}

fn matches_trace(path: &Path, trace: Option<&str>) -> bool {
    let Some(trace) = trace else {
        return true;
    };
    path.parent()
        .and_then(|parent| parent.file_name())
        .and_then(|name| name.to_str())
        .map(|bundle| bundle.contains(trace))
        .unwrap_or(false)
}

pub(crate) fn load_metadata(conn: &Connection) -> Result<BTreeMap<String, String>> {
    let mut stmt = conn.prepare("SELECT key, value FROM metadata ORDER BY key")?;
    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;
    let mut metadata = BTreeMap::new();
    for row in rows {
        let (key, value) = row?;
        metadata.insert(key, value);
    }
    Ok(metadata)
}

fn load_term_scores(conn: &Connection, terms: &[String]) -> Result<HashMap<String, i64>> {
    if terms.is_empty() {
        return Ok(HashMap::new());
    }
    let placeholders = placeholders(terms.len());
    let sql = format!(
        "SELECT symbol_id, SUM(weight) FROM symbol_terms WHERE term IN ({placeholders}) GROUP BY symbol_id"
    );
    let params = terms
        .iter()
        .cloned()
        .map(Value::Text)
        .collect::<Vec<Value>>();
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params_from_iter(params.iter()), |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
    })?;
    let mut scores = HashMap::new();
    for row in rows {
        let (symbol_id, score) = row?;
        scores.insert(symbol_id, score);
    }
    Ok(scores)
}

fn load_symbols(
    conn: &Connection,
    kind: Option<&str>,
    path: Option<&str>,
) -> Result<Vec<SymbolHit>> {
    let mut sql = String::from(
        r#"
        SELECT
            id, name, qualified_name, kind, language, file_path, start_line, end_line,
            signature, visibility, parent_symbol_id, module_path, doc_summary, role,
            importance_score, source_providers_json
        FROM symbols
        "#,
    );
    let mut clauses = Vec::new();
    let mut params = Vec::new();

    if let Some(kind) = clean_filter(kind) {
        clauses.push("lower(kind) = lower(?)");
        params.push(Value::Text(kind));
    }
    if let Some(path) = clean_filter(path) {
        clauses.push("lower(replace(file_path, char(92), '/')) LIKE lower(?)");
        params.push(Value::Text(format!("%{}%", normalize_path(&path))));
    }
    if !clauses.is_empty() {
        sql.push_str(" WHERE ");
        sql.push_str(&clauses.join(" AND "));
    }
    sql.push_str(" ORDER BY file_path, start_line");

    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params_from_iter(params.iter()), |row| {
        let source_json: String = row.get(15)?;
        Ok(SymbolHit {
            id: row.get(0)?,
            name: row.get(1)?,
            qualified_name: row.get(2)?,
            kind: row.get(3)?,
            language: row.get(4)?,
            file_path: normalize_path(&row.get::<_, String>(5)?),
            start_line: to_usize(row.get::<_, i64>(6)?),
            end_line: to_usize(row.get::<_, i64>(7)?),
            signature: row.get(8)?,
            visibility: row.get(9)?,
            parent_symbol_id: row.get(10)?,
            module_path: row.get(11)?,
            doc_summary: row.get(12)?,
            role: row.get(13)?,
            importance_score: row.get(14)?,
            source_providers: parse_sources(&source_json),
            score: 0.0,
            matched_terms: Vec::new(),
        })
    })?;
    let mut symbols = Vec::new();
    for row in rows {
        symbols.push(row?);
    }
    Ok(symbols)
}

fn load_edges(
    conn: &Connection,
    symbol_ids: &[String],
    edge_kind: Option<&str>,
    limit: usize,
) -> Result<Vec<SymbolEdgeHit>> {
    if symbol_ids.is_empty() {
        return Ok(Vec::new());
    }

    let placeholders = placeholders(symbol_ids.len());
    let mut sql = format!(
        r#"
        SELECT
            id, source, kind, from_symbol_id, from_path, line, to_symbol_id,
            to_symbol_name, to_path, confidence, reason
        FROM edges
        WHERE (from_symbol_id IN ({placeholders}) OR to_symbol_id IN ({placeholders}))
        "#
    );
    let mut params = symbol_ids
        .iter()
        .chain(symbol_ids.iter())
        .cloned()
        .map(Value::Text)
        .collect::<Vec<Value>>();

    if let Some(kind) = clean_filter(edge_kind) {
        sql.push_str(" AND lower(kind) = lower(?)");
        params.push(Value::Text(kind));
    }
    sql.push_str(" ORDER BY confidence DESC, source, kind, from_path, line LIMIT ?");
    params.push(Value::Integer(i64::try_from(limit).unwrap_or(i64::MAX)));

    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params_from_iter(params.iter()), |row| {
        Ok(SymbolEdgeHit {
            id: row.get(0)?,
            source: row.get(1)?,
            kind: row.get(2)?,
            from_symbol_id: row.get(3)?,
            from_path: normalize_path(&row.get::<_, String>(4)?),
            line: to_usize(row.get::<_, i64>(5)?),
            to_symbol_id: row.get(6)?,
            to_symbol_name: row.get(7)?,
            to_path: row
                .get::<_, Option<String>>(8)?
                .map(|path| normalize_path(&path)),
            confidence: row.get(9)?,
            reason: row.get(10)?,
        })
    })?;
    let mut edges = Vec::new();
    for row in rows {
        edges.push(row?);
    }
    Ok(edges)
}

fn score_symbols(symbols: &mut [SymbolHit], terms: &[String], term_scores: &HashMap<String, i64>) {
    for symbol in symbols {
        let (score, matched_terms) = score_symbol(symbol, terms, term_scores);
        symbol.score = score;
        symbol.matched_terms = matched_terms;
    }
}

fn score_symbol(
    symbol: &SymbolHit,
    terms: &[String],
    term_scores: &HashMap<String, i64>,
) -> (f64, Vec<String>) {
    let mut score = term_scores.get(&symbol.id).copied().unwrap_or_default() as f64;
    let mut matched = BTreeSet::new();
    if terms.is_empty() {
        score += symbol.importance_score.unwrap_or_default() * 10.0;
    }

    let name = symbol.name.to_ascii_lowercase();
    let qualified = symbol.qualified_name.to_ascii_lowercase();
    let path = symbol.file_path.to_ascii_lowercase();
    let signature = symbol.signature.to_ascii_lowercase();
    let docs = symbol
        .doc_summary
        .as_deref()
        .unwrap_or_default()
        .to_ascii_lowercase();
    for term in terms {
        let mut term_matched = false;
        if name == *term {
            score += 140.0;
            term_matched = true;
        } else if name.contains(term) {
            score += 90.0;
            term_matched = true;
        }
        if qualified.contains(term) {
            score += 55.0;
            term_matched = true;
        }
        if path.contains(term) {
            score += 30.0;
            term_matched = true;
        }
        if signature.contains(term) {
            score += 20.0;
            term_matched = true;
        }
        if docs.contains(term) {
            score += 12.0;
            term_matched = true;
        }
        if term_matched {
            matched.insert(term.clone());
        }
    }
    if symbol.visibility == "pub" {
        score += 4.0;
    }
    if symbol
        .source_providers
        .iter()
        .any(|source| source == "rust_analyzer_lsp")
    {
        score += 6.0;
    }
    if !terms.is_empty() {
        score += symbol.importance_score.unwrap_or_default();
    }
    (score, matched.into_iter().collect())
}

fn compare_symbols(left: &SymbolHit, right: &SymbolHit) -> Ordering {
    right
        .score
        .partial_cmp(&left.score)
        .unwrap_or(Ordering::Equal)
        .then_with(|| left.file_path.cmp(&right.file_path))
        .then_with(|| left.start_line.cmp(&right.start_line))
        .then_with(|| left.qualified_name.cmp(&right.qualified_name))
}

fn parse_sources(value: &str) -> Vec<String> {
    serde_json::from_str(value).unwrap_or_default()
}

fn placeholders(count: usize) -> String {
    std::iter::repeat("?")
        .take(count)
        .collect::<Vec<_>>()
        .join(",")
}

fn clean_filter(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn tokenize(value: &str) -> Vec<String> {
    value
        .split(|ch: char| !(ch == '_' || ch.is_ascii_alphanumeric()))
        .filter(|term| !term.is_empty())
        .map(|term| term.to_ascii_lowercase())
        .collect()
}

fn safe_component(value: &str) -> String {
    value
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || *ch == '-' || *ch == '_')
        .take(64)
        .collect()
}

fn to_usize(value: i64) -> usize {
    usize::try_from(value).unwrap_or_default()
}
