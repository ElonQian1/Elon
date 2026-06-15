use std::{
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{bail, Context, Result};
use rusqlite::{params, Connection};
use serde::Serialize;
use uuid::Uuid;

use super::{
    symbol_index_eval::evaluate_latest_symbol_retrieval,
    symbol_index_eval_types::{
        SymbolRetrievalEvalBatchCaseResponse, SymbolRetrievalEvalBatchMetrics,
        SymbolRetrievalEvalBatchQuery, SymbolRetrievalEvalBatchResponse,
        SymbolRetrievalEvalResponse,
    },
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

fn aggregate_metrics(
    cases: &[SymbolRetrievalEvalBatchCaseResponse],
) -> SymbolRetrievalEvalBatchMetrics {
    let successful = cases
        .iter()
        .filter_map(|case| case.result.as_ref())
        .collect::<Vec<_>>();
    let evaluated_count = successful.len();
    let requirement_count = successful
        .iter()
        .map(|result| result.metrics.requirement_count)
        .sum::<usize>();
    let hit_count_at_k = successful
        .iter()
        .map(|result| result.metrics.hit_count_at_k)
        .sum::<usize>();
    let missing_requirement_count = successful
        .iter()
        .map(|result| result.missing_requirements.len())
        .sum::<usize>();
    let total_token_count_at_k = successful
        .iter()
        .map(|result| result.metrics.total_token_count_at_k)
        .sum::<usize>();
    let candidate_count = successful
        .iter()
        .map(|result| result.candidates.len())
        .sum::<usize>();

    SymbolRetrievalEvalBatchMetrics {
        requirement_count,
        hit_count_at_k,
        missing_requirement_count,
        mean_recall_at_k: average(
            successful
                .iter()
                .map(|result| result.metrics.recall_at_k)
                .sum::<f64>(),
            evaluated_count,
        ),
        mean_reciprocal_rank: average(
            successful
                .iter()
                .map(|result| result.metrics.mean_reciprocal_rank)
                .sum::<f64>(),
            evaluated_count,
        ),
        has_test_context_rate: average(
            successful
                .iter()
                .filter(|result| result.metrics.has_test_context_at_k)
                .count() as f64,
            evaluated_count,
        ),
        total_token_count_at_k,
        average_token_count_at_k: average(total_token_count_at_k as f64, evaluated_count),
        candidate_count,
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

    let selected_chunks_json = serde_json::to_string(&response.cases)?;
    let scores_json = serde_json::to_string(&RetrievalRunScores {
        run_id: &response.run_id,
        trace_id: &response.trace_id,
        case_count: response.case_count,
        evaluated_count: response.evaluated_count,
        failed_count: response.failed_count,
        aggregate: &response.aggregate,
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

    Ok(db_path.to_string_lossy().replace('\\', "/"))
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
