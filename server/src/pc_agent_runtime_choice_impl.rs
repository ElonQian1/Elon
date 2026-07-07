use super::*;

pub(super) fn choice_from_cli(
    cli_name: String,
    option: Option<&AiCliOption>,
    agent_name: Option<&str>,
    route_preference: Option<PcRuntimeRoutePreference>,
) -> PcAgentRuntimeChoice {
    let route_label = runtime_route_model_label(route_preference, &cli_name);
    match cli_name.as_str() {
        "copilot" => PcAgentRuntimeChoice {
            cli_name,
            copilot_model: option.and_then(|o| o.model.clone()),
            codex_reasoning_effort: None,
            model_label: route_label.or_else(|| {
                option
                    .map(AiCliOption::display_label)
                    .or_else(|| agent_name.map(String::from))
            }),
            error: None,
        },
        "codex" => PcAgentRuntimeChoice {
            cli_name,
            copilot_model: option.and_then(|o| o.model.clone()),
            codex_reasoning_effort: option.and_then(|o| o.reasoning_effort.clone()),
            model_label: route_label.or_else(|| {
                option
                    .map(AiCliOption::display_label)
                    .or_else(|| agent_name.map(String::from))
            }),
            error: None,
        },
        "server-runtime" => PcAgentRuntimeChoice {
            cli_name,
            copilot_model: None,
            codex_reasoning_effort: None,
            model_label: route_label.or_else(|| Some("平台AI".to_string())),
            error: None,
        },
        "api-runtime" => PcAgentRuntimeChoice {
            cli_name,
            copilot_model: None,
            codex_reasoning_effort: None,
            model_label: route_label.or_else(|| Some("我的 API key".to_string())),
            error: None,
        },
        _ => PcAgentRuntimeChoice {
            cli_name,
            copilot_model: None,
            codex_reasoning_effort: None,
            model_label: route_label.or_else(|| {
                option
                    .map(AiCliOption::display_label)
                    .or_else(|| agent_name.map(String::from))
            }),
            error: None,
        },
    }
}

pub(super) fn runtime_route_model_label(
    route_preference: Option<PcRuntimeRoutePreference>,
    cli_name: &str,
) -> Option<String> {
    match route_preference {
        Some(PcRuntimeRoutePreference::RouteC2) => {
            Some("远程AI（其他用户 PC 节点 + 一龙 CLI）".to_string())
        }
        Some(PcRuntimeRoutePreference::RouteC3) => Some(format!(
            "远程Codex（其他用户 PC 节点 + {}）",
            cli_display_name(cli_name)
        )),
        _ => None,
    }
}

pub(super) fn cli_display_name(cli_name: &str) -> &'static str {
    match cli_name {
        "copilot" => "Copilot",
        "codex" => "Codex",
        "claude" => "Claude",
        "gemini" => "Gemini",
        _ => "AI工具",
    }
}

pub(super) fn requested_cli_name(option: Option<&AiCliOption>, agent_name: Option<&str>) -> String {
    if let Some(option) = option {
        return cli_name_from_parts(&option.provider, &option.id, &option.bin);
    }
    agent_name
        .map(|name| cli_name_from_parts(name, name, name))
        .unwrap_or_else(|| "codex".to_string())
}

pub(super) fn cli_name_from_parts(provider: &str, id: &str, bin: &str) -> String {
    for value in [provider, id, bin] {
        let lower = value.to_ascii_lowercase();
        for cli in ["api-runtime", "server-runtime"] {
            if lower.contains(cli) {
                return cli.to_string();
            }
        }
        for cli in ["codex", "copilot", "claude", "gemini"] {
            if lower.contains(cli) {
                return cli.to_string();
            }
        }
    }
    "codex".to_string()
}

pub(super) fn cli_available(allowed_clis: &[String], cli: &str) -> bool {
    allowed_clis
        .iter()
        .any(|item| item.eq_ignore_ascii_case(cli))
}

pub(super) fn first_available_route_a_cli(allowed_clis: &[String]) -> Option<String> {
    ["codex", "copilot", "claude", "gemini"]
        .iter()
        .find(|cli| cli_available(allowed_clis, cli))
        .map(|cli| (*cli).to_string())
}

pub(super) fn route_a_runtime_ready(dev_runtime: Option<&NodeDevRuntimeProfile>) -> bool {
    dev_runtime
        .map(|runtime| runtime.route_a_ready)
        .unwrap_or(true)
}

pub(super) fn choose_cli_for_runtime(
    allowed_clis: &[String],
    dev_runtime: Option<&NodeDevRuntimeProfile>,
    requested_cli: String,
    route_preference: Option<PcRuntimeRoutePreference>,
) -> Result<String, String> {
    if let Some(preference) = route_preference {
        return choose_forced_route(allowed_clis, dev_runtime, requested_cli, preference);
    }
    let route_a_ready = route_a_runtime_ready(dev_runtime);
    if route_a_ready && cli_available(allowed_clis, &requested_cli) {
        return Ok(requested_cli);
    }
    if route_a_ready {
        if let Some(route_a_cli) = first_available_route_a_cli(allowed_clis) {
            return Ok(route_a_cli);
        }
    }
    let Some(runtime) = dev_runtime else {
        return Ok(requested_cli);
    };
    // 用户配置了自己的 API key 时优先走本机，避免消耗平台AI预算。
    // 模型调用和本地工具循环都由用户 PC 自己承担，不消耗平台服务器模型。
    if runtime.api_runtime_ready {
        return Ok("api-runtime".to_string());
    }
    if route_c_runtime_ready(Some(runtime)) {
        return Ok("server-runtime".to_string());
    }
    Ok(requested_cli)
}

pub(super) fn choose_forced_route(
    allowed_clis: &[String],
    dev_runtime: Option<&NodeDevRuntimeProfile>,
    requested_cli: String,
    preference: PcRuntimeRoutePreference,
) -> Result<String, String> {
    match preference {
        PcRuntimeRoutePreference::RouteA => {
            if !route_a_runtime_ready(dev_runtime) {
                return Err(
                    "已选择本机AI，但这台电脑上的 AI 工具检测未通过；请修复登录/安装，或切回自动。"
                        .to_string(),
                );
            }
            if cli_available(allowed_clis, &requested_cli)
                && matches!(
                    requested_cli.as_str(),
                    "codex" | "copilot" | "claude" | "gemini"
                )
            {
                return Ok(requested_cli);
            }
            first_available_route_a_cli(allowed_clis).ok_or_else(|| {
                "已选择本机AI，但此 PC 节点没有可用的 Codex/Copilot/Claude/Gemini。".to_string()
            })
        }
        PcRuntimeRoutePreference::RouteB => {
            if dev_runtime.is_some_and(|runtime| runtime.api_runtime_ready) {
                Ok("api-runtime".to_string())
            } else {
                Err("已选择我的 API key，但本机 API key 未就绪；请配置 API key 和模型，或切回自动。".to_string())
            }
        }
        PcRuntimeRoutePreference::RouteC => {
            if route_c_runtime_ready(dev_runtime) {
                Ok("server-runtime".to_string())
            } else {
                Err("已选择平台AI，但平台AI暂时不可用或被限流/预算保护挡住；请确认 Win 客户端已登录，或切回自动。".to_string())
            }
        }
        PcRuntimeRoutePreference::RouteC2 => {
            if dev_runtime.is_some_and(|runtime| runtime.api_runtime_ready) {
                Ok("api-runtime".to_string())
            } else {
                Err("已选择远程AI，但目标其他用户 PC 节点的 API key 未就绪；请选择已配置 API key 的远程节点，或切回自动。".to_string())
            }
        }
        PcRuntimeRoutePreference::RouteC3 => {
            if !route_a_runtime_ready(dev_runtime) {
                return Err(
                    "已选择远程Codex，但目标其他用户 PC 节点的 AI 工具检测未通过；请选择已登录 Codex/Copilot 的远程节点，或切回自动。"
                        .to_string(),
                );
            }
            if cli_available(allowed_clis, &requested_cli)
                && matches!(
                    requested_cli.as_str(),
                    "codex" | "copilot" | "claude" | "gemini"
                )
            {
                return Ok(requested_cli);
            }
            first_available_route_a_cli(allowed_clis).ok_or_else(|| {
                "已选择远程Codex，但目标其他用户 PC 节点没有可用的 Codex/Copilot/Claude/Gemini。"
                    .to_string()
            })
        }
    }
}

pub(super) fn route_c_runtime_ready(dev_runtime: Option<&NodeDevRuntimeProfile>) -> bool {
    let Some(runtime) = dev_runtime else {
        return false;
    };
    if !runtime.server_runtime_ready {
        return false;
    }
    runtime
        .server_runtime_status
        .as_ref()
        .map(route_c_status_allows_selection)
        .unwrap_or(true)
}

pub(super) fn route_c_status_allows_selection(status: &Value) -> bool {
    if status.get("ready").and_then(Value::as_bool) == Some(false) {
        return false;
    }
    if let Some(status) = status.get("status").and_then(Value::as_str) {
        if route_c_status_is_blocking(status) {
            return false;
        }
    }
    if blocking_reasons_present(status) {
        return false;
    }
    if nested_bool(status, &["policy", "enabled"]) == Some(false)
        || nested_bool(status, &["agentPolicy", "ready"]) == Some(false)
        || nested_bool(status, &["agent_policy", "ready"]) == Some(false)
    {
        return false;
    }
    if nested_status_is_blocking(status, &["agentPolicy"])
        || nested_status_is_blocking(status, &["agent_policy"])
        || nested_status_is_blocking(status, &["admissionAvailability"])
        || nested_status_is_blocking(status, &["admission_availability"])
    {
        return false;
    }
    if nested_bool(status, &["admissionAvailability", "ready"]) == Some(false)
        || nested_bool(status, &["admission_availability", "ready"]) == Some(false)
    {
        return false;
    }
    if nested_bool(status, &["budget", "ready"]) == Some(false) {
        return false;
    }
    if let Some(budget_status) = status
        .get("budget")
        .and_then(|budget| budget.get("status"))
        .and_then(Value::as_str)
    {
        if matches!(
            budget_status,
            "exhausted" | "user_exhausted" | "unavailable"
        ) {
            return false;
        }
    }
    true
}

pub(super) fn route_c_status_is_blocking(status: &str) -> bool {
    matches!(
        status.trim().to_ascii_lowercase().as_str(),
        "disabled"
            | "blocked"
            | "missing_agent"
            | "admission_limited"
            | "limited"
            | "rate_limited"
            | "budget_exhausted"
            | "platform_budget_exhausted"
            | "user_budget_exhausted"
            | "agent_policy_blocked"
            | "no_server_api_key_agent"
            | "unsupported_agent_usage_mode"
            | "unavailable"
            | "http_error"
    )
}

pub(super) fn blocking_reasons_present(status: &Value) -> bool {
    status
        .get("blockingReasons")
        .or_else(|| status.get("blocking_reasons"))
        .and_then(Value::as_array)
        .is_some_and(|reasons| !reasons.is_empty())
}

pub(super) fn nested_status_is_blocking(value: &Value, path: &[&str]) -> bool {
    nested_string(value, path, "status")
        .or_else(|| nested_string(value, path, "reason"))
        .is_some_and(route_c_status_is_blocking)
}

pub(super) fn nested_string<'a>(value: &'a Value, path: &[&str], leaf: &str) -> Option<&'a str> {
    let mut current = value;
    for key in path {
        current = current.get(*key)?;
    }
    current.get(leaf)?.as_str()
}

pub(super) fn nested_bool(value: &Value, path: &[&str]) -> Option<bool> {
    let mut current = value;
    for key in path {
        current = current.get(*key)?;
    }
    current.as_bool()
}

#[cfg(test)]
#[path = "pc_agent_runtime_choice_tests.rs"]
mod tests;
