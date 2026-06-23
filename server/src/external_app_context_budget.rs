//! server/src/external_app_context_budget.rs
//! Budgeting and prompt projection for external app context packs.

use serde_json::{json, Value};

use crate::{
    external_app_context_answer_policy::prompt_answer_rules_block,
    external_app_context_tools::prompt_tool_contract_block,
};

const DEFAULT_MAX_CONTEXT_CHARS: usize = 16_000;
const MIN_MAX_CONTEXT_CHARS: usize = 4_000;
const MAX_MAX_CONTEXT_CHARS: usize = 48_000;

pub(crate) fn budgeted_context(mut context: Value) -> Value {
    let max_chars = max_context_chars();
    let before_chars = json_char_len(&context);
    let mut trimmed = false;

    if before_chars > max_chars {
        trim_heavy_field(&mut context, "group_messages", 24);
        trim_heavy_field(&mut context, "matches", 24);
        trim_heavy_field(&mut context, "user_orders", 12);
        trim_context_pack(&mut context, max_chars / 2);
        trimmed = true;
    }

    let after_chars = json_char_len(&context);
    if trimmed || before_chars > 0 {
        context["_context_budget"] = json!({
            "max_chars": max_chars,
            "before_chars": before_chars,
            "after_chars": after_chars,
            "trimmed": trimmed
        });
    }
    context
}

pub(crate) fn prompt_context_block(context: &Value) -> String {
    let source = context["source"].as_str().unwrap_or("external_app_context");
    let status = context["status"].as_str().unwrap_or("unknown");
    let generated_at = context
        .get("generated_at")
        .and_then(value_as_prompt_string)
        .unwrap_or_else(|| "unknown".to_string());
    let budget = context
        .get("_context_budget")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let usage_policy = context
        .get("usage_policy")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let answer_policy = context
        .get("answer_policy")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let context_quality = context
        .get("context_quality")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let gap_summary = context_gap_summary(context);
    let external_metrics = context.get("metrics").cloned().unwrap_or_else(|| json!({}));
    let context_audit_id = context
        .get("context_audit_id")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let fact_summary = context_fact_summary(context);
    let tool_contract = prompt_tool_contract_block(context);
    let answer_rules = prompt_answer_rules_block(context);

    let body = context["context_pack"]
        .as_str()
        .map(ToOwned::to_owned)
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| serde_json::to_string_pretty(context).unwrap_or_default());

    format!(
        "外部项目业务上下文：\n\
         <external_app_context source=\"{source}\" status=\"{status}\" generated_at=\"{generated_at}\">\n\
         <metadata>\n\
         usage_policy={}\n\
         answer_policy={}\n\
         context_quality={}\n\
         context_gap_summary={}\n\
         context_budget={}\n\
         external_metrics={}\n\
         context_fact_summary={}\n\
         context_audit_id={}\n\
         </metadata>\n\n\
         {}\n\n\
         {}\n\n\
         {}\n\
         </external_app_context>",
        serde_json::to_string(&usage_policy).unwrap_or_else(|_| "{}".into()),
        serde_json::to_string(&answer_policy).unwrap_or_else(|_| "{}".into()),
        serde_json::to_string(&context_quality).unwrap_or_else(|_| "{}".into()),
        serde_json::to_string(&gap_summary).unwrap_or_else(|_| "{}".into()),
        serde_json::to_string(&budget).unwrap_or_else(|_| "{}".into()),
        serde_json::to_string(&external_metrics).unwrap_or_else(|_| "{}".into()),
        serde_json::to_string(&fact_summary).unwrap_or_else(|_| "{}".into()),
        context_audit_id,
        tool_contract,
        body.trim(),
        answer_rules
    )
}

fn max_context_chars() -> usize {
    std::env::var("ELON_EXTERNAL_APP_CONTEXT_MAX_CHARS")
        .ok()
        .and_then(|value| value.trim().parse::<usize>().ok())
        .unwrap_or(DEFAULT_MAX_CONTEXT_CHARS)
        .clamp(MIN_MAX_CONTEXT_CHARS, MAX_MAX_CONTEXT_CHARS)
}

fn trim_heavy_field(context: &mut Value, field: &str, keep: usize) {
    let Some(values) = context.get_mut(field).and_then(Value::as_array_mut) else {
        return;
    };
    if values.len() <= keep {
        return;
    }
    let original_len = values.len();
    values.truncate(keep);
    context[format!("{}_truncated", field)] = json!({
        "original_count": original_len,
        "kept_count": keep,
        "reason": "external_app_context_budget"
    });
}

fn trim_context_pack(context: &mut Value, max_chars: usize) {
    let Some(pack) = context.get("context_pack").and_then(Value::as_str) else {
        return;
    };
    if pack.chars().count() <= max_chars {
        return;
    }
    let mut trimmed = pack.chars().take(max_chars).collect::<String>();
    trimmed.push_str(
        "\n\n[context_pack 已按主项目 token budget 截断，fb2 可通过后续工具接口继续查询细节]",
    );
    context["context_pack"] = Value::String(trimmed);
}

fn json_char_len(value: &Value) -> usize {
    serde_json::to_string(value)
        .map(|text| text.chars().count())
        .unwrap_or(0)
}

fn value_as_prompt_string(value: &Value) -> Option<String> {
    value.as_str().map(ToOwned::to_owned).or_else(|| {
        if value.is_null() {
            None
        } else {
            Some(value.to_string())
        }
    })
}

fn context_fact_summary(context: &Value) -> Value {
    json!({
        "match_count": array_len(context, "matches"),
        "user_order_count": array_len(context, "user_orders"),
        "group_message_count": array_len(context, "group_messages"),
        "preflight_readiness": preflight_readiness_summary(context.get("preflight_readiness")),
        "source_id_samples": {
            "match_ids": id_samples(context.get("matches"), &["id", "match_id"], 5),
            "order_ids": id_samples(context.get("user_orders"), &["order_id", "id", "ticket_id"], 5),
            "message_ids": id_samples(context.get("group_messages"), &["message_id", "id"], 5),
            "platform_summary_ids": citation_ids_by_kind(
                context.get("citation_sources"),
                "platform_order_summary",
                3
            )
        },
        "citation_source_samples": citation_source_samples(context.get("citation_sources"), 8),
        "user_order_samples": order_samples(context.get("user_orders"), 3),
        "user_orders_scope": if array_len(context, "user_orders") > 0 {
            "current_user_only_after_external_user_id_header_check"
        } else {
            "none_in_context_pack"
        }
    })
}

fn context_gap_summary(context: &Value) -> Value {
    let warnings = context_warning_codes(context);
    let readiness_status = context
        .get("preflight_readiness")
        .and_then(|readiness| readiness.get("status"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("unknown");
    let budget_status = context
        .get("metrics")
        .and_then(|metrics| metrics.get("budget_status"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("unknown");
    let status = context
        .get("status")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("unknown");
    let context_budget_trimmed = context
        .get("_context_budget")
        .and_then(|budget| budget.get("trimmed"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let partial = context_budget_trimmed
        || matches!(readiness_status, "degraded" | "partial")
        || matches!(budget_status, "too_large")
        || warnings
            .iter()
            .any(|warning| warning.as_str() == "fb2_budget_too_large");
    let blocking = status != "ready"
        || matches!(
            readiness_status,
            "blocked" | "unavailable" | "not_configured"
        )
        || matches!(budget_status, "empty")
        || warnings.iter().any(|warning| {
            matches!(
                warning.as_str(),
                "fb2_readiness_blocked" | "fb2_budget_empty" | "missing_context_pack"
            )
        });

    // 这个摘要专门给模型快速读取缺口：它不替代 context_quality 细节，只把会导致保守回答的信号前置。
    json!({
        "status": status,
        "readiness_status": readiness_status,
        "budget_status": budget_status,
        "warning_codes": warnings,
        "business_data_available": {
            "matches": array_len(context, "matches") > 0,
            "user_orders": array_len(context, "user_orders") > 0,
            "group_messages": array_len(context, "group_messages") > 0,
            "platform_order_summary": !context.get("platform_order_summary").unwrap_or(&Value::Null).is_null()
        },
        "truncation": {
            "context_budget_trimmed": context_budget_trimmed,
            "fields": truncated_fields(context)
        },
        "fact_answer_allowed": !blocking,
        "partial_answer_only": partial,
        "required_user_notice": if blocking {
            "fb2_context_gap_or_unverified_data_present"
        } else if partial {
            "fb2_context_partial_or_truncated_context_present"
        } else {
            "none"
        }
    })
}

fn context_warning_codes(context: &Value) -> Vec<String> {
    context
        .get("context_quality")
        .and_then(|quality| quality.get("warnings"))
        .and_then(Value::as_array)
        .map(|warnings| {
            warnings
                .iter()
                .filter_map(Value::as_str)
                .map(str::trim)
                .filter(|warning| !warning.is_empty())
                .take(12)
                .map(ToOwned::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

fn truncated_fields(context: &Value) -> Vec<String> {
    [
        ("matches_truncated", "matches"),
        ("user_orders_truncated", "user_orders"),
        ("group_messages_truncated", "group_messages"),
    ]
    .into_iter()
    .filter(|(marker, _)| context.get(*marker).is_some())
    .map(|(_, field)| field.to_string())
    .collect()
}

fn preflight_readiness_summary(value: Option<&Value>) -> Value {
    let Some(readiness) = value else {
        return json!({
            "status": "unknown",
            "warnings": []
        });
    };
    json!({
        "status": first_prompt_value(readiness, &["status"]),
        "warnings": readiness
            .get("warnings")
            .and_then(Value::as_array)
            .map(|warnings| warnings
                .iter()
                .filter_map(|warning| warning.as_str().map(str::trim))
                .filter(|warning| !warning.is_empty())
                .take(5)
                .collect::<Vec<_>>())
            .unwrap_or_default()
    })
}

fn array_len(context: &Value, field: &str) -> usize {
    context
        .get(field)
        .and_then(Value::as_array)
        .map(Vec::len)
        .unwrap_or_default()
}

fn id_samples(value: Option<&Value>, fields: &[&str], limit: usize) -> Vec<String> {
    value
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| first_string_field(item, fields))
                .take(limit)
                .collect()
        })
        .unwrap_or_default()
}

fn first_string_field(value: &Value, fields: &[&str]) -> Option<String> {
    fields.iter().find_map(|field| {
        value
            .get(*field)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
    })
}

fn citation_ids_by_kind(value: Option<&Value>, kind: &str, limit: usize) -> Vec<String> {
    value
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter(|item| item.get("kind").and_then(Value::as_str) == Some(kind))
                .filter_map(|item| first_string_field(item, &["id"]))
                .take(limit)
                .collect()
        })
        .unwrap_or_default()
}

fn citation_source_samples(value: Option<&Value>, limit: usize) -> Vec<Value> {
    value
        .and_then(Value::as_array)
        .map(|sources| {
            let mut selected = sources
                .iter()
                .filter(|source| {
                    source.get("kind").and_then(Value::as_str) == Some("platform_order_summary")
                })
                .take(2)
                .collect::<Vec<_>>();
            let remaining = limit.saturating_sub(selected.len());
            selected.extend(
                sources
                    .iter()
                    .filter(|source| {
                        source.get("kind").and_then(Value::as_str) != Some("platform_order_summary")
                    })
                    .take(remaining),
            );
            selected
                .into_iter()
                .take(limit)
                .map(|source| {
                    json!({
                        "kind": first_prompt_value(source, &["kind"]),
                        "id": first_prompt_value(source, &["id"]),
                        "label": first_prompt_value(source, &["label"])
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

fn order_samples(value: Option<&Value>, limit: usize) -> Vec<Value> {
    value
        .and_then(Value::as_array)
        .map(|orders| {
            orders
                .iter()
                .take(limit)
                .map(|order| {
                    json!({
                        "order_id": first_prompt_value(order, &["order_id", "id", "ticket_id"]),
                        "status": first_prompt_value(order, &["status", "order_status"]),
                        "amount": first_prompt_value(order, &["total_amount", "amount", "stake"]),
                        "bet_slip_count": order
                            .get("bet_slips")
                            .or_else(|| order.get("slips"))
                            .and_then(Value::as_array)
                            .map(Vec::len)
                            .unwrap_or_default(),
                        "first_slip": order
                            .get("bet_slips")
                            .or_else(|| order.get("slips"))
                            .and_then(Value::as_array)
                            .and_then(|slips| slips.first())
                            .map(compact_slip_value)
                            .unwrap_or(Value::Null)
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

fn compact_slip_value(slip: &Value) -> Value {
    json!({
        "match_id": first_prompt_value(slip, &["match_id", "id"]),
        "home_team": first_prompt_value(slip, &["home_team"]),
        "away_team": first_prompt_value(slip, &["away_team"]),
        "selection": first_prompt_value(slip, &["selection", "pick", "bet_selection"]),
        "odds": first_prompt_value(slip, &["odds", "actual_odds", "original_odds"])
    })
}

fn first_prompt_value(value: &Value, fields: &[&str]) -> Value {
    fields
        .iter()
        .find_map(|field| value.get(*field))
        .map(|value| match value {
            Value::String(text) => Value::String(text.trim().to_string()),
            Value::Number(_) | Value::Bool(_) => value.clone(),
            _ => Value::Null,
        })
        .filter(|value| !matches!(value, Value::String(text) if text.is_empty()))
        .unwrap_or(Value::Null)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prompt_prefers_context_pack_body() {
        let block = prompt_context_block(&json!({
            "source": "fb2:/api/main-project/context/pack",
            "status": "ready",
            "context_pack": "<fb2_context_pack>hello</fb2_context_pack>",
            "tool_contract": {"tools": [{"name": "get_match_detail"}]},
            "usage_policy": {"no_guaranteed_win": true},
            "answer_policy": {"schema": "fb2.answer_policy.v1"},
            "context_audit_id": "audit-1",
            "metrics": {"budget_status": "ok"},
            "preflight_readiness": {
                "status": "degraded",
                "warnings": ["fb2_readiness_degraded"]
            },
            "matches": [{"id": "match-1"}],
            "citation_sources": [
                {"kind": "match", "id": "match-1", "label": "比赛 match-1"},
                {"kind": "platform_order_summary", "id": "platform_order_summary:2026-06-21:all", "label": "平台订单摘要"}
            ],
            "user_orders": [{
                "order_id": "order-1",
                "status": "pending",
                "total_amount": 54,
                "bet_slips": [{
                    "match_id": "match-1",
                    "home_team": "主队",
                    "away_team": "客队",
                    "selection": "主胜",
                    "odds": 1.96
                }]
            }],
            "group_messages": [{"message_id": "message-1"}]
        }));
        assert!(block.contains("<fb2_context_pack>hello</fb2_context_pack>"));
        assert!(block.contains("answer_policy="));
        assert!(block.contains("fb2.answer_policy.v1"));
        assert!(block.contains("context_quality="));
        assert!(block.contains("context_gap_summary="));
        assert!(block.contains("external_metrics="));
        assert!(block.contains("context_fact_summary="));
        assert!(block.contains("\"user_order_count\":1"));
        assert!(block.contains("\"preflight_readiness\""));
        assert!(block.contains("\"status\":\"degraded\""));
        assert!(block.contains("fb2_readiness_degraded"));
        assert!(block.contains("platform_order_summary:2026-06-21:all"));
        assert!(block.contains("\"kind\":\"platform_order_summary\""));
        assert!(block.contains("order-1"));
        assert!(block.contains("\"bet_slip_count\":1"));
        assert!(block.contains("\"selection\":\"主胜\""));
        assert!(block.contains("context_audit_id=audit-1"));
        assert!(block.contains("get_match_detail"));
        assert!(block.contains("tool_readiness.status"));
        assert!(block.contains("必须区分"));
    }

    #[test]
    fn prompt_gap_summary_surfaces_blocked_or_empty_context() {
        let context = json!({
            "source": "fb2:/api/main-project/context/pack",
            "status": "ready",
            "generated_at": "2026-06-22T12:00:00+08:00",
            "context_pack": "<fb2_context_pack>没有可用比赛</fb2_context_pack>",
            "metrics": {"budget_status": "empty"},
            "preflight_readiness": {
                "status": "blocked",
                "warnings": ["fb2_readiness_blocked"]
            },
            "context_quality": {
                "warnings": ["fb2_readiness_blocked", "fb2_budget_empty", "empty_matches"]
            },
            "matches": [],
            "user_orders": [],
            "group_messages": []
        });

        let block = prompt_context_block(&context);

        assert!(block.contains("context_gap_summary="));
        assert!(block.contains("\"readiness_status\":\"blocked\""));
        assert!(block.contains("\"budget_status\":\"empty\""));
        assert!(block.contains("\"matches\":false"));
        assert!(block.contains("\"user_orders\":false"));
        assert!(block.contains("\"fact_answer_allowed\":false"));
        assert!(block.contains("fb2_context_gap_or_unverified_data_present"));
    }

    #[test]
    fn trims_large_arrays() {
        let context = json!({
            "group_messages": (0..80).map(|index| json!({"id": index, "content": "x".repeat(200)})).collect::<Vec<_>>(),
            "context_pack": "y".repeat(60_000)
        });
        let budgeted = budgeted_context(context);
        assert!(budgeted["_context_budget"]["trimmed"].as_bool().unwrap());
        assert!(budgeted["group_messages"].as_array().unwrap().len() <= 24);
    }

    #[test]
    fn gap_summary_records_budget_truncation_fields() {
        let context = json!({
            "status": "ready",
            "_context_budget": {"trimmed": true},
            "group_messages_truncated": {"original_count": 80, "kept_count": 24},
            "matches_truncated": {"original_count": 60, "kept_count": 24},
            "context_quality": {"warnings": ["fb2_budget_too_large"]},
            "matches": [{"id": "match-1"}],
            "user_orders": [{"order_id": "order-1"}],
            "group_messages": [{"message_id": "message-1"}],
            "metrics": {"budget_status": "too_large"},
            "preflight_readiness": {"status": "degraded"}
        });

        let summary = context_gap_summary(&context);

        assert_eq!(summary["truncation"]["context_budget_trimmed"], true);
        assert!(summary["truncation"]["fields"]
            .as_array()
            .unwrap()
            .contains(&json!("group_messages")));
        assert!(summary["truncation"]["fields"]
            .as_array()
            .unwrap()
            .contains(&json!("matches")));
        assert_eq!(summary["business_data_available"]["user_orders"], true);
        assert_eq!(summary["fact_answer_allowed"], true);
        assert_eq!(summary["partial_answer_only"], true);
        assert_eq!(
            summary["required_user_notice"],
            "fb2_context_partial_or_truncated_context_present"
        );
    }
}
