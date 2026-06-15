use std::path::Path;

use anyhow::{Context, Result};
use rusqlite::{params, Connection, OpenFlags};

use super::{
    symbol_index::normalize_path,
    symbol_index_embedding_types::{
        SymbolEmbeddingMissingChunk, SymbolEmbeddingModelSummary, SymbolEmbeddingStatusQuery,
        SymbolEmbeddingStatusQueryEcho, SymbolEmbeddingStatusResponse, SymbolEmbeddingTotals,
    },
    symbol_index_query::{find_symbol_index_db, load_metadata},
};

pub(crate) use super::symbol_index_embedding_types::SymbolEmbeddingStatusQuery as SymbolEmbeddingStatus;

pub(crate) fn create_embedding_schema(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS embeddings (
            chunk_id TEXT PRIMARY KEY,
            model TEXT NOT NULL,
            dim INTEGER NOT NULL,
            vector BLOB NOT NULL,
            content_hash TEXT NOT NULL,
            created_at INTEGER NOT NULL,
            FOREIGN KEY(chunk_id) REFERENCES chunks(id) ON DELETE CASCADE
        );

        CREATE INDEX IF NOT EXISTS idx_embeddings_model
            ON embeddings(model);
        CREATE INDEX IF NOT EXISTS idx_embeddings_created_at
            ON embeddings(created_at);
        "#,
    )
}

pub(crate) fn load_latest_symbol_embedding_status(
    data_dir: &Path,
    query: &SymbolEmbeddingStatusQuery,
) -> Result<SymbolEmbeddingStatusResponse> {
    let db_path = find_symbol_index_db(data_dir, query.trace_id.as_deref())
        .context("没有找到可查询的 symbol_index.sqlite，请先运行一次 context compiler")?;
    load_symbol_embedding_status_db(&db_path, query)
}

pub(crate) fn load_symbol_embedding_status_db(
    db_path: &Path,
    query: &SymbolEmbeddingStatusQuery,
) -> Result<SymbolEmbeddingStatusResponse> {
    let conn = Connection::open_with_flags(db_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .with_context(|| format!("打开符号索引数据库失败: {}", db_path.display()))?;
    let metadata = load_metadata(&conn)?;
    let has_embeddings_table = table_exists(&conn, "embeddings")?;

    Ok(SymbolEmbeddingStatusResponse {
        db_path: db_path.to_string_lossy().replace('\\', "/"),
        query: SymbolEmbeddingStatusQueryEcho {
            trace_id: query.trace_id.clone(),
            model: query.model.clone(),
            limit: query.limit(),
        },
        metadata,
        totals: load_embedding_totals(&conn, query.model.as_deref(), has_embeddings_table)?,
        models: load_model_summaries(&conn, has_embeddings_table)?,
        missing_chunks: load_missing_chunks(&conn, query, has_embeddings_table)?,
    })
}

fn load_embedding_totals(
    conn: &Connection,
    model: Option<&str>,
    has_embeddings_table: bool,
) -> Result<SymbolEmbeddingTotals> {
    if !has_embeddings_table {
        let chunk_count = count_chunks(conn)?;
        return Ok(SymbolEmbeddingTotals {
            embeddings_table_available: false,
            chunk_count,
            embedded_count: 0,
            missing_count: chunk_count,
            stale_count: 0,
            coverage: 0.0,
        });
    }

    let (chunk_count, embedded_count, missing_count, stale_count) = if let Some(model) = model {
        conn.query_row(
            r#"
            SELECT
                COUNT(*),
                COALESCE(SUM(CASE WHEN EXISTS (
                    SELECT 1 FROM embeddings e
                    WHERE e.chunk_id = c.id AND e.model = ?1 AND e.content_hash = c.hash
                ) THEN 1 ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN NOT EXISTS (
                    SELECT 1 FROM embeddings e
                    WHERE e.chunk_id = c.id AND e.model = ?1 AND e.content_hash = c.hash
                ) THEN 1 ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN EXISTS (
                    SELECT 1 FROM embeddings e
                    WHERE e.chunk_id = c.id AND e.model = ?1
                ) AND NOT EXISTS (
                    SELECT 1 FROM embeddings e
                    WHERE e.chunk_id = c.id AND e.model = ?1 AND e.content_hash = c.hash
                ) THEN 1 ELSE 0 END), 0)
            FROM chunks c
            "#,
            params![model],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i64>(3)?,
                ))
            },
        )?
    } else {
        conn.query_row(
            r#"
            SELECT
                COUNT(*),
                COALESCE(SUM(CASE WHEN EXISTS (
                    SELECT 1 FROM embeddings e
                    WHERE e.chunk_id = c.id AND e.content_hash = c.hash
                ) THEN 1 ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN NOT EXISTS (
                    SELECT 1 FROM embeddings e
                    WHERE e.chunk_id = c.id AND e.content_hash = c.hash
                ) THEN 1 ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN EXISTS (
                    SELECT 1 FROM embeddings e
                    WHERE e.chunk_id = c.id
                ) AND NOT EXISTS (
                    SELECT 1 FROM embeddings e
                    WHERE e.chunk_id = c.id AND e.content_hash = c.hash
                ) THEN 1 ELSE 0 END), 0)
            FROM chunks c
            "#,
            [],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i64>(3)?,
                ))
            },
        )?
    };

    let chunk_count = to_usize(chunk_count);
    let embedded_count = to_usize(embedded_count);
    let missing_count = to_usize(missing_count);
    let stale_count = to_usize(stale_count);
    Ok(SymbolEmbeddingTotals {
        embeddings_table_available: true,
        chunk_count,
        embedded_count,
        missing_count,
        stale_count,
        coverage: ratio(embedded_count, chunk_count),
    })
}

fn load_model_summaries(
    conn: &Connection,
    has_embeddings_table: bool,
) -> Result<Vec<SymbolEmbeddingModelSummary>> {
    if !has_embeddings_table {
        return Ok(Vec::new());
    }

    let mut stmt = conn.prepare(
        r#"
        SELECT model, COUNT(*), MIN(dim), MAX(dim), MIN(created_at), MAX(created_at)
        FROM embeddings
        GROUP BY model
        ORDER BY model
        "#,
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(SymbolEmbeddingModelSummary {
            model: row.get(0)?,
            embedding_count: to_usize(row.get::<_, i64>(1)?),
            min_dim: to_usize(row.get::<_, i64>(2)?),
            max_dim: to_usize(row.get::<_, i64>(3)?),
            first_created_at: row.get(4)?,
            last_created_at: row.get(5)?,
        })
    })?;
    collect_rows(rows)
}

fn load_missing_chunks(
    conn: &Connection,
    query: &SymbolEmbeddingStatusQuery,
    has_embeddings_table: bool,
) -> Result<Vec<SymbolEmbeddingMissingChunk>> {
    if !has_embeddings_table {
        return load_missing_chunks_without_embedding_table(conn, query.limit());
    }

    if let Some(model) = query.model.as_deref() {
        let mut stmt = conn.prepare(
            r#"
            SELECT
                c.id, c.chunk_type, c.file_path, c.symbol_id, c.qualified_name,
                c.kind, c.start_line, c.end_line, c.hash, c.token_count,
                EXISTS (
                    SELECT 1 FROM embeddings e
                    WHERE e.chunk_id = c.id AND e.model = ?1
                ) AS has_embedding
            FROM chunks c
            WHERE NOT EXISTS (
                SELECT 1 FROM embeddings e
                WHERE e.chunk_id = c.id AND e.model = ?1 AND e.content_hash = c.hash
            )
            ORDER BY CASE c.chunk_type WHEN 'symbol' THEN 0 WHEN 'module' THEN 1 ELSE 2 END,
                c.file_path, c.start_line
            LIMIT ?2
            "#,
        )?;
        let rows = stmt.query_map(params![model, to_i64(query.limit())], row_to_missing_chunk)?;
        return collect_rows(rows);
    }

    let mut stmt = conn.prepare(
        r#"
        SELECT
            c.id, c.chunk_type, c.file_path, c.symbol_id, c.qualified_name,
            c.kind, c.start_line, c.end_line, c.hash, c.token_count,
            EXISTS (
                SELECT 1 FROM embeddings e
                WHERE e.chunk_id = c.id
            ) AS has_embedding
        FROM chunks c
        WHERE NOT EXISTS (
            SELECT 1 FROM embeddings e
            WHERE e.chunk_id = c.id AND e.content_hash = c.hash
        )
        ORDER BY CASE c.chunk_type WHEN 'symbol' THEN 0 WHEN 'module' THEN 1 ELSE 2 END,
            c.file_path, c.start_line
        LIMIT ?1
        "#,
    )?;
    let rows = stmt.query_map(params![to_i64(query.limit())], row_to_missing_chunk)?;
    collect_rows(rows)
}

fn load_missing_chunks_without_embedding_table(
    conn: &Connection,
    limit: usize,
) -> Result<Vec<SymbolEmbeddingMissingChunk>> {
    let mut stmt = conn.prepare(
        r#"
        SELECT
            id, chunk_type, file_path, symbol_id, qualified_name, kind,
            start_line, end_line, hash, token_count, 0 AS has_embedding
        FROM chunks
        ORDER BY CASE chunk_type WHEN 'symbol' THEN 0 WHEN 'module' THEN 1 ELSE 2 END,
            file_path, start_line
        LIMIT ?1
        "#,
    )?;
    let rows = stmt.query_map(params![to_i64(limit)], row_to_missing_chunk)?;
    collect_rows(rows)
}

fn count_chunks(conn: &Connection) -> Result<usize> {
    conn.query_row("SELECT COUNT(*) FROM chunks", [], |row| {
        Ok(to_usize(row.get::<_, i64>(0)?))
    })
    .map_err(Into::into)
}

fn table_exists(conn: &Connection, name: &str) -> Result<bool> {
    conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1)",
        params![name],
        |row| row.get::<_, bool>(0),
    )
    .map_err(Into::into)
}

fn row_to_missing_chunk(row: &rusqlite::Row<'_>) -> rusqlite::Result<SymbolEmbeddingMissingChunk> {
    Ok(SymbolEmbeddingMissingChunk {
        id: row.get(0)?,
        chunk_type: row.get(1)?,
        file_path: normalize_path(&row.get::<_, String>(2)?),
        symbol_id: row.get(3)?,
        qualified_name: row.get(4)?,
        kind: row.get(5)?,
        start_line: row.get::<_, Option<i64>>(6)?.map(to_usize),
        end_line: row.get::<_, Option<i64>>(7)?.map(to_usize),
        hash: row.get(8)?,
        token_count: to_usize(row.get::<_, i64>(9)?),
        has_embedding: row.get::<_, i64>(10)? != 0,
    })
}

fn collect_rows<T>(
    rows: rusqlite::MappedRows<'_, impl FnMut(&rusqlite::Row<'_>) -> rusqlite::Result<T>>,
) -> Result<Vec<T>> {
    let mut items = Vec::new();
    for row in rows {
        items.push(row?);
    }
    Ok(items)
}

fn ratio(value: usize, total: usize) -> f64 {
    if total == 0 {
        0.0
    } else {
        value as f64 / total as f64
    }
}

fn to_i64(value: usize) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

fn to_usize(value: i64) -> usize {
    usize::try_from(value).unwrap_or_default()
}
