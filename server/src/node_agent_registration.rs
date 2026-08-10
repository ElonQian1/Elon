use anyhow::{anyhow, bail, Result};
use serde::Deserialize;
use std::time::Duration;

use crate::{
    machine_label, node_agent_cloud_net, node_agent_endpoint_credentials::EndpointAuthorityBinding,
    Credentials, NodeConfig,
};

pub(crate) enum ProvisionNodeOutcome {
    Legacy(Credentials),
    SecureBootstrapAnchor(SecureBootstrapAnchor),
    EndpointAuthorityRequired(EndpointAuthorityBinding),
}

pub(crate) struct SecureBootstrapAnchor {
    pub(crate) agent_id: String,
    pub(crate) owner_user_id: String,
}

/// 注册 legacy 节点锚；secure bootstrap 只接收 agent/owner 元数据，
/// legacy 模式才接收并返回旧式 secret。
pub(crate) async fn provision_node(
    cfg: &NodeConfig,
    token: &str,
    existing: Option<&Credentials>,
    install_id: &str,
) -> Result<ProvisionNodeOutcome> {
    let secure_endpoint = cfg.endpoint_https_origin.as_deref();
    let base = secure_endpoint.unwrap_or(&cfg.cloud_http_url);
    let url = registration_endpoint(base)?;
    let client = if secure_endpoint.is_some() {
        crate::node_agent_endpoint_credentials::secure_https_client(Duration::from_secs(15))?
    } else {
        node_agent_cloud_net::direct_cloud_client_or_default(Duration::from_secs(15))
    };
    let device_name = machine_label();
    let mut body = serde_json::json!({
        "label": device_name,
        "device_name": device_name,
        "install_id": install_id,
    });
    if secure_endpoint.is_none() {
        if let Some(creds) = existing {
            body["existing_agent_id"] = serde_json::Value::String(creds.agent_id.clone());
            body["existing_secret"] = serde_json::Value::String(creds.agent_secret.clone());
        }
    }
    let resp = client
        .post(&url)
        .bearer_auth(token)
        .json(&body)
        .send()
        .await
        .map_err(|error| {
            anyhow!(
                "无法连接一龙云端注册节点（{}）。首次绑定必须联网；已有节点凭证不会因此失效，仍可使用本机离线任务。底层错误: {}",
                base,
                error
            )
        })?;
    let status = resp.status();
    if secure_endpoint.is_some() && status == reqwest::StatusCode::CONFLICT {
        let conflict: EndpointAuthorityRequiredResponse =
            crate::node_agent_endpoint_credentials::read_https_json_limited(resp).await?;
        if conflict.error != "NODE_ENDPOINT_AUTHORITY_REQUIRED" {
            bail!("secure 节点注册冲突响应无效");
        }
        conflict.endpoint_authority.validate()?;
        if conflict.endpoint_authority.install_id != install_id {
            bail!("NODE_ENDPOINT_BOOTSTRAP_IDENTITY_DRIFT");
        }
        return Ok(ProvisionNodeOutcome::EndpointAuthorityRequired(
            conflict.endpoint_authority,
        ));
    }
    if !status.is_success() {
        if secure_endpoint.is_some() {
            let error: serde_json::Value =
                crate::node_agent_endpoint_credentials::read_https_json_limited(resp)
                    .await
                    .unwrap_or_default();
            let code = error
                .get("error")
                .and_then(|value| value.as_str())
                .unwrap_or("NODE_ENDPOINT_BOOTSTRAP_REGISTRATION_DENIED");
            return Err(anyhow!("secure 节点注册失败 {status}: {code}"));
        }
        let body = resp.text().await.unwrap_or_default();
        return Err(anyhow!("注册节点失败 {status}: {body}"));
    }
    if secure_endpoint.is_some() {
        let registered: SecureRegistrationResponse =
            crate::node_agent_endpoint_credentials::read_https_json_limited(resp).await?;
        validate_registration_identity(&registered.agent_id, &registered.owner_user_id)?;
        return Ok(ProvisionNodeOutcome::SecureBootstrapAnchor(
            SecureBootstrapAnchor {
                agent_id: registered.agent_id,
                owner_user_id: registered.owner_user_id,
            },
        ));
    }
    let registered: LegacyRegistrationResponse = resp.json().await?;
    validate_registration_identity(&registered.agent_id, &registered.owner_user_id)?;
    if registered.agent_secret.trim().is_empty() {
        bail!("节点注册响应缺少 legacy anchor secret");
    }
    Ok(ProvisionNodeOutcome::Legacy(Credentials {
        agent_id: registered.agent_id,
        agent_secret: registered.agent_secret,
        owner_user_id: registered.owner_user_id,
        user_token: Some(token.to_string()),
    }))
}

#[derive(Deserialize)]
struct LegacyRegistrationResponse {
    agent_id: String,
    agent_secret: String,
    owner_user_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SecureRegistrationResponse {
    agent_id: String,
    owner_user_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EndpointAuthorityRequiredResponse {
    error: String,
    endpoint_authority: EndpointAuthorityBinding,
}

fn validate_registration_identity(agent_id: &str, owner_user_id: &str) -> Result<()> {
    if agent_id.is_empty()
        || agent_id != agent_id.trim()
        || agent_id.len() > 160
        || owner_user_id.is_empty()
        || owner_user_id != owner_user_id.trim()
        || owner_user_id.len() > 160
        || agent_id.chars().any(|character| character.is_control())
        || owner_user_id
            .chars()
            .any(|character| character.is_control())
    {
        bail!("NODE_ENDPOINT_BOOTSTRAP_REGISTRATION_IDENTITY_INVALID");
    }
    Ok(())
}

fn registration_endpoint(cloud_http_url: &str) -> Result<String> {
    let base = cloud_http_url.trim().trim_end_matches('/');
    let parsed = reqwest::Url::parse(base)
        .map_err(|error| anyhow!("NODE_CLOUD_HTTP_URL 不是有效地址: {error}"))?;
    if !matches!(parsed.scheme(), "http" | "https") || parsed.host_str().is_none() {
        return Err(anyhow!(
            "NODE_CLOUD_HTTP_URL 必须是有效的 http/https 云端地址"
        ));
    }
    if parsed.port_or_known_default() == Some(9)
        && parsed
            .host_str()
            .is_some_and(|host| matches!(host, "127.0.0.1" | "localhost" | "::1"))
    {
        return Err(anyhow!(
            "节点云端地址被配置为 {base}；127.0.0.1:9 是不可用的测试/代理地址。请检查安装目录 _internal/node-agent.env 或系统环境变量 NODE_CLOUD_URL、NODE_CLOUD_HTTP_URL"
        ));
    }
    Ok(format!("{base}/api/me/nodes/register"))
}

#[cfg(test)]
mod tests {
    use super::registration_endpoint;

    #[test]
    fn registration_endpoint_rejects_loopback_discard_port_with_actionable_error() {
        let error = registration_endpoint("http://127.0.0.1:9")
            .expect_err("discard port must not be used as cloud endpoint")
            .to_string();
        assert!(error.contains("127.0.0.1:9"));
        assert!(error.contains("NODE_CLOUD_URL"));
    }

    #[test]
    fn registration_endpoint_accepts_production_and_local_development_servers() {
        assert_eq!(
            registration_endpoint("http://43.139.149.158:8080/").unwrap(),
            "http://43.139.149.158:8080/api/me/nodes/register"
        );
        assert_eq!(
            registration_endpoint("http://127.0.0.1:8080").unwrap(),
            "http://127.0.0.1:8080/api/me/nodes/register"
        );
    }
}
