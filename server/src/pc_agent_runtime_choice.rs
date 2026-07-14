// server/src/pc_agent_runtime_choice.rs

mod capability_wait;

use capability_wait::agent_summary_after_capability_scan;
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
    let summary = agent_summary_after_capability_scan(state, agent_id, route_preference).await;
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
            model_label: Some("AI方式不可用".to_string()),
            error: Some(error),
        },
    }
}

#[path = "pc_agent_runtime_choice_impl.rs"]
mod impl_funcs;
use self::impl_funcs::*;
