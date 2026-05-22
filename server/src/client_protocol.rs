use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ClientChatPayload {
    pub user_id: Option<String>,
    pub project_id: Option<String>,
    pub message: Option<String>,
    pub agent: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentRequest {
    pub user_id: String,
    pub workspace_user_id: String,
    pub content: String,
    pub agent: Option<String>,
}

pub fn parse_client_message(raw: &str) -> AgentRequest {
    if let Ok(payload) = serde_json::from_str::<ClientChatPayload>(raw) {
        let user_id = payload
            .user_id
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| "default".to_string());
        let workspace_user_id = workspace_user_id(&user_id, payload.project_id.as_deref());
        let content = payload.message.unwrap_or_else(|| raw.to_string());
        let agent = payload
            .agent
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());

        AgentRequest {
            user_id,
            workspace_user_id,
            content,
            agent,
        }
    } else {
        AgentRequest {
            user_id: "default".to_string(),
            workspace_user_id: "default".to_string(),
            content: raw.to_string(),
            agent: None,
        }
    }
}

pub fn workspace_user_id(user_id: &str, project_id: Option<&str>) -> String {
    project_id
        .map(|project_id| format!("{}__{}", user_id, safe_workspace_part(project_id)))
        .unwrap_or_else(|| user_id.to_string())
}

fn safe_workspace_part(value: &str) -> String {
    let safe = value
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .take(40)
        .collect::<String>();
    if safe.is_empty() {
        "project".into()
    } else {
        safe
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_text_uses_default_workspace() {
        let req = parse_client_message("hello");

        assert_eq!(req.user_id, "default");
        assert_eq!(req.workspace_user_id, "default");
        assert_eq!(req.content, "hello");
        assert_eq!(req.agent, None);
    }

    #[test]
    fn json_payload_is_shared_by_web_apk_and_future_clients() {
        let req = parse_client_message(
            r#"{"user_id":"u1","project_id":"my app/../../bad","message":"build","agent":" codex_cli "}"#,
        );

        assert_eq!(req.user_id, "u1");
        assert_eq!(req.workspace_user_id, "u1__myappbad");
        assert_eq!(req.content, "build");
        assert_eq!(req.agent.as_deref(), Some("codex_cli"));
    }
}
