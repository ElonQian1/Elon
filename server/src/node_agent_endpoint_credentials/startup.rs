use anyhow::Result;
use tracing::{info, warn};

use super::EndpointCredentialManager;
use crate::node_agent_config::{
    initial_credentials, load_persisted, save_persisted, Credentials, NodeConfig, PersistedState,
};
use crate::node_agent_registration::{provision_node, ProvisionNodeOutcome};

pub(crate) async fn load_and_bind(config: &mut NodeConfig) -> Result<EndpointCredentialManager> {
    let credentials = EndpointCredentialManager::load_default()?;
    let persisted_origin = credentials.endpoint_https_origin().await;
    super::bind_persisted_endpoint_origin(config, persisted_origin.as_deref())?;
    if let Some(origin) = config.endpoint_https_origin.as_deref() {
        crate::node_agent_proxy::ensure_cloud_no_proxy(origin, origin);
    }
    Ok(credentials)
}

pub(crate) fn clear_legacy_credentials_before_startup(
    config: &NodeConfig,
    persisted: &mut PersistedState,
) -> Result<()> {
    if config.endpoint_https_origin.is_some() {
        persisted.set_credentials(None);
        save_persisted(persisted)?;
    }
    Ok(())
}

pub(crate) async fn initial_runtime_credentials(
    config: &NodeConfig,
    persisted: &mut PersistedState,
    install_id: &str,
) -> Result<Option<Credentials>> {
    if config.endpoint_https_origin.is_some() {
        // NODE_AGENT_* / NODE_USER_TOKEN are ignored. Disk fields were
        // atomically erased immediately after node.json was loaded.
        info!("secure endpoint 模式已清除 legacy 节点凭据；账号密码 step-up 前不会注册或连接");
        return Ok(None);
    }

    let mut credentials = initial_credentials(persisted);
    if credentials.is_none() {
        let token = std::env::var("NODE_USER_TOKEN")
            .ok()
            .filter(|value| !value.is_empty())
            .or_else(|| persisted.user_token.clone());
        if let Some(token) = token {
            info!("检测到登录 token，正在自动注册节点…");
            match provision_node(config, &token, None, install_id).await {
                Ok(ProvisionNodeOutcome::Legacy(registered)) => {
                    info!("✅ 节点已自动注册: {}", registered.agent_id);
                    let mut next_persisted = load_persisted()?;
                    next_persisted.set_install_id(install_id);
                    next_persisted.set_credentials(Some(&registered));
                    save_persisted(&next_persisted)?;
                    credentials = Some(registered);
                }
                Ok(
                    ProvisionNodeOutcome::SecureBootstrapAnchor(_)
                    | ProvisionNodeOutcome::EndpointAuthorityRequired(_),
                ) => {
                    warn!("legacy 启动注册意外收到 endpoint authority；已拒绝自动采纳")
                }
                Err(error) => warn!("自动注册失败（可在管理页重新登录）: {error:#}"),
            }
        }
    }
    Ok(credentials)
}
