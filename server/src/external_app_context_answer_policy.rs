//! Answer policy for AI replies grounded in external app context.

use serde_json::{json, Value};

const FB2_ANSWER_RULES: &[&str] = &[
    "必须区分「数据事实」「用户订单」「群友观点」「AI推断」。",
    "涉及比赛预测时必须说明不确定性，不承诺命中，不诱导投注。",
    "引用比赛时尽量带 match id；引用订单/票据时尽量带 order id 或 ticket id；引用群友观点时必须带 message id。",
    "如果上下文缺少用户订单、赔率更新时间或消息来源，必须说明信息不足，不能编造。",
    "如果 context_quality.warnings 非空，回答中必须显式提示相关数据缺口或新鲜度风险。",
    "如果需要更多比赛、订单或群友观点明细，只能提出需要调用的外部工具，不能把未调用工具的结果当事实。",
    "如果 context_quality.tool_readiness.status 不是 ready，说明外部项目按需检索能力还不完整，回答要更保守。",
];

pub(crate) fn public_answer_policy_guidance(app_id: &str) -> Option<Value> {
    match app_id {
        "fb2" => Some(json!({
            "app_id": "fb2",
            "schema": "fb2.answer_policy.v1",
            "grounding_sections": [
                {
                    "name": "data_facts",
                    "description": "fb2 提供的比赛、赔率、赛程、数据源和更新时间。",
                    "required_source_ids": ["match_id", "odds_updated_at", "source"]
                },
                {
                    "name": "user_orders",
                    "description": "当前用户自己的订单、票据和组合风险。",
                    "required_source_ids": ["order_id", "ticket_id"],
                    "visibility": "current_user_only"
                },
                {
                    "name": "group_opinions",
                    "description": "群友围绕比赛、赔率或订单的观点摘要。",
                    "required_source_ids": ["message_id"],
                    "visibility": "group_visible"
                },
                {
                    "name": "ai_inference",
                    "description": "AI 基于事实、订单和观点做出的分析或预测。",
                    "must_disclose_uncertainty": true
                }
            ],
            "forbidden_behaviors": [
                "不能把 AI 推断写成 fb2 数据事实。",
                "不能编造未提供的赔率、伤停、订单或群友观点。",
                "不能承诺命中、诱导投注或代替用户决策。",
                "不能暴露其他用户的订单明细。"
            ],
            "prompt_answer_rules": FB2_ANSWER_RULES
        })),
        _ => None,
    }
}

pub(crate) fn prompt_answer_rules_block(_context: &Value) -> String {
    let rules = FB2_ANSWER_RULES
        .iter()
        .map(|rule| format!("- {rule}"))
        .collect::<Vec<_>>()
        .join("\n");
    format!("<answer_rules>\n{rules}\n</answer_rules>")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exposes_public_fb2_answer_policy() {
        let guidance = public_answer_policy_guidance("fb2").unwrap();
        assert_eq!(guidance["schema"], "fb2.answer_policy.v1");
        assert!(guidance["grounding_sections"]
            .as_array()
            .unwrap()
            .iter()
            .any(|section| section["name"] == "group_opinions"));
        assert!(guidance["forbidden_behaviors"]
            .as_array()
            .unwrap()
            .iter()
            .any(|rule| rule.as_str().unwrap_or("").contains("编造")));
        assert!(public_answer_policy_guidance("unknown").is_none());
    }

    #[test]
    fn prompt_rules_keep_source_boundaries() {
        let block = prompt_answer_rules_block(&json!({}));
        assert!(block.contains("<answer_rules>"));
        assert!(block.contains("必须区分"));
        assert!(block.contains("message id"));
        assert!(block.contains("不能编造"));
    }
}
