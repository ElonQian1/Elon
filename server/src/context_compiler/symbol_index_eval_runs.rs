use std::{
    collections::BTreeMap,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{bail, Context, Result};
use rusqlite::{params, Connection, OpenFlags, OptionalExtension};
use serde::Serialize;
use serde_json::Value;
use uuid::Uuid;

use super::{
    symbol_index_eval::evaluate_latest_symbol_retrieval,
    symbol_index_eval_types::{
        SymbolRetrievalEvalBatchCaseResponse, SymbolRetrievalEvalBatchMetrics,
        SymbolRetrievalEvalBatchQuery, SymbolRetrievalEvalBatchResponse,
        SymbolRetrievalEvalIntentGroupMetrics, SymbolRetrievalEvalResponse,
        SymbolRetrievalRunDetail, SymbolRetrievalRunDetailResponse, SymbolRetrievalRunHistoryQuery,
        SymbolRetrievalRunHistoryQueryEcho, SymbolRetrievalRunLookupQuery,
        SymbolRetrievalRunSummary, SymbolRetrievalRunsResponse,
    },
    symbol_index_product::{create_product_schema, record_real_task_eval_case},
    symbol_index_query::find_symbol_index_db,
    symbol_index_store::create_retrieval_runs_schema,
};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RetrievalRunScores<'a> {
    run_id: &'a str,
    trace_id: &'a Option<String>,
    case_count: usize,
    evaluated_count: usize,
    failed_count: usize,
    aggregate: &'a SymbolRetrievalEvalBatchMetrics,
    intent_groups: &'a [SymbolRetrievalEvalIntentGroupMetrics],
}

pub(crate) fn evaluate_latest_symbol_retrieval_batch(
    data_dir: &Path,
    query: &SymbolRetrievalEvalBatchQuery,
) -> Result<SymbolRetrievalEvalBatchResponse> {
    if query.cases.is_empty() {
        bail!("cases 不能为空");
    }

    let run_id = Uuid::new_v4().to_string();
    let mut cases = Vec::with_capacity(query.cases.len());
    let mut db_path = None;

    for case in &query.cases {
        let id = case.id.clone();
        match evaluate_latest_symbol_retrieval(data_dir, &case.query) {
            Ok(result) => {
                db_path.get_or_insert_with(|| result.db_path.clone());
                cases.push(success_case(id, result));
            }
            Err(error) => cases.push(failed_case(id, error.to_string())),
        }
    }

    let aggregate = aggregate_metrics(&cases);
    let intent_groups = intent_group_metrics(&cases);
    let evaluated_count = cases.iter().filter(|case| case.ok).count();
    let failed_count = cases.len().saturating_sub(evaluated_count);
    let mut response = SymbolRetrievalEvalBatchResponse {
        run_id: run_id.clone(),
        trace_id: query.trace_id.clone(),
        db_path,
        record_db_path: None,
        recorded: false,
        record_error: None,
        case_count: cases.len(),
        evaluated_count,
        failed_count,
        aggregate,
        intent_groups,
        cases,
    };

    if query.record_runs {
        match record_retrieval_run(data_dir, query, &response) {
            Ok(path) => {
                response.recorded = true;
                response.record_db_path = Some(path);
            }
            Err(error) => response.record_error = Some(error.to_string()),
        }
    }

    Ok(response)
}

pub(crate) fn list_latest_retrieval_runs(
    data_dir: &Path,
    query: &SymbolRetrievalRunHistoryQuery,
) -> Result<SymbolRetrievalRunsResponse> {
    let db_path = find_symbol_index_db(data_dir, query.trace_id.as_deref())
        .context("没有找到可查询 retrieval_runs 的 symbol_index.sqlite")?;
    let conn = open_read_only(&db_path)?;
    if !retrieval_runs_table_exists(&conn)? {
        return Ok(SymbolRetrievalRunsResponse {
            db_path: db_path.to_string_lossy().replace('\\', "/"),
            query: SymbolRetrievalRunHistoryQueryEcho {
                trace_id: query.trace_id.clone(),
                limit: query.limit(),
            },
            runs: Vec::new(),
        });
    }

    let mut stmt = conn.prepare(
        r#"
        SELECT id, query, scores_json, created_at
        FROM retrieval_runs
        ORDER BY created_at DESC, id DESC
        LIMIT ?1
        "#,
    )?;
    let rows = stmt.query_map([i64::try_from(query.limit()).unwrap_or(i64::MAX)], |row| {
        let scores_json: String = row.get(2)?;
        Ok(SymbolRetrievalRunSummary {
            id: row.get(0)?,
            query: row.get(1)?,
            scores: parse_json(&scores_json),
            created_at: row.get(3)?,
        })
    })?;

    let mut runs = Vec::new();
    for row in rows {
        runs.push(row?);
    }

    Ok(SymbolRetrievalRunsResponse {
        db_path: db_path.to_string_lossy().replace('\\', "/"),
        query: SymbolRetrievalRunHistoryQueryEcho {
            trace_id: query.trace_id.clone(),
            limit: query.limit(),
        },
        runs,
    })
}

pub(crate) fn load_latest_retrieval_run(
    data_dir: &Path,
    query: &SymbolRetrievalRunLookupQuery,
) -> Result<SymbolRetrievalRunDetailResponse> {
    if query.id.trim().is_empty() {
        bail!("id 不能为空");
    }

    let db_path = find_symbol_index_db(data_dir, query.trace_id.as_deref())
        .context("没有找到可查询 retrieval_runs 的 symbol_index.sqlite")?;
    let conn = open_read_only(&db_path)?;
    if !retrieval_runs_table_exists(&conn)? {
        bail!("retrieval_runs 表不存在，请先运行一次批量评测并记录结果");
    }

    let run = conn
        .query_row(
            r#"
            SELECT id, query, selected_chunks_json, scores_json, created_at
            FROM retrieval_runs
            WHERE id = ?1
            "#,
            [query.id.trim()],
            |row| {
                let selected_chunks_json: String = row.get(2)?;
                let scores_json: String = row.get(3)?;
                Ok(SymbolRetrievalRunDetail {
                    id: row.get(0)?,
                    query: row.get(1)?,
                    selected_chunks: parse_json(&selected_chunks_json),
                    scores: parse_json(&scores_json),
                    created_at: row.get(4)?,
                })
            },
        )
        .optional()?
        .ok_or_else(|| anyhow::anyhow!("retrieval run 不存在: {}", query.id))?;

    Ok(SymbolRetrievalRunDetailResponse {
        db_path: db_path.to_string_lossy().replace('\\', "/"),
        run,
    })
}

fn success_case(
    id: String,
    result: SymbolRetrievalEvalResponse,
) -> SymbolRetrievalEvalBatchCaseResponse {
    SymbolRetrievalEvalBatchCaseResponse {
        id,
        ok: true,
        error: None,
        result: Some(result),
    }
}

fn failed_case(id: String, error: String) -> SymbolRetrievalEvalBatchCaseResponse {
    SymbolRetrievalEvalBatchCaseResponse {
        id,
        ok: false,
        error: Some(error),
        result: None,
    }
}

fn open_read_only(db_path: &Path) -> Result<Connection> {
    Connection::open_with_flags(db_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .with_context(|| format!("打开符号索引数据库失败: {}", db_path.display()))
}

fn retrieval_runs_table_exists(conn: &Connection) -> rusqlite::Result<bool> {
    conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'retrieval_runs')",
        [],
        |row| row.get::<_, bool>(0),
    )
}

fn parse_json(value: &str) -> Value {
    serde_json::from_str(value).unwrap_or_else(|_| Value::String(value.to_string()))
}

fn aggregate_metrics(
    cases: &[SymbolRetrievalEvalBatchCaseResponse],
) -> SymbolRetrievalEvalBatchMetrics {
    let mut accumulator = EvalMetricsAccumulator::default();
    for result in cases.iter().filter_map(|case| case.result.as_ref()) {
        accumulator.add(result);
    }
    accumulator.finish_batch()
}

fn intent_group_metrics(
    cases: &[SymbolRetrievalEvalBatchCaseResponse],
) -> Vec<SymbolRetrievalEvalIntentGroupMetrics> {
    let mut groups = BTreeMap::<String, EvalMetricsAccumulator>::new();
    for result in cases.iter().filter_map(|case| case.result.as_ref()) {
        groups
            .entry(result.retrieval_plan.intent.as_str().to_string())
            .or_default()
            .add(result);
    }
    groups
        .into_iter()
        .map(|(intent, accumulator)| accumulator.finish_intent(intent))
        .collect()
}

#[derive(Debug, Default)]
struct EvalMetricsAccumulator {
    evaluated_count: usize,
    requirement_count: usize,
    hit_count_at_k: usize,
    missing_requirement_count: usize,
    recall_at_k_total: f64,
    mean_reciprocal_rank_total: f64,
    has_test_context_count: usize,
    noise_count_at_k: usize,
    noise_rate_at_k_total: f64,
    total_token_count_at_k: usize,
    candidate_count: usize,
}

impl EvalMetricsAccumulator {
    fn add(&mut self, result: &SymbolRetrievalEvalResponse) {
        self.evaluated_count += 1;
        self.requirement_count += result.metrics.requirement_count;
        self.hit_count_at_k += result.metrics.hit_count_at_k;
        self.missing_requirement_count += result.missing_requirements.len();
        self.recall_at_k_total += result.metrics.recall_at_k;
        self.mean_reciprocal_rank_total += result.metrics.mean_reciprocal_rank;
        self.has_test_context_count += usize::from(result.metrics.has_test_context_at_k);
        self.noise_count_at_k += result.metrics.noise_count_at_k;
        self.noise_rate_at_k_total += result.metrics.noise_rate_at_k;
        self.total_token_count_at_k += result.metrics.total_token_count_at_k;
        self.candidate_count += result.candidates.len();
    }

    fn finish_batch(self) -> SymbolRetrievalEvalBatchMetrics {
        SymbolRetrievalEvalBatchMetrics {
            requirement_count: self.requirement_count,
            hit_count_at_k: self.hit_count_at_k,
            missing_requirement_count: self.missing_requirement_count,
            mean_recall_at_k: average(self.recall_at_k_total, self.evaluated_count),
            mean_reciprocal_rank: average(self.mean_reciprocal_rank_total, self.evaluated_count),
            has_test_context_rate: average(
                self.has_test_context_count as f64,
                self.evaluated_count,
            ),
            noise_count_at_k: self.noise_count_at_k,
            mean_noise_rate_at_k: average(self.noise_rate_at_k_total, self.evaluated_count),
            total_token_count_at_k: self.total_token_count_at_k,
            average_token_count_at_k: average(
                self.total_token_count_at_k as f64,
                self.evaluated_count,
            ),
            candidate_count: self.candidate_count,
        }
    }

    fn finish_intent(self, intent: String) -> SymbolRetrievalEvalIntentGroupMetrics {
        SymbolRetrievalEvalIntentGroupMetrics {
            intent,
            evaluated_count: self.evaluated_count,
            requirement_count: self.requirement_count,
            hit_count_at_k: self.hit_count_at_k,
            missing_requirement_count: self.missing_requirement_count,
            mean_recall_at_k: average(self.recall_at_k_total, self.evaluated_count),
            mean_reciprocal_rank: average(self.mean_reciprocal_rank_total, self.evaluated_count),
            has_test_context_rate: average(
                self.has_test_context_count as f64,
                self.evaluated_count,
            ),
            noise_count_at_k: self.noise_count_at_k,
            mean_noise_rate_at_k: average(self.noise_rate_at_k_total, self.evaluated_count),
            total_token_count_at_k: self.total_token_count_at_k,
            average_token_count_at_k: average(
                self.total_token_count_at_k as f64,
                self.evaluated_count,
            ),
            candidate_count: self.candidate_count,
        }
    }
}

fn record_retrieval_run(
    data_dir: &Path,
    query: &SymbolRetrievalEvalBatchQuery,
    response: &SymbolRetrievalEvalBatchResponse,
) -> Result<String> {
    let db_path = find_symbol_index_db(data_dir, query.trace_id.as_deref())
        .context("没有找到可记录 retrieval_runs 的 symbol_index.sqlite")?;
    let conn = Connection::open(&db_path)
        .with_context(|| format!("打开符号索引数据库失败: {}", db_path.display()))?;
    create_retrieval_runs_schema(&conn)?;
    create_product_schema(&conn)?;

    let selected_chunks_json = serde_json::to_string(&response.cases)?;
    let scores_json = serde_json::to_string(&RetrievalRunScores {
        run_id: &response.run_id,
        trace_id: &response.trace_id,
        case_count: response.case_count,
        evaluated_count: response.evaluated_count,
        failed_count: response.failed_count,
        aggregate: &response.aggregate,
        intent_groups: &response.intent_groups,
    })?;
    conn.execute(
        r#"
        INSERT OR REPLACE INTO retrieval_runs(
            id, query, selected_chunks_json, scores_json, created_at
        ) VALUES (?1, ?2, ?3, ?4, ?5)
        "#,
        params![
            response.run_id.as_str(),
            batch_query_text(query).as_str(),
            selected_chunks_json.as_str(),
            scores_json.as_str(),
            unix_timestamp(),
        ],
    )?;
    record_retrieval_eval_cases(&conn, query, response)?;

    Ok(db_path.to_string_lossy().replace('\\', "/"))
}

fn record_retrieval_eval_cases(
    conn: &Connection,
    query: &SymbolRetrievalEvalBatchQuery,
    response: &SymbolRetrievalEvalBatchResponse,
) -> Result<()> {
    for (case_query, case_response) in query.cases.iter().zip(response.cases.iter()) {
        let Some(result) = case_response.result.as_ref() else {
            continue;
        };
        let must_include_json = serde_json::to_string(&case_query.query.must_include)?;
        record_real_task_eval_case(
            conn,
            case_query
                .query
                .trace_id
                .as_deref()
                .or(query.trace_id.as_deref()),
            &response.run_id,
            &case_query.id,
            case_query.query.text.as_deref().unwrap_or_default(),
            &must_include_json,
            Some(result.metrics.recall_at_k),
            Some(result.missing_requirements.len()),
        )?;
    }
    Ok(())
}

fn batch_query_text(query: &SymbolRetrievalEvalBatchQuery) -> String {
    query
        .cases
        .iter()
        .map(|case| {
            format!(
                "{}: {}",
                case.id,
                case.query.text.as_deref().unwrap_or_default()
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn average(total: f64, count: usize) -> f64 {
    if count == 0 {
        0.0
    } else {
        total / count as f64
    }
}

fn unix_timestamp() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| i64::try_from(duration.as_secs()).unwrap_or(i64::MAX))
        .unwrap_or_default()
}
