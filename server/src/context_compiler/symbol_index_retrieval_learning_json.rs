use std::collections::BTreeSet;

use serde_json::Value;

pub(super) fn parse_json(value: &str) -> Value {
    serde_json::from_str(value).unwrap_or_else(|_| Value::Null)
}

pub(super) fn top_candidates(result: &Value, top_k: usize) -> Vec<&Value> {
    field(result, "candidates")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .enumerate()
        .filter(|(index, candidate)| {
            let rank = usize_field(candidate, "rank").unwrap_or(index + 1);
            rank <= top_k
        })
        .map(|(_, candidate)| candidate)
        .collect()
}

pub(super) fn candidate_sources(candidate: &Value) -> Vec<String> {
    let mut sources = string_items(candidate, "sources")
        .into_iter()
        .collect::<BTreeSet<_>>();
    if let Some(source) = string_field(candidate, "source") {
        sources.insert(source);
    }
    if sources.is_empty() {
        sources.insert("unknown".to_string());
    }
    sources.into_iter().collect()
}

pub(super) fn result_intent(result: &Value) -> String {
    field(result, "retrievalPlan")
        .and_then(|plan| string_field(plan, "intent"))
        .unwrap_or_else(|| "unknown".to_string())
}

pub(super) fn result_top_k(result: &Value) -> Option<usize> {
    field(result, "query").and_then(|query| usize_field(query, "k"))
}

pub(super) fn string_items(value: &Value, key: &str) -> Vec<String> {
    field(value, key)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn string_field(value: &Value, key: &str) -> Option<String> {
    field(value, key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(ToOwned::to_owned)
}

pub(super) fn bool_field(value: &Value, key: &str) -> bool {
    field(value, key)
        .and_then(Value::as_bool)
        .unwrap_or_default()
}

pub(super) fn usize_field(value: &Value, key: &str) -> Option<usize> {
    number_field(value, key).map(|number| number.max(0.0) as usize)
}

pub(super) fn number_field(value: &Value, key: &str) -> Option<f64> {
    field(value, key).and_then(|item| {
        item.as_f64()
            .or_else(|| item.as_i64().map(|number| number as f64))
            .or_else(|| item.as_u64().map(|number| number as f64))
    })
}

pub(super) fn field<'a>(value: &'a Value, key: &str) -> Option<&'a Value> {
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
