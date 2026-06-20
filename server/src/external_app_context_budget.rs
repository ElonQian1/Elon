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
    let context_quality = context
        .get("context_quality")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let external_metrics = context.get("metrics").cloned().unwrap_or_else(|| json!({}));
    let context_audit_id = context
        .get("context_audit_id")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
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
         context_quality={}\n\
         context_budget={}\n\
         external_metrics={}\n\
         context_audit_id={}\n\
         </metadata>\n\n\
         {}\n\n\
         {}\n\n\
         {}\n\
         </external_app_context>",
        serde_json::to_string(&usage_policy).unwrap_or_else(|_| "{}".into()),
        serde_json::to_string(&context_quality).unwrap_or_else(|_| "{}".into()),
        serde_json::to_string(&budget).unwrap_or_else(|_| "{}".into()),
        serde_json::to_string(&external_metrics).unwrap_or_else(|_| "{}".into()),
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
            "context_audit_id": "audit-1",
            "metrics": {"budget_status": "ok"}
        }));
        assert!(block.contains("<fb2_context_pack>hello</fb2_context_pack>"));
        assert!(block.contains("context_quality="));
        assert!(block.contains("external_metrics="));
        assert!(block.contains("context_audit_id=audit-1"));
        assert!(block.contains("get_match_detail"));
        assert!(block.contains("tool_readiness.status"));
        assert!(block.contains("必须区分"));
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
}
