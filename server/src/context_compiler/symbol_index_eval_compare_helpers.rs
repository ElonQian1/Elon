use super::{
    CaseSnapshot, MetricDirection, MetricSpec, RunSnapshot, SymbolRetrievalCaseDelta,
    SymbolRetrievalIntentDelta, SymbolRetrievalMetricDelta, SymbolRetrievalRunCompareResponse,
    SymbolRetrievalRunCompareRun, DEFAULT_CASE_LIMIT, EPSILON, MAX_CASE_LIMIT,
};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn aggregate_specs() -> &'static [MetricSpec] {
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

pub(super) fn case_specs() -> &'static [MetricSpec] {
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

pub(super) fn compare_intents(
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

pub(super) fn compare_cases(
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

pub(super) fn compare_case(
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

pub(super) fn metric_deltas(
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

pub(super) fn case_status(
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

pub(super) fn compare_verdict(
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

pub(super) fn build_recommendations(
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

pub(super) fn metric_status(delta: f64, direction: MetricDirection) -> &'static str {
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

pub(super) fn deltas_status(deltas: &[SymbolRetrievalMetricDelta]) -> String {
    if deltas.iter().any(|delta| delta.status == "regressed") {
        "regressed".to_string()
    } else if deltas.iter().any(|delta| delta.status == "improved") {
        "improved".to_string()
    } else {
        "unchanged".to_string()
    }
}

pub(super) fn direction_name(direction: MetricDirection) -> &'static str {
    match direction {
        MetricDirection::HigherIsBetter => "higher_is_better",
        MetricDirection::LowerIsBetter => "lower_is_better",
        MetricDirection::Neutral => "neutral",
    }
}

pub(super) fn compare_case_delta(
    left: &SymbolRetrievalCaseDelta,
    right: &SymbolRetrievalCaseDelta,
) -> std::cmp::Ordering {
    case_status_rank(&left.status)
        .cmp(&case_status_rank(&right.status))
        .then_with(|| left.id.cmp(&right.id))
}

pub(super) fn case_status_rank(status: &str) -> usize {
    match status {
        "regressed" => 0,
        "improved" => 1,
        _ => 2,
    }
}

pub(super) fn delta_for(deltas: &[SymbolRetrievalMetricDelta], metric: &str) -> f64 {
    deltas
        .iter()
        .find(|delta| delta.metric == metric)
        .map(|delta| delta.delta)
        .unwrap_or_default()
}

pub(super) fn merged_run_metrics(scores: &Value, aggregate: &Value) -> Value {
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

pub(super) fn intent_map(scores: &Value) -> BTreeMap<String, Value> {
    field(scores, "intentGroups")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|item| Some((string_field(item, "intent")?, item.clone())))
        .collect()
}

pub(super) fn case_map(selected_chunks: &Value) -> BTreeMap<String, CaseSnapshot> {
    selected_chunks
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(CaseSnapshot::from_value)
        .map(|case| (case.id.clone(), case))
        .collect()
}

pub(super) fn top_candidates(value: &Value) -> Vec<String> {
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

pub(super) fn set_difference(
    left: Option<&BTreeSet<String>>,
    right: Option<&BTreeSet<String>>,
) -> Vec<String> {
    let empty = BTreeSet::new();
    left.unwrap_or(&empty)
        .difference(right.unwrap_or(&empty))
        .cloned()
        .collect()
}

pub(super) fn string_set(value: &Value) -> BTreeSet<String> {
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

pub(super) fn value_usize(value: &Value, key: &str) -> usize {
    value_f64(value, key).max(0.0) as usize
}

pub(super) fn value_f64(value: &Value, key: &str) -> f64 {
    field(value, key)
        .and_then(|item| {
            item.as_f64()
                .or_else(|| item.as_i64().map(|number| number as f64))
                .or_else(|| item.as_u64().map(|number| number as f64))
        })
        .unwrap_or_default()
}

pub(super) fn bool_field(value: &Value, key: &str) -> bool {
    field(value, key)
        .and_then(Value::as_bool)
        .unwrap_or_default()
}

pub(super) fn string_field(value: &Value, key: &str) -> Option<String> {
    field(value, key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(ToOwned::to_owned)
}

pub(super) fn field<'a>(value: &'a Value, key: &str) -> Option<&'a Value> {
    value.get(key).or_else(|| {
        let snake = camel_to_snake(key);
        value.get(&snake)
    })
}

pub(super) fn camel_to_snake(value: &str) -> String {
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
