use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

use anyhow::{bail, Context, Result};
use rusqlite::{params_from_iter, types::Value, Connection, OpenFlags, Row};
use serde::Serialize;

use super::{
    symbol_index::normalize_path,
    symbol_index_query::find_symbol_index_db,
    symbol_index_query_types::{SymbolEdgeHit, SymbolHit, MAX_EDGE_LIMIT},
};

const DEFAULT_GRAPH_LIMIT: usize = 80;

#[derive(Debug, Clone)]
pub(crate) struct SymbolGraphQuery {
    pub(crate) trace_id: Option<String>,
    pub(crate) symbol_id: String,
    pub(crate) edge_kind: Option<String>,
    pub(crate) direction: SymbolRelationDirection,
    pub(crate) limit: usize,
}

impl SymbolGraphQuery {
    fn limit(&self) -> usize {
        if self.limit == 0 {
            DEFAULT_GRAPH_LIMIT
        } else {
            self.limit.min(MAX_EDGE_LIMIT)
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum SymbolRelationDirection {
    Incoming,
    Outgoing,
    #[default]
    Both,
}

impl SymbolRelationDirection {
    pub(crate) fn from_query_value(value: Option<&str>) -> Self {
        match value
            .map(str::trim)
            .unwrap_or_default()
            .to_ascii_lowercase()
            .as_str()
        {
            "incoming" | "in" | "callers" | "references" => Self::Incoming,
            "outgoing" | "out" | "callees" | "calls" => Self::Outgoing,
            _ => Self::Both,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SymbolGraphResponse {
    pub(crate) db_path: String,
    pub(crate) query: SymbolGraphQueryEcho,
    pub(crate) metadata: BTreeMap<String, String>,
    pub(crate) symbol: SymbolHit,
    pub(crate) edges: Vec<SymbolEdgeHit>,
    pub(crate) related_symbols: Vec<SymbolHit>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SymbolGraphQueryEcho {
    pub(crate) trace_id: Option<String>,
    pub(crate) symbol_id: String,
    pub(crate) edge_kind: Option<String>,
    pub(crate) direction: SymbolRelationDirection,
    pub(crate) limit: usize,
}

pub(crate) fn load_latest_symbol_graph(
    data_dir: &Path,
    query: &SymbolGraphQuery,
) -> Result<SymbolGraphResponse> {
    let db_path = find_symbol_index_db(data_dir, query.trace_id.as_deref())
        .context("没有找到可查询的 symbol_index.sqlite，请先运行一次 context compiler")?;
    load_symbol_graph_db(&db_path, query)
}

pub(crate) fn load_symbol_graph_db(
    db_path: &Path,
    query: &SymbolGraphQuery,
) -> Result<SymbolGraphResponse> {
    let conn = Connection::open_with_flags(db_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .with_context(|| format!("打开符号索引数据库失败: {}", db_path.display()))?;
    let metadata = load_metadata(&conn)?;
    let symbol = load_symbol(&conn, &query.symbol_id)?
        .ok_or_else(|| anyhow::anyhow!("symbol_id 不存在: {}", query.symbol_id))?;
    let edges = load_symbol_edges(&conn, query)?;
    let related_symbols = load_related_symbols(&conn, &query.symbol_id, &edges)?;

    Ok(SymbolGraphResponse {
        db_path: db_path.to_string_lossy().replace('\\', "/"),
        query: SymbolGraphQueryEcho {
            trace_id: query.trace_id.clone(),
            symbol_id: query.symbol_id.clone(),
            edge_kind: query.edge_kind.clone(),
            direction: query.direction,
            limit: query.limit(),
        },
        metadata,
        symbol,
        edges,
        related_symbols,
    })
}

fn load_metadata(conn: &Connection) -> Result<BTreeMap<String, String>> {
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

fn load_symbol(conn: &Connection, symbol_id: &str) -> Result<Option<SymbolHit>> {
    let mut stmt = conn.prepare(&format!(
        "{} WHERE id = ? ORDER BY file_path, start_line LIMIT 1",
        symbol_select_sql()
    ))?;
    let mut rows = stmt.query_map([symbol_id], symbol_from_row)?;
    rows.next().transpose().map_err(Into::into)
}

fn load_related_symbols(
    conn: &Connection,
    symbol_id: &str,
    edges: &[SymbolEdgeHit],
) -> Result<Vec<SymbolHit>> {
    let ids = edges
        .iter()
        .flat_map(|edge| [edge.from_symbol_id.as_deref(), edge.to_symbol_id.as_deref()])
        .flatten()
        .filter(|id| *id != symbol_id)
        .map(ToOwned::to_owned)
        .collect::<BTreeSet<_>>();
    if ids.is_empty() {
        return Ok(Vec::new());
    }

    let placeholders = placeholders(ids.len());
    let sql = format!(
        "{} WHERE id IN ({placeholders}) ORDER BY file_path, start_line",
        symbol_select_sql()
    );
    let params = ids.into_iter().map(Value::Text).collect::<Vec<_>>();
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params_from_iter(params.iter()), symbol_from_row)?;
    let mut symbols = Vec::new();
    for row in rows {
        symbols.push(row?);
    }
    Ok(symbols)
}

fn load_symbol_edges(conn: &Connection, query: &SymbolGraphQuery) -> Result<Vec<SymbolEdgeHit>> {
    let mut sql = String::from(
        r#"
        SELECT
            id, source, kind, from_symbol_id, from_path, line, to_symbol_id,
            to_symbol_name, to_path, confidence, reason
        FROM edges
        "#,
    );
    let mut clauses = Vec::new();
    let mut params = Vec::new();

    match query.direction {
        SymbolRelationDirection::Incoming => {
            clauses.push("to_symbol_id = ?");
            params.push(Value::Text(query.symbol_id.clone()));
        }
        SymbolRelationDirection::Outgoing => {
            clauses.push("from_symbol_id = ?");
            params.push(Value::Text(query.symbol_id.clone()));
        }
        SymbolRelationDirection::Both => {
            clauses.push("(from_symbol_id = ? OR to_symbol_id = ?)");
            params.push(Value::Text(query.symbol_id.clone()));
            params.push(Value::Text(query.symbol_id.clone()));
        }
    }
    if let Some(edge_kind) = clean_filter(query.edge_kind.as_deref()) {
        clauses.push("lower(kind) = lower(?)");
        params.push(Value::Text(edge_kind));
    }
    if clauses.is_empty() {
        bail!("symbol graph query 缺少过滤条件");
    }

    sql.push_str(" WHERE ");
    sql.push_str(&clauses.join(" AND "));
    sql.push_str(" ORDER BY confidence DESC, source, kind, from_path, line LIMIT ?");
    params.push(Value::Integer(
        i64::try_from(query.limit()).unwrap_or(i64::MAX),
    ));

    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params_from_iter(params.iter()), edge_from_row)?;
    let mut edges = Vec::new();
    for row in rows {
        edges.push(row?);
    }
    Ok(edges)
}

fn symbol_select_sql() -> &'static str {
    r#"
        SELECT
            id, name, qualified_name, kind, language, file_path, start_line, end_line,
            signature, visibility, parent_symbol_id, module_path, doc_summary, role,
            importance_score, source_providers_json
        FROM symbols
        "#
}

fn symbol_from_row(row: &Row<'_>) -> rusqlite::Result<SymbolHit> {
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
        source_providers: serde_json::from_str(&source_json).unwrap_or_default(),
        score: 0.0,
        matched_terms: Vec::new(),
    })
}

fn edge_from_row(row: &Row<'_>) -> rusqlite::Result<SymbolEdgeHit> {
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

fn to_usize(value: i64) -> usize {
    usize::try_from(value).unwrap_or_default()
}
