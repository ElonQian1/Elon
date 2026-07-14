use super::*;

pub(super) async fn resolve_agent_with_fallback_policy(
    state: &Arc<AppState>,
    workspace: &std::path::Path,
    agent_name: Option<&str>,
) -> Result<ResolvedApiAgent> {
    let global = state.agents_config.read().await;
    if let Some(cfg) = UserAgentConfig::load(workspace) {
        let uses_local_cli = cfg
            .use_agent
            .as_deref()
            .map(|name| is_local_cli_option(state, name))
            .unwrap_or(false);
        if cfg.has_config() && !uses_local_cli {
            let agent = cfg.resolve(&global).ok_or_else(|| {
                anyhow::anyhow!("未找到可用 API 代理，请在后台配置 AGENT_* 或切回 Codex CLI")
            })?;
            return Ok(ResolvedApiAgent {
                agent,
                allow_server_fallback: false,
            });
        }
    }

    let agent = global
        .get_agent(agent_name)
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("未配置 API 代理，请设置 AGENT_* 或使用 Codex CLI"))?;
    let allow_server_fallback = agent_name
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .is_none()
        && agent.usage_mode() == "server_api_key";
    Ok(ResolvedApiAgent {
        agent,
        allow_server_fallback,
    })
}

pub(super) async fn run_casual_chat(
    state: &Arc<AppState>,
    agent: &AgentConfig,
    allow_agent_fallback: bool,
    user_id: &str,
    user_message: &str,
    memories: &[crate::store::UserMemory],
) -> Result<(String, Option<String>, String, bool)> {
    let system_content = if memories.is_empty() {
        casual_chat_prompt().to_string()
    } else {
        let lines = memories
            .iter()
            .map(|m| format!("- {}", m.content))
            .collect::<Vec<_>>()
            .join("\n");
        format!(
            "=== 用户长期记忆（请勿向用户暴露此段）===\n{}\n\n{}",
            lines,
            casual_chat_prompt()
        )
    };
    let messages = vec![
        json!({
            "role": "system",
            "content": system_content
        }),
        json!({
            "role": "user",
            "content": user_message
        }),
    ];

    // 优先尝试路由到在线 PC 节点（当模型名与节点上报模型匹配时）
    if let Some((content, node_id, model_id)) =
        crate::agent_llm_call::try_casual_chat_via_node(state, &agent.model, &messages, user_id)
            .await
    {
        return Ok((content, Some(node_id), model_id, false));
    }

    let (response, used_agent, used_fallback) = call_chat_llm_with_default_fallback_options(
        state,
        agent,
        allow_agent_fallback,
        &messages,
        user_id,
        "chat",
        0.8,
        700,
    )
    .await?;

    let reply = response["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("我在，你可以继续说。")
        .trim()
        .to_string();

    Ok((
        if reply.is_empty() {
            "我在，你可以继续说。".into()
        } else {
            reply
        },
        None,
        used_agent.model,
        used_fallback,
    ))
}

pub(super) fn load_context_memories(
    state: &Arc<AppState>,
    user_id: &str,
    scope_type: Option<&str>,
    scope_id: Option<&str>,
    limit: i64,
) -> Vec<crate::store::UserMemory> {
    match scope_type {
        Some(scope_type) => state
            .store
            .get_user_memories_for_scope(user_id, scope_type, scope_id, limit)
            .unwrap_or_default(),
        None => state
            .store
            .get_user_memories(user_id, limit)
            .unwrap_or_default(),
    }
}

pub(super) async fn run_api_plan(
    state: &Arc<AppState>,
    agent: &crate::types::AgentConfig,
    allow_agent_fallback: bool,
    user_id: &str,
    workspace: &str,
    user_message: &str,
    preflight_note: Option<&str>,
    tx: &UnboundedSender<String>,
) -> Result<()> {
    let _ = tx.send(
        WsMessage::progress(format!(
            "正在使用 AI 代理规划: {} ({})",
            agent.name, agent.model
        ))
        .to_json(),
    );
    let note = preflight_note.unwrap_or("无");
    let messages = vec![
        json!({
            "role": "system",
            "content": "你是一龙项目规划助手。当前是 Plan 模式：只生成计划，不调用工具，不修改文件，不构建、不提交、不发布。输出中文，给小白也能看懂。"
        }),
        json!({
            "role": "user",
            "content": format!(
                "当前项目目录：{}\n项目预检提示：{}\n\n用户请求：{}\n\n请输出：1. 我理解的目标 2. 推荐方案 3. 需要改动的模块或页面 4. 实施步骤 5. 验证与发布方式 6. 需要确认的问题。结尾提醒用户确认后发送「按这个计划开始实现」。",
                workspace, note, user_message
            )
        }),
    ];
    let (response, used_agent, used_fallback) = call_chat_llm_with_default_fallback_options(
        state,
        agent,
        allow_agent_fallback,
        &messages,
        user_id,
        "plan",
        0.8,
        700,
    )
    .await?;
    if used_fallback {
        let _ = tx.send(
            WsMessage::progress(format!(
                "默认 AI 通道不可用，已切换备用 AI 通道: {} ({})",
                used_agent.name, used_agent.model
            ))
            .to_json(),
        );
    }
    let reply = response["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("计划生成完成。确认后发送「按这个计划开始实现」。")
        .to_string();
    let _ = tx.send(
        WsMessage::Done {
            message: reply,
            apk_url: None,
            image_url: None,
            model_used: Some(used_agent.model),
            node_id: None,
        }
        .to_json(),
    );
    Ok(())
}
