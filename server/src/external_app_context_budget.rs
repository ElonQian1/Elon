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


#[path = "external_app_context_budget_impl.rs"]
mod impl_funcs;
use self::impl_funcs::*;
