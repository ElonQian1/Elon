use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

use anyhow::{bail, Result};
use serde::Serialize;
use serde_json::Value;

use super::{
    symbol_index_eval_runs::load_latest_retrieval_run,
    symbol_index_eval_types::{SymbolRetrievalRunDetail, SymbolRetrievalRunLookupQuery},
};

const DEFAULT_CASE_LIMIT: usize = 50;
const MAX_CASE_LIMIT: usize = 200;
const EPSILON: f64 = 0.000_001;

#[derive(Debug, Clone, Default)]
pub(crate) struct SymbolRetrievalRunCompareQuery {
    pub(crate) trace_id: Option<String>,
    pub(crate) baseline_id: String,
    pub(crate) current_id: String,
    pub(crate) case_limit: usize,
}

impl SymbolRetrievalRunCompareQuery {
    pub(crate) fn case_limit(&self) -> usize {
        if self.case_limit == 0 {
            DEFAULT_CASE_LIMIT
        } else {
            self.case_limit.min(MAX_CASE_LIMIT)
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SymbolRetrievalRunCompareResponse {
    pub(crate) db_path: String,
    pub(crate) baseline: SymbolRetrievalRunCompareRun,
    pub(crate) current: SymbolRetrievalRunCompareRun,
    pub(crate) verdict: String,
    pub(crate) regression_count: usize,
    pub(crate) improvement_count: usize,
    pub(crate) unchanged_count: usize,
    pub(crate) total_compared_cases: usize,
    pub(crate) returned_case_count: usize,
    pub(crate) aggregate_deltas: Vec<SymbolRetrievalMetricDelta>,
    pub(crate) intent_deltas: Vec<SymbolRetrievalIntentDelta>,
    pub(crate) case_deltas: Vec<SymbolRetrievalCaseDelta>,
    pub(crate) recommendations: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SymbolRetrievalRunCompareRun {
    pub(crate) id: String,
    pub(crate) query: String,
    pub(crate) created_at: i64,
    pub(crate) case_count: usize,
    pub(crate) evaluated_count: usize,
    pub(crate) failed_count: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SymbolRetrievalMetricDelta {
    pub(crate) metric: String,
    pub(crate) baseline: f64,
    pub(crate) current: f64,
    pub(crate) delta: f64,
    pub(crate) direction: String,
    pub(crate) status: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SymbolRetrievalIntentDelta {
    pub(crate) intent: String,
    pub(crate) status: String,
    pub(crate) deltas: Vec<SymbolRetrievalMetricDelta>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SymbolRetrievalCaseDelta {
    pub(crate) id: String,
    pub(crate) query: Option<String>,
    pub(crate) status: String,
    pub(crate) baseline_ok: Option<bool>,
    pub(crate) current_ok: Option<bool>,
    pub(crate) baseline_error: Option<String>,
    pub(crate) current_error: Option<String>,
    pub(crate) deltas: Vec<SymbolRetrievalMetricDelta>,
    pub(crate) resolved_missing_requirements: Vec<String>,
    pub(crate) new_missing_requirements: Vec<String>,
    pub(crate) baseline_top_candidates: Vec<String>,
    pub(crate) current_top_candidates: Vec<String>,
}

#[derive(Debug, Clone)]
struct RunSnapshot {
    summary: SymbolRetrievalRunCompareRun,
    metrics: Value,
    intents: BTreeMap<String, Value>,
    cases: BTreeMap<String, CaseSnapshot>,
}

#[derive(Debug, Clone)]
struct CaseSnapshot {
    id: String,
    query: Option<String>,
    ok: bool,
    error: Option<String>,
    metrics: Value,
    missing_requirements: BTreeSet<String>,
    top_candidates: Vec<String>,
}

#[derive(Debug, Clone, Copy)]
struct MetricSpec {
    key: &'static str,
    label: &'static str,
    direction: MetricDirection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MetricDirection {
    HigherIsBetter,
    LowerIsBetter,
    Neutral,
}

pub(crate) fn compare_latest_retrieval_runs(
    data_dir: &Path,
    query: &SymbolRetrievalRunCompareQuery,
) -> Result<SymbolRetrievalRunCompareResponse> {
    if query.baseline_id.trim().is_empty() {
        bail!("baselineId 不能为空");
    }
    if query.current_id.trim().is_empty() {
        bail!("currentId 不能为空");
    }

    let baseline = load_latest_retrieval_run(
        data_dir,
        &SymbolRetrievalRunLookupQuery {
            trace_id: query.trace_id.clone(),
            id: query.baseline_id.clone(),
        },
    )?;
    let current = load_latest_retrieval_run(
        data_dir,
        &SymbolRetrievalRunLookupQuery {
            trace_id: query.trace_id.clone(),
            id: query.current_id.clone(),
        },
    )?;

    Ok(compare_retrieval_run_details(
        baseline.db_path,
        baseline.run,
        current.run,
        query.case_limit(),
    ))
}

pub(crate) fn compare_retrieval_run_details(
    db_path: String,
    baseline: SymbolRetrievalRunDetail,
    current: SymbolRetrievalRunDetail,
    case_limit: usize,
) -> SymbolRetrievalRunCompareResponse {
    let baseline = RunSnapshot::from_detail(baseline);
    let current = RunSnapshot::from_detail(current);
    let aggregate_deltas = metric_deltas(&baseline.metrics, &current.metrics, aggregate_specs());
    let intent_deltas = compare_intents(&baseline.intents, &current.intents);
    let mut case_deltas = compare_cases(&baseline.cases, &current.cases);
    case_deltas.sort_by(compare_case_delta);

    let regression_count = case_deltas
        .iter()
        .filter(|case| case.status == "regressed")
        .count();
    let improvement_count = case_deltas
        .iter()
        .filter(|case| case.status == "improved")
        .count();
    let unchanged_count = case_deltas
        .iter()
        .filter(|case| case.status == "unchanged")
        .count();
    let total_compared_cases = case_deltas.len();
    let limit = if case_limit == 0 {
        DEFAULT_CASE_LIMIT
    } else {
        case_limit.min(MAX_CASE_LIMIT)
    };
    case_deltas.truncate(limit);
    let returned_case_count = case_deltas.len();
    let verdict = compare_verdict(&aggregate_deltas, regression_count, improvement_count);
    let recommendations = build_recommendations(&aggregate_deltas, regression_count);

    SymbolRetrievalRunCompareResponse {
        db_path,
        baseline: baseline.summary,
        current: current.summary,
        verdict,
        regression_count,
        improvement_count,
        unchanged_count,
        total_compared_cases,
        returned_case_count,
        aggregate_deltas,
        intent_deltas,
        case_deltas,
        recommendations,
    }
}

impl RunSnapshot {
    fn from_detail(run: SymbolRetrievalRunDetail) -> Self {
        let aggregate = field(&run.scores, "aggregate")
            .cloned()
            .unwrap_or(Value::Null);
        let metrics = merged_run_metrics(&run.scores, &aggregate);
        Self {
            summary: SymbolRetrievalRunCompareRun {
                id: run.id,
                query: run.query,
                created_at: run.created_at,
                case_count: value_usize(&run.scores, "caseCount"),
                evaluated_count: value_usize(&run.scores, "evaluatedCount"),
                failed_count: value_usize(&run.scores, "failedCount"),
            },
            metrics,
            intents: intent_map(&run.scores),
            cases: case_map(&run.selected_chunks),
        }
    }
}

impl CaseSnapshot {
    fn from_value(value: &Value) -> Option<Self> {
        let id = string_field(value, "id")?;
        let result = field(value, "result");
        Some(Self {
            id,
            query: result
                .and_then(|item| field(item, "query"))
                .and_then(|item| string_field(item, "q").or_else(|| string_field(item, "query"))),
            ok: bool_field(value, "ok"),
            error: string_field(value, "error"),
            metrics: result
                .and_then(|item| field(item, "metrics"))
                .cloned()
                .unwrap_or(Value::Null),
            missing_requirements: result
                .and_then(|item| field(item, "missingRequirements"))
                .map(string_set)
                .unwrap_or_default(),
            top_candidates: result
                .and_then(|item| field(item, "candidates"))
                .map(top_candidates)
                .unwrap_or_default(),
        })
    }
}

#[path = "symbol_index_eval_compare_helpers.rs"]
mod helpers;
use self::helpers::*;
