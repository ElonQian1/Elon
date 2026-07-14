use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

use anyhow::{bail, Context, Result};
use rusqlite::{params_from_iter, types::Value, Connection, OpenFlags, Row};

use super::{
    symbol_index::normalize_path,
    symbol_index_impact_types::{
        ImpactFile, ImpactTestHint, SymbolImpactQuery, SymbolImpactQueryEcho, SymbolImpactResponse,
    },
    symbol_index_query::find_symbol_index_db,
    symbol_index_query_types::{SymbolEdgeHit, SymbolHit},
};

#[derive(Default)]
struct FileImpactAccumulator {
    seed: bool,
    symbol_count: usize,
    edge_count: usize,
    test_hint_count: usize,
}

pub(crate) fn load_latest_symbol_impact(
    data_dir: &Path,
    query: &SymbolImpactQuery,
) -> Result<SymbolImpactResponse> {
    let db_path = find_symbol_index_db(data_dir, query.trace_id.as_deref())
        .context("没有找到可查询的 symbol_index.sqlite，请先运行一次 context compiler")?;
    load_symbol_impact_db(&db_path, query)
}

pub(crate) fn load_symbol_impact_db(
    db_path: &Path,
    query: &SymbolImpactQuery,
) -> Result<SymbolImpactResponse> {
    let conn = Connection::open_with_flags(db_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .with_context(|| format!("打开符号索引数据库失败: {}", db_path.display()))?;
    let metadata = load_metadata(&conn)?;
    let seed_symbols = load_seed_symbols(&conn, query)?;
    if seed_symbols.is_empty() {
        bail!("没有找到影响面查询种子，请检查 id 或 path");
    }

    let seed_ids = seed_symbols
        .iter()
        .map(|symbol| symbol.id.clone())
        .collect::<BTreeSet<_>>();
    let edges = traverse_edges(&conn, &seed_ids, query)?;
    let impacted_ids = collect_impacted_ids(&seed_ids, &edges);
    let impacted_symbols = load_symbols_by_ids(&conn, &impacted_ids)?;
    let mut symbol_lookup = seed_symbols
        .iter()
        .chain(impacted_symbols.iter())
        .map(|symbol| (symbol.id.clone(), symbol.clone()))
        .collect::<BTreeMap<_, _>>();
    for symbol in load_edge_path_symbols(&conn, &edges)? {
        symbol_lookup.entry(symbol.id.clone()).or_insert(symbol);
    }
    let test_hints = build_test_hints(&symbol_lookup, &edges);
    let impacted_files =
        build_impacted_files(&seed_symbols, &impacted_symbols, &edges, &test_hints);

    Ok(SymbolImpactResponse {
        db_path: db_path.to_string_lossy().replace('\\', "/"),
        query: SymbolImpactQueryEcho {
            trace_id: query.trace_id.clone(),
            symbol_id: query.symbol_id.clone(),
            path: query.path.clone(),
            edge_kind: query.edge_kind.clone(),
            depth: query.depth(),
            limit: query.limit(),
        },
        metadata,
        seed_symbols,
        impacted_symbols,
        edges,
        impacted_files,
        test_hints,
    })
}

#[path = "symbol_index_impact_query_impl.rs"]
mod impl_funcs;
use self::impl_funcs::*;
