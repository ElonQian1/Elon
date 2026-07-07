use super::*;

pub(super) fn load_metadata(conn: &Connection) -> Result<BTreeMap<String, String>> {
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

pub(super) fn load_seed_symbols(conn: &Connection, query: &SymbolImpactQuery) -> Result<Vec<SymbolHit>> {
    let mut by_id = BTreeMap::new();
    if let Some(symbol_id) = clean_filter(query.symbol_id.as_deref()) {
        if let Some(symbol) = load_symbol_by_id(conn, &symbol_id)? {
            by_id.insert(symbol.id.clone(), symbol);
        }
    }
    if let Some(path) = clean_filter(query.path.as_deref()) {
        for symbol in load_symbols_by_path(conn, &path, query.limit())? {
            by_id.insert(symbol.id.clone(), symbol);
        }
    }
    let mut symbols = by_id.into_values().collect::<Vec<_>>();
    sort_symbols(&mut symbols);
    Ok(symbols)
}

pub(super) fn load_symbol_by_id(conn: &Connection, symbol_id: &str) -> Result<Option<SymbolHit>> {
    let mut stmt = conn.prepare(&format!(
        "{} WHERE id = ? ORDER BY file_path, start_line LIMIT 1",
        symbol_select_sql()
    ))?;
    let mut rows = stmt.query_map([symbol_id], symbol_from_row)?;
    rows.next().transpose().map_err(Into::into)
}

pub(super) fn load_symbols_by_path(conn: &Connection, path: &str, limit: usize) -> Result<Vec<SymbolHit>> {
    let sql = format!(
        "{} WHERE lower(replace(file_path, char(92), '/')) LIKE lower(?) ORDER BY file_path, start_line LIMIT ?",
        symbol_select_sql()
    );
    let params = [
        Value::Text(format!("%{}%", normalize_path(path))),
        Value::Integer(i64::try_from(limit).unwrap_or(i64::MAX)),
    ];
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params_from_iter(params.iter()), symbol_from_row)?;
    collect_rows(rows)
}

pub(super) fn load_symbols_by_ids(conn: &Connection, ids: &BTreeSet<String>) -> Result<Vec<SymbolHit>> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    let placeholders = placeholders(ids.len());
    let sql = format!(
        "{} WHERE id IN ({placeholders}) ORDER BY file_path, start_line",
        symbol_select_sql()
    );
    let params = ids.iter().cloned().map(Value::Text).collect::<Vec<_>>();
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params_from_iter(params.iter()), symbol_from_row)?;
    collect_rows(rows)
}

pub(super) fn load_edge_path_symbols(conn: &Connection, edges: &[SymbolEdgeHit]) -> Result<Vec<SymbolHit>> {
    let ids = edges
        .iter()
        .flat_map(|edge| [edge.from_symbol_id.as_deref(), edge.to_symbol_id.as_deref()])
        .flatten()
        .map(ToOwned::to_owned)
        .collect::<BTreeSet<_>>();
    load_symbols_by_ids(conn, &ids)
}

pub(super) fn traverse_edges(
    conn: &Connection,
    seed_ids: &BTreeSet<String>,
    query: &SymbolImpactQuery,
) -> Result<Vec<SymbolEdgeHit>> {
    let mut visited = seed_ids.clone();
    let mut frontier = seed_ids.clone();
    let mut seen_edges = BTreeSet::new();
    let mut edges = Vec::new();

    for _ in 0..query.depth() {
        if frontier.is_empty() || edges.len() >= query.limit() {
            break;
        }
        let remaining = query.limit().saturating_sub(edges.len());
        let frontier_edges =
            load_edges_for_symbols(conn, &frontier, query.edge_kind.as_deref(), remaining)?;
        let mut next_frontier = BTreeSet::new();
        for edge in frontier_edges {
            let edge_id = edge.id.clone();
            if !seen_edges.insert(edge_id) {
                continue;
            }
            for symbol_id in edge
                .from_symbol_id
                .iter()
                .chain(edge.to_symbol_id.iter())
                .filter(|id| !id.is_empty())
            {
                if visited.insert(symbol_id.clone()) {
                    next_frontier.insert(symbol_id.clone());
                }
            }
            edges.push(edge);
        }
        frontier = next_frontier;
    }
    Ok(edges)
}

pub(super) fn load_edges_for_symbols(
    conn: &Connection,
    symbol_ids: &BTreeSet<String>,
    edge_kind: Option<&str>,
    limit: usize,
) -> Result<Vec<SymbolEdgeHit>> {
    if symbol_ids.is_empty() || limit == 0 {
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
        .collect::<Vec<_>>();
    if let Some(kind) = clean_filter(edge_kind) {
        sql.push_str(" AND lower(kind) = lower(?)");
        params.push(Value::Text(kind));
    }
    sql.push_str(" ORDER BY confidence DESC, source, kind, from_path, line LIMIT ?");
    params.push(Value::Integer(i64::try_from(limit).unwrap_or(i64::MAX)));

    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params_from_iter(params.iter()), edge_from_row)?;
    collect_rows(rows)
}

pub(super) fn collect_impacted_ids(seed_ids: &BTreeSet<String>, edges: &[SymbolEdgeHit]) -> BTreeSet<String> {
    edges
        .iter()
        .flat_map(|edge| [edge.from_symbol_id.as_deref(), edge.to_symbol_id.as_deref()])
        .flatten()
        .filter(|id| !seed_ids.contains(*id))
        .map(ToOwned::to_owned)
        .collect()
}

pub(super) fn build_test_hints(
    symbols: &BTreeMap<String, SymbolHit>,
    edges: &[SymbolEdgeHit],
) -> Vec<ImpactTestHint> {
    let mut hints = Vec::new();
    let mut seen = BTreeSet::new();

    for edge in edges.iter().filter(|edge| edge.kind == "test_covers") {
        if let Some(test_id) = edge.from_symbol_id.as_deref() {
            let symbol = symbols.get(test_id);
            let name = symbol
                .map(|symbol| symbol.name.clone())
                .or_else(|| edge.to_symbol_name.clone())
                .unwrap_or_else(|| test_id.to_string());
            push_hint(
                &mut hints,
                &mut seen,
                ImpactTestHint {
                    symbol_id: test_id.to_string(),
                    symbol_name: name,
                    path: edge.from_path.clone(),
                    line: edge.line,
                    reason: edge.reason.clone(),
                    edge_kind: Some(edge.kind.clone()),
                    target_symbol_id: edge.to_symbol_id.clone(),
                },
            );
        }
    }

    for symbol in symbols.values().filter(|symbol| looks_like_test(symbol)) {
        push_hint(
            &mut hints,
            &mut seen,
            ImpactTestHint {
                symbol_id: symbol.id.clone(),
                symbol_name: symbol.name.clone(),
                path: symbol.file_path.clone(),
                line: symbol.start_line,
                reason: "symbol looks like a test".to_string(),
                edge_kind: None,
                target_symbol_id: None,
            },
        );
    }
    hints.sort_by(|left, right| {
        left.path
            .cmp(&right.path)
            .then_with(|| left.line.cmp(&right.line))
            .then_with(|| left.symbol_name.cmp(&right.symbol_name))
    });
    hints
}

pub(super) fn push_hint(
    hints: &mut Vec<ImpactTestHint>,
    seen: &mut BTreeSet<(String, String, usize)>,
    hint: ImpactTestHint,
) {
    if seen.insert((hint.symbol_id.clone(), hint.path.clone(), hint.line)) {
        hints.push(hint);
    }
}

pub(super) fn looks_like_test(symbol: &SymbolHit) -> bool {
    let path = symbol.file_path.to_ascii_lowercase();
    let name = symbol.name.to_ascii_lowercase();
    let signature = symbol.signature.to_ascii_lowercase();
    path.contains("/tests/")
        || path.ends_with("_test.rs")
        || path.ends_with("_tests.rs")
        || name.contains("test")
        || signature.contains("#[test]")
        || signature.contains("#[tokio::test]")
}

pub(super) fn build_impacted_files(
    seed_symbols: &[SymbolHit],
    impacted_symbols: &[SymbolHit],
    edges: &[SymbolEdgeHit],
    test_hints: &[ImpactTestHint],
) -> Vec<ImpactFile> {
    let mut files = BTreeMap::<String, FileImpactAccumulator>::new();
    for symbol in seed_symbols {
        let entry = files.entry(symbol.file_path.clone()).or_default();
        entry.seed = true;
        entry.symbol_count += 1;
    }
    for symbol in impacted_symbols {
        files
            .entry(symbol.file_path.clone())
            .or_default()
            .symbol_count += 1;
    }

    let mut counted_edges = BTreeSet::new();
    for edge in edges {
        count_edge_file(&mut files, &mut counted_edges, &edge.from_path, &edge.id);
        if let Some(path) = edge.to_path.as_deref() {
            count_edge_file(&mut files, &mut counted_edges, path, &edge.id);
        }
    }
    for hint in test_hints {
        files.entry(hint.path.clone()).or_default().test_hint_count += 1;
    }

    let mut impacts = files
        .into_iter()
        .map(|(path, item)| ImpactFile {
            path,
            seed: item.seed,
            symbol_count: item.symbol_count,
            edge_count: item.edge_count,
            test_hint_count: item.test_hint_count,
        })
        .collect::<Vec<_>>();
    impacts.sort_by(|left, right| {
        right
            .seed
            .cmp(&left.seed)
            .then_with(|| right.test_hint_count.cmp(&left.test_hint_count))
            .then_with(|| right.edge_count.cmp(&left.edge_count))
            .then_with(|| right.symbol_count.cmp(&left.symbol_count))
            .then_with(|| left.path.cmp(&right.path))
    });
    impacts
}

pub(super) fn count_edge_file(
    files: &mut BTreeMap<String, FileImpactAccumulator>,
    counted: &mut BTreeSet<(String, String)>,
    path: &str,
    edge_id: &str,
) {
    let path = normalize_path(path);
    if counted.insert((path.clone(), edge_id.to_string())) {
        files.entry(path).or_default().edge_count += 1;
    }
}

pub(super) fn symbol_select_sql() -> &'static str {
    r#"
        SELECT
            id, name, qualified_name, kind, language, file_path, start_line, end_line,
            signature, visibility, parent_symbol_id, module_path, doc_summary, role,
            importance_score, source_providers_json
        FROM symbols
        "#
}

pub(super) fn symbol_from_row(row: &Row<'_>) -> rusqlite::Result<SymbolHit> {
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

pub(super) fn edge_from_row(row: &Row<'_>) -> rusqlite::Result<SymbolEdgeHit> {
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

pub(super) fn collect_rows<T>(
    rows: rusqlite::MappedRows<'_, impl FnMut(&Row<'_>) -> rusqlite::Result<T>>,
) -> Result<Vec<T>> {
    let mut items = Vec::new();
    for row in rows {
        items.push(row?);
    }
    Ok(items)
}

pub(super) fn sort_symbols(symbols: &mut [SymbolHit]) {
    symbols.sort_by(|left, right| {
        left.file_path
            .cmp(&right.file_path)
            .then_with(|| left.start_line.cmp(&right.start_line))
            .then_with(|| left.qualified_name.cmp(&right.qualified_name))
    });
}

pub(super) fn placeholders(count: usize) -> String {
    std::iter::repeat("?")
        .take(count)
        .collect::<Vec<_>>()
        .join(",")
}

pub(super) fn clean_filter(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

pub(super) fn to_usize(value: i64) -> usize {
    usize::try_from(value).unwrap_or_default()
}
