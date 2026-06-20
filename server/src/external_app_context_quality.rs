//! Quality contract and warning generation for external app context packs.

use serde_json::{json, Value};

use crate::external_app_context_tools::{tool_contract_quality_warning, tool_contract_readiness};

pub(crate) fn public_context_quality_guidance(app_id: &str) -> Option<Value> {
    match app_id {
        "fb2" => Some(json!({
            "app_id": "fb2",
            "schema": "fb2.context_quality.v1",
            "warning_catalog": context_quality_warning_catalog(),
            "rules": [
                "context_quality.warnings 非空时，AI 回答必须显式说明相关数据缺口或新鲜度风险。",
                "比赛、订单、群友观点必须尽量带 source id，方便用户和后续代理复盘。",
                "缺少 context_pack 时，主项目只能基于结构化 JSON 保守回答，不能编造 fb2 没有提供的信息。"
            ]
        })),
        _ => None,
    }
}

pub(crate) fn context_quality(context: &Value, expects_context_pack: bool) -> Value {
    let mut warnings = Vec::new();
    if !has_prompt_text(context.get("generated_at")) {
        warnings.push("missing_generated_at");
    }
    if expects_context_pack && !has_prompt_text(context.get("context_pack")) {
        warnings.push("missing_context_pack");
    }
    if expects_context_pack && !has_prompt_text(context.get("context_pack_version")) {
        warnings.push("missing_context_pack_version");
    }
    if context
        .get("matches")
        .and_then(Value::as_array)
        .map(|items| items.is_empty())
        .unwrap_or(true)
    {
        warnings.push("empty_matches");
    }
    if expects_context_pack {
        if let Some(warning) = tool_contract_quality_warning(context) {
            warnings.push(warning);
        }
        if let Some(warning) = budget_status_quality_warning(context) {
            warnings.push(warning);
        }
    }

    json!({
        "warnings": warnings,
        "requires_source_citations": true,
        "requires_freshness_notice": warnings.contains(&"missing_generated_at"),
        "schema": if expects_context_pack { "fb2.context_pack.v1" } else { "fb2.today_matches.v1" },
        "tool_readiness": if expects_context_pack { tool_contract_readiness(context) } else { json!({"status": "not_applicable"}) }
    })
}

fn context_quality_warning_catalog() -> Value {
    json!([
        {
            "code": "missing_generated_at",
            "severity": "degraded",
            "meaning": "fb2 没有返回上下文生成时间。",
            "ai_impact": "AI 必须提示数据新鲜度不足，不能假设赔率、订单或讨论是最新的。",
            "fb2_fix": "在 context pack 响应外层补充 generated_at，建议使用带时区的 ISO-8601 时间。"
        },
        {
            "code": "missing_context_pack",
            "severity": "blocking_for_rich_answer",
            "meaning": "fb2 没有返回模型友好的 XML-wrapped Markdown context_pack。",
            "ai_impact": "主项目只能投影结构化 JSON，回答会更保守，难以复用 fb2 的领域总结。",
            "fb2_fix": "返回 context_pack，并按比赛事实、用户订单、群友观点、平台摘要分区。"
        },
        {
            "code": "missing_context_pack_version",
            "severity": "degraded",
            "meaning": "fb2 没有声明 context pack 版本。",
            "ai_impact": "主项目难以追踪契约演进，后续兼容和评测会缺少版本依据。",
            "fb2_fix": "返回 context_pack_version，例如 fb2-chat-pack-v1。"
        },
        {
            "code": "empty_matches",
            "severity": "degraded",
            "meaning": "本次上下文没有比赛数据。",
            "ai_impact": "AI 不能假设今日有可分析比赛，也不能自行联网替代 fb2 的平台数据。",
            "fb2_fix": "按权限返回 matches；确实无比赛时返回空数组并在 context_pack 中说明无比赛。"
        },
        {
            "code": "missing_tool_contract",
            "severity": "degraded",
            "meaning": "fb2 没有声明按需查询工具。",
            "ai_impact": "AI 信息不足时只能说明缺口，不能计划或声称调用 get_match_detail 等工具。",
            "fb2_fix": "返回 tool_contract，至少声明 search_matches、get_match_detail、search_user_orders。"
        },
        {
            "code": "empty_tool_contract",
            "severity": "degraded",
            "meaning": "fb2 返回了 tool_contract，但工具列表为空。",
            "ai_impact": "主项目会视为工具不可用，回答仍只能依赖当前 context_pack。",
            "fb2_fix": "在 tool_contract.tools 中补充工具 name、description、input_schema、permission、when_to_use。"
        },
        {
            "code": "fb2_budget_empty",
            "severity": "blocking_for_fact_answer",
            "meaning": "fb2 metrics.budget_status=empty，本次上下文没有有效业务来源。",
            "ai_impact": "AI 必须说明缺少 fb2 业务上下文，不能基于空数据预测比赛或剖析订单。",
            "fb2_fix": "检查比赛、订单、群观点召回链路；确实无数据时在 context_pack 中明确说明无可用来源。"
        },
        {
            "code": "fb2_budget_large",
            "severity": "degraded",
            "meaning": "fb2 metrics.budget_status=large，本次上下文偏大。",
            "ai_impact": "AI 可以回答，但应优先使用精选证据，后续追问建议转向细分工具查询。",
            "fb2_fix": "优化 context_pack 摘要密度，减少重复原始数据，优先返回可引用的精选来源。"
        },
        {
            "code": "fb2_budget_too_large",
            "severity": "blocking_for_rich_answer",
            "meaning": "fb2 metrics.budget_status=too_large，本次上下文过大。",
            "ai_impact": "主项目可能截断上下文，AI 回答必须提示可能遗漏证据，并建议先用 search/detail 工具缩小范围。",
            "fb2_fix": "先通过 search_matches/search_user_orders/search_group_opinions 缩小候选，再按需返回详情。"
        }
    ])
}

fn budget_status_quality_warning(context: &Value) -> Option<&'static str> {
    match context
        .get("metrics")
        .and_then(|metrics| metrics.get("budget_status"))
        .and_then(Value::as_str)
        .map(str::trim)
    {
        Some("empty") => Some("fb2_budget_empty"),
        Some("large") => Some("fb2_budget_large"),
        Some("too_large") => Some("fb2_budget_too_large"),
        _ => None,
    }
}

fn has_prompt_text(value: Option<&Value>) -> bool {
    value
        .and_then(Value::as_str)
        .map(|text| !text.trim().is_empty())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exposes_public_context_quality_guidance() {
        let guidance = public_context_quality_guidance("fb2").unwrap();
        assert_eq!(guidance["schema"], "fb2.context_quality.v1");
        assert!(guidance["warning_catalog"]
            .as_array()
            .unwrap()
            .iter()
            .any(|warning| warning["code"] == "missing_context_pack"));
        assert!(public_context_quality_guidance("unknown").is_none());
    }

    #[test]
    fn quality_reports_missing_contract_fields() {
        let context = json!({
            "matches": [],
            "tool_contract": null
        });
        let quality = context_quality(&context, true);

        let warnings = quality["warnings"].as_array().unwrap();
        assert!(warnings.contains(&json!("missing_context_pack")));
        assert!(warnings.contains(&json!("missing_context_pack_version")));
        assert!(warnings.contains(&json!("missing_generated_at")));
        assert!(warnings.contains(&json!("missing_tool_contract")));
        assert_eq!(quality["tool_readiness"]["status"], "missing");
        assert_eq!(quality["schema"], "fb2.context_pack.v1");
    }

    #[test]
    fn match_quality_uses_today_matches_schema() {
        let quality = context_quality(
            &json!({
                "generated_at": "2026-06-20T16:00:00+08:00",
                "matches": [{"id": "m1"}]
            }),
            false,
        );

        assert_eq!(quality["schema"], "fb2.today_matches.v1");
        assert_eq!(quality["tool_readiness"]["status"], "not_applicable");
        assert_eq!(quality["warnings"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn quality_promotes_budget_status_to_warning() {
        let quality = context_quality(
            &json!({
                "generated_at": "2026-06-20T16:00:00+08:00",
                "context_pack_version": "fb2-chat-pack-v1",
                "context_pack": "<fb2_context_pack>large</fb2_context_pack>",
                "matches": [{"id": "m1"}],
                "tool_contract": {"tools": [{"name": "get_match_detail"}]},
                "metrics": {"budget_status": "too_large"}
            }),
            true,
        );

        let warnings = quality["warnings"].as_array().unwrap();
        assert!(warnings.contains(&json!("fb2_budget_too_large")));
    }
}
