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
}

impl PcAgentRuntimeChoice {
    pub(crate) fn progress_label(&self) -> &str {
        self.model_label
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or(self.cli_name.as_str())
    }
}

pub(crate) async fn choose_pc_agent_runtime(
    state: &Arc<AppState>,
    agent_id: &str,
    agent_name: Option<&str>,
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
    );

    choice_from_cli(chosen_cli, option.as_ref(), agent_name)
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
        },
        "codex" => PcAgentRuntimeChoice {
            cli_name,
            copilot_model: option.and_then(|o| o.model.clone()),
            codex_reasoning_effort: option.and_then(|o| o.reasoning_effort.clone()),
            model_label: option
                .map(AiCliOption::display_label)
                .or_else(|| agent_name.map(String::from)),
        },
        "server-runtime" => PcAgentRuntimeChoice {
            cli_name,
            copilot_model: None,
            codex_reasoning_effort: None,
            model_label: Some("一龙服务器模型（Route C）".to_string()),
        },
        "api-runtime" => PcAgentRuntimeChoice {
            cli_name,
            copilot_model: None,
            codex_reasoning_effort: None,
            model_label: Some("本机 API Runtime（Route B）".to_string()),
        },
        _ => PcAgentRuntimeChoice {
            cli_name,
            copilot_model: None,
            codex_reasoning_effort: None,
            model_label: option
                .map(AiCliOption::display_label)
                .or_else(|| agent_name.map(String::from)),
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
) -> String {
    if cli_available(allowed_clis, &requested_cli) {
        return requested_cli;
    }
    if let Some(route_a_cli) = first_available_route_a_cli(allowed_clis) {
        return route_a_cli;
    }
    let Some(runtime) = dev_runtime else {
        return requested_cli;
    };
    // Route B 必须排在 Route C 前面：用户配置了自己的 API key 时，
    // 模型调用和本地工具循环都由用户 PC 自己承担，不消耗平台服务器模型。
    if runtime.api_runtime_ready {
        return "api-runtime".to_string();
    }
    if runtime.server_runtime_ready {
        return "server-runtime".to_string();
    }
    requested_cli
}

#[cfg(test)]
mod tests {
    use super::{choose_cli_for_runtime, cli_name_from_parts, first_available_route_a_cli};
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
            choose_cli_for_runtime(&[], Some(&runtime), "codex".to_string()),
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
            choose_cli_for_runtime(&[], Some(&runtime), "codex".to_string()),
            "server-runtime"
        );
    }
}
