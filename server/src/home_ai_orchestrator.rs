//! 首页总 AI 的通用工具编排层。
//!
//! 首页不应该为每一种自然语言问法写一条路由规则。这里把“是否需要实时信息”
//! 交给模型判断，只向模型暴露首页允许使用的只读工具；模型拿到工具结果后再生成
//! 最终回答。项目文件和命令工具不会出现在这个目录中。

use anyhow::{anyhow, Result};
use serde_json::{json, Value};
use std::sync::Arc;

use crate::{
    agent_fallback::call_tool_llm_with_definitions,
    agent_tool_calls::extract_tool_calls,
    home_ai_search, home_ai_weather,
    store::ConversationMessage,
    types::{AgentConfig, AppState},
};

const HOME_AI_TOOL_NOTE: &str = r#"=== 首页总 AI 工具使用规则 ===
你是首页总 AI，负责回答普通知识、分析问题和帮助用户获取信息。
你可以按需使用只读工具：
- 用户需要最新、实时、新闻、价格、地点或外部资料时，调用 web_search。
- 用户询问天气、温度、下雨、降雨时段、是否需要带伞等时，调用 weather。
- 普通知识、解释、写作、翻译、方案建议等直接回答，不要为了调用工具而调用工具。
- 工具调用结果是事实依据；不要编造工具没有返回的数据。
- 如果天气问题缺少城市，先自然地询问城市，不要猜测用户位置。
- 不要向用户展示工具调用 JSON 或内部路由过程。"#;

pub(crate) struct HomeAiAnswer {
    pub(crate) reply: String,
    pub(crate) agent_name: String,
    pub(crate) model: String,
    pub(crate) used_fallback: bool,
    pub(crate) tool_used: Option<String>,
    pub(crate) sources: Vec<Value>,
}

struct ToolExecution {
    content: String,
    tool_used: String,
    sources: Vec<Value>,
}

pub(crate) fn tool_definitions() -> Value {
    json!([
        {
            "type": "function",
            "function": {
                "name": "web_search",
                "description": "搜索互联网上的公开资料，用于最新新闻、实时信息、价格、人物、地点和需要外部资料的问题。",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "query": {
                            "type": "string",
                            "description": "要搜索的完整问题或关键词"
                        }
                    },
                    "required": ["query"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "weather",
                "description": "查询指定城市或地区的天气。可以查询今天、明天、后天的概况，也可以查询小时级降雨时段。",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "location": {
                            "type": "string",
                            "description": "城市或地区；如果上下文已有明确地点，可以复用上下文"
                        },
                        "day_offset": {
                            "type": "integer",
                            "enum": [0, 1, 2],
                            "description": "今天为 0，明天为 1，后天为 2"
                        },
                        "detail": {
                            "type": "string",
                            "enum": ["summary", "hourly_rain"],
                            "description": "普通天气概况使用 summary；询问几点下雨或降雨时段使用 hourly_rain"
                        }
                    },
                    "required": ["location"]
                }
            }
        }
    ])
}

pub(crate) async fn run(
    state: &Arc<AppState>,
    preferred: &AgentConfig,
    allow_fallback: bool,
    messages: &[Value],
    user_id: &str,
    history: &[ConversationMessage],
) -> Result<HomeAiAnswer> {
    let mut messages = with_tool_note(messages);
    let tools = tool_definitions();
    let mut used_tool = None;
    let mut sources = Vec::new();
    let mut active_agent = preferred.clone();
    let mut used_fallback = false;
    let latest_user_content = messages
        .iter()
        .rev()
        .find(|message| message["role"].as_str() == Some("user"))
        .and_then(|message| message["content"].as_str())
        .unwrap_or("")
        .to_string();

    for _ in 0..4 {
        let (response, used_agent, fell_back) = call_tool_llm_with_definitions(
            state,
            &active_agent,
            allow_fallback,
            &messages,
            user_id,
            "home_ai_tool",
            &tools,
        )
        .await?;
        active_agent = used_agent;
        used_fallback |= fell_back;

        let choice = &response["choices"][0];
        let assistant_message = choice["message"].clone();
        let finish_reason = choice["finish_reason"].as_str().unwrap_or("");
        let tool_calls = extract_tool_calls(&assistant_message);
        messages.push(assistant_message.clone());

        if tool_calls.is_empty() {
            let reply = assistant_message["content"]
                .as_str()
                .unwrap_or("")
                .trim()
                .to_string();
            if reply.is_empty() {
                return Err(anyhow!(
                    "首页总 AI 没有生成可读回答（finish_reason={finish_reason}）"
                ));
            }
            return Ok(HomeAiAnswer {
                reply,
                agent_name: active_agent.name,
                model: active_agent.model,
                used_fallback,
                tool_used: used_tool,
                sources,
            });
        }

        for tool_call in tool_calls {
            let execution = execute_tool(
                state,
                &tool_call.name,
                &tool_call.args,
                Some(&latest_user_content),
                history,
            )
            .await?;
            used_tool = Some(execution.tool_used.clone());
            sources.extend(execution.sources);
            if tool_call.legacy_function_call {
                messages.push(json!({
                    "role": "function",
                    "name": tool_call.name,
                    "content": execution.content,
                }));
            } else {
                messages.push(json!({
                    "role": "tool",
                    "tool_call_id": tool_call.id,
                    "content": execution.content,
                }));
            }
        }
    }

    Err(anyhow!("首页总 AI 工具调用次数过多，请换一种说法重试"))
}

async fn execute_tool(
    state: &Arc<AppState>,
    name: &str,
    args: &Value,
    latest_content: Option<&str>,
    history: &[ConversationMessage],
) -> Result<ToolExecution> {
    match name {
        "web_search" => {
            let query = args["query"]
                .as_str()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .unwrap_or(latest_content.unwrap_or(""));
            let Some(result) = home_ai_search::search(state, query).await else {
                return Ok(ToolExecution {
                    content: json!({
                        "ok": false,
                        "message": "联网搜索暂时没有返回可用资料，请明确说明无法核实，不要编造最新事实。"
                    })
                    .to_string(),
                    tool_used: "web_search".to_string(),
                    sources: Vec::new(),
                });
            };
            let sources = result
                .sources
                .iter()
                .map(|source| json!({ "title": source.title, "url": source.url }))
                .collect::<Vec<_>>();
            Ok(ToolExecution {
                content: json!({
                    "ok": true,
                    "query": result.query,
                    "context": result.context,
                    "sources": sources.clone(),
                })
                .to_string(),
                tool_used: "web_search".to_string(),
                sources,
            })
        }
        "weather" => {
            let query = latest_content.unwrap_or("");
            let location = args["location"]
                .as_str()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string)
                .or_else(|| home_ai_weather::resolve_location(query, history));
            let Some(location) = location else {
                return Ok(ToolExecution {
                    content: json!({
                        "ok": false,
                        "needs_location": true,
                        "message": home_ai_weather::missing_location_reply()
                    })
                    .to_string(),
                    tool_used: "weather_location_required".to_string(),
                    sources: Vec::new(),
                });
            };
            let day_offset = args["day_offset"]
                .as_u64()
                .map(|value| value.min(2) as usize)
                .unwrap_or_else(|| home_ai_weather::day_offset(query));
            let hourly_detail = args["detail"].as_str() == Some("hourly_rain")
                || home_ai_weather::is_hourly_weather_request(query);
            let lookup = home_ai_weather::lookup(state, &location, day_offset, hourly_detail).await;
            match lookup {
                home_ai_weather::WeatherLookup::Answer(answer) => Ok(ToolExecution {
                    content: json!({
                        "ok": true,
                        "answer": answer.reply,
                        "source": { "title": answer.source_title.clone(), "url": answer.source_url.clone() }
                    })
                    .to_string(),
                    tool_used: "weather".to_string(),
                    sources: vec![json!({
                        "title": answer.source_title,
                        "url": answer.source_url
                    })],
                }),
                home_ai_weather::WeatherLookup::NotFound { location } => Ok(ToolExecution {
                    content: json!({ "ok": false, "message": home_ai_weather::not_found_reply(&location) })
                        .to_string(),
                    tool_used: "weather_location_required".to_string(),
                    sources: Vec::new(),
                }),
                home_ai_weather::WeatherLookup::Unavailable { location } => Ok(ToolExecution {
                    content: json!({ "ok": false, "message": home_ai_weather::unavailable_reply(&location) })
                        .to_string(),
                    tool_used: "weather_unavailable".to_string(),
                    sources: Vec::new(),
                }),
            }
        }
        _ => Err(anyhow!("首页总 AI 请求了未开放的工具：{name}")),
    }
}

fn with_tool_note(messages: &[Value]) -> Vec<Value> {
    let mut messages = messages.to_vec();
    let note = HOME_AI_TOOL_NOTE;
    let has_system = messages
        .first()
        .and_then(|message| message["role"].as_str())
        == Some("system");
    if has_system {
        if let Some(system) = messages.first_mut() {
            let content = system["content"].as_str().unwrap_or("");
            system["content"] = json!(format!("{content}\n\n{note}"));
        }
    } else {
        messages.insert(0, json!({ "role": "system", "content": note }));
    }
    messages
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exposes_only_home_read_tools() {
        let tools = tool_definitions().as_array().cloned().unwrap_or_default();
        let names = tools
            .iter()
            .filter_map(|tool| tool["function"]["name"].as_str())
            .collect::<Vec<_>>();
        assert_eq!(names, vec!["web_search", "weather"]);
    }

    #[test]
    fn appends_capability_note_to_existing_system_message() {
        let messages = with_tool_note(&[json!({
            "role": "system",
            "content": "你是助手"
        })]);
        let content = messages[0]["content"].as_str().unwrap_or("");
        assert!(content.contains("首页总 AI 工具使用规则"));
        assert!(content.contains("你是助手"));
    }
}
