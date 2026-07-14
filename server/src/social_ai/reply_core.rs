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
use tracing::info;

use crate::{
    intent_router,
    store::SocialAiHistoryMessage,
    types::{AgentConfig, AppState},
};

static SOCIAL_AI_IN_FLIGHT: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
const DIRECT_SOCIAL_AI_SCENE: &str = "一龙AI私聊";

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

fn first_grounded_opinion_memory_reference(
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
#[path = "reply_core_tests.rs"]
mod reply_core_tests;
