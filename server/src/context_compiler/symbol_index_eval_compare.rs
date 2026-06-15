use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

use anyhow::{Result, bail};
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

fn aggregate_specs() -> &'static [MetricSpec] {
    &[
        MetricSpec {
            key: "caseCount",
            label: "caseCount",
            direction: MetricDirection::Neutral,
        },
        MetricSpec {
            key: "evaluatedCount",
            label: "evaluatedCount",
            direction: MetricDirection::HigherIsBetter,
        },
        MetricSpec {
            key: "failedCount",
            label: "failedCount",
            direction: MetricDirection::LowerIsBetter,
        },
        MetricSpec {
            key: "requirementCount",
            label: "requirementCount",
            direction: MetricDirection::Neutral,
        },
        MetricSpec {
            key: "hitCountAtK",
            label: "hitCountAtK",
            direction: MetricDirection::HigherIsBetter,
        },
        MetricSpec {
            key: "missingRequirementCount",
            label: "missingRequirementCount",
            direction: MetricDirection::LowerIsBetter,
        },
        MetricSpec {
            key: "meanRecallAtK",
            label: "meanRecallAtK",
            direction: MetricDirection::HigherIsBetter,
        },
        MetricSpec {
            key: "meanReciprocalRank",
            label: "meanReciprocalRank",
            direction: MetricDirection::HigherIsBetter,
        },
        MetricSpec {
            key: "hasTestContextRate",
            label: "hasTestContextRate",
            direction: MetricDirection::HigherIsBetter,
        },
        MetricSpec {
            key: "noiseCountAtK",
            label: "noiseCountAtK",
            direction: MetricDirection::LowerIsBetter,
        },
        MetricSpec {
            key: "meanNoiseRateAtK",
            label: "meanNoiseRateAtK",
            direction: MetricDirection::LowerIsBetter,
        },
        MetricSpec {
            key: "totalTokenCountAtK",
            label: "totalTokenCountAtK",
            direction: MetricDirection::LowerIsBetter,
        },
        MetricSpec {
            key: "averageTokenCountAtK",
            label: "averageTokenCountAtK",
            direction: MetricDirection::LowerIsBetter,
        },
        MetricSpec {
            key: "candidateCount",
            label: "candidateCount",
            direction: MetricDirection::Neutral,
        },
    ]
}

fn case_specs() -> &'static [MetricSpec] {
    &[
        MetricSpec {
            key: "recallAtK",
            label: "recallAtK",
            direction: MetricDirection::HigherIsBetter,
        },
        MetricSpec {
            key: "meanReciprocalRank",
            label: "meanReciprocalRank",
            direction: MetricDirection::HigherIsBetter,
        },
        MetricSpec {
            key: "noiseRateAtK",
            label: "noiseRateAtK",
            direction: MetricDirection::LowerIsBetter,
        },
        MetricSpec {
            key: "totalTokenCountAtK",
            label: "totalTokenCountAtK",
            direction: MetricDirection::LowerIsBetter,
        },
    ]
}

fn compare_intents(
    baseline: &BTreeMap<String, Value>,
    current: &BTreeMap<String, Value>,
) -> Vec<SymbolRetrievalIntentDelta> {
    let mut intents = baseline.keys().cloned().collect::<BTreeSet<_>>();
    intents.extend(current.keys().cloned());
    intents
        .into_iter()
        .map(|intent| {
            let empty = Value::Null;
            let deltas = metric_deltas(
                baseline.get(&intent).unwrap_or(&empty),
                current.get(&intent).unwrap_or(&empty),
                aggregate_specs(),
            );
            let status = deltas_status(&deltas);
            SymbolRetrievalIntentDelta {
                intent,
                status,
                deltas,
            }
        })
        .collect()
}

fn compare_cases(
    baseline: &BTreeMap<String, CaseSnapshot>,
    current: &BTreeMap<String, CaseSnapshot>,
) -> Vec<SymbolRetrievalCaseDelta> {
    let mut case_ids = baseline.keys().cloned().collect::<BTreeSet<_>>();
    case_ids.extend(current.keys().cloned());
    case_ids
        .into_iter()
        .map(|id| compare_case(id.clone(), baseline.get(&id), current.get(&id)))
        .collect()
}

fn compare_case(
    id: String,
    baseline: Option<&CaseSnapshot>,
    current: Option<&CaseSnapshot>,
) -> SymbolRetrievalCaseDelta {
    let empty = Value::Null;
    let deltas = metric_deltas(
        baseline.map(|case| &case.metrics).unwrap_or(&empty),
        current.map(|case| &case.metrics).unwrap_or(&empty),
        case_specs(),
    );
    let resolved_missing_requirements = set_difference(
        baseline.map(|case| &case.missing_requirements),
        current.map(|case| &case.missing_requirements),
    );
    let new_missing_requirements = set_difference(
        current.map(|case| &case.missing_requirements),
        baseline.map(|case| &case.missing_requirements),
    );
    let status = case_status(
        baseline,
        current,
        &deltas,
        &resolved_missing_requirements,
        &new_missing_requirements,
    );

    SymbolRetrievalCaseDelta {
        id,
        query: current
            .and_then(|case| case.query.clone())
            .or_else(|| baseline.and_then(|case| case.query.clone())),
        status,
        baseline_ok: baseline.map(|case| case.ok),
        current_ok: current.map(|case| case.ok),
        baseline_error: baseline.and_then(|case| case.error.clone()),
        current_error: current.and_then(|case| case.error.clone()),
        deltas,
        resolved_missing_requirements,
        new_missing_requirements,
        baseline_top_candidates: baseline
            .map(|case| case.top_candidates.clone())
            .unwrap_or_default(),
        current_top_candidates: current
            .map(|case| case.top_candidates.clone())
            .unwrap_or_default(),
    }
}

fn metric_deltas(
    baseline: &Value,
    current: &Value,
    specs: &[MetricSpec],
) -> Vec<SymbolRetrievalMetricDelta> {
    specs
        .iter()
        .map(|spec| {
            let baseline_value = value_f64(baseline, spec.key);
            let current_value = value_f64(current, spec.key);
            let delta = current_value - baseline_value;
            SymbolRetrievalMetricDelta {
                metric: spec.label.to_string(),
                baseline: baseline_value,
                current: current_value,
                delta,
                direction: direction_name(spec.direction).to_string(),
                status: metric_status(delta, spec.direction).to_string(),
            }
        })
        .collect()
}

fn case_status(
    baseline: Option<&CaseSnapshot>,
    current: Option<&CaseSnapshot>,
    deltas: &[SymbolRetrievalMetricDelta],
    resolved_missing: &[String],
    new_missing: &[String],
) -> String {
    match (baseline, current) {
        (Some(_), None) => return "regressed".to_string(),
        (None, Some(_)) => return "improved".to_string(),
        (None, None) => return "unchanged".to_string(),
        (Some(before), Some(after)) if before.ok && !after.ok => return "regressed".to_string(),
        (Some(before), Some(after)) if !before.ok && after.ok => return "improved".to_string(),
        _ => {}
    }

    if !new_missing.is_empty() || deltas.iter().any(|delta| delta.status == "regressed") {
        "regressed".to_string()
    } else if !resolved_missing.is_empty() || deltas.iter().any(|delta| delta.status == "improved")
    {
        "improved".to_string()
    } else {
        "unchanged".to_string()
    }
}

fn compare_verdict(
    aggregate_deltas: &[SymbolRetrievalMetricDelta],
    regression_count: usize,
    improvement_count: usize,
) -> String {
    if regression_count > 0
        || aggregate_deltas
            .iter()
            .any(|delta| delta.status == "regressed")
    {
        "regressed".to_string()
    } else if improvement_count > 0
        || aggregate_deltas
            .iter()
            .any(|delta| delta.status == "improved")
    {
        "improved".to_string()
    } else {
        "unchanged".to_string()
    }
}

fn build_recommendations(
    aggregate_deltas: &[SymbolRetrievalMetricDelta],
    regression_count: usize,
) -> Vec<String> {
    let mut recommendations = Vec::new();
    if regression_count > 0 {
        recommendations.push(format!(
            "发现 {regression_count} 个回归 case，先查看 caseDeltas 中 status=regressed 的 query、missing requirements 和 top candidates。"
        ));
    }
    if delta_for(aggregate_deltas, "failedCount") > EPSILON {
        recommendations
            .push("current run 的失败 case 增加，先修复评测执行错误再比较检索质量。".to_string());
    }
    if delta_for(aggregate_deltas, "meanRecallAtK") > EPSILON
        && delta_for(aggregate_deltas, "meanNoiseRateAtK") <= EPSILON
    {
        recommendations
            .push("current run 提升了平均召回且没有增加平均噪声，可以作为候选基线。".to_string());
    }
    if recommendations.is_empty() {
        recommendations.push("未发现明显质量变化，可结合具体任务继续观察。".to_string());
    }
    recommendations
}

fn metric_status(delta: f64, direction: MetricDirection) -> &'static str {
    if delta.abs() <= EPSILON || direction == MetricDirection::Neutral {
        "unchanged"
    } else {
        match direction {
            MetricDirection::HigherIsBetter if delta > 0.0 => "improved",
            MetricDirection::HigherIsBetter => "regressed",
            MetricDirection::LowerIsBetter if delta < 0.0 => "improved",
            MetricDirection::LowerIsBetter => "regressed",
            MetricDirection::Neutral => "unchanged",
        }
    }
}

fn deltas_status(deltas: &[SymbolRetrievalMetricDelta]) -> String {
    if deltas.iter().any(|delta| delta.status == "regressed") {
        "regressed".to_string()
    } else if deltas.iter().any(|delta| delta.status == "improved") {
        "improved".to_string()
    } else {
        "unchanged".to_string()
    }
}

fn direction_name(direction: MetricDirection) -> &'static str {
    match direction {
        MetricDirection::HigherIsBetter => "higher_is_better",
        MetricDirection::LowerIsBetter => "lower_is_better",
        MetricDirection::Neutral => "neutral",
    }
}

fn compare_case_delta(
    left: &SymbolRetrievalCaseDelta,
    right: &SymbolRetrievalCaseDelta,
) -> std::cmp::Ordering {
    case_status_rank(&left.status)
        .cmp(&case_status_rank(&right.status))
        .then_with(|| left.id.cmp(&right.id))
}

fn case_status_rank(status: &str) -> usize {
    match status {
        "regressed" => 0,
        "improved" => 1,
        _ => 2,
    }
}

fn delta_for(deltas: &[SymbolRetrievalMetricDelta], metric: &str) -> f64 {
    deltas
        .iter()
        .find(|delta| delta.metric == metric)
        .map(|delta| delta.delta)
        .unwrap_or_default()
}

fn merged_run_metrics(scores: &Value, aggregate: &Value) -> Value {
    let mut object = serde_json::Map::new();
    for key in ["caseCount", "evaluatedCount", "failedCount"] {
        object.insert(
            key.to_string(),
            field(scores, key).cloned().unwrap_or(Value::Null),
        );
    }
    if let Some(aggregate) = aggregate.as_object() {
        for (key, value) in aggregate {
            object.insert(key.clone(), value.clone());
        }
    }
    Value::Object(object)
}

fn intent_map(scores: &Value) -> BTreeMap<String, Value> {
    field(scores, "intentGroups")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|item| Some((string_field(item, "intent")?, item.clone())))
        .collect()
}

fn case_map(selected_chunks: &Value) -> BTreeMap<String, CaseSnapshot> {
    selected_chunks
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(CaseSnapshot::from_value)
        .map(|case| (case.id.clone(), case))
        .collect()
}

fn top_candidates(value: &Value) -> Vec<String> {
    value
        .as_array()
        .into_iter()
        .flatten()
        .take(5)
        .map(|candidate| {
            let file_path = string_field(candidate, "filePath").unwrap_or_default();
            let label = string_field(candidate, "label").unwrap_or_default();
            if file_path.is_empty() {
                label
            } else if label.is_empty() {
                file_path
            } else {
                format!("{file_path}::{label}")
            }
        })
        .filter(|item| !item.is_empty())
        .collect()
}

fn set_difference(
    left: Option<&BTreeSet<String>>,
    right: Option<&BTreeSet<String>>,
) -> Vec<String> {
    let empty = BTreeSet::new();
    left.unwrap_or(&empty)
        .difference(right.unwrap_or(&empty))
        .cloned()
        .collect()
}

fn string_set(value: &Value) -> BTreeSet<String> {
    value
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn value_usize(value: &Value, key: &str) -> usize {
    value_f64(value, key).max(0.0) as usize
}

fn value_f64(value: &Value, key: &str) -> f64 {
    field(value, key)
        .and_then(|item| {
            item.as_f64()
                .or_else(|| item.as_i64().map(|number| number as f64))
                .or_else(|| item.as_u64().map(|number| number as f64))
        })
        .unwrap_or_default()
}

fn bool_field(value: &Value, key: &str) -> bool {
    field(value, key)
        .and_then(Value::as_bool)
        .unwrap_or_default()
}

fn string_field(value: &Value, key: &str) -> Option<String> {
    field(value, key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(ToOwned::to_owned)
}

fn field<'a>(value: &'a Value, key: &str) -> Option<&'a Value> {
    value.get(key).or_else(|| {
        let snake = camel_to_snake(key);
        value.get(&snake)
    })
}

fn camel_to_snake(value: &str) -> String {
    let mut out = String::new();
    for ch in value.chars() {
        if ch.is_ascii_uppercase() {
            out.push('_');
            out.push(ch.to_ascii_lowercase());
        } else {
            out.push(ch);
        }
    }
    out
}
