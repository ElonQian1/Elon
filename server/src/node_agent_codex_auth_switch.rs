use anyhow::{bail, Context, Result};
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::Duration;
use tracing::warn;

use super::{node_agent_codex_vault, node_agent_codex_vault_emergency, NodeRuntime};

#[derive(Debug, Deserialize, Default)]
struct SharingStatusResponse {
    #[serde(default)]
    grants: Vec<SharingGrantSummary>,
    #[serde(default)]
    sharing: Option<SharingStatusPayload>,
}

#[derive(Debug, Deserialize, Default)]
struct SharingStatusPayload {
    #[serde(default)]
    grants: Vec<SharingGrantSummary>,
}

#[derive(Debug, Clone, Deserialize, Default)]
struct SharingGrantSummary {
    #[serde(default)]
    provider_user_id: Option<String>,
    #[serde(default)]
    provider_account: Option<String>,
    #[serde(default)]
    provider_nickname: Option<String>,
    #[serde(default)]
    consumer_user_id: Option<String>,
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    provider_vault_available: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct AutoSharedProvider {
    provider_user_id: Option<String>,
    provider_account: Option<String>,
    label: String,
}

pub(crate) async fn try_after_failure(
    runtime: &Arc<NodeRuntime>,
    stdout_text: &str,
    stderr_text: &str,
) -> Option<String> {
    match node_agent_codex_vault::try_auto_switch_after_codex_failure(
        runtime,
        stdout_text,
        stderr_text,
    )
    .await
    {
        Ok(Some(message)) => return Some(message),
        Ok(None) => {}
        Err(error) => warn!("Codex 保险箱自动切换检查失败: {error:#}"),
    }
    match try_shared_provider_after_failure(runtime, stdout_text, stderr_text).await {
        Ok(message) => message,
        Err(error) => {
            warn!("Codex 共享授权自动切换检查失败: {error:#}");
            None
        }
    }
}

async fn try_shared_provider_after_failure(
    rt: &Arc<NodeRuntime>,
    stdout_text: &str,
    stderr_text: &str,
) -> Result<Option<String>> {
    let combined = format!("{stdout_text}\n{stderr_text}");
    let classified = crate::errors::classify_ai_error(&combined);
    if !matches!(
        classified.category,
        crate::errors::AiErrorCategory::Quota | crate::errors::AiErrorCategory::AuthConfig
    ) {
        return Ok(None);
    }
    let reason = classified
        .operator_detail
        .as_deref()
        .unwrap_or(classified.code)
        .to_string();
    let provider = match auto_shared_provider(rt).await {
        Ok(Some(provider)) => provider,
        Ok(None) => return Ok(None),
        Err(error) => {
            warn!("Codex 共享授权自动切换前置检查失败: {error:#}");
            return Ok(None);
        }
    };
    let lease = match node_agent_codex_vault_emergency::restore_emergency_from_cloud(
        rt,
        node_agent_codex_vault_emergency::EmergencyRestoreRequest {
            provider_user_id: provider.provider_user_id.clone(),
            provider_account: provider.provider_account.clone(),
            purpose: Some("auto_switch_to_shared_codex_after_failure".to_string()),
            failure_reason: Some(reason),
        },
    )
    .await
    {
        Ok(lease) => lease,
        Err(error) => {
            warn!("Codex 共享授权自动切换失败: {error:#}");
            return Ok(None);
        }
    };
    Ok(Some(format!(
        "Codex 当前账号额度或认证不可用，已自动切换到 {} 的共享账号{}，正在重试本轮任务。",
        provider.label,
        lease
            .account_hint_hash
            .as_deref()
            .map(|hint| format!(" ({hint})"))
            .unwrap_or_default()
    )))
}

async fn auto_shared_provider(rt: &Arc<NodeRuntime>) -> Result<Option<AutoSharedProvider>> {
    if let Some(provider) = configured_auto_shared_provider() {
        return Ok(Some(provider));
    }
    let creds = rt.creds().await.context("请先绑定本机节点账号")?;
    let owner_user_id = creds.owner_user_id.clone();
    let token = creds
        .user_token
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .context("本机节点缺少云端登录 token，请重新绑定")?
        .to_string();
    let url = format!(
        "{}/api/me/codex-vault/sharing",
        rt.cloud_http_url().trim_end_matches('/')
    );
    let value = cloud_get(&url, &token).await?;
    let status: SharingStatusResponse =
        serde_json::from_value(value).context("云端共享授权状态响应格式不正确")?;
    Ok(select_auto_shared_provider(
        &status.into_grants(),
        &owner_user_id,
    ))
}

impl SharingStatusResponse {
    fn into_grants(self) -> Vec<SharingGrantSummary> {
        if !self.grants.is_empty() {
            return self.grants;
        }
        self.sharing
            .map(|sharing| sharing.grants)
            .unwrap_or_default()
    }
}

fn configured_auto_shared_provider() -> Option<AutoSharedProvider> {
    let provider_user_id = env_nonempty("ELON_CODEX_AUTO_SHARED_PROVIDER_USER_ID");
    let provider_account = env_nonempty("ELON_CODEX_AUTO_SHARED_PROVIDER_ACCOUNT")
        .or_else(|| env_nonempty("ELON_CODEX_AUTO_SHARED_PROVIDER"))
        .or_else(|| env_nonempty("NODE_CODEX_AUTO_SHARED_PROVIDER"));
    if provider_user_id.is_none() && provider_account.is_none() {
        return None;
    }
    let label = provider_account
        .clone()
        .or_else(|| provider_user_id.clone())
        .unwrap_or_else(|| "授权提供方".to_string());
    Some(AutoSharedProvider {
        provider_user_id,
        provider_account,
        label,
    })
}

fn env_nonempty(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn select_auto_shared_provider(
    grants: &[SharingGrantSummary],
    owner_user_id: &str,
) -> Option<AutoSharedProvider> {
    grants
        .iter()
        .find(|grant| grant_is_auto_share_candidate(grant, owner_user_id))
        .and_then(AutoSharedProvider::from_grant)
}

fn grant_is_auto_share_candidate(grant: &SharingGrantSummary, owner_user_id: &str) -> bool {
    let status = grant.status.as_deref().unwrap_or("active").trim();
    if !status.eq_ignore_ascii_case("active") {
        return false;
    }
    if grant.provider_vault_available == Some(false) {
        return false;
    }
    let provider_user_id = clean_grant_value(grant.provider_user_id.as_deref());
    let Some(provider_user_id) = provider_user_id else {
        return false;
    };
    let owner_user_id = owner_user_id.trim();
    if owner_user_id.is_empty() {
        return true;
    }
    if provider_user_id == owner_user_id {
        return false;
    }
    clean_grant_value(grant.consumer_user_id.as_deref()) == Some(owner_user_id)
}

impl AutoSharedProvider {
    fn from_grant(grant: &SharingGrantSummary) -> Option<Self> {
        let provider_user_id = clean_grant_value(grant.provider_user_id.as_deref())?.to_string();
        let provider_account =
            clean_grant_value(grant.provider_account.as_deref()).map(ToOwned::to_owned);
        let label = clean_grant_value(grant.provider_nickname.as_deref())
            .or(provider_account.as_deref())
            .unwrap_or(provider_user_id.as_str())
            .to_string();
        Some(Self {
            provider_user_id: Some(provider_user_id),
            provider_account,
            label,
        })
    }
}

fn clean_grant_value(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

async fn cloud_get(url: &str, token: &str) -> Result<Value> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .unwrap_or_default();
    let resp = client.get(url).bearer_auth(token).send().await?;
    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();
    let value: Value = serde_json::from_str(&text).unwrap_or_else(|_| json!({ "raw": text }));
    if !status.is_success() {
        let message = value
            .get("error")
            .or_else(|| value.get("message"))
            .and_then(Value::as_str)
            .unwrap_or("云端请求失败");
        bail!("云端返回 {}: {}", status, message);
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::{
        configured_auto_shared_provider, select_auto_shared_provider, AutoSharedProvider,
        SharingGrantSummary,
    };
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn auto_shared_provider_selects_incoming_active_available_grant() {
        let grants = vec![
            grant("provider-a", "consumer-other", true, "active"),
            grant("provider-b", "consumer-1", true, "active")
                .with_account("15160532860")
                .with_nickname("全嘉"),
        ];

        let selected = select_auto_shared_provider(&grants, "consumer-1");

        assert_eq!(
            selected,
            Some(AutoSharedProvider {
                provider_user_id: Some("provider-b".to_string()),
                provider_account: Some("15160532860".to_string()),
                label: "全嘉".to_string(),
            })
        );
    }

    #[test]
    fn auto_shared_provider_skips_unavailable_and_revoked_grants() {
        let grants = vec![
            grant("provider-a", "consumer-1", false, "active"),
            grant("provider-b", "consumer-1", true, "revoked"),
            grant("provider-c", "consumer-1", true, "active"),
        ];

        let selected = select_auto_shared_provider(&grants, "consumer-1");

        assert_eq!(
            selected,
            Some(AutoSharedProvider {
                provider_user_id: Some("provider-c".to_string()),
                provider_account: None,
                label: "provider-c".to_string(),
            })
        );
    }

    #[test]
    fn configured_auto_shared_provider_takes_precedence() {
        let _guard = ENV_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let old_user_id = std::env::var("ELON_CODEX_AUTO_SHARED_PROVIDER_USER_ID").ok();
        let old_account = std::env::var("ELON_CODEX_AUTO_SHARED_PROVIDER_ACCOUNT").ok();
        let old_generic = std::env::var("ELON_CODEX_AUTO_SHARED_PROVIDER").ok();
        let old_node_generic = std::env::var("NODE_CODEX_AUTO_SHARED_PROVIDER").ok();
        std::env::set_var("ELON_CODEX_AUTO_SHARED_PROVIDER_USER_ID", "usr_quanjia");
        std::env::set_var("ELON_CODEX_AUTO_SHARED_PROVIDER_ACCOUNT", "15160532860");
        std::env::remove_var("ELON_CODEX_AUTO_SHARED_PROVIDER");
        std::env::remove_var("NODE_CODEX_AUTO_SHARED_PROVIDER");

        let selected = configured_auto_shared_provider();

        restore_env("ELON_CODEX_AUTO_SHARED_PROVIDER_USER_ID", old_user_id);
        restore_env("ELON_CODEX_AUTO_SHARED_PROVIDER_ACCOUNT", old_account);
        restore_env("ELON_CODEX_AUTO_SHARED_PROVIDER", old_generic);
        restore_env("NODE_CODEX_AUTO_SHARED_PROVIDER", old_node_generic);

        assert_eq!(
            selected,
            Some(AutoSharedProvider {
                provider_user_id: Some("usr_quanjia".to_string()),
                provider_account: Some("15160532860".to_string()),
                label: "15160532860".to_string(),
            })
        );
    }

    fn grant(
        provider_user_id: &str,
        consumer_user_id: &str,
        provider_vault_available: bool,
        status: &str,
    ) -> SharingGrantSummary {
        SharingGrantSummary {
            provider_user_id: Some(provider_user_id.to_string()),
            consumer_user_id: Some(consumer_user_id.to_string()),
            provider_vault_available: Some(provider_vault_available),
            status: Some(status.to_string()),
            ..SharingGrantSummary::default()
        }
    }

    trait GrantTestExt {
        fn with_account(self, account: &str) -> Self;
        fn with_nickname(self, nickname: &str) -> Self;
    }

    impl GrantTestExt for SharingGrantSummary {
        fn with_account(mut self, account: &str) -> Self {
            self.provider_account = Some(account.to_string());
            self
        }

        fn with_nickname(mut self, nickname: &str) -> Self {
            self.provider_nickname = Some(nickname.to_string());
            self
        }
    }

    fn restore_env(name: &str, value: Option<String>) {
        match value {
            Some(value) => std::env::set_var(name, value),
            None => std::env::remove_var(name),
        }
    }
}
