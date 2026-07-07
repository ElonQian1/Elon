use super::*;

pub(super) fn mvp_chat_enabled() -> bool {
    env_flag("ELON_EXTERNAL_APP_MVP_CHAT_ENABLED")
}

pub(super) fn env_flag(name: &str) -> bool {
    std::env::var(name)
        .ok()
        .map(|value| {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
        .unwrap_or(false)
}

pub(super) fn build_messages(app_id: &str, user_message: &str, req: &ExternalAppMvpChatRequest) -> Vec<Value> {
    let mut messages = vec![json!({
        "role": "system",
        "content": system_prompt(app_id)
    })];

    messages.extend(
        req.history
            .iter()
            .filter_map(project_history_message)
            .take(MAX_HISTORY_ITEMS),
    );

    messages.push(json!({
        "role": "user",
        "content": build_user_prompt(app_id, user_message, &req.client, &req.local_context)
    }));
    messages
}

pub(super) fn system_prompt(app_id: &str) -> &'static str {
    if app_id == "bb64a" {
        return "你是 ElonSpeed / BB64A Windows AI 助手。你根据用户问题、本地诊断快照、代理状态、节点状态和日志摘要进行排查。必须区分：用户本机配置问题、节点/订阅问题、Windows 系统代理/TUN/路由问题、以及 BB64A 产品 bug。你不能声称已经执行工具；如果需要本地工具，只能建议客户端调用工具，并说明原因。危险动作必须要求用户确认。不要输出订阅 URL、token、节点密码或无关本地文件内容。输出中文，先给结论，再给 2 到 5 条排查/建议。";
    }
    "你是主项目提供给外部子项目的临时 MVP AI 助手。请根据用户问题、子项目提供的上下文和最近对话给出中文回答。不要编造上下文中没有的事实；如果需要子项目本地工具或数据，请说明需要客户端补充什么。"
}

pub(super) fn build_user_prompt(
    app_id: &str,
    user_message: &str,
    client: &Value,
    local_context: &Value,
) -> String {
    let client_json = compact_json(client, MAX_JSON_CONTEXT_CHARS / 4);
    let context_json = compact_json(local_context, MAX_JSON_CONTEXT_CHARS);
    let tool_note = if app_id == "bb64a" {
        "\n\nBB64A 本地工具边界：客户端可能提供 bb64a_doctor、get_status、test_google、detect_conflicts、get_system_proxy_status、get_logs_filtered、auto_select_best 等工具。你只能建议调用，不能假装已经调用。force_close_proxy、close_all_proxies、exit_app、clear_all_nodes 等危险动作必须用户确认。"
    } else {
        ""
    };
    format!(
        "用户问题：\n{user_message}\n\n客户端信息：\n{client_json}\n\n子项目/本地上下文：\n{context_json}{tool_note}\n\n请给出适合直接展示在子项目 AI 面板里的回答。"
    )
}

pub(super) fn project_history_message(message: &ExternalAppMvpChatMessage) -> Option<Value> {
    let role = match message.role.trim() {
        "user" => "user",
        "assistant" => "assistant",
        _ => return None,
    };
    let content = normalize_text(&message.content, MAX_HISTORY_CONTENT_CHARS);
    if content.is_empty() {
        return None;
    }
    Some(json!({ "role": role, "content": content }))
}

pub(super) async fn call_mvp_chat_model(
    state: &Arc<AppState>,
    requested_agent: Option<&str>,
    messages: &[Value],
) -> Result<(String, String, String, bool)> {
    let agents = candidate_agents(state, requested_agent).await?;
    let mut last_retryable_error = None;
    for (index, agent) in agents.iter().enumerate() {
        match send_chat_completion(state, agent, messages).await {
            Ok(response) => {
                let reply = extract_reply(&response)
                    .unwrap_or_else(|| "我已收到问题，但模型没有返回可展示的文本。".to_string());
                return Ok((reply, agent.name.clone(), agent.model.clone(), index > 0));
            }
            Err(error) => {
                let message = error.to_string();
                let has_next = index + 1 < agents.len();
                if !is_retryable_agent_error(&message) || !has_next {
                    return Err(anyhow!(message));
                }
                last_retryable_error = Some(message);
            }
        }
    }

    Err(anyhow!(
        "{}",
        last_retryable_error.unwrap_or_else(|| "未配置可用 server_api_key AI 代理".to_string())
    ))
}

pub(super) async fn candidate_agents(
    state: &Arc<AppState>,
    requested_agent: Option<&str>,
) -> Result<Vec<AgentConfig>> {
    let agents = server_api_agents_in_fallback_order(state).await;
    if agents.is_empty() {
        return Err(anyhow!("未配置可用 server_api_key AI 代理"));
    }
    let Some(requested) = requested_agent
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return Ok(agents);
    };
    let Some(index) = agents
        .iter()
        .position(|agent| agent.name.eq_ignore_ascii_case(requested))
    else {
        return Err(anyhow!("请求的 AI 代理未开放给 MVP 外部应用对话"));
    };
    let mut ordered = vec![agents[index].clone()];
    ordered.extend(agents.into_iter().enumerate().filter_map(|(i, agent)| {
        if i == index {
            None
        } else {
            Some(agent)
        }
    }));
    Ok(ordered)
}

pub(super) async fn send_chat_completion(
    state: &Arc<AppState>,
    agent: &AgentConfig,
    messages: &[Value],
) -> Result<Value> {
    let url = format!("{}/chat/completions", agent.api_base.trim_end_matches('/'));
    let body = json!({
        "model": agent.model,
        "messages": messages,
        "stream": false,
        "temperature": 0.4,
        "max_tokens": MVP_MAX_OUTPUT_TOKENS,
    });
    let response = state
        .http_client
        .post(url)
        .bearer_auth(&agent.api_key)
        .json(&body)
        .send()
        .await
        .map_err(|error| {
            if error.is_timeout() {
                anyhow!("AI 请求超时，请稍后重试")
            } else {
                anyhow!("AI 请求失败: {error}")
            }
        })?;
    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(anyhow!("{}", friendly_ai_api_error(status, &text)));
    }
    Ok(response.json::<Value>().await?)
}

pub(super) fn extract_reply(value: &Value) -> Option<String> {
    value
        .pointer("/choices/0/message/content")
        .and_then(Value::as_str)
        .map(|text| normalize_text(text, 12_000))
        .filter(|text| !text.is_empty())
}

pub(super) fn suggest_tools(app_id: &str, message: &str, local_context: &Value) -> Vec<SuggestedTool> {
    if app_id != "bb64a" {
        return Vec::new();
    }
    let mut tools = Vec::new();
    if local_context.is_null()
        || local_context
            .as_object()
            .map(|obj| obj.is_empty())
            .unwrap_or(false)
    {
        push_tool(
            &mut tools,
            SuggestedTool {
                tool: "bb64a_doctor",
                reason: "先采集一次本机代理、路由、节点和日志诊断快照。",
                dangerous: false,
            },
        );
    }
    let haystack =
        format!("{} {}", message, compact_json(local_context, 4_000)).to_ascii_lowercase();
    if contains_any(
        &haystack,
        &[
            "google",
            "youtube",
            "打不开",
            "上不了",
            "连不上",
            "无法访问",
        ],
    ) {
        push_tool(
            &mut tools,
            SuggestedTool {
                tool: "test_google",
                reason: "验证当前代理链路是否能访问 Google。",
                dangerous: false,
            },
        );
    }
    if contains_any(
        &haystack,
        &[
            "代理",
            "系统代理",
            "冲突",
            "端口",
            "clash",
            "v2ray",
            "sing-box",
        ],
    ) {
        push_tool(
            &mut tools,
            SuggestedTool {
                tool: "detect_conflicts",
                reason: "检查本机是否有其它代理软件或端口占用冲突。",
                dangerous: false,
            },
        );
        push_tool(
            &mut tools,
            SuggestedTool {
                tool: "get_system_proxy_status",
                reason: "读取 Windows 系统代理是否被正确接管。",
                dangerous: false,
            },
        );
    }
    if contains_any(&haystack, &["节点", "延迟", "很慢", "最快", "切换"]) {
        push_tool(
            &mut tools,
            SuggestedTool {
                tool: "auto_select_best",
                reason: "测试候选节点并连接当前最可用节点。",
                dangerous: false,
            },
        );
    }
    if contains_any(
        &haystack,
        &["日志", "报错", "错误", "失败", "reality", "tls"],
    ) {
        push_tool(
            &mut tools,
            SuggestedTool {
                tool: "get_logs_filtered",
                reason: "读取相关错误日志，辅助判断是节点、协议还是客户端问题。",
                dangerous: false,
            },
        );
    }
    if contains_any(&haystack, &["强制关闭", "杀进程", "关闭冲突"]) {
        push_tool(
            &mut tools,
            SuggestedTool {
                tool: "force_close_proxy",
                reason: "只有在用户确认具体冲突进程后，才可关闭对应代理进程。",
                dangerous: true,
            },
        );
    }
    tools.truncate(5);
    tools
}

pub(super) fn push_tool(tools: &mut Vec<SuggestedTool>, tool: SuggestedTool) {
    if !tools.iter().any(|existing| existing.tool == tool.tool) {
        tools.push(tool);
    }
}

pub(super) fn contains_any(haystack: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| haystack.contains(needle))
}

pub(super) fn compact_json(value: &Value, max_chars: usize) -> String {
    serde_json::to_string_pretty(value)
        .unwrap_or_else(|_| "null".to_string())
        .chars()
        .take(max_chars)
        .collect()
}

pub(super) fn json_chars(value: &Value) -> usize {
    serde_json::to_string(value)
        .map(|text| text.chars().count())
        .unwrap_or(0)
}

pub(super) fn normalize_text(value: &str, max_chars: usize) -> String {
    value
        .trim()
        .chars()
        .filter(|ch| !ch.is_control() || *ch == '\n' || *ch == '\t')
        .take(max_chars)
        .collect::<String>()
        .trim()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_control_chars_and_limits() {
        assert_eq!(normalize_text("  a\u{0}bcdef  ", 4), "abcd");
    }

    #[test]
    fn ignores_unknown_history_roles() {
        let item = ExternalAppMvpChatMessage {
            role: "tool".to_string(),
            content: "secret".to_string(),
        };
        assert!(project_history_message(&item).is_none());
    }

    #[test]
    fn suggests_bb64a_tools_from_user_problem() {
        let tools = suggest_tools(
            "bb64a",
            "开了代理还是上不了 Google，怀疑 clash 端口冲突",
            &json!({ "status": "connected" }),
        );
        let names = tools.iter().map(|tool| tool.tool).collect::<Vec<_>>();
        assert!(names.contains(&"test_google"));
        assert!(names.contains(&"detect_conflicts"));
        assert!(names.contains(&"get_system_proxy_status"));
    }

    #[test]
    fn non_bb64a_has_no_local_tool_suggestions() {
        assert!(suggest_tools("fb2", "google 打不开", &Value::Null).is_empty());
    }
}
