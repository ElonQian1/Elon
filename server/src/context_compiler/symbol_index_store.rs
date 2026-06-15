use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

use rusqlite::{params, Connection, Transaction};

use super::{
    symbol_index::{SymbolEdge, SymbolIndex, SymbolRecord},
    symbol_index_chunks::{create_chunk_schema, insert_symbol_chunks},
};

pub(crate) const SYMBOL_INDEX_DB_FILE: &str = "symbol_index.sqlite";

pub(crate) fn write_symbol_index_sqlite(
    path: &Path,
    index: &SymbolIndex,
    files: &mut Vec<PathBuf>,
) -> Option<usize> {
    if path.exists() {
        fs::remove_file(path).ok()?;
    }

    let mut conn = Connection::open(path).ok()?;
    create_schema(&conn).ok()?;
    {
        let tx = conn.transaction().ok()?;
        insert_metadata(&tx, index).ok()?;
        insert_symbols(&tx, index).ok()?;
        insert_edges(&tx, index).ok()?;
        insert_symbol_chunks(&tx, index).ok()?;
        tx.commit().ok()?;
    }
    files.push(path.to_path_buf());
    fs::metadata(path)
        .ok()
        .map(|metadata| metadata.len().min(usize::MAX as u64) as usize)
}

fn create_schema(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        r#"
        PRAGMA user_version = 3;

        CREATE TABLE metadata (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );

        CREATE TABLE symbols (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            qualified_name TEXT NOT NULL,
            kind TEXT NOT NULL,
            language TEXT NOT NULL,
            file_path TEXT NOT NULL,
            start_line INTEGER NOT NULL,
            end_line INTEGER NOT NULL,
            signature TEXT NOT NULL,
            visibility TEXT NOT NULL,
            parent_symbol_id TEXT,
            module_path TEXT NOT NULL,
            doc_summary TEXT,
            role TEXT NOT NULL,
            importance_score REAL,
            signature_hash TEXT NOT NULL,
            source_providers_json TEXT NOT NULL
        );

        CREATE TABLE edges (
            id TEXT PRIMARY KEY,
            source TEXT NOT NULL,
            kind TEXT NOT NULL,
            from_symbol_id TEXT,
            from_path TEXT NOT NULL,
            line INTEGER NOT NULL,
            to_symbol_id TEXT,
            to_symbol_name TEXT,
            to_path TEXT,
            confidence REAL NOT NULL,
            reason TEXT NOT NULL
        );

        CREATE TABLE symbol_sources (
            symbol_id TEXT NOT NULL,
            source TEXT NOT NULL
        );

        CREATE TABLE symbol_terms (
            term TEXT NOT NULL,
            symbol_id TEXT NOT NULL,
            weight INTEGER NOT NULL,
            source TEXT NOT NULL
        );

        CREATE INDEX idx_symbols_name ON symbols(name);
        CREATE INDEX idx_symbols_qualified_name ON symbols(qualified_name);
        CREATE INDEX idx_symbols_kind ON symbols(kind);
        CREATE INDEX idx_symbols_file_path ON symbols(file_path);
        CREATE INDEX idx_edges_from_symbol ON edges(from_symbol_id);
        CREATE INDEX idx_edges_to_symbol ON edges(to_symbol_id);
        CREATE INDEX idx_edges_kind ON edges(kind);
        CREATE INDEX idx_edges_source ON edges(source);
        CREATE INDEX idx_symbol_sources_symbol ON symbol_sources(symbol_id);
        CREATE INDEX idx_symbol_sources_source ON symbol_sources(source);
        CREATE INDEX idx_symbol_terms_term ON symbol_terms(term);
        CREATE INDEX idx_symbol_terms_symbol ON symbol_terms(symbol_id);
        "#,
    )?;
    create_chunk_schema(conn)?;
    create_retrieval_runs_schema(conn)
}

pub(crate) fn create_retrieval_runs_schema(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS retrieval_runs (
            id TEXT PRIMARY KEY,
            query TEXT NOT NULL,
            selected_chunks_json TEXT NOT NULL,
            scores_json TEXT NOT NULL,
            created_at INTEGER NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_retrieval_runs_created_at
            ON retrieval_runs(created_at);
        "#,
    )
}

fn insert_metadata(tx: &Transaction<'_>, index: &SymbolIndex) -> rusqlite::Result<()> {
    let summary = index.lookup_summary();
    tx.execute(
        "INSERT INTO metadata(key, value) VALUES (?1, ?2)",
        params!["schema_version", "3"],
    )?;
    tx.execute(
        "INSERT INTO metadata(key, value) VALUES (?1, ?2)",
        params!["symbol_count", summary.symbol_count.to_string()],
    )?;
    tx.execute(
        "INSERT INTO metadata(key, value) VALUES (?1, ?2)",
        params!["edge_count", summary.edge_count.to_string()],
    )?;
    tx.execute(
        "INSERT INTO metadata(key, value) VALUES (?1, ?2)",
        params![
            "chunk_count",
            (summary.symbol_count + summary.file_count + count_test_symbols(index)).to_string()
        ],
    )?;
    tx.execute(
        "INSERT INTO metadata(key, value) VALUES (?1, ?2)",
        params![
            "lookup_summary_json",
            serde_json::to_string(&summary).unwrap_or_else(|_| "{}".to_string())
        ],
    )?;
    Ok(())
}

fn insert_symbols(tx: &Transaction<'_>, index: &SymbolIndex) -> rusqlite::Result<()> {
    let mut symbol_stmt = tx.prepare(
        r#"
        INSERT INTO symbols(
            id, name, qualified_name, kind, language, file_path, start_line, end_line,
            signature, visibility, parent_symbol_id, module_path, doc_summary, role,
            importance_score, signature_hash, source_providers_json
        ) VALUES (
            ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8,
            ?9, ?10, ?11, ?12, ?13, ?14,
            ?15, ?16, ?17
        )
        "#,
    )?;
    let mut source_stmt =
        tx.prepare("INSERT INTO symbol_sources(symbol_id, source) VALUES (?1, ?2)")?;
    let mut term_stmt = tx.prepare(
        "INSERT INTO symbol_terms(term, symbol_id, weight, source) VALUES (?1, ?2, ?3, ?4)",
    )?;

    for record in &index.records {
        symbol_stmt.execute(params![
            record.id.as_str(),
            record.name.as_str(),
            record.qualified_name.as_str(),
            record.kind.as_str(),
            record.language,
            record.file_path.as_str(),
            to_i64(record.start_line),
            to_i64(record.end_line),
            record.signature.as_str(),
            record.visibility.as_str(),
            record.parent_symbol_id.as_deref(),
            record.module_path.as_str(),
            record.doc_summary.as_deref(),
            record.role,
            record.importance_score,
            record.signature_hash.as_str(),
            serde_json::to_string(&record.source_providers).unwrap_or_else(|_| "[]".to_string()),
        ])?;

        for source in &record.source_providers {
            source_stmt.execute(params![record.id.as_str(), source.as_str()])?;
        }
        insert_symbol_terms(&mut term_stmt, record)?;
    }
    Ok(())
}

fn insert_edges(tx: &Transaction<'_>, index: &SymbolIndex) -> rusqlite::Result<()> {
    let mut stmt = tx.prepare(
        r#"
        INSERT INTO edges(
            id, source, kind, from_symbol_id, from_path, line, to_symbol_id,
            to_symbol_name, to_path, confidence, reason
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
        "#,
    )?;
    for edge in &index.edges {
        insert_edge(&mut stmt, edge)?;
    }
    Ok(())
}

fn insert_edge(stmt: &mut rusqlite::Statement<'_>, edge: &SymbolEdge) -> rusqlite::Result<()> {
    stmt.execute(params![
        edge.id.as_str(),
        edge.source,
        edge.kind.as_str(),
        edge.from_symbol_id.as_deref(),
        edge.from_path.as_str(),
        to_i64(edge.line),
        edge.to_symbol_id.as_deref(),
        edge.to_symbol_name.as_deref(),
        edge.to_path.as_deref(),
        f64::from(edge.confidence),
        edge.reason.as_str(),
    ])?;
    Ok(())
}

fn insert_symbol_terms(
    stmt: &mut rusqlite::Statement<'_>,
    record: &SymbolRecord,
) -> rusqlite::Result<()> {
    let mut terms = BTreeSet::new();
    push_terms(&mut terms, &record.name, 120, "name");
    push_terms(&mut terms, &record.qualified_name, 80, "qualified_name");
    push_terms(&mut terms, &record.kind, 40, "kind");
    push_terms(&mut terms, &record.file_path, 30, "file_path");
    push_terms(&mut terms, &record.signature, 20, "signature");
    if let Some(docs) = record.doc_summary.as_deref() {
        push_terms(&mut terms, docs, 10, "doc_summary");
    }

    for (term, weight, source) in terms {
        stmt.execute(params![term, record.id.as_str(), weight, source])?;
    }
    Ok(())
}

fn count_test_symbols(index: &SymbolIndex) -> usize {
    index
        .records
        .iter()
        .filter(|record| {
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
        })
        .count()
}

fn push_terms(
    terms: &mut BTreeSet<(String, i64, &'static str)>,
    value: &str,
    weight: i64,
    source: &'static str,
) {
    for term in tokenize(value) {
        terms.insert((term, weight, source));
    }
}

fn tokenize(value: &str) -> impl Iterator<Item = String> + '_ {
    value
        .split(|ch: char| !(ch == '_' || ch.is_ascii_alphanumeric()))
        .filter(|term| !term.is_empty())
        .map(|term| term.to_ascii_lowercase())
}

fn to_i64(value: usize) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}
