use std::{
    collections::BTreeMap,
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::Result;
use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;

use super::symbol_index_embedding_provider::is_remote_embedding_model;

pub(crate) const INDEX_STATUS_UNINDEXED: &str = "unindexed";
pub(crate) const INDEX_STATUS_INDEXED: &str = "indexed";
pub(crate) const INDEX_STATUS_EMBEDDING_MISSING: &str = "embedding_missing";
pub(crate) const INDEX_STATUS_NEEDS_REBUILD: &str = "needs_rebuild";

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProjectIndexProductStatus {
    pub(crate) status: String,
    pub(crate) reason: String,
    pub(crate) trace_id: Option<String>,
    pub(crate) model: Option<String>,
    pub(crate) chunk_count: usize,
    pub(crate) embedded_count: usize,
    pub(crate) missing_count: usize,
    pub(crate) stale_count: usize,
    pub(crate) queued_jobs: usize,
    pub(crate) running_jobs: usize,
    pub(crate) failed_jobs: usize,
    pub(crate) last_failure_reason: Option<String>,
    pub(crate) last_indexed_at: Option<i64>,
    pub(crate) updated_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EmbeddingQueueSummary {
    pub(crate) queued: usize,
    pub(crate) running: usize,
    pub(crate) succeeded: usize,
    pub(crate) failed: usize,
    pub(crate) latest_jobs: Vec<EmbeddingJobSummary>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EmbeddingJobSummary {
    pub(crate) id: String,
    pub(crate) trace_id: Option<String>,
    pub(crate) model: String,
    pub(crate) status: String,
    pub(crate) limit: usize,
    pub(crate) force: bool,
    pub(crate) scanned_count: usize,
    pub(crate) upserted_count: usize,
    pub(crate) skipped_count: usize,
    pub(crate) input_token_count: usize,
    pub(crate) estimated_cost_micro_usd: usize,
    pub(crate) failure_reason: Option<String>,
    pub(crate) created_at: i64,
    pub(crate) started_at: Option<i64>,
    pub(crate) finished_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EmbeddingModelCostSummary {
    pub(crate) model: String,
    pub(crate) provider: String,
    pub(crate) request_count: usize,
    pub(crate) chunk_count: usize,
    pub(crate) input_token_count: usize,
    pub(crate) estimated_cost_micro_usd: usize,
    pub(crate) failure_count: usize,
    pub(crate) last_failure_reason: Option<String>,
    pub(crate) first_used_at: Option<i64>,
    pub(crate) last_used_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RetrievalEvalSetSummary {
    pub(crate) case_count: usize,
    pub(crate) latest_cases: Vec<RetrievalEvalCaseSummary>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RetrievalEvalCaseSummary {
    pub(crate) id: String,
    pub(crate) trace_id: Option<String>,
    pub(crate) query: String,
    pub(crate) must_include_count: usize,
    pub(crate) source: String,
    pub(crate) last_run_id: Option<String>,
    pub(crate) last_recall_at_k: Option<f64>,
    pub(crate) last_missing_count: Option<usize>,
    pub(crate) updated_at: i64,
}

#[derive(Debug, Clone)]
pub(crate) struct EmbeddingJobStart {
    pub(crate) id: String,
    pub(crate) trace_id: Option<String>,
    pub(crate) model: String,
    pub(crate) limit: usize,
    pub(crate) force: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct EmbeddingJobFinish {
    pub(crate) scanned_count: usize,
    pub(crate) upserted_count: usize,
    pub(crate) skipped_count: usize,
    pub(crate) input_token_count: usize,
    pub(crate) estimated_cost_micro_usd: usize,
    pub(crate) failure_reason: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct EmbeddingUsageRecord<'a> {
    pub(crate) job_id: &'a str,
    pub(crate) chunk_id: &'a str,
    pub(crate) model: &'a str,
    pub(crate) token_count: usize,
    pub(crate) estimated_cost_micro_usd: usize,
}

pub(crate) fn create_product_schema(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS embedding_index_jobs (
            id TEXT PRIMARY KEY,
            trace_id TEXT,
            model TEXT NOT NULL,
            status TEXT NOT NULL,
            force INTEGER NOT NULL,
            limit_count INTEGER NOT NULL,
            scanned_count INTEGER NOT NULL DEFAULT 0,
            upserted_count INTEGER NOT NULL DEFAULT 0,
            skipped_count INTEGER NOT NULL DEFAULT 0,
            input_token_count INTEGER NOT NULL DEFAULT 0,
            estimated_cost_micro_usd INTEGER NOT NULL DEFAULT 0,
            failure_reason TEXT,
            created_at INTEGER NOT NULL,
            started_at INTEGER,
            finished_at INTEGER
        );

        CREATE INDEX IF NOT EXISTS idx_embedding_index_jobs_status
            ON embedding_index_jobs(status, created_at);
        CREATE INDEX IF NOT EXISTS idx_embedding_index_jobs_trace
            ON embedding_index_jobs(trace_id, created_at);

        CREATE TABLE IF NOT EXISTS project_index_status (
            project_key TEXT PRIMARY KEY,
            trace_id TEXT,
            status TEXT NOT NULL,
            reason TEXT NOT NULL,
            model TEXT,
            chunk_count INTEGER NOT NULL,
            embedded_count INTEGER NOT NULL,
            missing_count INTEGER NOT NULL,
            stale_count INTEGER NOT NULL,
            queued_jobs INTEGER NOT NULL,
            running_jobs INTEGER NOT NULL,
            failed_jobs INTEGER NOT NULL,
            last_failure_reason TEXT,
            last_indexed_at INTEGER,
            updated_at INTEGER NOT NULL
        );

        CREATE TABLE IF NOT EXISTS embedding_usage_events (
            id TEXT PRIMARY KEY,
            job_id TEXT NOT NULL,
            chunk_id TEXT,
            model TEXT NOT NULL,
            provider TEXT NOT NULL,
            input_token_count INTEGER NOT NULL,
            estimated_cost_micro_usd INTEGER NOT NULL,
            created_at INTEGER NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_embedding_usage_model
            ON embedding_usage_events(model, created_at);

        CREATE TABLE IF NOT EXISTS remote_embedding_failures (
            id TEXT PRIMARY KEY,
            job_id TEXT,
            trace_id TEXT,
            model TEXT NOT NULL,
            chunk_id TEXT,
            reason TEXT NOT NULL,
            created_at INTEGER NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_remote_embedding_failures_model
            ON remote_embedding_failures(model, created_at);

        CREATE TABLE IF NOT EXISTS retrieval_eval_cases (
            id TEXT PRIMARY KEY,
            trace_id TEXT,
            query TEXT NOT NULL,
            must_include_json TEXT NOT NULL,
            source TEXT NOT NULL,
            last_run_id TEXT,
            last_recall_at_k REAL,
            last_missing_count INTEGER,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_retrieval_eval_cases_trace
            ON retrieval_eval_cases(trace_id, updated_at);
        "#,
    )
}

pub(crate) fn start_embedding_job(conn: &Connection, input: &EmbeddingJobStart) -> Result<()> {
    create_product_schema(conn)?;
    let now = unix_timestamp();
    conn.execute(
        r#"
        INSERT OR REPLACE INTO embedding_index_jobs(
            id, trace_id, model, status, force, limit_count, created_at, started_at
        ) VALUES (?1, ?2, ?3, 'running', ?4, ?5, ?6, ?6)
        "#,
        params![
            input.id.as_str(),
            input.trace_id.as_deref(),
            input.model.as_str(),
            if input.force { 1_i64 } else { 0_i64 },
            to_i64(input.limit),
            now,
        ],
    )?;
    Ok(())
}

pub(crate) fn finish_embedding_job(
    conn: &Connection,
    job_id: &str,
    finish: &EmbeddingJobFinish,
) -> Result<()> {
    create_product_schema(conn)?;
    let status = if finish.failure_reason.is_some() {
        "failed"
    } else {
        "succeeded"
    };
    conn.execute(
        r#"
        UPDATE embedding_index_jobs
        SET status = ?2,
            scanned_count = ?3,
            upserted_count = ?4,
            skipped_count = ?5,
            input_token_count = ?6,
            estimated_cost_micro_usd = ?7,
            failure_reason = ?8,
            finished_at = ?9
        WHERE id = ?1
        "#,
        params![
            job_id,
            status,
            to_i64(finish.scanned_count),
            to_i64(finish.upserted_count),
            to_i64(finish.skipped_count),
            to_i64(finish.input_token_count),
            to_i64(finish.estimated_cost_micro_usd),
            finish.failure_reason.as_deref(),
            unix_timestamp(),
        ],
    )?;
    Ok(())
}

pub(crate) fn record_embedding_usage(
    conn: &Connection,
    usage: &EmbeddingUsageRecord<'_>,
) -> Result<()> {
    create_product_schema(conn)?;
    conn.execute(
        r#"
        INSERT INTO embedding_usage_events(
            id, job_id, chunk_id, model, provider, input_token_count,
            estimated_cost_micro_usd, created_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
        "#,
        params![
            format!("{}:{}", usage.job_id, usage.chunk_id),
            usage.job_id,
            usage.chunk_id,
            usage.model,
            provider_kind(usage.model),
            to_i64(usage.token_count),
            to_i64(usage.estimated_cost_micro_usd),
            unix_timestamp(),
        ],
    )?;
    Ok(())
}

pub(crate) fn record_remote_embedding_failure(
    conn: &Connection,
    job_id: Option<&str>,
    trace_id: Option<&str>,
    model: &str,
    chunk_id: Option<&str>,
    reason: &str,
) -> Result<()> {
    create_product_schema(conn)?;
    conn.execute(
        r#"
        INSERT INTO remote_embedding_failures(
            id, job_id, trace_id, model, chunk_id, reason, created_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
        "#,
        params![
            format!(
                "{}:{}:{}",
                unix_timestamp(),
                job_id.unwrap_or("no-job"),
                chunk_id.unwrap_or("provider")
            ),
            job_id,
            trace_id,
            model,
            chunk_id,
            truncate_reason(reason),
            unix_timestamp(),
        ],
    )?;
    Ok(())
}

pub(crate) fn upsert_project_index_status(
    conn: &Connection,
    trace_id: Option<&str>,
    model: Option<&str>,
    chunk_count: usize,
    embedded_count: usize,
    missing_count: usize,
    stale_count: usize,
) -> Result<ProjectIndexProductStatus> {
    create_product_schema(conn)?;
    let queue = load_embedding_queue_summary(conn, 0)?;
    let last_failure_reason = latest_remote_failure_reason(conn)?;
    let status = derive_index_status(chunk_count, missing_count, stale_count);
    let reason = status_reason(&status, chunk_count, missing_count, stale_count);
    let last_indexed_at = latest_successful_embedding_time(conn)?;
    let updated_at = unix_timestamp();
    let product_status = ProjectIndexProductStatus {
        status: status.clone(),
        reason: reason.clone(),
        trace_id: trace_id.map(ToOwned::to_owned),
        model: model.map(ToOwned::to_owned),
        chunk_count,
        embedded_count,
        missing_count,
        stale_count,
        queued_jobs: queue.queued,
        running_jobs: queue.running,
        failed_jobs: queue.failed,
        last_failure_reason,
        last_indexed_at,
        updated_at: Some(updated_at),
    };
    conn.execute(
        r#"
        INSERT OR REPLACE INTO project_index_status(
            project_key, trace_id, status, reason, model, chunk_count,
            embedded_count, missing_count, stale_count, queued_jobs, running_jobs,
            failed_jobs, last_failure_reason, last_indexed_at, updated_at
        ) VALUES (
            'latest', ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14
        )
        "#,
        params![
            trace_id,
            product_status.status.as_str(),
            product_status.reason.as_str(),
            model,
            to_i64(chunk_count),
            to_i64(embedded_count),
            to_i64(missing_count),
            to_i64(stale_count),
            to_i64(product_status.queued_jobs),
            to_i64(product_status.running_jobs),
            to_i64(product_status.failed_jobs),
            product_status.last_failure_reason.as_deref(),
            product_status.last_indexed_at,
            updated_at,
        ],
    )?;
    Ok(product_status)
}

pub(crate) fn load_embedding_queue_summary(
    conn: &Connection,
    latest_limit: usize,
) -> Result<EmbeddingQueueSummary> {
    create_product_schema(conn)?;
    let counts = queue_counts(conn)?;
    let latest_jobs = if latest_limit == 0 {
        Vec::new()
    } else {
        load_latest_jobs(conn, latest_limit)?
    };
    Ok(EmbeddingQueueSummary {
        queued: *counts.get("queued").unwrap_or(&0),
        running: *counts.get("running").unwrap_or(&0),
        succeeded: *counts.get("succeeded").unwrap_or(&0),
        failed: *counts.get("failed").unwrap_or(&0),
        latest_jobs,
    })
}

pub(crate) fn load_embedding_model_costs(
    conn: &Connection,
) -> Result<Vec<EmbeddingModelCostSummary>> {
    create_product_schema(conn)?;
    let failures = latest_failures_by_model(conn)?;
    let mut stmt = conn.prepare(
        r#"
        SELECT model, provider, COUNT(*), COUNT(DISTINCT chunk_id),
            COALESCE(SUM(input_token_count), 0),
            COALESCE(SUM(estimated_cost_micro_usd), 0),
            MIN(created_at), MAX(created_at)
        FROM embedding_usage_events
        GROUP BY model, provider
        ORDER BY model
        "#,
    )?;
    let rows = stmt.query_map([], |row| {
        let model: String = row.get(0)?;
        let failure = failures.get(&model);
        Ok(EmbeddingModelCostSummary {
            model,
            provider: row.get(1)?,
            request_count: to_usize(row.get::<_, i64>(2)?),
            chunk_count: to_usize(row.get::<_, i64>(3)?),
            input_token_count: to_usize(row.get::<_, i64>(4)?),
            estimated_cost_micro_usd: to_usize(row.get::<_, i64>(5)?),
            failure_count: failure.map(|item| item.0).unwrap_or_default(),
            last_failure_reason: failure.and_then(|item| item.1.clone()),
            first_used_at: row.get(6)?,
            last_used_at: row.get(7)?,
        })
    })?;
    collect_rows(rows)
}

pub(crate) fn estimated_embedding_cost_micro_usd(model: &str, input_tokens: usize) -> usize {
    let cost_per_1k = if model.contains("text-embedding-3-large") {
        130
    } else if model.contains("text-embedding-3-small") {
        20
    } else if is_remote_embedding_model(model) {
        100
    } else {
        0
    };
    ((input_tokens as u128 * cost_per_1k as u128).div_ceil(1000)).min(usize::MAX as u128) as usize
}

pub(crate) fn record_real_task_eval_case(
    conn: &Connection,
    trace_id: Option<&str>,
    run_id: &str,
    case_id: &str,
    query: &str,
    must_include_json: &str,
    recall_at_k: Option<f64>,
    missing_count: Option<usize>,
) -> Result<()> {
    create_product_schema(conn)?;
    let now = unix_timestamp();
    conn.execute(
        r#"
        INSERT INTO retrieval_eval_cases(
            id, trace_id, query, must_include_json, source, last_run_id,
            last_recall_at_k, last_missing_count, created_at, updated_at
        ) VALUES (?1, ?2, ?3, ?4, 'real_task', ?5, ?6, ?7, ?8, ?8)
        ON CONFLICT(id) DO UPDATE SET
            trace_id = excluded.trace_id,
            query = excluded.query,
            must_include_json = excluded.must_include_json,
            source = excluded.source,
            last_run_id = excluded.last_run_id,
            last_recall_at_k = excluded.last_recall_at_k,
            last_missing_count = excluded.last_missing_count,
            updated_at = excluded.updated_at
        "#,
        params![
            case_id,
            trace_id,
            query,
            must_include_json,
            run_id,
            recall_at_k,
            missing_count.map(to_i64),
            now,
        ],
    )?;
    Ok(())
}

pub(crate) fn load_retrieval_eval_set_summary(
    conn: &Connection,
    latest_limit: usize,
) -> Result<RetrievalEvalSetSummary> {
    create_product_schema(conn)?;
    let case_count = conn.query_row("SELECT COUNT(*) FROM retrieval_eval_cases", [], |row| {
        Ok(to_usize(row.get::<_, i64>(0)?))
    })?;
    let mut stmt = conn.prepare(
        r#"
        SELECT id, trace_id, query, must_include_json, source, last_run_id,
            last_recall_at_k, last_missing_count, updated_at
        FROM retrieval_eval_cases
        ORDER BY updated_at DESC, id DESC
        LIMIT ?1
        "#,
    )?;
    let rows = stmt.query_map([to_i64(latest_limit)], |row| {
        let must_include_json: String = row.get(3)?;
        Ok(RetrievalEvalCaseSummary {
            id: row.get(0)?,
            trace_id: row.get(1)?,
            query: row.get(2)?,
            must_include_count: serde_json::from_str::<Vec<String>>(&must_include_json)
                .map(|items| items.len())
                .unwrap_or_default(),
            source: row.get(4)?,
            last_run_id: row.get(5)?,
            last_recall_at_k: row.get(6)?,
            last_missing_count: row.get::<_, Option<i64>>(7)?.map(to_usize),
            updated_at: row.get(8)?,
        })
    })?;
    Ok(RetrievalEvalSetSummary {
        case_count,
        latest_cases: collect_rows(rows)?,
    })
}

fn derive_index_status(chunk_count: usize, missing_count: usize, stale_count: usize) -> String {
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

fn status_reason(
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

fn queue_counts(conn: &Connection) -> Result<BTreeMap<String, usize>> {
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

fn load_latest_jobs(conn: &Connection, limit: usize) -> Result<Vec<EmbeddingJobSummary>> {
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

fn row_to_job(row: &rusqlite::Row<'_>) -> rusqlite::Result<EmbeddingJobSummary> {
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

fn latest_successful_embedding_time(conn: &Connection) -> Result<Option<i64>> {
    conn.query_row(
        "SELECT MAX(finished_at) FROM embedding_index_jobs WHERE status = 'succeeded'",
        [],
        |row| row.get(0),
    )
    .map_err(Into::into)
}

fn latest_remote_failure_reason(conn: &Connection) -> Result<Option<String>> {
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

fn latest_failures_by_model(
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

fn provider_kind(model: &str) -> &'static str {
    if is_remote_embedding_model(model) {
        "remote"
    } else {
        "local"
    }
}

fn truncate_reason(reason: &str) -> String {
    reason.chars().take(500).collect()
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

fn unix_timestamp() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| i64::try_from(duration.as_secs()).unwrap_or(i64::MAX))
        .unwrap_or_default()
}

fn to_i64(value: usize) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

fn to_usize(value: i64) -> usize {
    usize::try_from(value).unwrap_or_default()
}
