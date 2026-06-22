//! Deterministic gap notices for external app answers.
//!
//! fb2 answers already receive strict prompt rules, but this post-generation
//! guard keeps replies honest when the context payload itself reports a gap.

use serde_json::Value;

pub(crate) fn ensure_fb2_context_gap_notice(
    reply: &str,
    external_context: Option<&Value>,
) -> String {
    let reply = reply.trim();
    let Some(context) = external_context else {
        return reply.to_string();
    };
    if reply.is_empty() || !is_fb2_context(context) || contains_gap_notice(reply) {
        return reply.to_string();
    }

    let reasons = collect_gap_reasons(context);
    if reasons.is_empty() {
        return reply.to_string();
    }

    format!(
        "{reply}\n数据缺口：当前 fb2 上下文存在{}，不能把缺失数据编造成比赛、赔率、订单或群友观点事实。",
        reasons.join("、")
    )
}

fn is_fb2_context(context: &Value) -> bool {
    context
        .get("app_id")
        .and_then(Value::as_str)
        .is_some_and(|value| value.trim() == "fb2")
        || context
            .get("source")
            .and_then(Value::as_str)
            .is_some_and(|value| value.contains("fb2"))
        || context
            .get("answer_policy")
            .and_then(|policy| policy.get("schema"))
            .and_then(Value::as_str)
            == Some("fb2.answer_policy.v1")
        || context
            .get("context_pack")
            .and_then(Value::as_str)
            .is_some_and(|pack| pack.contains("<fb2_context_pack"))
}

fn collect_gap_reasons(context: &Value) -> Vec<String> {
    let mut reasons = Vec::new();
    let warnings = warning_codes(context);

    if let Some(status) = prompt_string(context.get("status")) {
        if !matches!(status.as_str(), "ready" | "ok") {
            push_reason(&mut reasons, "服务状态未就绪");
        }
    }

    if let Some(readiness) = prompt_string(context.pointer("/preflight_readiness/status")) {
        if matches!(
            readiness.as_str(),
            "blocked" | "degraded" | "unavailable" | "not_configured" | "partial"
        ) {
            push_reason(&mut reasons, "readiness 未完全可用");
        }
    }

    if let Some(budget_status) = prompt_string(context.pointer("/metrics/budget_status")) {
        if matches!(budget_status.as_str(), "empty" | "too_large") {
            push_reason(&mut reasons, "Context Pack 预算不可用");
        }
    }

    if context
        .get("_context_budget")
        .and_then(|budget| budget.get("trimmed"))
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        push_reason(&mut reasons, "Context Pack 已截断");
    }

    if missing_context_pack(context) || warnings.iter().any(|code| code == "missing_context_pack") {
        push_reason(&mut reasons, "缺少可引用 Context Pack");
    }

    for warning in warnings {
        match warning.as_str() {
            "fb2_readiness_blocked" => push_reason(&mut reasons, "readiness 被阻断"),
            "fb2_budget_empty" => push_reason(&mut reasons, "业务上下文为空"),
            "fb2_budget_too_large" => push_reason(&mut reasons, "业务上下文过大"),
            _ => {}
        }
    }

    reasons
}

fn warning_codes(context: &Value) -> Vec<String> {
    let mut warnings = Vec::new();
    collect_warning_codes(context.pointer("/context_quality/warnings"), &mut warnings);
    collect_warning_codes(
        context.pointer("/preflight_readiness/warnings"),
        &mut warnings,
    );
    warnings
}

fn collect_warning_codes(value: Option<&Value>, out: &mut Vec<String>) {
    let Some(value) = value else {
        return;
    };
    let Some(values) = value.as_array() else {
        return;
    };
    for value in values {
        let code = value
            .as_str()
            .or_else(|| value.get("code").and_then(Value::as_str));
        if let Some(code) = code {
            let code = code.trim();
            if !code.is_empty() {
                push_reason(out, code);
            }
        }
    }
}

fn missing_context_pack(context: &Value) -> bool {
    context
        .get("context_pack")
        .and_then(Value::as_str)
        .map(str::trim)
        .map(str::is_empty)
        .unwrap_or(true)
}

fn contains_gap_notice(reply: &str) -> bool {
    ["数据缺口", "信息不足", "上下文不足"]
        .iter()
        .any(|needle| reply.contains(needle))
}

fn prompt_string(value: Option<&Value>) -> Option<String> {
    value
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn push_reason(out: &mut Vec<String>, reason: &str) {
    if !out.iter().any(|existing| existing == reason) {
        out.push(reason.to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::ensure_fb2_context_gap_notice;
    use serde_json::json;

    #[test]
    fn appends_gap_notice_for_blocked_fb2_context() {
        let context = json!({
            "app_id": "fb2",
            "source": "fb2:/api/main-project/context/pack",
            "status": "ready",
            "answer_policy": {"schema": "fb2.answer_policy.v1"},
            "metrics": {"budget_status": "empty"},
            "_context_budget": {"trimmed": true},
            "preflight_readiness": {
                "status": "blocked",
                "warnings": [{"code": "fb2_readiness_blocked"}]
            },
            "context_quality": {
                "warnings": ["fb2_budget_empty", "missing_context_pack"]
            },
            "context_pack": ""
        });

        let reply = ensure_fb2_context_gap_notice(
            "数据事实：暂时只能看到部分摘要。\nAI推断：先保守看。\n风险边界：不保证命中。",
            Some(&context),
        );

        assert!(reply.contains("数据缺口："));
        assert!(reply.contains("readiness 被阻断"));
        assert!(reply.contains("业务上下文为空"));
        assert!(reply.contains("缺少可引用 Context Pack"));
        assert!(reply.contains("不能把缺失数据编造成比赛、赔率、订单或群友观点事实"));
    }

    #[test]
    fn keeps_ready_fb2_context_unchanged() {
        let context = json!({
            "app_id": "fb2",
            "status": "ready",
            "metrics": {"budget_status": "ok"},
            "preflight_readiness": {"status": "ready"},
            "context_quality": {"warnings": []},
            "context_pack": "<fb2_context_pack>match_id M1</fb2_context_pack>"
        });
        let reply = "数据事实：match_id M1。\nAI推断：谨慎。\n风险边界：不保证命中。";

        assert_eq!(ensure_fb2_context_gap_notice(reply, Some(&context)), reply);
    }

    #[test]
    fn does_not_duplicate_existing_gap_notice() {
        let context = json!({
            "app_id": "fb2",
            "preflight_readiness": {"status": "blocked"},
            "context_pack": ""
        });
        let reply = "数据缺口：fb2 当前没有返回订单。";

        assert_eq!(ensure_fb2_context_gap_notice(reply, Some(&context)), reply);
    }

    #[test]
    fn ignores_non_fb2_context() {
        let context = json!({
            "app_id": "other",
            "preflight_readiness": {"status": "blocked"},
            "context_pack": ""
        });
        let reply = "普通回答";

        assert_eq!(ensure_fb2_context_gap_notice(reply, Some(&context)), reply);
    }
}
