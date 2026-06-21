// server/src/pc_agent_runtime_choice.rs

use homecli_proto::NodeDevRuntimeProfile;
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
            "route_c" | "route-c" | "c" | "server-runtime" | "server_runtime" => {
                Ok(Some(Self::RouteC))
            }
            _ => Err("runtimeRoute 必须为 auto、route_a、route_b 或 route_c".to_string()),
        }
    }

    pub(crate) fn as_request_value(self) -> &'static str {
        match self {
            Self::RouteA => "route_a",
            Self::RouteB => "route_b",
            Self::RouteC => "route_c",
        }
    }
}

pub(crate) async fn choose_pc_agent_runtime(
    state: &Arc<AppState>,
    agent_id: &str,
    agent_name: Option<&str>,
    route_preference: Option<PcRuntimeRoutePreference>,
) -> PcAgentRuntimeChoice {
    let option = agent_name.and_then(|name| state.ai_cli.find_option(Some(name)).cloned());
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
        Ok(cli) => choice_from_cli(cli, option.as_ref(), agent_name),
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
) -> PcAgentRuntimeChoice {
    match cli_name.as_str() {
        "copilot" => PcAgentRuntimeChoice {
            cli_name,
            copilot_model: option.and_then(|o| o.model.clone()),
            codex_reasoning_effort: None,
            model_label: option
                .map(AiCliOption::display_label)
                .or_else(|| agent_name.map(String::from)),
            error: None,
        },
        "codex" => PcAgentRuntimeChoice {
            cli_name,
            copilot_model: option.and_then(|o| o.model.clone()),
            codex_reasoning_effort: option.and_then(|o| o.reasoning_effort.clone()),
            model_label: option
                .map(AiCliOption::display_label)
                .or_else(|| agent_name.map(String::from)),
            error: None,
        },
        "server-runtime" => PcAgentRuntimeChoice {
            cli_name,
            copilot_model: None,
            codex_reasoning_effort: None,
            model_label: Some("一龙服务器模型（Route C）".to_string()),
            error: None,
        },
        "api-runtime" => PcAgentRuntimeChoice {
            cli_name,
            copilot_model: None,
            codex_reasoning_effort: None,
            model_label: Some("本机 API Runtime（Route B）".to_string()),
            error: None,
        },
        _ => PcAgentRuntimeChoice {
            cli_name,
            copilot_model: None,
            codex_reasoning_effort: None,
            model_label: option
                .map(AiCliOption::display_label)
                .or_else(|| agent_name.map(String::from)),
            error: None,
        },
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

fn choose_cli_for_runtime(
    allowed_clis: &[String],
    dev_runtime: Option<&NodeDevRuntimeProfile>,
    requested_cli: String,
    route_preference: Option<PcRuntimeRoutePreference>,
) -> Result<String, String> {
    if let Some(preference) = route_preference {
        return choose_forced_route(allowed_clis, dev_runtime, requested_cli, preference);
    }
    if cli_available(allowed_clis, &requested_cli) {
        return Ok(requested_cli);
    }
    if let Some(route_a_cli) = first_available_route_a_cli(allowed_clis) {
        return Ok(route_a_cli);
    }
    let Some(runtime) = dev_runtime else {
        return Ok(requested_cli);
    };
    // Route B 必须排在 Route C 前面：用户配置了自己的 API key 时，
    // 模型调用和本地工具循环都由用户 PC 自己承担，不消耗平台服务器模型。
    if runtime.api_runtime_ready {
        return Ok("api-runtime".to_string());
    }
    if runtime.server_runtime_ready {
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
            if dev_runtime.is_some_and(|runtime| runtime.server_runtime_ready) {
                Ok("server-runtime".to_string())
            } else {
                Err("已强制 Route C，但服务器模型 Runtime 未就绪；请确认 Win 客户端已登录并连接云端，或切回自动。".to_string())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        choose_cli_for_runtime, cli_name_from_parts, first_available_route_a_cli,
        PcRuntimeRoutePreference,
    };
    use homecli_proto::NodeDevRuntimeProfile;

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
}
