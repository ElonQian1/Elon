//! Budgeting and prompt projection for external app context packs.

use serde_json::{json, Value};

use crate::external_app_context_tools::prompt_tool_contract_block;

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
    let tool_contract = prompt_tool_contract_block(context);

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
         </metadata>\n\n\
         {}\n\n\
         {}\n\n\
         <answer_rules>\n\
         - 必须区分「数据事实」「群友观点」「AI推断」。\n\
         - 涉及比赛预测时必须说明不确定性，不承诺命中，不诱导投注。\n\
         - 引用比赛时尽量带 match id；引用订单/票据时尽量带 order id 或 ticket id；引用群友观点时必须带 message id。\n\
         - 如果上下文缺少用户订单、赔率更新时间或消息来源，必须说明信息不足，不能编造。\n\
         - 如果 context_quality.warnings 非空，回答中必须显式提示相关数据缺口或新鲜度风险。\n\
         - 如果需要更多比赛、订单或群友观点明细，只能提出需要调用的外部工具，不能把未调用工具的结果当事实。\n\
         - 如果 context_quality.tool_readiness.status 不是 ready，说明外部项目按需检索能力还不完整，回答要更保守。\n\
         </answer_rules>\n\
         </external_app_context>",
        serde_json::to_string(&usage_policy).unwrap_or_else(|_| "{}".into()),
        serde_json::to_string(&context_quality).unwrap_or_else(|_| "{}".into()),
        serde_json::to_string(&budget).unwrap_or_else(|_| "{}".into()),
        tool_contract,
        body.trim()
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
            "usage_policy": {"no_guaranteed_win": true}
        }));
        assert!(block.contains("<fb2_context_pack>hello</fb2_context_pack>"));
        assert!(block.contains("context_quality="));
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
