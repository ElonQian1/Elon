use std::{cmp::Ordering, collections::HashMap, path::Path};

use anyhow::{bail, Context, Result};
use rusqlite::{params, Connection, OpenFlags};

use super::{
    symbol_index::normalize_path,
    symbol_index_embedding_provider::{embedding_terms, resolve_embedding_provider, vector_norm},
    symbol_index_embeddings::create_embedding_schema,
    symbol_index_query::{find_symbol_index_db, load_metadata},
    symbol_index_vector_types::{
        SymbolVectorBackfillQuery, SymbolVectorBackfillQueryEcho, SymbolVectorBackfillResponse,
        SymbolVectorHit, SymbolVectorSearch, SymbolVectorSearchQueryEcho,
        SymbolVectorSearchResponse,
    },
};

pub(crate) use super::symbol_index_vector_types::{
    SymbolVectorBackfillQuery as SymbolVectorBackfill,
    SymbolVectorSearch as SymbolVectorSearchQuery,
};

#[derive(Debug, Clone)]
struct ChunkForEmbedding {
    id: String,
    content: String,
    summary: Option<String>,
    hash: String,
}

#[derive(Debug, Clone)]
struct StoredVectorChunk {
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
    vector: Vec<f32>,
}

pub(crate) fn backfill_latest_symbol_vectors(
    data_dir: &Path,
    query: &SymbolVectorBackfillQuery,
) -> Result<SymbolVectorBackfillResponse> {
    let db_path = find_symbol_index_db(data_dir, query.trace_id.as_deref())
        .context("没有找到可回填向量的 symbol_index.sqlite，请先运行一次 context compiler")?;
    backfill_symbol_vectors_db(&db_path, query)
}

pub(crate) fn search_latest_symbol_vectors(
    data_dir: &Path,
    search: &SymbolVectorSearch,
) -> Result<SymbolVectorSearchResponse> {
    let db_path = find_symbol_index_db(data_dir, search.trace_id.as_deref())
        .context("没有找到可查询向量的 symbol_index.sqlite，请先运行一次 context compiler")?;
    search_symbol_vectors_db(&db_path, search)
}

pub(crate) fn backfill_symbol_vectors_db(
    db_path: &Path,
    query: &SymbolVectorBackfillQuery,
) -> Result<SymbolVectorBackfillResponse> {
    let conn = Connection::open_with_flags(db_path, OpenFlags::SQLITE_OPEN_READ_WRITE)
        .with_context(|| format!("打开符号索引数据库失败: {}", db_path.display()))?;
    create_embedding_schema(&conn)?;
    let metadata = load_metadata(&conn)?;
    let model = query.model();
    let provider = resolve_embedding_provider(&model)?;
    let existing = load_existing_hashes(&conn, &model)?;
    let chunks = load_chunks_for_embedding(&conn, query.limit())?;

    let mut upserted = 0;
    let mut skipped = 0;
    for chunk in &chunks {
        let current = existing.get(chunk.id.as_str());
        if !query.force && current.is_some_and(|hash| hash == &chunk.hash) {
            skipped += 1;
            continue;
        }
        let vector = provider.embed_chunk(&chunk.content, chunk.summary.as_deref());
        conn.execute(
            r#"
            INSERT OR REPLACE INTO embeddings(chunk_id, model, dim, vector, content_hash, created_at)
            VALUES (?1, ?2, ?3, ?4, ?5, strftime('%s','now'))
            "#,
            params![
                chunk.id.as_str(),
                provider.model(),
                to_i64(provider.dim()),
                vector_to_blob(&vector),
                chunk.hash.as_str(),
            ],
        )?;
        upserted += 1;
    }

    Ok(SymbolVectorBackfillResponse {
        db_path: db_path.to_string_lossy().replace('\\', "/"),
        query: SymbolVectorBackfillQueryEcho {
            trace_id: query.trace_id.clone(),
            model: model.clone(),
            limit: query.limit(),
            force: query.force,
        },
        metadata,
        model,
        dim: provider.dim(),
        scanned_count: chunks.len(),
        upserted_count: upserted,
        skipped_count: skipped,
    })
}

pub(crate) fn search_symbol_vectors_db(
    db_path: &Path,
    search: &SymbolVectorSearch,
) -> Result<SymbolVectorSearchResponse> {
    let text = search
        .text
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow::anyhow!("q 不能为空"))?
        .to_string();
    let conn = Connection::open_with_flags(db_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .with_context(|| format!("打开符号索引数据库失败: {}", db_path.display()))?;
    let metadata = load_metadata(&conn)?;
    let model = search.model();
    let provider = resolve_embedding_provider(&model)?;
    let query_vector = provider.embed_text(&text);
    if vector_norm(&query_vector) == 0.0 {
        bail!("q 没有可用于向量检索的词项");
    }
    let terms = embedding_terms(&text);
    let chunks = load_vector_chunks(&conn, &model, provider.dim(), search)?;
    let mut hits = chunks
        .into_iter()
        .filter_map(|chunk| {
            let score = dot(&query_vector, &chunk.vector) as f64;
            (score > 0.0).then(|| vector_hit(chunk, score, &terms))
        })
        .collect::<Vec<_>>();
    hits.sort_by(compare_hits);
    hits.truncate(search.limit());

    Ok(SymbolVectorSearchResponse {
        db_path: db_path.to_string_lossy().replace('\\', "/"),
        query: SymbolVectorSearchQueryEcho {
            trace_id: search.trace_id.clone(),
            q: text,
            model: model.clone(),
            path: clean_filter(search.path.as_deref()),
            chunk_type: clean_filter(search.chunk_type.as_deref()),
            limit: search.limit(),
        },
        metadata,
        model,
        dim: provider.dim(),
        chunks: hits,
    })
}

fn load_existing_hashes(conn: &Connection, model: &str) -> Result<HashMap<String, String>> {
    let mut stmt =
        conn.prepare("SELECT chunk_id, content_hash FROM embeddings WHERE model = ?1")?;
    let rows = stmt.query_map(params![model], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;
    let mut hashes = HashMap::new();
    for row in rows {
        let (chunk_id, hash) = row?;
        hashes.insert(chunk_id, hash);
    }
    Ok(hashes)
}

fn load_chunks_for_embedding(conn: &Connection, limit: usize) -> Result<Vec<ChunkForEmbedding>> {
    let mut stmt = conn.prepare(
        r#"
        SELECT id, content, summary, hash
        FROM chunks
        ORDER BY CASE chunk_type WHEN 'symbol' THEN 0 WHEN 'module' THEN 1 ELSE 2 END,
            file_path, start_line
        LIMIT ?1
        "#,
    )?;
    let rows = stmt.query_map(params![to_i64(limit)], |row| {
        Ok(ChunkForEmbedding {
            id: row.get(0)?,
            content: row.get(1)?,
            summary: row.get(2)?,
            hash: row.get(3)?,
        })
    })?;
    collect_rows(rows)
}

fn load_vector_chunks(
    conn: &Connection,
    model: &str,
    dim: usize,
    search: &SymbolVectorSearch,
) -> Result<Vec<StoredVectorChunk>> {
    let mut sql = String::from(
        r#"
        SELECT
            c.id, c.chunk_type, c.file_path, c.symbol_id, c.qualified_name,
            c.kind, c.start_line, c.end_line, c.content, c.summary,
            c.hash, c.token_count, e.vector
        FROM chunks c
        JOIN embeddings e ON e.chunk_id = c.id
        WHERE e.model = ?1
            AND e.dim = ?2
            AND e.content_hash = c.hash
        "#,
    );
    let mut values = vec![
        rusqlite::types::Value::Text(model.to_string()),
        rusqlite::types::Value::Integer(to_i64(dim)),
    ];
    if let Some(path) = clean_filter(search.path.as_deref()) {
        sql.push_str(" AND lower(replace(c.file_path, char(92), '/')) LIKE lower(?)");
        values.push(rusqlite::types::Value::Text(format!(
            "%{}%",
            normalize_path(&path)
        )));
    }
    if let Some(chunk_type) = clean_filter(search.chunk_type.as_deref()) {
        sql.push_str(" AND lower(c.chunk_type) = lower(?)");
        values.push(rusqlite::types::Value::Text(chunk_type));
    }
    sql.push_str(" ORDER BY c.file_path, c.start_line");

    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(rusqlite::params_from_iter(values.iter()), |row| {
        let blob = row.get::<_, Vec<u8>>(12)?;
        Ok(StoredVectorChunk {
            id: row.get(0)?,
            chunk_type: row.get(1)?,
            file_path: normalize_path(&row.get::<_, String>(2)?),
            symbol_id: row.get(3)?,
            qualified_name: row.get(4)?,
            kind: row.get(5)?,
            start_line: row.get::<_, Option<i64>>(6)?.map(to_usize),
            end_line: row.get::<_, Option<i64>>(7)?.map(to_usize),
            content: row.get(8)?,
            summary: row.get(9)?,
            hash: row.get(10)?,
            token_count: to_usize(row.get::<_, i64>(11)?),
            vector: blob_to_vector(&blob, dim).unwrap_or_default(),
        })
    })?;
    let chunks = collect_rows(rows)?
        .into_iter()
        .filter(|chunk| chunk.vector.len() == dim)
        .collect();
    Ok(chunks)
}

fn dot(left: &[f32], right: &[f32]) -> f32 {
    left.iter()
        .zip(right.iter())
        .map(|(left, right)| left * right)
        .sum()
}

fn vector_to_blob(vector: &[f32]) -> Vec<u8> {
    let mut blob = Vec::with_capacity(vector.len() * 4);
    for value in vector {
        blob.extend_from_slice(&value.to_le_bytes());
    }
    blob
}

fn blob_to_vector(blob: &[u8], dim: usize) -> Option<Vec<f32>> {
    if blob.len() != dim * 4 {
        return None;
    }
    Some(
        blob.chunks_exact(4)
            .map(|bytes| f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
            .collect(),
    )
}

fn vector_hit(chunk: StoredVectorChunk, score: f64, terms: &[String]) -> SymbolVectorHit {
    SymbolVectorHit {
        id: chunk.id,
        chunk_type: chunk.chunk_type,
        file_path: chunk.file_path,
        symbol_id: chunk.symbol_id,
        qualified_name: chunk.qualified_name,
        kind: chunk.kind,
        start_line: chunk.start_line,
        end_line: chunk.end_line,
        matched_terms: matched_terms(&chunk.content, terms),
        content: chunk.content,
        summary: chunk.summary,
        hash: chunk.hash,
        token_count: chunk.token_count,
        score,
    }
}

fn compare_hits(left: &SymbolVectorHit, right: &SymbolVectorHit) -> Ordering {
    right
        .score
        .partial_cmp(&left.score)
        .unwrap_or(Ordering::Equal)
        .then_with(|| left.file_path.cmp(&right.file_path))
        .then_with(|| left.start_line.cmp(&right.start_line))
        .then_with(|| left.id.cmp(&right.id))
}

fn matched_terms(content: &str, terms: &[String]) -> Vec<String> {
    let content = content.to_ascii_lowercase();
    terms
        .iter()
        .filter(|term| content.contains(term.as_str()))
        .cloned()
        .collect()
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
