// server/src/pc_agent_runtime_choice.rs

use homecli_proto::NodeDevRuntimeProfile;
use serde_json::Value;
use std::sync::Arc;

use crate::types::{AiCliOption, AppState};

#[derive(Debug, Clone)]
pub(crate) struct PcAgentRuntimeChoice {
    pub cli_name: String,
    pub copilot_model: Option<String>,
    pub codex_reasoning_effort: Option<String>,
    pub model_label: Option<String>,
    pub error: Option<String>,
}

impl PcAgentRuntimeChoice {
    pub(crate) fn progress_label(&self) -> &str {
        self.model_label
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or(self.cli_name.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PcRuntimeRoutePreference {
    RouteA,
    RouteB,
    RouteC,
    RouteC2,
    RouteC3,
}

impl PcRuntimeRoutePreference {
    pub(crate) fn from_request(value: &str) -> Result<Option<Self>, String> {
        let clean = value.trim().to_ascii_lowercase();
        if clean.is_empty() || clean == "auto" || clean == "route_auto" {
            return Ok(None);
        }
        match clean.as_str() {
            "route_a" | "route-a" | "a" | "cli-wrapper" | "cli_wrapper" => Ok(Some(Self::RouteA)),
            "route_b" | "route-b" | "b" | "api-runtime" | "api_runtime" => Ok(Some(Self::RouteB)),
            "route_c" | "route-c" | "route_c1" | "route-c1" | "c" | "c1" | "server-runtime"
            | "server_runtime" => Ok(Some(Self::RouteC)),
            "route_c2" | "route-c2" | "c2" | "remote-api-runtime" | "remote_api_runtime" => {
                Ok(Some(Self::RouteC2))
            }
            "route_c3" | "route-c3" | "c3" | "remote-cli-runtime" | "remote_cli_runtime" => {
                Ok(Some(Self::RouteC3))
            }
            _ => Err(
                "runtimeRoute 必须为 auto、route_a、route_b、route_c1、route_c2 或 route_c3"
                    .to_string(),
            ),
        }
    }

    pub(crate) fn as_request_value(self) -> &'static str {
        match self {
            Self::RouteA => "route_a",
            Self::RouteB => "route_b",
            Self::RouteC => "route_c",
            Self::RouteC2 => "route_c2",
            Self::RouteC3 => "route_c3",
        }
    }
}

pub(crate) async fn choose_pc_agent_runtime(
    state: &Arc<AppState>,
    agent_id: &str,
    agent_name: Option<&str>,
    route_preference: Option<PcRuntimeRoutePreference>,
) -> PcAgentRuntimeChoice {
    let option = agent_name
        .filter(|name| state.ai_cli.has_option(name))
        .and_then(|name| state.ai_cli.find_option(Some(name)).cloned());
    let requested_cli = requested_cli_name(option.as_ref(), agent_name);
    let summary = state
        .agent_manager
        .list()
        .await
        .into_iter()
        .find(|agent| agent.agent_id == agent_id);
    let allowed_clis = summary
        .as_ref()
        .map(|agent| agent.allowed_clis.as_slice())
        .unwrap_or(&[]);

    let chosen_cli = choose_cli_for_runtime(
        allowed_clis,
        summary
            .as_ref()
            .and_then(|agent| agent.dev_runtime.as_ref()),
        requested_cli,
        route_preference,
    );

    match chosen_cli {
        Ok(cli) => choice_from_cli(cli, option.as_ref(), agent_name, route_preference),
        Err(error) => PcAgentRuntimeChoice {
            cli_name: requested_cli_name(option.as_ref(), agent_name),
            copilot_model: None,
            codex_reasoning_effort: None,
            model_label: Some("运行路线不可用".to_string()),
            error: Some(error),
        },
    }
}

fn choice_from_cli(
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
            model_label: route_label.or_else(|| Some("一龙服务器模型（Route C.1）".to_string())),
            error: None,
        },
        "api-runtime" => PcAgentRuntimeChoice {
            cli_name,
            copilot_model: None,
            codex_reasoning_effort: None,
            model_label: route_label.or_else(|| Some("本机 API Runtime（Route B）".to_string())),
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

fn runtime_route_model_label(
    route_preference: Option<PcRuntimeRoutePreference>,
    cli_name: &str,
) -> Option<String> {
    match route_preference {
        Some(PcRuntimeRoutePreference::RouteC2) => {
            Some("远程 PC API Runtime（Route C.2）".to_string())
        }
        Some(PcRuntimeRoutePreference::RouteC3) => Some(format!(
            "远程 PC CLI（Route C.3 · {}）",
            cli_display_name(cli_name)
        )),
        _ => None,
    }
}

fn cli_display_name(cli_name: &str) -> &'static str {
    match cli_name {
        "copilot" => "Copilot",
        "codex" => "Codex",
        "claude" => "Claude",
        "gemini" => "Gemini",
        _ => "AI CLI",
    }
}

fn requested_cli_name(option: Option<&AiCliOption>, agent_name: Option<&str>) -> String {
    if let Some(option) = option {
        return cli_name_from_parts(&option.provider, &option.id, &option.bin);
    }
    agent_name
        .map(|name| cli_name_from_parts(name, name, name))
        .unwrap_or_else(|| "codex".to_string())
}

fn cli_name_from_parts(provider: &str, id: &str, bin: &str) -> String {
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

fn cli_available(allowed_clis: &[String], cli: &str) -> bool {
    allowed_clis
        .iter()
        .any(|item| item.eq_ignore_ascii_case(cli))
}

fn first_available_route_a_cli(allowed_clis: &[String]) -> Option<String> {
    ["codex", "copilot", "claude", "gemini"]
        .iter()
        .find(|cli| cli_available(allowed_clis, cli))
        .map(|cli| (*cli).to_string())
}

fn route_a_runtime_ready(dev_runtime: Option<&NodeDevRuntimeProfile>) -> bool {
    dev_runtime
        .map(|runtime| runtime.route_a_ready)
        .unwrap_or(true)
}

fn choose_cli_for_runtime(
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
    // Route B 必须排在 Route C 前面：用户配置了自己的 API key 时，
    // 模型调用和本地工具循环都由用户 PC 自己承担，不消耗平台服务器模型。
    if runtime.api_runtime_ready {
        return Ok("api-runtime".to_string());
    }
    if route_c_runtime_ready(Some(runtime)) {
        return Ok("server-runtime".to_string());
    }
    Ok(requested_cli)
}

fn choose_forced_route(
    allowed_clis: &[String],
    dev_runtime: Option<&NodeDevRuntimeProfile>,
    requested_cli: String,
    preference: PcRuntimeRoutePreference,
) -> Result<String, String> {
    match preference {
        PcRuntimeRoutePreference::RouteA => {
            if !route_a_runtime_ready(dev_runtime) {
                return Err(
                    "已强制 Route A，但本机 AI CLI 版本探测未通过；请修复 CLI 登录/安装，或切回自动使用 Route C。"
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
                "已强制 Route A，但此 PC 节点没有可用的 Codex/Copilot/Claude/Gemini CLI".to_string()
            })
        }
        PcRuntimeRoutePreference::RouteB => {
            if dev_runtime.is_some_and(|runtime| runtime.api_runtime_ready) {
                Ok("api-runtime".to_string())
            } else {
                Err("已强制 Route B，但本机 API Runtime 未就绪；请配置 API key 和模型，或切回自动。".to_string())
            }
        }
        PcRuntimeRoutePreference::RouteC => {
            if route_c_runtime_ready(dev_runtime) {
                Ok("server-runtime".to_string())
            } else {
                Err("已强制 Route C，但服务器模型 Runtime 未就绪或被限流/预算保护挡住；请确认 Win 客户端已登录、云端 Route C 可接单，或切回自动。".to_string())
            }
        }
        PcRuntimeRoutePreference::RouteC2 => {
            if dev_runtime.is_some_and(|runtime| runtime.api_runtime_ready) {
                Ok("api-runtime".to_string())
            } else {
                Err("已强制 Route C.2，但目标远程 PC 节点的 API Runtime 未就绪；请选择已配置 API key 的远程节点，或切回自动。".to_string())
            }
        }
        PcRuntimeRoutePreference::RouteC3 => {
            if !route_a_runtime_ready(dev_runtime) {
                return Err(
                    "已强制 Route C.3，但目标远程 PC 节点的 CLI 探测未通过；请选择已登录 Codex/Copilot 的远程节点，或切回自动。"
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
                "已强制 Route C.3，但目标远程 PC 节点没有可用的 Codex/Copilot/Claude/Gemini CLI"
                    .to_string()
            })
        }
    }
}

fn route_c_runtime_ready(dev_runtime: Option<&NodeDevRuntimeProfile>) -> bool {
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

fn route_c_status_allows_selection(status: &Value) -> bool {
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

fn route_c_status_is_blocking(status: &str) -> bool {
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

fn blocking_reasons_present(status: &Value) -> bool {
    status
        .get("blockingReasons")
        .or_else(|| status.get("blocking_reasons"))
        .and_then(Value::as_array)
        .is_some_and(|reasons| !reasons.is_empty())
}

fn nested_status_is_blocking(value: &Value, path: &[&str]) -> bool {
    nested_string(value, path, "status")
        .or_else(|| nested_string(value, path, "reason"))
        .is_some_and(route_c_status_is_blocking)
}

fn nested_string<'a>(value: &'a Value, path: &[&str], leaf: &str) -> Option<&'a str> {
    let mut current = value;
    for key in path {
        current = current.get(*key)?;
    }
    current.get(leaf)?.as_str()
}

fn nested_bool(value: &Value, path: &[&str]) -> Option<bool> {
    let mut current = value;
    for key in path {
        current = current.get(*key)?;
    }
    current.as_bool()
}

#[cfg(test)]
mod tests {
    use super::{
        choose_cli_for_runtime, cli_name_from_parts, first_available_route_a_cli,
        route_c_runtime_ready, PcRuntimeRoutePreference,
    };
    use homecli_proto::NodeDevRuntimeProfile;
    use serde_json::json;

    #[test]
    fn cli_name_detects_known_providers() {
        assert_eq!(cli_name_from_parts("codex", "x", "x"), "codex");
        assert_eq!(cli_name_from_parts("x", "github-copilot", "x"), "copilot");
        assert_eq!(cli_name_from_parts("x", "x", "claude.exe"), "claude");
        assert_eq!(cli_name_from_parts("x", "api-runtime", "x"), "api-runtime");
        assert_eq!(
            cli_name_from_parts("x", "server-runtime", "x"),
            "server-runtime"
        );
    }

    #[test]
    fn route_a_preference_is_stable() {
        let allowed = vec!["gemini".to_string(), "codex".to_string()];
        assert_eq!(
            first_available_route_a_cli(&allowed).as_deref(),
            Some("codex")
        );
    }

    #[test]
    fn route_b_is_selected_before_server_runtime() {
        let runtime = NodeDevRuntimeProfile {
            api_runtime_ready: true,
            server_runtime_ready: true,
            ..Default::default()
        };
        assert_eq!(
            choose_cli_for_runtime(&[], Some(&runtime), "codex".to_string(), None).unwrap(),
            "api-runtime"
        );
    }

    #[test]
    fn route_c_is_selected_when_no_cli_or_api_runtime_exists() {
        let runtime = NodeDevRuntimeProfile {
            server_runtime_ready: true,
            ..Default::default()
        };
        assert_eq!(
            choose_cli_for_runtime(&[], Some(&runtime), "codex".to_string(), None).unwrap(),
            "server-runtime"
        );
    }

    #[test]
    fn route_c_status_gate_preserves_old_nodes_without_cloud_detail() {
        let runtime = NodeDevRuntimeProfile {
            server_runtime_ready: true,
            server_runtime_status: None,
            ..Default::default()
        };

        assert!(route_c_runtime_ready(Some(&runtime)));
        assert_eq!(
            choose_cli_for_runtime(&[], Some(&runtime), "codex".to_string(), None).unwrap(),
            "server-runtime"
        );
    }

    #[test]
    fn auto_route_skips_route_c_when_admission_is_limited() {
        let runtime = NodeDevRuntimeProfile {
            route_a_ready: false,
            server_runtime_ready: true,
            server_runtime_status: Some(json!({
                "ready": true,
                "status": "ready",
                "admissionAvailability": {
                    "ready": false,
                    "reason": "rate_limited",
                    "retryAfterSecs": 17
                }
            })),
            ..Default::default()
        };

        assert!(!route_c_runtime_ready(Some(&runtime)));
        assert_eq!(
            choose_cli_for_runtime(&[], Some(&runtime), "codex".to_string(), None).unwrap(),
            "codex"
        );
    }

    #[test]
    fn auto_route_skips_route_c_when_budget_is_exhausted() {
        let runtime = NodeDevRuntimeProfile {
            route_a_ready: false,
            server_runtime_ready: true,
            server_runtime_status: Some(json!({
                "ready": true,
                "status": "ready",
                "budget": {
                    "status": "user_exhausted",
                    "remainingCallsTodayForUser": 0
                }
            })),
            ..Default::default()
        };

        assert!(!route_c_runtime_ready(Some(&runtime)));
        assert_eq!(
            choose_cli_for_runtime(&[], Some(&runtime), "codex".to_string(), None).unwrap(),
            "codex"
        );
    }

    #[test]
    fn auto_route_skips_route_c_when_blocking_reasons_are_reported() {
        let runtime = NodeDevRuntimeProfile {
            route_a_ready: false,
            server_runtime_ready: true,
            server_runtime_status: Some(json!({
                "ready": true,
                "status": "ready",
                "blockingReasons": [{
                    "code": "platform_budget_exhausted",
                    "scope": "budget",
                    "message": "Route C 今日平台预算已用完"
                }]
            })),
            ..Default::default()
        };

        assert!(!route_c_runtime_ready(Some(&runtime)));
        assert_eq!(
            choose_cli_for_runtime(&[], Some(&runtime), "codex".to_string(), None).unwrap(),
            "codex"
        );
    }

    #[test]
    fn auto_route_skips_route_c_when_agent_policy_blocks_selection() {
        let runtime = NodeDevRuntimeProfile {
            route_a_ready: false,
            server_runtime_ready: true,
            server_runtime_status: Some(json!({
                "ready": true,
                "status": "ready",
                "agentPolicy": {
                    "ready": false,
                    "reason": "no_server_api_key_agent"
                }
            })),
            ..Default::default()
        };

        assert!(!route_c_runtime_ready(Some(&runtime)));
        assert_eq!(
            choose_cli_for_runtime(&[], Some(&runtime), "codex".to_string(), None).unwrap(),
            "codex"
        );
    }

    #[test]
    fn auto_route_skips_detected_route_a_when_profile_probe_failed() {
        let runtime = NodeDevRuntimeProfile {
            route_a_ready: false,
            server_runtime_ready: true,
            ..Default::default()
        };
        let allowed = vec!["codex".to_string()];
        assert_eq!(
            choose_cli_for_runtime(&allowed, Some(&runtime), "codex".to_string(), None).unwrap(),
            "server-runtime"
        );
    }

    #[test]
    fn auto_route_keeps_route_a_when_profile_probe_is_ready() {
        let runtime = NodeDevRuntimeProfile {
            route_a_ready: true,
            server_runtime_ready: true,
            ..Default::default()
        };
        let allowed = vec!["codex".to_string()];
        assert_eq!(
            choose_cli_for_runtime(&allowed, Some(&runtime), "codex".to_string(), None).unwrap(),
            "codex"
        );
    }

    #[test]
    fn forced_route_b_skips_available_route_a() {
        let runtime = NodeDevRuntimeProfile {
            api_runtime_ready: true,
            server_runtime_ready: true,
            ..Default::default()
        };
        let allowed = vec!["codex".to_string()];
        assert_eq!(
            choose_cli_for_runtime(
                &allowed,
                Some(&runtime),
                "codex".to_string(),
                Some(PcRuntimeRoutePreference::RouteB),
            )
            .unwrap(),
            "api-runtime"
        );
    }

    #[test]
    fn route_c1_alias_maps_to_server_runtime() {
        assert_eq!(
            PcRuntimeRoutePreference::from_request("route_c1").unwrap(),
            Some(PcRuntimeRoutePreference::RouteC)
        );
        assert_eq!(
            PcRuntimeRoutePreference::from_request("c1").unwrap(),
            Some(PcRuntimeRoutePreference::RouteC)
        );
    }

    #[test]
    fn route_c2_and_c3_aliases_are_open() {
        assert_eq!(
            PcRuntimeRoutePreference::from_request("route_c2").unwrap(),
            Some(PcRuntimeRoutePreference::RouteC2)
        );
        assert_eq!(
            PcRuntimeRoutePreference::from_request("remote-api-runtime").unwrap(),
            Some(PcRuntimeRoutePreference::RouteC2)
        );
        assert_eq!(
            PcRuntimeRoutePreference::from_request("route_c3").unwrap(),
            Some(PcRuntimeRoutePreference::RouteC3)
        );
        assert_eq!(
            PcRuntimeRoutePreference::from_request("remote-cli-runtime").unwrap(),
            Some(PcRuntimeRoutePreference::RouteC3)
        );
    }

    #[test]
    fn forced_route_c2_selects_remote_api_runtime() {
        let runtime = NodeDevRuntimeProfile {
            api_runtime_ready: true,
            route_a_ready: true,
            ..Default::default()
        };
        assert_eq!(
            choose_cli_for_runtime(
                &["codex".to_string()],
                Some(&runtime),
                "codex".to_string(),
                Some(PcRuntimeRoutePreference::RouteC2),
            )
            .unwrap(),
            "api-runtime"
        );
    }

    #[test]
    fn forced_route_c2_requires_api_runtime() {
        let err = choose_cli_for_runtime(
            &["codex".to_string()],
            Some(&NodeDevRuntimeProfile {
                route_a_ready: true,
                ..Default::default()
            }),
            "codex".to_string(),
            Some(PcRuntimeRoutePreference::RouteC2),
        )
        .expect_err("route C.2 should require API Runtime readiness");
        assert!(err.contains("Route C.2"));
        assert!(err.contains("API Runtime"));
    }

    #[test]
    fn forced_route_c3_selects_remote_cli() {
        let runtime = NodeDevRuntimeProfile {
            route_a_ready: true,
            api_runtime_ready: true,
            ..Default::default()
        };
        assert_eq!(
            choose_cli_for_runtime(
                &["copilot".to_string()],
                Some(&runtime),
                "codex".to_string(),
                Some(PcRuntimeRoutePreference::RouteC3),
            )
            .unwrap(),
            "copilot"
        );
    }

    #[test]
    fn forced_route_c3_requires_cli_probe() {
        let err = choose_cli_for_runtime(
            &["codex".to_string()],
            Some(&NodeDevRuntimeProfile {
                route_a_ready: false,
                api_runtime_ready: true,
                ..Default::default()
            }),
            "codex".to_string(),
            Some(PcRuntimeRoutePreference::RouteC3),
        )
        .expect_err("route C.3 should require remote CLI readiness");
        assert!(err.contains("Route C.3"));
        assert!(err.contains("CLI"));
    }

    #[test]
    fn forced_unavailable_route_returns_actionable_error() {
        let err = choose_cli_for_runtime(
            &["codex".to_string()],
            Some(&NodeDevRuntimeProfile::default()),
            "codex".to_string(),
            Some(PcRuntimeRoutePreference::RouteC),
        )
        .expect_err("route C should not be selected when server runtime is not ready");
        assert!(err.contains("Route C"));
        assert!(err.contains("未就绪"));
    }

    #[test]
    fn forced_route_c_reports_operational_protection_block() {
        let runtime = NodeDevRuntimeProfile {
            server_runtime_ready: true,
            server_runtime_status: Some(json!({
                "ready": true,
                "status": "ready",
                "admissionAvailability": {
                    "ready": false,
                    "reason": "user_concurrency_limited"
                }
            })),
            ..Default::default()
        };

        let err = choose_cli_for_runtime(
            &[],
            Some(&runtime),
            "codex".to_string(),
            Some(PcRuntimeRoutePreference::RouteC),
        )
        .expect_err("route C should not bypass cloud admission protection");
        assert!(err.contains("Route C"));
        assert!(err.contains("限流"));
        assert!(err.contains("预算"));
    }

    #[test]
    fn forced_route_c_does_not_bypass_blocking_reasons() {
        let runtime = NodeDevRuntimeProfile {
            server_runtime_ready: true,
            server_runtime_status: Some(json!({
                "ready": true,
                "status": "ready",
                "blocking_reasons": [{
                    "code": "agent_policy_blocked",
                    "scope": "agent_policy"
                }]
            })),
            ..Default::default()
        };

        let err = choose_cli_for_runtime(
            &[],
            Some(&runtime),
            "codex".to_string(),
            Some(PcRuntimeRoutePreference::RouteC),
        )
        .expect_err("route C should not bypass cloud blocking reasons");
        assert!(err.contains("Route C"));
        assert!(err.contains("限流"));
        assert!(err.contains("预算"));
    }

    #[test]
    fn forced_route_a_requires_successful_runtime_probe() {
        let runtime = NodeDevRuntimeProfile {
            route_a_ready: false,
            server_runtime_ready: true,
            ..Default::default()
        };
        let err = choose_cli_for_runtime(
            &["codex".to_string()],
            Some(&runtime),
            "codex".to_string(),
            Some(PcRuntimeRoutePreference::RouteA),
        )
        .expect_err("route A should not be selected when CLI probe failed");
        assert!(err.contains("Route A"));
        assert!(err.contains("未通过"));
    }
}
