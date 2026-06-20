//! Contract normalization for external app business context.

use serde_json::{json, Value};

use crate::external_app_context_tools::{tool_contract_quality_warning, tool_contract_readiness};

pub(crate) fn public_context_pack_example(app_id: &str) -> Option<Value> {
    match app_id {
        "fb2" => Some(json!({
            "endpoint": "GET /api/main-project/context/pack",
            "auth_header": "X-FB2-AI-CENTER-TOKEN",
            "query_example": {
                "group_id": "official",
                "external_user_id": "fb2-user-id",
                "topic_hint": "总结预测今天的比赛",
                "limit": 30,
                "discussion_limit": 80,
                "order_limit": 20,
                "include_platform_orders": false
            },
            "response_shape": {
                "success": true,
                "data": fb2_context_pack_example_data()
            },
            "minimum_required_fields": [
                "context_pack_version",
                "generated_at",
                "context_pack",
                "matches",
                "user_orders",
                "group_messages",
                "tool_contract"
            ]
        })),
        _ => None,
    }
}

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
                "tool_readiness_status"
            ],
            "privacy_rules": [
                "日志不得记录 shared secret、完整用户票据明细或其他用户订单。",
                "普通群聊的平台订单只能记录匿名聚合指标。",
                "用户订单指标必须能区分 current_user_only 和 anonymous_aggregate。"
            ]
        })),
        _ => None,
    }
}

pub(crate) fn public_usage_policy_guidance(app_id: &str) -> Option<Value> {
    match app_id {
        "fb2" => Some(json!({
            "app_id": "fb2",
            "schema": "fb2.usage_policy.v1",
            "free_channels": [
                {
                    "name": "android_system_asr",
                    "metering": "free",
                    "reason": "手机系统本地语音识别不消耗主项目算力。",
                    "billing_gate": "never"
                },
                {
                    "name": "cloud_asr_fallback",
                    "metering": "free",
                    "endpoint": "/api/voice/asr",
                    "reason": "云端 ASR 是聊天基础输入通道，不因 AI 额度为 0 拒绝。",
                    "billing_gate": "never"
                },
                {
                    "name": "tts",
                    "metering": "free",
                    "endpoint": "/api/voice/tts",
                    "reason": "TTS 是聊天基础输出通道，不按模型 token 计费。",
                    "billing_gate": "never"
                },
                {
                    "name": "external_context_fetch",
                    "metering": "free",
                    "endpoint": "/api/main-project/context/pack",
                    "reason": "上下文拉取是 AI 回复前的数据准备，不直接消耗用户 token。",
                    "billing_gate": "auth_and_limits_only"
                }
            ],
            "billable_channels": [
                {
                    "name": "ai_reply_generation",
                    "metering": "token_or_model_usage",
                    "reason": "只有模型生成回复内容才消耗 AI 额度。",
                    "examples": [
                        "群聊 @AI 后生成回答",
                        "AI 助手回答用户问题",
                        "赛事分析生成文本",
                        "用户订单/票据风险剖析生成文本"
                    ],
                    "billing_gate": "before_model_call"
                }
            ],
            "integration_rules": [
                "fb2 不应在 ASR/TTS 按钮或上下文拉取前检查 AI token 余额。",
                "余额不足只能阻断 AI 生成回复，不能阻断录音转文字、文字转语音或 context pack 拉取。",
                "ASR/TTS/context fetch 仍必须保留鉴权、文件大小、时长、频率和安全限制。",
                "免费试用额度应配置在 AI 回复/模型调用层，不配置在 ASR/TTS 层。"
            ],
            "default_usage_policy": default_usage_policy()
        })),
        _ => None,
    }
}

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

fn fb2_context_pack_example_data() -> Value {
    json!({
        "context_pack_version": "fb2-chat-pack-v1",
        "generated_at": "2026-06-20T12:00:00+08:00",
        "context_pack": "<fb2_context_pack version=\"1.0\" project=\"fb2\">\n\n## 使用边界\n\n- 只作为比赛讨论和订单剖析参考。\n- 不承诺命中，不诱导投注。\n- 必须区分数据事实、群友观点和 AI 推断。\n\n## 今日/近期比赛与赔率\n\n- match_id=m-001 联赛示例 主队 vs 客队，赔率更新时间 2026-06-20T11:58:00+08:00。\n\n## 当前用户订单/票据\n\n- order_id=o-001 仅当前用户可见，用于风险拆解。\n\n## 群讨论观点\n\n- message_id=msg-001 群友观点示例。\n\n## 平台/店铺订单摘要\n\n- 仅匿名聚合，不暴露其他用户订单明细。\n\n</fb2_context_pack>",
        "matches": [
            {
                "id": "m-001",
                "lottery_type": "JingCai",
                "league": "示例联赛",
                "home_team": "主队",
                "away_team": "客队",
                "match_time": "2026-06-20T19:30:00+08:00",
                "status": "scheduled",
                "odds": {
                    "updated_at": "2026-06-20T11:58:00+08:00",
                    "win": 1.95,
                    "draw": 3.20,
                    "lose": 3.60
                }
            }
        ],
        "user_orders": [
            {
                "order_id": "o-001",
                "ticket_id": "t-001",
                "match_ids": ["m-001"],
                "visibility": "current_user_only",
                "summary": "当前用户票据摘要"
            }
        ],
        "group_messages": [
            {
                "message_id": "msg-001",
                "match_id": "m-001",
                "author_role": "group_member",
                "opinion": "群友观点摘要",
                "created_at": "2026-06-20T11:50:00+08:00"
            }
        ],
        "platform_order_summary": {
            "visibility": "anonymous_aggregate",
            "note": "普通群聊只返回匿名聚合数据"
        },
        "tool_contract": {
            "schema": "fb2.tools.v1",
            "tools": [
                {
                    "name": "get_match_detail",
                    "description": "按 match id 查询比赛、赔率、伤停和更新时间",
                    "input_schema": {
                        "type": "object",
                        "required": ["match_id"],
                        "properties": {
                            "match_id": {"type": "string"}
                        }
                    },
                    "permission": "group_context",
                    "when_to_use": "用户追问某一场比赛细节或 context_pack 被截断时"
                }
            ]
        },
        "usage_policy": default_usage_policy()
    })
}

pub(crate) fn fb2_pack_context(app_id: &str, external_group_id: &str, data: Value) -> Value {
    let mut context = json!({
        "app_id": app_id,
        "group": external_group_id,
        "status": "ready",
        "source": "fb2:/api/main-project/context/pack",
        "generated_at": data.get("generated_at"),
        "context_pack_version": data.get("context_pack_version").or_else(|| data.get("version")),
        "context_pack": data.get("context_pack"),
        "matches": data.get("matches"),
        "user_orders": data.get("user_orders"),
        "group_messages": data.get("group_messages"),
        "platform_order_summary": data.get("platform_order_summary"),
        "tool_contract": data.get("tool_contract"),
        "usage_policy": data.get("usage_policy").cloned().unwrap_or_else(default_usage_policy)
    });
    context["context_quality"] = context_quality(&context, true);
    context
}

pub(crate) fn fb2_match_context(app_id: &str, external_group_id: &str, data: Value) -> Value {
    let matches = data["matches"]
        .as_array()
        .map(|matches| matches.iter().map(slim_match).collect::<Vec<_>>())
        .unwrap_or_default();
    let mut context = json!({
        "app_id": app_id,
        "group": external_group_id,
        "status": "ready",
        "source": "fb2:/api/main-project/context/today-matches",
        "generated_at": data.get("generated_at"),
        "count": matches.len(),
        "matches": matches,
        "usage_policy": default_usage_policy()
    });
    context["context_quality"] = context_quality(&context, false);
    context
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
        }
    ])
}

fn context_quality(context: &Value, expects_context_pack: bool) -> Value {
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
    }

    json!({
        "warnings": warnings,
        "requires_source_citations": true,
        "requires_freshness_notice": warnings.contains(&"missing_generated_at"),
        "schema": if expects_context_pack { "fb2.context_pack.v1" } else { "fb2.today_matches.v1" },
        "tool_readiness": if expects_context_pack { tool_contract_readiness(context) } else { json!({"status": "not_applicable"}) }
    })
}

fn slim_match(raw: &Value) -> Value {
    json!({
        "id": raw.get("id"),
        "lottery_type": raw.get("lottery_type"),
        "league": raw.get("league"),
        "home_team": raw.get("home_team"),
        "away_team": raw.get("away_team"),
        "match_time": raw.get("match_time"),
        "status": raw.get("status"),
        "odds": raw.get("odds"),
    })
}

fn has_prompt_text(value: Option<&Value>) -> bool {
    value
        .and_then(Value::as_str)
        .map(|text| !text.trim().is_empty())
        .unwrap_or(false)
}

fn default_usage_policy() -> Value {
    json!({
        "asr_free": true,
        "tts_free": true,
        "context_fetch_free": true,
        "ai_reply_billable": true,
        "no_guaranteed_win": true,
        "no_betting_commitment": true,
        "explain_uncertainty": true
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pack_context_reports_missing_contract_fields() {
        let context = fb2_pack_context("fb2", "official", json!({ "matches": [] }));

        let warnings = context["context_quality"]["warnings"].as_array().unwrap();
        assert!(warnings.contains(&json!("missing_context_pack")));
        assert!(warnings.contains(&json!("missing_context_pack_version")));
        assert!(warnings.contains(&json!("missing_generated_at")));
        assert!(warnings.contains(&json!("missing_tool_contract")));
        assert_eq!(
            context["context_quality"]["tool_readiness"]["status"],
            "missing"
        );
        assert_eq!(context["context_quality"]["schema"], "fb2.context_pack.v1");
    }

    #[test]
    fn match_context_slims_match_payload_and_marks_schema() {
        let context = fb2_match_context(
            "fb2",
            "official",
            json!({
                "generated_at": "2026-06-20T16:00:00+08:00",
                "matches": [{
                    "id": "m1",
                    "league": "J1",
                    "home_team": "A",
                    "away_team": "B",
                    "internal_field": "hidden"
                }]
            }),
        );

        assert_eq!(context["context_quality"]["schema"], "fb2.today_matches.v1");
        assert!(context["matches"][0].get("internal_field").is_none());
        assert_eq!(context["count"], 1);
    }

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
    fn exposes_public_context_pack_example() {
        let example = public_context_pack_example("fb2").unwrap();
        assert_eq!(example["auth_header"], "X-FB2-AI-CENTER-TOKEN");
        assert!(example["minimum_required_fields"]
            .as_array()
            .unwrap()
            .contains(&json!("context_pack")));
        assert_eq!(
            example["response_shape"]["data"]["usage_policy"]["ai_reply_billable"],
            true
        );
        assert!(public_context_pack_example("unknown").is_none());
    }

    #[test]
    fn exposes_public_context_observability_guidance() {
        let guidance = public_context_observability_guidance("fb2").unwrap();
        assert_eq!(guidance["schema"], "fb2.context_observability.v1");
        assert!(guidance["recommended_metrics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|metric| metric["name"] == "citation_coverage"));
        assert!(guidance["privacy_rules"]
            .as_array()
            .unwrap()
            .iter()
            .any(|rule| rule.as_str().unwrap_or_default().contains("shared secret")));
        assert!(public_context_observability_guidance("unknown").is_none());
    }

    #[test]
    fn exposes_public_usage_policy_guidance() {
        let guidance = public_usage_policy_guidance("fb2").unwrap();
        assert_eq!(guidance["schema"], "fb2.usage_policy.v1");
        assert!(guidance["free_channels"]
            .as_array()
            .unwrap()
            .iter()
            .any(|channel| channel["name"] == "cloud_asr_fallback"));
        assert_eq!(guidance["default_usage_policy"]["ai_reply_billable"], true);
        assert!(public_usage_policy_guidance("unknown").is_none());
    }
}
