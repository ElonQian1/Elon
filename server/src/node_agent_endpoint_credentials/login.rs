use std::time::Duration;

use anyhow::{anyhow, bail, Result};
use serde::Deserialize;

use crate::{machine_label, NodeConfig};

const MAX_SECURE_ACCOUNT_BYTES: usize = 320;
const MAX_SECURE_PASSWORD_BYTES: usize = 4_096;
const MAX_SECURE_BEARER_BYTES: usize = 2_048;

pub(crate) fn endpoint_origin_from_env() -> Result<Option<String>> {
    std::env::var("NODE_ENDPOINT_HTTPS_ORIGIN")
        .ok()
        .filter(|value| !value.is_empty())
        .map(|value| super::normalize_endpoint_https_origin(&value))
        .transpose()
}

pub(crate) fn bind_persisted_endpoint_origin(
    config: &mut NodeConfig,
    persisted_origin: Option<&str>,
) -> Result<()> {
    let Some(persisted_origin) = persisted_origin else {
        return Ok(());
    };
    let persisted_origin = super::normalize_endpoint_https_origin(persisted_origin)?;
    if config
        .endpoint_https_origin
        .as_deref()
        .is_some_and(|configured| configured != persisted_origin)
    {
        bail!("NODE_ENDPOINT_HTTPS_ORIGIN_DRIFT: 环境配置与已固定 endpoint origin 不一致");
    }
    config.endpoint_https_origin = Some(persisted_origin);
    Ok(())
}

pub(crate) async fn cloud_login(
    config: &NodeConfig,
    account: &str,
    password: &str,
) -> Result<String> {
    let secure_origin = config.endpoint_https_origin.as_deref();
    if secure_origin.is_some()
        && (account.is_empty()
            || account != account.trim()
            || account.len() > MAX_SECURE_ACCOUNT_BYTES
            || account.chars().any(char::is_control)
            || password.is_empty()
            || password.len() > MAX_SECURE_PASSWORD_BYTES)
    {
        bail!("NODE_ENDPOINT_BOOTSTRAP_LOGIN_REQUEST_INVALID");
    }
    let base = secure_origin.unwrap_or(&config.cloud_http_url);
    let url = format!("{}/api/auth/login", base.trim_end_matches('/'));
    let client = if secure_origin.is_some() {
        super::secure_https_client(Duration::from_secs(15))?
    } else {
        crate::node_agent_cloud_net::direct_cloud_client_or_default(Duration::from_secs(15))
    };
    let body = if secure_origin.is_some() {
        serde_json::json!({
            "account": account,
            "password": password,
        })
    } else {
        serde_json::json!({
            "account": account,
            "password": password,
            "device_name": machine_label(),
        })
    };
    let response = client.post(url).json(&body).send().await?;
    if !response.status().is_success() {
        let status = response.status();
        if secure_origin.is_some() {
            let body: serde_json::Value = super::read_https_json_limited(response)
                .await
                .unwrap_or_default();
            let code = body
                .get("error")
                .and_then(|value| value.as_str())
                .unwrap_or("NODE_ENDPOINT_BOOTSTRAP_LOGIN_DENIED");
            return Err(anyhow!("endpoint 登录失败 {status}: {code}"));
        }
        let body = response.text().await.unwrap_or_default();
        return Err(anyhow!("登录失败 {status}: {body}"));
    }
    let body: LoginResponse = if secure_origin.is_some() {
        super::read_https_json_limited(response).await?
    } else {
        response.json().await?
    };
    let max_token_bytes = if secure_origin.is_some() {
        MAX_SECURE_BEARER_BYTES
    } else {
        8_192
    };
    if body.token.is_empty()
        || body.token.len() > max_token_bytes
        || body.token.contains(['\r', '\n', '\0'])
    {
        bail!("登录响应 token 无效");
    }
    Ok(body.token)
}

#[derive(Deserialize)]
struct LoginResponse {
    token: String,
}
