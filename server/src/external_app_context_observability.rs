//! Public observability contract for external app business context.

use serde_json::{json, Value};

pub(crate) fn public_context_observability_guidance(app_id: &str) -> Option<Value> {
    match app_id {
        "fb2" => Some(json!({
            "app_id": "fb2",
            "schema": "fb2.context_observability.v1",
            "recommended_metrics": [
                {
                    "name": "context_pack_latency_ms",
                    "type": "integer",
                    "owner": "fb2",
                    "meaning": "fb2 生成 /api/main-project/context/pack 的耗时。",
                    "target": "优先小于 1500ms；超过 3000ms 应记录慢查询原因。"
                },
                {
                    "name": "context_chars",
                    "type": "integer",
                    "owner": "main_project",
                    "meaning": "主项目预算裁剪后的外部上下文字符数。",
                    "target": "默认不超过 ELON_EXTERNAL_APP_CONTEXT_MAX_CHARS。"
                },
                {
                    "name": "source_counts",
                    "type": "object",
                    "owner": "fb2",
                    "meaning": "本次上下文包含的比赛、订单、群观点和平台摘要数量。",
                    "target": "AI 回答需要引用事实时，应至少有对应来源计数。"
                },
                {
                    "name": "fallback_used",
                    "type": "boolean",
                    "owner": "main_project",
                    "meaning": "主项目是否从 context pack 回退到 today-matches。",
                    "target": "长期应降低回退率，回退时回答更保守。"
                },
                {
                    "name": "stale_source_count",
                    "type": "integer",
                    "owner": "fb2",
                    "meaning": "赔率、比赛状态、订单或群观点中超过业务新鲜度窗口的数据数量。",
                    "target": "大于 0 时 context_pack 应说明新鲜度风险。"
                },
                {
                    "name": "permission_denied_count",
                    "type": "integer",
                    "owner": "fb2",
                    "meaning": "因当前用户权限不足而被裁剪的订单或平台数据数量。",
                    "target": "必须只记录数量或原因，不返回其他用户隐私明细。"
                },
                {
                    "name": "citation_coverage",
                    "type": "number",
                    "owner": "main_project",
                    "meaning": "AI 回复中可追溯到 match_id/order_id/message_id 的关键判断比例。",
                    "target": "涉及比赛预测和订单剖析时应逐步提高。"
                },
                {
                    "name": "external_tool_grounding",
                    "type": "object",
                    "owner": "main_project",
                    "meaning": "主项目记录每次 fb2 工具执行的 grounded/weak/unsafe 结果数量、source_id 覆盖和耗时。",
                    "target": "grounded_result_count 应逐步提高，unsafe_result_count 应保持为 0。"
                }
            ],
            "recommended_log_fields": [
                "app_id",
                "group_id",
                "external_group_id",
                "external_user_id_present",
                "context_pack_version",
                "generated_at",
                "context_pack_latency_ms",
                "context_chars",
                "source_counts",
                "fallback_used",
                "context_quality_warnings",
                "tool_readiness_status",
                "tool_execution_id",
                "grounded_result_count",
                "weak_result_count",
                "unsafe_result_count"
            ],
            "main_project_persistence": {
                "table": "external_app_tool_executions",
                "retains": [
                    "execution_id",
                    "app_id",
                    "main_group_id",
                    "external_group_id",
                    "main_user_id",
                    "external_user_id",
                    "context_audit_id",
                    "topic_hint",
                    "status",
                    "planned_count",
                    "ready_count",
                    "grounded_result_count",
                    "weak_result_count",
                    "unsafe_result_count",
                    "source_id_count",
                    "duration_ms",
                    "plan_json",
                    "audit_json"
                ],
                "purpose": "支撑后续 planner 评测、工具召回质量、fb2 数据质量和回答 grounding 覆盖率优化。"
            },
            "privacy_rules": [
                "日志不得记录 shared secret、完整用户票据明细或其他用户订单。",
                "普通群聊的平台订单只能记录匿名聚合指标。",
                "用户订单指标必须能区分 current_user_only 和 anonymous_aggregate。"
            ]
        })),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exposes_public_context_observability_guidance() {
        let guidance = public_context_observability_guidance("fb2").unwrap();
        assert_eq!(guidance["schema"], "fb2.context_observability.v1");
        assert!(guidance["recommended_metrics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|metric| metric["name"] == "external_tool_grounding"));
        assert_eq!(
            guidance["main_project_persistence"]["table"],
            "external_app_tool_executions"
        );
        assert!(guidance["privacy_rules"]
            .as_array()
            .unwrap()
            .iter()
            .any(|rule| rule.as_str().unwrap_or_default().contains("shared secret")));
        assert!(public_context_observability_guidance("unknown").is_none());
    }
}
