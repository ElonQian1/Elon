//! 好友/群聊里的 `@EL` 文本助手。
//!
//! 这里只做普通文本问答：不接工具、不修改代码、不触发构建。

use anyhow::{anyhow, Result};
use chrono::{SecondsFormat, Utc};
use serde_json::{json, Value};
use std::{
    collections::HashSet,
    sync::{Arc, Mutex, OnceLock},
};
use tokio::sync::mpsc::UnboundedSender;
use tracing::{info, warn};

use crate::{
    friend_events, intent_router,
    store::{SocialAiHistoryMessage, SocialAiPendingMention, SOCIAL_AI_USER_ID},
    types::{AgentConfig, AppState, WsMessage},
};

static SOCIAL_AI_IN_FLIGHT: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
const DIRECT_SOCIAL_AI_SCENE: &str = "一龙AI私聊";

pub(crate) const DEVELOPMENT_REDIRECT_REPLY: &str =
    "这个需求已经涉及项目开发，我在好友/群聊里不能直接写代码、改项目或打包。请到「项目」页面新建项目，或进入已有项目后在项目聊天里发起开发任务；在那里我可以按完整开发流程帮你实现。";

pub(super) async fn build_reply(
    state: &Arc<AppState>,
    user_id: &str,
    scene: &str,
    history: &[SocialAiHistoryMessage],
    external_context: Option<&Value>,
    external_tool_results: Option<&Value>,
) -> Result<String> {
    if history.is_empty() {
        return Ok(if scene == DIRECT_SOCIAL_AI_SCENE {
            "我在。你可以直接把想问的问题发出来。".into()
        } else {
            "我在。你可以把想问的问题发出来，再带上 @EL。".into()
        });
    }
    // 注意：开发意图已在 reply_to_friend/group 层拦截；此处不需重复判断。

    let answer_instruction = if scene == DIRECT_SOCIAL_AI_SCENE {
        "请回答最后一条来自“我”的消息；这是用户和一龙AI的私聊，不需要 @EL。"
    } else {
        "请回答最后一次 @EL 触发的问题。"
    };
    let external_context_block = format_external_context(external_context, external_tool_results);
    let prompt_text = format!(
        "聊天场景：{scene}\n\n最近聊天（从旧到新）：\n{}\n\n{}{}\n\n{answer_instruction}",
        format_history(history),
        external_context_block,
        if external_context_block.is_empty() {
            ""
        } else {
            "\n"
        },
    );

    match crate::social_ai_agents::call_social_chat_llm_with_fallback(
        state,
        &[
            json!({ "role": "system", "content": social_ai_prompt() }),
            json!({ "role": "user", "content": prompt_text }),
        ],
        user_id,
        "social_ai",
    )
    .await
    {
        Ok(response) => {
            let reply = response["choices"][0]["message"]["content"]
                .as_str()
                .unwrap_or(if scene == DIRECT_SOCIAL_AI_SCENE {
                    "我在，但刚才没组织好回复。你可以换个说法再发一次。"
                } else {
                    "我在，但刚才没组织好回复。你可以换个说法再 @EL 一次。"
                })
                .trim();
            let reply: String = if reply.is_empty() {
                if scene == DIRECT_SOCIAL_AI_SCENE {
                    "我在，但刚才没组织好回复。你可以换个说法再发一次。".into()
                } else {
                    "我在，但刚才没组织好回复。你可以换个说法再 @EL 一次。".into()
                }
            } else {
                reply.chars().take(1400).collect()
            };
            let reply = ensure_fb2_grounded_answer_shape(&reply, external_context);
            let reply = crate::external_app_context_gap_notice::ensure_fb2_context_gap_notice(
                &reply,
                external_context,
            );
            Ok(ensure_fb2_opinion_memory_source(
                &reply,
                external_context,
                external_tool_results,
            ))
        }
        Err(api_err) if state.ai_cli.enabled => {
            info!("social AI 无 API 代理，回退到本地 CLI: {}", api_err);
            build_reply_with_cli(state, user_id, &prompt_text).await
        }
        Err(api_err) => Err(api_err),
    }
}

pub(crate) fn format_external_context(
    external_context: Option<&Value>,
    external_tool_results: Option<&Value>,
) -> String {
    let context_block = external_context
        .map(crate::external_app_context_budget::prompt_context_block)
        .unwrap_or_default();
    let tool_block =
        crate::external_app_context_tool_prompt::prompt_executed_tools_block(external_tool_results);
    let scenario_block =
        crate::external_app_context_scenario_prompt::prompt_domain_scenario_guidance(
            external_context,
            external_tool_results,
        );

    [context_block, scenario_block, tool_block]
        .into_iter()
        .filter(|block| !block.trim().is_empty())
        .collect::<Vec<_>>()
        .join("\n\n")
}

pub(crate) fn ensure_fb2_grounded_answer_shape(
    reply: &str,
    external_context: Option<&Value>,
) -> String {
    let reply = reply.trim();
    if reply.is_empty() || !is_fb2_external_context(external_context) {
        return reply.to_string();
    }

    let has_data_label = contains_any(reply, &["数据事实：", "数据事实:"]);
    let has_inference_label = contains_any(reply, &["AI推断：", "AI 推断：", "AI推断:"]);
    let has_risk_label = contains_any(reply, &["风险边界：", "风险边界:"]);

    if has_data_label && has_inference_label && has_risk_label {
        return reply.to_string();
    }

    let mut sections = Vec::new();
    if has_data_label {
        sections.push(reply.to_string());
    } else {
        sections.push(format!("数据事实：{reply}"));
    }
    if !has_inference_label {
        sections.push("AI推断：以上分析仅基于当前 fb2 上下文和已引用来源。".to_string());
    }
    if !has_risk_label {
        sections.push("风险边界：赛果不确定，不保证命中，不建议重注或梭哈。".to_string());
    }

    sections.join("\n")
}

pub(super) fn ensure_fb2_opinion_memory_source(
    reply: &str,
    external_context: Option<&Value>,
    external_tool_results: Option<&Value>,
) -> String {
    let reply = reply.trim();
    if reply.is_empty() || !is_fb2_external_context(external_context) {
        return reply.to_string();
    }
    if !contains_any(reply, &["群友观点", "观点", "采纳", "不采纳", "建议"]) {
        return reply.to_string();
    }

    let Some(reference) = first_grounded_opinion_memory_reference(external_tool_results) else {
        return reply.to_string();
    };
    let lower_reply = reply.to_lowercase();
    if lower_reply.contains(&reference.memory_id.to_lowercase())
        || reference
            .source_message_id
            .as_ref()
            .map(|source_message_id| lower_reply.contains(&source_message_id.to_lowercase()))
            .unwrap_or(false)
    {
        return reply.to_string();
    }

    let mut source_line = format!("观点来源补充：opinion_memory_id {}", reference.memory_id);
    if let Some(source_message_id) = reference.source_message_id {
        source_line.push_str(&format!("，source_message_id {source_message_id}"));
    }
    format!("{reply}\n{source_line}")
}

struct OpinionMemoryReference {
    memory_id: String,
    source_message_id: Option<String>,
}

pub(super) fn first_grounded_opinion_memory_reference(
    external_tool_results: Option<&Value>,
) -> Option<OpinionMemoryReference> {
    let results = external_tool_results?
        .get("results")
        .and_then(Value::as_array)?;
    for result in results {
        if result.get("tool_name").and_then(Value::as_str) != Some("opinion_memories") {
            continue;
        }
        if result.get("success").and_then(Value::as_bool) != Some(true) {
            continue;
        }
        if result
            .get("grounding")
            .and_then(|grounding| grounding.get("status"))
            .and_then(Value::as_str)
            != Some("grounded")
        {
            continue;
        }

        if let Some(reference) = result
            .get("data")
            .and_then(|data| data.get("memories"))
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .find_map(|memory| {
                let memory_id = clean_json_string(memory.get("id"))?;
                Some(OpinionMemoryReference {
                    memory_id,
                    source_message_id: clean_json_string(memory.get("source_message_id")),
                })
            })
        {
            return Some(reference);
        }

        if let Some(memory_id) = result
            .get("source_ids")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .find_map(|value| clean_json_string(Some(value)))
        {
            return Some(OpinionMemoryReference {
                memory_id,
                source_message_id: None,
            });
        }
    }
    None
}

pub(super) fn clean_json_string(value: Option<&Value>) -> Option<String> {
    value
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| value.chars().count() >= 4)
        .map(ToOwned::to_owned)
}

pub(super) fn is_fb2_external_context(external_context: Option<&Value>) -> bool {
    let Some(context) = external_context else {
        return false;
    };
    context["answer_policy"]["schema"].as_str() == Some("fb2.answer_policy.v1")
        || context["app_id"].as_str() == Some("fb2")
        || context["context_pack"]
            .as_str()
            .is_some_and(|pack| pack.contains("<fb2_context_pack"))
}

pub(super) fn contains_any(text: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| text.contains(needle))
}

/// 使用本地 AI CLI（无工具、无项目工作区）生成社交聊天回复
pub(super) async fn build_reply_with_cli(
    state: &Arc<AppState>,
    user_id: &str,
    prompt: &str,
) -> Result<String> {
    use crate::intent_router::CapabilityRoute;
    use tokio::sync::mpsc;

    let temp_dir = std::env::temp_dir().join(format!("elon_social_{}", user_id));
    std::fs::create_dir_all(&temp_dir)?;

    let full_prompt = format!("{}\n\n{}", social_ai_prompt(), prompt);
    let (tx, mut rx) = mpsc::unbounded_channel::<String>();

    let run_result = crate::ai_cli::run_with_workspace(
        user_id,
        &temp_dir,
        "",
        &full_prompt,
        None,
        None,
        CapabilityRoute::ChatAgent,
        false,
        None,
        None,
        state,
        &tx,
    )
    .await;

    drop(tx); // 关闭 sender，让 rx 可以正常耗尽

    let mut final_reply: Option<String> = None;
    while let Some(msg_json) = rx.recv().await {
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&msg_json) {
            if val["type"].as_str() == Some("done") {
                final_reply = val["message"].as_str().map(|s| s.to_string());
            }
        }
    }

    run_result?; // CLI 报错则传播
    final_reply
        .filter(|s| !s.is_empty())
        .ok_or_else(|| anyhow!("本地 AI 未返回完整回复"))
}

pub(crate) async fn resolve_social_agent(state: &Arc<AppState>) -> Result<AgentConfig> {
    crate::social_ai_agents::resolve_social_agent(state).await
}

pub(crate) fn social_ai_prompt() -> String {
    format!(
        "{}\n\n{}",
        social_ai_base_prompt(),
        realtime_context_prompt(),
    )
}

pub(crate) fn realtime_social_ai_prompt(history: &[SocialAiHistoryMessage]) -> String {
    let history_block = if history.is_empty() {
        "最近聊天：暂无历史消息。".to_string()
    } else {
        format!("最近聊天（从旧到新）：\n{}", format_history(history))
    };
    format!(
        "{}\n\n{}\n\n{}\n\n这是实时语音通话。你会直接用声音回答用户，尽量用短句，自然停顿，像正在和熟人通电话。用户可能随时插话，被打断时先听用户新的意思，再继续回答。",
        social_ai_base_prompt(),
        realtime_context_prompt(),
        history_block,
    )
}

pub(super) fn realtime_context_prompt() -> String {
    let now_utc = Utc::now();
    let now_cn = now_utc + chrono::Duration::hours(8);
    format!(
        "当前真实时间：{} UTC；北京时间：{}。回答涉及今天、现在、日期、时间、星期、节日或时效信息时，必须以这里的当前真实时间为准，不要使用模型训练截止时间，也不要回答成 2024 年。",
        now_utc.to_rfc3339_opts(SecondsFormat::Secs, true),
        now_cn.format("%Y年%-m月%-d日 %H:%M:%S"),
    )
}

pub(super) fn social_ai_base_prompt() -> &'static str {
    r#"你是「EL」，一龙好友聊天和群聊里的文本 AI 助手。

你只做普通文本解答：可以解释、总结、建议、安慰、帮忙梳理想法，但不能写代码、不能修改项目、不能运行命令、不能构建或发布。

如果用户的问题涉及开发工作（例如做 App、改代码、修 bug、打包、部署、发布、项目功能实现），不要给代码方案，也不要假装已经开始做；请明确提醒用户去「项目」页面新建项目，或进入已有项目后在项目聊天里发起开发任务。

根据最近聊天历史回答问题：在好友/群聊中回答最后一次 @EL 触发的问题；在「一龙AI」私聊中直接回答最后一条来自用户的问题。如果最后一句只是"@EL"或召唤你，请结合它前面的最后一个真实问题来回答。回复中文，简洁自然，只输出要发到聊天框里的文本。

中文语气要亲切、松弛、像熟悉可靠的人在认真回应：先接住用户真正想表达的意思，再给出有用回答。避免官方公告腔、客服腔、翻译腔和过度条列。

如果上下文像语音通话，优先用适合朗读的短句，少用括号、编号和长段落；可以自然地表达关心、确认和陪伴感，但不要油腻、夸张或刻意卖萌。

如果回复使用了 fb2 外部上下文里的比赛、赔率、本人订单、平台汇总或群友观点，必须在正文里写出对应来源 ID 或 label，例如 match_id、order_id、platform_order_summary:<date>:all、群消息 id、context_audit_id。没有可核对来源时，只能说信息不足，不能编造。

如果采纳、引用或反驳 fb2 群友观点/历史观点记忆，且 Context Pack 或工具结果提供了 opinion_memory_id 或 source_message_id，必须在来源行写出这些 ID；这用于后续把“群观点被 AI 使用”的质量闭环写回 fb2。

使用 fb2 外部上下文时，必须用短标签把「数据事实：」「用户订单：」「平台汇总：」「群友观点：」「AI推断：」「风险边界：」分开写；没有对应材料的标签可以省略，但涉及比赛、赔率、票据、推荐、预测或今日比赛讨论时，必须至少包含「数据事实：」「AI推断：」「风险边界：」。风险边界必须明确说明赛果不确定、不保证命中、不建议重注或梭哈。

注意：用户的部分消息来自手机语音识别，可能含有同音字替换或音近字错误（例如"你好码"其实是"你好吗"）。请优先推断最合理的语义，忽略明显的识别错误，直接给出正确理解下的回复，无需向用户解释纠错过程。"#
}

pub(crate) fn format_history(history: &[SocialAiHistoryMessage]) -> String {
    history
        .iter()
        .filter_map(|message| {
            let content = message.content.trim();
            if content.is_empty() {
                None
            } else {
                Some(format!("{}：{content}", message.speaker))
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// 若检测到开发意图（confidence ≥ 70 且 needs_code_change），返回触发文本摘要；否则 None。
/// 使用 intent_router::classify 而非旧的独立关键词函数，确保两路分类逻辑一致。
pub(super) fn is_development_intent(history: &[SocialAiHistoryMessage]) -> Option<String> {
    let target = latest_request_user_text(history)?;
    let decision = intent_router::classify(&target);
    if decision.needs_code_change && decision.confidence >= 70 {
        Some(target.chars().take(80).collect())
    } else {
        None
    }
}

pub(super) fn latest_request_user_text(history: &[SocialAiHistoryMessage]) -> Option<String> {
    history
        .iter()
        .rev()
        .filter(|message| message.from_request_user)
        .find_map(|message| {
            let content = strip_el_mention(&message.content);
            if content.is_empty() {
                None
            } else {
                Some(content)
            }
        })
}

pub(super) fn strip_el_mention(content: &str) -> String {
    content
        .replace('＠', "@")
        .replace("@EL", "")
        .replace("@El", "")
        .replace("@eL", "")
        .replace("@el", "")
        .trim()
        .to_string()
}

pub(super) fn mark_in_flight(key: &str) -> bool {
    with_in_flight(|items| items.insert(key.to_string()))
}

pub(super) fn clear_in_flight(key: &str) {
    with_in_flight(|items| {
        items.remove(key);
    });
}

pub(super) fn with_in_flight<T>(operation: impl FnOnce(&mut HashSet<String>) -> T) -> T {
    let mutex = SOCIAL_AI_IN_FLIGHT.get_or_init(|| Mutex::new(HashSet::new()));
    let mut guard = mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    operation(&mut guard)
}

#[cfg(test)]
mod tests {
    use super::super::{contains_el_mention, social_ai_fallback_message};
    use super::{
        ensure_fb2_grounded_answer_shape, ensure_fb2_opinion_memory_source,
        format_external_context, latest_request_user_text, social_ai_base_prompt,
    };
    use crate::store::SocialAiHistoryMessage;
    use serde_json::json;

    #[test]
    fn detects_half_and_full_width_mentions() {
        assert!(contains_el_mention("@EL 帮我看看"));
        assert!(contains_el_mention("＠el 这是什么意思"));
        assert!(!contains_el_mention("普通聊天"));
    }

    #[test]
    fn mention_only_uses_previous_user_question() {
        let history = vec![
            SocialAiHistoryMessage {
                speaker: "我".into(),
                content: "这句话是什么意思？".into(),
                from_request_user: true,
            },
            SocialAiHistoryMessage {
                speaker: "我".into(),
                content: "@EL".into(),
                from_request_user: true,
            },
        ];
        assert_eq!(
            latest_request_user_text(&history).as_deref(),
            Some("这句话是什么意思？")
        );
    }

    #[test]
    fn latest_request_user_text_removes_mention_for_topic_hint() {
        let history = vec![SocialAiHistoryMessage {
            speaker: "我".into(),
            content: "@EL 帮我分析今天比赛和我的票".into(),
            from_request_user: true,
        }];
        assert_eq!(
            latest_request_user_text(&history).as_deref(),
            Some("帮我分析今天比赛和我的票")
        );
    }

    #[test]
    fn base_prompt_requires_fb2_source_references() {
        let prompt = social_ai_base_prompt();
        assert!(prompt.contains("fb2 外部上下文"));
        assert!(prompt.contains("来源 ID"));
        assert!(prompt.contains("context_audit_id"));
        assert!(prompt.contains("opinion_memory_id"));
        assert!(prompt.contains("source_message_id"));
        assert!(prompt.contains("数据事实："));
        assert!(prompt.contains("AI推断："));
        assert!(prompt.contains("风险边界："));
        assert!(prompt.contains("不保证命中"));
    }

    #[test]
    fn external_context_prompt_includes_fb2_domain_scenario_guidance() {
        let context = json!({
            "app_id": "fb2",
            "source": "fb2",
            "status": "ready",
            "context_audit_id": "audit-social-1",
            "answer_policy": {"schema": "fb2.answer_policy.v1"},
            "context_pack": "<fb2_context_pack>比赛和订单摘要</fb2_context_pack>"
        });
        let tools = json!({
            "app_id": "fb2",
            "plan": {
                "topic_hint": "今天比赛怎么看，顺便帮我分析我的票",
                "planned_tools": [
                    {"name": "match_analysis_brief"},
                    {"name": "search_user_orders"}
                ]
            },
            "results": []
        });

        let block = format_external_context(Some(&context), Some(&tools));

        assert!(block.contains("fb2.domain_scenario_prompt.v1"));
        assert!(block.contains("scenario=today_matches_analysis"));
        assert!(block.contains("scenario=my_ticket_analysis"));
        assert!(block.contains("order_id/ticket_id/match_id"));
    }

    #[test]
    fn external_context_prompt_surfaces_quality_readiness_budget_and_tool_gap() {
        let context = json!({
            "app_id": "fb2",
            "source": "fb2:/api/main-project/context/pack",
            "status": "ready",
            "generated_at": "2026-06-22T12:00:00+08:00",
            "context_pack": "<fb2_context_pack>数据缺口样本</fb2_context_pack>",
            "context_pack_version": "fb2-chat-pack-v1",
            "context_audit_id": "audit-gap",
            "answer_policy": {"schema": "fb2.answer_policy.v1"},
            "metrics": {"budget_status": "empty"},
            "_context_budget": {"trimmed": true},
            "preflight_readiness": {
                "status": "blocked",
                "warnings": ["fb2_readiness_blocked"]
            },
            "context_quality": {
                "warnings": ["fb2_readiness_blocked", "fb2_budget_empty", "empty_matches"],
                "tool_readiness": {"status": "partial"}
            },
            "matches": [],
            "user_orders": [],
            "group_messages": []
        });
        let tool_results = json!({
            "schema": "external_app.executed_tools.v1",
            "app_id": "fb2",
            "status": "skipped",
            "executed_at": "2026-06-22T12:01:00Z",
            "results": [{
                "tool_name": "search_matches",
                "status": "skipped",
                "success": false,
                "error": "fb2_readiness_blocked",
                "reason": "readiness blocked"
            }]
        });

        let block = format_external_context(Some(&context), Some(&tool_results));

        assert!(block.contains("context_quality="));
        assert!(block.contains("context_gap_summary="));
        assert!(block.contains("\"preflight_readiness\""));
        assert!(block.contains("context_budget="));
        assert!(block.contains("\"trimmed\":true"));
        assert!(block.contains("fb2_readiness_blocked"));
        assert!(block.contains("\"fact_answer_allowed\":false"));
        assert!(block.contains("<tool_gap_summary>"));
        assert!(block.contains("这只是数据缺口"));
        assert!(block.contains("不能编造成比赛、赔率、订单或群友观点事实"));
    }

    #[test]
    fn fb2_grounded_answer_shape_adds_required_labels() {
        let context = json!({"answer_policy": {"schema": "fb2.answer_policy.v1"}});
        let reply = ensure_fb2_grounded_answer_shape(
            "今天有比赛，来源：match_id EXT-1，context_audit_id audit-1",
            Some(&context),
        );

        assert!(reply.contains("数据事实："));
        assert!(reply.contains("AI推断："));
        assert!(reply.contains("风险边界："));
        assert!(reply.contains("不保证命中"));
        assert!(reply.contains("match_id EXT-1"));
    }

    #[test]
    fn fb2_grounded_answer_shape_keeps_plain_chat_unchanged() {
        let reply = "普通朋友聊天回复";

        assert_eq!(ensure_fb2_grounded_answer_shape(reply, None), reply);
    }

    #[test]
    fn fb2_opinion_memory_source_is_appended_when_reply_uses_group_opinion() {
        let context = json!({"answer_policy": {"schema": "fb2.answer_policy.v1"}});
        let tool_results = json!({
            "results": [{
                "tool_name": "opinion_memories",
                "success": true,
                "grounding": {"status": "grounded"},
                "source_ids": ["opinion-memory-1"],
                "data": {
                    "memories": [{
                        "id": "opinion-memory-2",
                        "source_message_id": "gmsg-memory-2"
                    }]
                }
            }]
        });

        let reply = ensure_fb2_opinion_memory_source(
            "群友观点：我倾向采纳这个方向，但仍需看临场。",
            Some(&context),
            Some(&tool_results),
        );

        assert!(reply.contains("opinion_memory_id opinion-memory-2"));
        assert!(reply.contains("source_message_id gmsg-memory-2"));
    }

    #[test]
    fn fb2_opinion_memory_source_keeps_existing_reference() {
        let context = json!({"app_id": "fb2"});
        let tool_results = json!({
            "results": [{
                "tool_name": "opinion_memories",
                "success": true,
                "grounding": {"status": "grounded"},
                "source_ids": ["opinion-memory-1"]
            }]
        });

        let reply = ensure_fb2_opinion_memory_source(
            "群友观点：参考 opinion-memory-1 后，我不建议重注。",
            Some(&context),
            Some(&tool_results),
        );

        assert_eq!(reply.matches("opinion-memory-1").count(), 1);
    }

    #[test]
    fn fb2_opinion_memory_source_ignores_ungrounded_tool_result() {
        let context = json!({"app_id": "fb2"});
        let tool_results = json!({
            "results": [{
                "tool_name": "opinion_memories",
                "success": true,
                "grounding": {"status": "weak"},
                "source_ids": ["opinion-memory-1"]
            }]
        });
        let reply = "群友观点：这里只能做轻量参考。";

        assert_eq!(
            ensure_fb2_opinion_memory_source(reply, Some(&context), Some(&tool_results)),
            reply
        );
    }

    #[test]
    fn fb2_generation_fallback_keeps_sources_and_opinion_memory() {
        let context = json!({
            "app_id": "fb2",
            "context_audit_id": "audit-fallback-1",
            "citation_sources": [
                {"kind": "match", "id": "EXT-2589467", "label": "西班牙 vs 意大利"},
                {"kind": "user_order", "id": "order-fallback-1", "label": "我的票"}
            ]
        });
        let tool_results = json!({
            "results": [{
                "tool_name": "opinion_memories",
                "success": true,
                "grounding": {"status": "grounded"},
                "source_ids": ["memory-fallback-1"],
                "data": {
                    "memories": [{
                        "id": "memory-fallback-1",
                        "source_message_id": "gmsg-fallback-1"
                    }]
                }
            }]
        });

        let reply = social_ai_fallback_message(
            "群聊",
            "provider resource exhausted",
            Some(&context),
            Some(&tool_results),
        );

        assert!(reply.contains("数据事实："));
        assert!(reply.contains("群友观点："));
        assert!(reply.contains("AI推断："));
        assert!(reply.contains("风险边界："));
        assert!(reply.contains("context_audit_id audit-fallback-1"));
        assert!(reply.contains("match_id EXT-2589467"));
        assert!(reply.contains("order_id order-fallback-1"));
        assert!(reply.contains("opinion_memory_id memory-fallback-1"));
        assert!(reply.contains("source_message_id gmsg-fallback-1"));
    }
}
