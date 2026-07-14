use super::*;

pub(super) fn derive_index_status(
    chunk_count: usize,
    missing_count: usize,
    stale_count: usize,
) -> String {
    if chunk_count == 0 {
        INDEX_STATUS_UNINDEXED
    } else if stale_count > 0 {
        INDEX_STATUS_NEEDS_REBUILD
    } else if missing_count > 0 {
        INDEX_STATUS_EMBEDDING_MISSING
    } else {
        INDEX_STATUS_INDEXED
    }
    .to_string()
}

pub(super) fn status_reason(
    status: &str,
    chunk_count: usize,
    missing_count: usize,
    stale_count: usize,
) -> String {
    match status {
        INDEX_STATUS_UNINDEXED => "没有发现可检索 chunk，请先运行 context compiler".to_string(),
        INDEX_STATUS_NEEDS_REBUILD => {
            format!("{stale_count} 个 embedding 与当前 chunk hash 不一致")
        }
        INDEX_STATUS_EMBEDDING_MISSING => {
            format!("{missing_count}/{chunk_count} 个 chunk 缺少当前模型 embedding")
        }
        _ => "符号 chunk 与 embedding 均可用".to_string(),
    }
}

pub(super) fn queue_counts(conn: &Connection) -> Result<BTreeMap<String, usize>> {
    let mut stmt = conn.prepare(
        r#"
        SELECT status, COUNT(*)
        FROM embedding_index_jobs
        GROUP BY status
        "#,
    )?;
    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
    })?;
    let mut counts = BTreeMap::new();
    for row in rows {
        let (status, count) = row?;
        counts.insert(status, to_usize(count));
    }
    Ok(counts)
}

pub(super) fn load_latest_jobs(
    conn: &Connection,
    limit: usize,
) -> Result<Vec<EmbeddingJobSummary>> {
    let mut stmt = conn.prepare(
        r#"
        SELECT id, trace_id, model, status, limit_count, force, scanned_count,
            upserted_count, skipped_count, input_token_count,
            estimated_cost_micro_usd, failure_reason, created_at, started_at, finished_at
        FROM embedding_index_jobs
        ORDER BY created_at DESC, id DESC
        LIMIT ?1
        "#,
    )?;
    let rows = stmt.query_map([to_i64(limit)], row_to_job)?;
    collect_rows(rows)
}

pub(super) fn row_to_job(row: &rusqlite::Row<'_>) -> rusqlite::Result<EmbeddingJobSummary> {
    Ok(EmbeddingJobSummary {
        id: row.get(0)?,
        trace_id: row.get(1)?,
        model: row.get(2)?,
        status: row.get(3)?,
        limit: to_usize(row.get::<_, i64>(4)?),
        force: row.get::<_, i64>(5)? != 0,
        scanned_count: to_usize(row.get::<_, i64>(6)?),
        upserted_count: to_usize(row.get::<_, i64>(7)?),
        skipped_count: to_usize(row.get::<_, i64>(8)?),
        input_token_count: to_usize(row.get::<_, i64>(9)?),
        estimated_cost_micro_usd: to_usize(row.get::<_, i64>(10)?),
        failure_reason: row.get(11)?,
        created_at: row.get(12)?,
        started_at: row.get(13)?,
        finished_at: row.get(14)?,
    })
}

pub(super) fn latest_successful_embedding_time(conn: &Connection) -> Result<Option<i64>> {
    conn.query_row(
        "SELECT MAX(finished_at) FROM embedding_index_jobs WHERE status = 'succeeded'",
        [],
        |row| row.get(0),
    )
    .map_err(Into::into)
}

pub(super) fn latest_remote_failure_reason(conn: &Connection) -> Result<Option<String>> {
    conn.query_row(
        r#"
        SELECT reason
        FROM remote_embedding_failures
        ORDER BY created_at DESC, id DESC
        LIMIT 1
        "#,
        [],
        |row| row.get(0),
    )
    .optional()
    .map_err(Into::into)
}

pub(super) fn latest_failures_by_model(
    conn: &Connection,
) -> Result<BTreeMap<String, (usize, Option<String>)>> {
    let mut stmt = conn.prepare(
        r#"
        SELECT f.model, COUNT(*),
            (
                SELECT reason FROM remote_embedding_failures latest
                WHERE latest.model = f.model
                ORDER BY latest.created_at DESC, latest.id DESC
                LIMIT 1
            )
        FROM remote_embedding_failures f
        GROUP BY f.model
        "#,
    )?;
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            (
                to_usize(row.get::<_, i64>(1)?),
                row.get::<_, Option<String>>(2)?,
            ),
        ))
    })?;
    let mut failures = BTreeMap::new();
    for row in rows {
        let (model, data) = row?;
        failures.insert(model, data);
    }
    Ok(failures)
}

pub(super) fn provider_kind(model: &str) -> &'static str {
    if is_remote_embedding_model(model) {
        "remote"
    } else {
        "local"
    }
}

pub(super) fn truncate_reason(reason: &str) -> String {
    reason.chars().take(500).collect()
}

pub(super) fn collect_rows<T>(
    rows: rusqlite::MappedRows<'_, impl FnMut(&rusqlite::Row<'_>) -> rusqlite::Result<T>>,
) -> Result<Vec<T>> {
    let mut items = Vec::new();
    for row in rows {
        items.push(row?);
    }
    Ok(items)
}

pub(super) fn unix_timestamp() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| i64::try_from(duration.as_secs()).unwrap_or(i64::MAX))
        .unwrap_or_default()
}

pub(super) fn to_i64(value: usize) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

pub(super) fn to_usize(value: i64) -> usize {
    usize::try_from(value).unwrap_or_default()
}
