use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

use anyhow::{Context, Result};
use rusqlite::{params, params_from_iter, types::Value, Connection, OpenFlags, Transaction};

use super::{
    symbol_index::{normalize_path, stable_hash, SymbolIndex, SymbolRecord},
    symbol_index_query::{find_symbol_index_db, load_metadata},
};

pub(crate) use super::symbol_index_chunk_types::{
    SymbolChunkHit, SymbolChunkQueryEcho, SymbolChunkSearch, SymbolChunkSearchResponse,
};

#[derive(Debug, Clone)]
struct SymbolChunk {
    id: String,
    chunk_type: String,
    file_path: String,
    symbol_id: Option<String>,
    qualified_name: Option<String>,
    kind: Option<String>,
    start_line: Option<usize>,
    end_line: Option<usize>,
    content: String,
    summary: Option<String>,
    hash: String,
    token_count: usize,
}

pub(crate) fn create_chunk_schema(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE chunks (
            rowid INTEGER PRIMARY KEY,
            id TEXT NOT NULL UNIQUE,
            chunk_type TEXT NOT NULL,
            file_path TEXT NOT NULL,
            symbol_id TEXT,
            qualified_name TEXT,
            kind TEXT,
            start_line INTEGER,
            end_line INTEGER,
            content TEXT NOT NULL,
            summary TEXT,
            hash TEXT NOT NULL,
            token_count INTEGER NOT NULL,
            updated_at INTEGER
        );

        CREATE VIRTUAL TABLE chunks_fts USING fts5(
            content,
            summary,
            qualified_name,
            file_path,
            content='chunks',
            content_rowid='rowid'
        );

        CREATE INDEX idx_chunks_file_path ON chunks(file_path);
        CREATE INDEX idx_chunks_symbol ON chunks(symbol_id);
        CREATE INDEX idx_chunks_type ON chunks(chunk_type);
        "#,
    )
}

pub(crate) fn insert_symbol_chunks(
    tx: &Transaction<'_>,
    index: &SymbolIndex,
) -> rusqlite::Result<usize> {
    let chunks = build_symbol_chunks(index);
    let mut insert_chunk = tx.prepare(
        r#"
        INSERT INTO chunks(
            id, chunk_type, file_path, symbol_id, qualified_name, kind,
            start_line, end_line, content, summary, hash, token_count, updated_at
        ) VALUES (
            ?1, ?2, ?3, ?4, ?5, ?6,
            ?7, ?8, ?9, ?10, ?11, ?12, strftime('%s','now')
        )
        "#,
    )?;
    let mut insert_fts = tx.prepare(
        r#"
        INSERT INTO chunks_fts(rowid, content, summary, qualified_name, file_path)
        VALUES (?1, ?2, ?3, ?4, ?5)
        "#,
    )?;

    for chunk in &chunks {
        insert_chunk.execute(params![
            chunk.id,
            chunk.chunk_type,
            chunk.file_path,
            chunk.symbol_id,
            chunk.qualified_name,
            chunk.kind,
            chunk.start_line.map(to_i64),
            chunk.end_line.map(to_i64),
            chunk.content,
            chunk.summary,
            chunk.hash,
            to_i64(chunk.token_count),
        ])?;
        insert_fts.execute(params![
            tx.last_insert_rowid(),
            chunk.content,
            chunk.summary,
            chunk.qualified_name,
            chunk.file_path,
        ])?;
    }
    Ok(chunks.len())
}

pub(crate) fn search_latest_symbol_chunks(
    data_dir: &Path,
    search: &SymbolChunkSearch,
) -> Result<SymbolChunkSearchResponse> {
    let db_path = find_symbol_index_db(data_dir, search.trace_id.as_deref())
        .context("没有找到可查询的 symbol_index.sqlite，请先运行一次 context compiler")?;
    search_symbol_chunks_db(&db_path, search)
}

pub(crate) fn search_symbol_chunks_db(
    db_path: &Path,
    search: &SymbolChunkSearch,
) -> Result<SymbolChunkSearchResponse> {
    let conn = Connection::open_with_flags(db_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .with_context(|| format!("打开符号索引数据库失败: {}", db_path.display()))?;
    let metadata = load_metadata(&conn)?;
    let fts_query = build_fts_query(search.text.as_deref().unwrap_or_default());
    let chunks = if let Some(fts_query) = fts_query {
        search_fts_chunks(&conn, &fts_query, search)?
    } else {
        load_default_chunks(&conn, search)?
    };

    Ok(SymbolChunkSearchResponse {
        db_path: db_path.to_string_lossy().replace('\\', "/"),
        query: SymbolChunkQueryEcho {
            trace_id: search.trace_id.clone(),
            q: search.text.clone(),
            path: clean_filter(search.path.as_deref()),
            chunk_type: clean_filter(search.chunk_type.as_deref()),
            limit: search.limit(),
        },
        metadata,
        chunks,
    })
}

fn build_symbol_chunks(index: &SymbolIndex) -> Vec<SymbolChunk> {
    let mut chunks = Vec::new();
    for record in &index.records {
        chunks.push(symbol_chunk(record));
        if is_test_symbol(record) {
            chunks.push(test_chunk(record));
        }
    }
    chunks.extend(module_chunks(index));
    chunks
}

fn symbol_chunk(record: &SymbolRecord) -> SymbolChunk {
    let summary = Some(format!(
        "{} {} in {}:{}",
        record.kind, record.qualified_name, record.file_path, record.start_line
    ));
    let content = compact_lines([
        Some(format!("chunk_type: symbol")),
        Some(format!("symbol: {}", record.qualified_name)),
        Some(format!("name: {}", record.name)),
        Some(format!("kind: {}", record.kind)),
        Some(format!("file: {}", record.file_path)),
        Some(format!("lines: {}-{}", record.start_line, record.end_line)),
        Some(format!("module: {}", record.module_path)),
        Some(format!("visibility: {}", record.visibility)),
        Some(format!("role: {}", record.role)),
        Some(format!("signature: {}", record.signature)),
        record
            .doc_summary
            .as_deref()
            .map(|docs| format!("docs: {docs}")),
        Some(format!("sources: {}", record.source_providers.join(", "))),
    ]);
    make_chunk(
        format!("symbol:{}", record.id),
        "symbol",
        &record.file_path,
        Some(record),
        content,
        summary,
    )
}

fn test_chunk(record: &SymbolRecord) -> SymbolChunk {
    let content = compact_lines([
        Some(format!("chunk_type: test")),
        Some(format!("test: {}", record.qualified_name)),
        Some(format!("name: {}", record.name)),
        Some(format!("file: {}", record.file_path)),
        Some(format!("lines: {}-{}", record.start_line, record.end_line)),
        Some(format!("signature: {}", record.signature)),
        record
            .doc_summary
            .as_deref()
            .map(|docs| format!("docs: {docs}")),
    ]);
    make_chunk(
        format!("test:{}", record.id),
        "test",
        &record.file_path,
        Some(record),
        content,
        Some(format!(
            "test {} in {}:{}",
            record.qualified_name, record.file_path, record.start_line
        )),
    )
}

fn module_chunks(index: &SymbolIndex) -> Vec<SymbolChunk> {
    let mut by_file: BTreeMap<String, Vec<&SymbolRecord>> = BTreeMap::new();
    for record in &index.records {
        by_file
            .entry(normalize_path(&record.file_path))
            .or_default()
            .push(record);
    }

    by_file
        .into_iter()
        .map(|(path, mut symbols)| {
            symbols.sort_by_key(|symbol| symbol.start_line);
            let names = symbols
                .iter()
                .take(80)
                .map(|symbol| {
                    format!(
                        "{} {} line {}",
                        symbol.kind, symbol.qualified_name, symbol.start_line
                    )
                })
                .collect::<Vec<_>>();
            let content = compact_lines([
                Some("chunk_type: module".to_string()),
                Some(format!("file: {path}")),
                Some(format!("symbol_count: {}", symbols.len())),
                Some(format!("symbols:\n- {}", names.join("\n- "))),
            ]);
            let hash = stable_hash(&content);
            SymbolChunk {
                id: format!("module:{path}"),
                chunk_type: "module".to_string(),
                file_path: path.clone(),
                symbol_id: None,
                qualified_name: None,
                kind: None,
                start_line: symbols.first().map(|symbol| symbol.start_line),
                end_line: symbols.last().map(|symbol| symbol.end_line),
                token_count: estimate_token_count(&content),
                content,
                summary: Some(format!("module chunk for {path}")),
                hash,
            }
        })
        .collect()
}

fn make_chunk(
    id: String,
    chunk_type: &str,
    file_path: &str,
    record: Option<&SymbolRecord>,
    content: String,
    summary: Option<String>,
) -> SymbolChunk {
    let hash = stable_hash(&content);
    SymbolChunk {
        id,
        chunk_type: chunk_type.to_string(),
        file_path: normalize_path(file_path),
        symbol_id: record.map(|record| record.id.clone()),
        qualified_name: record.map(|record| record.qualified_name.clone()),
        kind: record.map(|record| record.kind.clone()),
        start_line: record.map(|record| record.start_line),
        end_line: record.map(|record| record.end_line),
        token_count: estimate_token_count(&content),
        content,
        summary,
        hash,
    }
}

fn search_fts_chunks(
    conn: &Connection,
    fts_query: &str,
    search: &SymbolChunkSearch,
) -> Result<Vec<SymbolChunkHit>> {
    let mut sql = String::from(
        r#"
        SELECT
            chunks.id, chunks.chunk_type, chunks.file_path, chunks.symbol_id,
            chunks.qualified_name, chunks.kind, chunks.start_line, chunks.end_line,
            chunks.content, chunks.summary, chunks.hash, chunks.token_count,
            bm25(chunks_fts) AS rank
        FROM chunks_fts
        JOIN chunks ON chunks.rowid = chunks_fts.rowid
        WHERE chunks_fts MATCH ?
        "#,
    );
    let mut values = vec![Value::Text(fts_query.to_string())];
    append_chunk_filters(&mut sql, &mut values, search);
    sql.push_str(
        " ORDER BY rank ASC, chunks.chunk_type, chunks.file_path, chunks.start_line LIMIT ?",
    );
    values.push(Value::Integer(to_i64(search.limit())));

    let terms = query_terms(search.text.as_deref().unwrap_or_default());
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params_from_iter(values.iter()), |row| {
        let rank = row.get::<_, f64>(12)?;
        Ok(row_to_hit(row, -rank, &terms))
    })?;
    collect_hits(rows)
}

fn load_default_chunks(
    conn: &Connection,
    search: &SymbolChunkSearch,
) -> Result<Vec<SymbolChunkHit>> {
    let mut sql = String::from(
        r#"
        SELECT
            id, chunk_type, file_path, symbol_id, qualified_name, kind,
            start_line, end_line, content, summary, hash, token_count
        FROM chunks
        WHERE 1 = 1
        "#,
    );
    let mut values = Vec::new();
    append_chunk_filters(&mut sql, &mut values, search);
    sql.push_str(
        " ORDER BY CASE chunk_type WHEN 'symbol' THEN 0 WHEN 'module' THEN 1 ELSE 2 END, file_path, start_line LIMIT ?",
    );
    values.push(Value::Integer(to_i64(search.limit())));

    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params_from_iter(values.iter()), |row| {
        Ok(row_to_hit(row, 0.0, &[]))
    })?;
    collect_hits(rows)
}

fn append_chunk_filters(sql: &mut String, values: &mut Vec<Value>, search: &SymbolChunkSearch) {
    if let Some(path) = clean_filter(search.path.as_deref()) {
        sql.push_str(" AND lower(replace(chunks.file_path, char(92), '/')) LIKE lower(?)");
        values.push(Value::Text(format!("%{}%", normalize_path(&path))));
    }
    if let Some(chunk_type) = clean_filter(search.chunk_type.as_deref()) {
        sql.push_str(" AND lower(chunks.chunk_type) = lower(?)");
        values.push(Value::Text(chunk_type));
    }
}

fn row_to_hit(row: &rusqlite::Row<'_>, score: f64, terms: &[String]) -> SymbolChunkHit {
    let content = row.get::<_, String>(8).unwrap_or_default();
    SymbolChunkHit {
        id: row.get(0).unwrap_or_default(),
        chunk_type: row.get(1).unwrap_or_default(),
        file_path: row
            .get::<_, String>(2)
            .map(|path| normalize_path(&path))
            .unwrap_or_default(),
        symbol_id: row.get(3).unwrap_or_default(),
        qualified_name: row.get(4).unwrap_or_default(),
        kind: row.get(5).unwrap_or_default(),
        start_line: row
            .get::<_, Option<i64>>(6)
            .unwrap_or_default()
            .map(to_usize),
        end_line: row
            .get::<_, Option<i64>>(7)
            .unwrap_or_default()
            .map(to_usize),
        summary: row.get(9).unwrap_or_default(),
        hash: row.get(10).unwrap_or_default(),
        token_count: row.get::<_, i64>(11).map(to_usize).unwrap_or_default(),
        matched_terms: matched_terms(&content, terms),
        content,
        score,
    }
}

fn collect_hits(
    rows: rusqlite::MappedRows<
        '_,
        impl FnMut(&rusqlite::Row<'_>) -> rusqlite::Result<SymbolChunkHit>,
    >,
) -> Result<Vec<SymbolChunkHit>> {
    let mut hits = Vec::new();
    for row in rows {
        hits.push(row?);
    }
    Ok(hits)
}

fn build_fts_query(value: &str) -> Option<String> {
    let terms = query_terms(value);
    (!terms.is_empty()).then(|| {
        terms
            .into_iter()
            .map(|term| format!("{term}*"))
            .collect::<Vec<_>>()
            .join(" OR ")
    })
}

fn query_terms(value: &str) -> Vec<String> {
    let mut terms = BTreeSet::new();
    for term in value
        .split(|ch: char| !(ch == '_' || ch.is_ascii_alphanumeric()))
        .filter(|term| !term.is_empty())
        .map(|term| term.to_ascii_lowercase())
    {
        terms.insert(term);
    }
    terms.into_iter().collect()
}

fn matched_terms(content: &str, terms: &[String]) -> Vec<String> {
    let content = content.to_ascii_lowercase();
    terms
        .iter()
        .filter(|term| content.contains(term.as_str()))
        .cloned()
        .collect()
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

fn is_test_symbol(record: &SymbolRecord) -> bool {
    let name = record.name.to_ascii_lowercase();
    let path = record.file_path.to_ascii_lowercase();
    let signature = record.signature.to_ascii_lowercase();
    path.contains("/tests/")
        || path.ends_with("_test.rs")
        || path.ends_with("_tests.rs")
        || path.contains("tests.rs")
        || name.contains("test")
        || signature.contains("#[test]")
        || signature.contains("#[tokio::test]")
}

fn estimate_token_count(value: &str) -> usize {
    value
        .split_whitespace()
        .map(|part| (part.len() / 4).max(1))
        .sum()
}

fn clean_filter(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn to_i64(value: usize) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

fn to_usize(value: i64) -> usize {
    usize::try_from(value).unwrap_or_default()
}
