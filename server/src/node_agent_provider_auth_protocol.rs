//! Provider-specific JSON-RPC authentication payload parsing.

use anyhow::{Context, Result};
use serde_json::{json, Value};

pub(crate) fn client_info() -> Value {
    json!({
        "name": "elon_win_client",
        "title": "Elon Win Client",
        "version": env!("CARGO_PKG_VERSION")
    })
}

pub(crate) struct CodexLoginInstructions {
    pub(crate) upstream_login_id: Option<String>,
    pub(crate) verification_url: Option<String>,
    pub(crate) user_code: Option<String>,
    pub(crate) auth_url: Option<String>,
}

fn https_url(value: Option<&str>) -> Option<String> {
    let value = value?.trim();
    let parsed = reqwest::Url::parse(value).ok()?;
    (parsed.scheme() == "https").then(|| value.chars().take(2048).collect())
}

pub(crate) fn codex_login_instructions(result: &Value) -> Result<CodexLoginInstructions> {
    let login_type = result
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let instructions = CodexLoginInstructions {
        upstream_login_id: result
            .get("loginId")
            .and_then(Value::as_str)
            .map(str::to_string),
        verification_url: https_url(result.get("verificationUrl").and_then(Value::as_str)),
        user_code: result
            .get("userCode")
            .and_then(Value::as_str)
            .map(|value| value.chars().take(64).collect()),
        auth_url: https_url(result.get("authUrl").and_then(Value::as_str)),
    };
    let valid = match login_type {
        "chatgptDeviceCode" => {
            instructions.verification_url.is_some() && instructions.user_code.is_some()
        }
        "chatgpt" => instructions.auth_url.is_some(),
        _ => false,
    };
    valid
        .then_some(instructions)
        .context("Codex 没有返回可用的官方登录地址")
}

pub(crate) fn select_gemini_auth_method(initialize: &Value) -> Option<String> {
    let methods = initialize.get("authMethods")?.as_array()?;
    methods
        .iter()
        .find(|method| method.get("id").and_then(Value::as_str) == Some("oauth-personal"))
        .or_else(|| {
            methods.iter().find(|method| {
                method
                    .get("type")
                    .and_then(Value::as_str)
                    .unwrap_or("agent")
                    == "agent"
                    && method
                        .get("name")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_ascii_lowercase()
                        .contains("google")
            })
        })
        .and_then(|method| method.get("id"))
        .and_then(Value::as_str)
        .map(str::to_string)
}
