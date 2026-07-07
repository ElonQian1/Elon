use super::*;
use std::collections::HashMap;

fn agent(name: &str, model: &str) -> AgentConfig {
    AgentConfig {
        name: name.to_string(),
        api_base: "https://example.invalid/v1".to_string(),
        api_key: "test-key".to_string(),
        model: model.to_string(),
        embedding_model: None,
        usage_mode: None,
    }
}

fn agent_with_usage(name: &str, model: &str, usage_mode: Option<&str>) -> AgentConfig {
    AgentConfig {
        usage_mode: usage_mode.map(str::to_string),
        ..agent(name, model)
    }
}

#[test]
fn orders_default_agent_first_then_stable_names() {
    let mut agents = HashMap::new();
    agents.insert("zeta".to_string(), agent("zeta", "z"));
    agents.insert("default".to_string(), agent("default", "d"));
    agents.insert("alpha".to_string(), agent("alpha", "a"));
    let config = AgentsConfig {
        agents,
        default_agent: "default".to_string(),
    };

    let names = ordered_server_api_agents(&config)
        .into_iter()
        .map(|agent| agent.name)
        .collect::<Vec<_>>();
    assert_eq!(names, ["default", "alpha", "zeta"]);
}

#[test]
fn orders_only_server_side_agents() {
    let mut agents = HashMap::new();
    agents.insert("default".to_string(), agent("default", "d"));
    agents.insert(
        "copilot:gpt-4o".to_string(),
        agent("copilot:gpt-4o", "gpt-4o"),
    );
    agents.insert(
        "user-proxy".to_string(),
        agent_with_usage("user-proxy", "u", Some("user_api_key_proxy")),
    );
    agents.insert("server-alt".to_string(), agent("server-alt", "s"));
    let config = AgentsConfig {
        agents,
        default_agent: "default".to_string(),
    };

    let names = ordered_server_api_agents(&config)
        .into_iter()
        .map(|agent| agent.name)
        .collect::<Vec<_>>();
    assert_eq!(names, ["default", "server-alt"]);
}

#[test]
fn retryable_errors_exclude_user_billing_failures() {
    assert!(is_retryable_agent_error(
        "当前 AI 模型额度已用尽或接口不可用，请切换可用模型"
    ));
    assert!(is_retryable_agent_error(
        "当前 AI 模型已下线，请管理员迁移到 TokenHub 的可用模型"
    ));
    assert!(is_retryable_agent_error(
        "当前 AI 模型额度已用尽或未开启后付费，请切换可用模型"
    ));
    assert!(is_retryable_agent_error("AI 请求超时，请稍后重试"));
    assert!(!is_retryable_agent_error(
        "余额不足（当前 0 分），请联系管理员充值后继续使用"
    ));
    assert!(!is_retryable_agent_error("用户已被封禁"));
}
