use anyhow::{bail, Context, Result};
use serde::Deserialize;
use serde_json::{json, Value};
use std::future::Future;
use std::sync::Arc;
use std::time::Duration;
use tracing::{info, warn};

use super::{node_agent_codex_vault, node_agent_codex_vault_emergency, NodeRuntime};

pub(crate) struct CodexAuthSwitchOutcome {
    pub(crate) message: String,
    pub(crate) frozen_codex_home: crate::node_agent_codex_child_env::FrozenCodexHome,
}

struct CodexAuthSwitchCandidate {
    message: String,
    cloud_control_deadline: Option<String>,
    cloud_control_issued_at: Option<String>,
    cloud_control_ttl_ms: Option<u64>,
}

#[derive(Debug)]
pub(crate) struct CodexAuthAttemptState {
    enabled: bool,
    owner_vault_attempted: bool,
    shared_provider_snapshot: Option<Vec<AutoSharedProvider>>,
    attempted_shared_providers: Vec<AutoSharedProvider>,
}

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

impl CodexAuthAttemptState {
    pub(crate) fn new(enabled: bool) -> Self {
        Self {
            enabled,
            owner_vault_attempted: false,
            shared_provider_snapshot: None,
            attempted_shared_providers: Vec::new(),
        }
    }

    fn reserve_owner_vault_attempt(&mut self) -> bool {
        if !self.enabled || self.owner_vault_attempted {
            return false;
        }
        self.owner_vault_attempted = true;
        true
    }

    fn reserve_shared_provider_attempt(&mut self, provider: &AutoSharedProvider) -> bool {
        if !self.enabled
            || self
                .attempted_shared_providers
                .iter()
                .any(|attempted| same_shared_provider(attempted, provider))
        {
            return false;
        }
        self.attempted_shared_providers.push(provider.clone());
        true
    }

    fn freeze_shared_provider_snapshot(
        &mut self,
        providers: Vec<AutoSharedProvider>,
    ) -> Vec<AutoSharedProvider> {
        self.shared_provider_snapshot
            .get_or_insert(providers)
            .clone()
    }

    fn attempt_count(&self) -> usize {
        usize::from(self.owner_vault_attempted) + self.attempted_shared_providers.len()
    }
}

pub(crate) async fn try_after_failure(
    runtime: &Arc<NodeRuntime>,
    req_id: &str,
    stdout_text: &str,
    stderr_text: &str,
    attempts: &mut CodexAuthAttemptState,
) -> Option<CodexAuthSwitchOutcome> {
    if !attempts.enabled {
        return None;
    }
    let combined = format!("{stdout_text}\n{stderr_text}");
    let classified = crate::errors::classify_ai_error(&combined);
    if !matches!(
        classified.category,
        crate::errors::AiErrorCategory::Quota | crate::errors::AiErrorCategory::AuthConfig
    ) {
        return None;
    }
    if attempts.reserve_owner_vault_attempt() {
        info!(
            %req_id,
            attempt = attempts.attempt_count(),
            "尝试 Codex 自有保险箱授权候选"
        );
        match node_agent_codex_vault::try_auto_switch_after_codex_failure(
            runtime,
            stdout_text,
            stderr_text,
        )
        .await
        {
            Ok(Some(candidate)) => {
                return finalize_auth_switch(
                    runtime,
                    req_id,
                    candidate.message,
                    candidate.cloud_control_deadline,
                    candidate.cloud_control_issued_at,
                    candidate.cloud_control_ttl_ms,
                )
                .await
            }
            Ok(None) => {}
            Err(error) => warn!(%req_id, "Codex 保险箱自动切换检查失败: {error:#}"),
        }
    }
    let reason = classified
        .operator_detail
        .as_deref()
        .unwrap_or(classified.code)
        .to_string();
    match try_shared_provider_after_failure(runtime, req_id, &reason, attempts).await {
        Ok(Some(candidate)) => {
            finalize_auth_switch(
                runtime,
                req_id,
                candidate.message,
                candidate.cloud_control_deadline,
                candidate.cloud_control_issued_at,
                candidate.cloud_control_ttl_ms,
            )
            .await
        }
        Ok(None) => None,
        Err(error) => {
            warn!("Codex 共享授权自动切换检查失败: {error:#}");
            None
        }
    }
}

async fn finalize_auth_switch(
    runtime: &Arc<NodeRuntime>,
    req_id: &str,
    message: String,
    server_deadline: Option<String>,
    server_issued_at: Option<String>,
    server_ttl_ms: Option<u64>,
) -> Option<CodexAuthSwitchOutcome> {
    let frozen_codex_home =
        match crate::node_agent_codex_child_env::FrozenCodexHome::capture_for_task() {
            Ok(home) => home,
            Err(error) => {
                warn!(%error, "Codex 账号切换后无法冻结新的 CODEX_HOME，拒绝重试");
                return None;
            }
        };
    if !frozen_codex_home.requires_cloud_control() {
        warn!(
            path = frozen_codex_home.path(),
            "Codex 云端账号切换没有落到节点托管 CODEX_HOME，拒绝重试"
        );
        return None;
    }
    let deadline = match freeze_auth_switch_cloud_control(
        server_deadline.as_deref(),
        server_issued_at.as_deref(),
        server_ttl_ms,
        frozen_codex_home.managed_lease_expires_at(),
    ) {
        Ok(deadline) => deadline,
        Err(error) => {
            warn!(%req_id, %error, "Codex 账号切换缺少有效云控截止时间，拒绝重试");
            return None;
        }
    };
    let Some(cancel_tx) = runtime.adopt_cli_prompt_cloud_control(req_id).await else {
        warn!(%req_id, "Codex 账号切换后任务控制句柄已消失，拒绝重试");
        return None;
    };
    let cancel_rx = cancel_tx.subscribe();
    if *cancel_rx.borrow() {
        warn!(%req_id, "Codex 账号切换期间任务已经被取消，拒绝重试");
        return None;
    }
    if let Err(error) = crate::node_agent_cloud_control::validate_registered_cloud_control(
        true,
        runtime.is_cloud_connected().await,
        Some(&deadline),
    ) {
        let _ = cancel_tx.send(true);
        warn!(%req_id, %error, "Codex 账号切换未通过注册后云控复查，已取消任务");
        return None;
    }
    crate::node_agent_cloud_control::spawn_absolute_deadline_cancel(
        Some(deadline),
        cancel_tx,
        req_id.to_string(),
    );
    Some(CodexAuthSwitchOutcome {
        message,
        frozen_codex_home,
    })
}

fn freeze_auth_switch_cloud_control(
    server_deadline: Option<&str>,
    server_issued_at: Option<&str>,
    server_ttl_ms: Option<u64>,
    managed_lease_deadline: Option<&str>,
) -> Result<crate::node_agent_cloud_control::CloudControlDeadline> {
    crate::node_agent_cloud_control::freeze_cloud_control_deadline(
        true,
        server_deadline,
        server_issued_at,
        server_ttl_ms,
        managed_lease_deadline,
    )?
    .ok_or_else(|| anyhow::anyhow!("云控账号切换没有冻结授权窗口"))
}

async fn try_shared_provider_after_failure(
    rt: &Arc<NodeRuntime>,
    req_id: &str,
    reason: &str,
    attempts: &mut CodexAuthAttemptState,
) -> Result<Option<CodexAuthSwitchCandidate>> {
    let providers = if let Some(providers) = attempts.shared_provider_snapshot.clone() {
        providers
    } else {
        let providers = match auto_shared_providers(rt).await {
            Ok(providers) => providers,
            Err(error) => {
                warn!("Codex 共享授权自动切换前置检查失败: {error:#}");
                return Ok(None);
            }
        };
        attempts.freeze_shared_provider_snapshot(providers)
    };
    let runtime = Arc::clone(rt);
    let req_id_owned = req_id.to_string();
    let reason_owned = reason.to_string();
    let Some((provider, lease)) =
        restore_first_available_shared_provider(req_id, attempts, providers, move |provider| {
            let runtime = Arc::clone(&runtime);
            let req_id = req_id_owned.clone();
            let reason = reason_owned.clone();
            async move {
                node_agent_codex_vault_emergency::restore_emergency_from_cloud(
                    &runtime,
                    node_agent_codex_vault_emergency::EmergencyRestoreRequest {
                        provider_user_id: provider.provider_user_id,
                        provider_account: provider.provider_account,
                        purpose: Some("auto_switch_to_shared_codex_after_failure".to_string()),
                        failure_reason: Some(reason),
                        compute_call_id: Some(format!("pc_agent_cli:{req_id}")),
                    },
                )
                .await
            }
        })
        .await
    else {
        return Ok(None);
    };
    Ok(Some(CodexAuthSwitchCandidate {
        message: format!(
            "Codex 当前账号额度或认证不可用，已自动切换到 {} 的共享账号{}，正在重试本轮任务。",
            provider.label,
            lease
                .account_hint_hash
                .as_deref()
                .map(|hint| format!(" ({hint})"))
                .unwrap_or_default()
        ),
        cloud_control_deadline: lease.cloud_control_deadline,
        cloud_control_issued_at: lease.cloud_control_issued_at,
        cloud_control_ttl_ms: lease.cloud_control_ttl_ms,
    }))
}

async fn restore_first_available_shared_provider<T, F, Fut>(
    req_id: &str,
    attempts: &mut CodexAuthAttemptState,
    providers: Vec<AutoSharedProvider>,
    mut restore: F,
) -> Option<(AutoSharedProvider, T)>
where
    F: FnMut(AutoSharedProvider) -> Fut,
    Fut: Future<Output = Result<T>>,
{
    for provider in providers {
        if !attempts.reserve_shared_provider_attempt(&provider) {
            continue;
        }
        info!(
            %req_id,
            attempt = attempts.attempt_count(),
            provider_user_id = provider.provider_user_id.as_deref().unwrap_or(""),
            "尝试 Codex 共享授权候选"
        );
        match restore(provider.clone()).await {
            Ok(restored) => return Some((provider, restored)),
            Err(error) => warn!(
                %req_id,
                provider_user_id = provider.provider_user_id.as_deref().unwrap_or(""),
                "Codex 共享授权候选恢复失败，继续下一个候选: {error:#}"
            ),
        }
    }
    None
}

async fn auto_shared_providers(rt: &Arc<NodeRuntime>) -> Result<Vec<AutoSharedProvider>> {
    let configured = configured_auto_shared_provider();
    let creds = match rt.creds().await {
        Some(creds) => creds,
        None if configured.is_some() => {
            return Ok(ordered_auto_shared_providers(configured, &[], ""));
        }
        None => return Err(anyhow::anyhow!("请先绑定本机节点账号")),
    };
    let owner_user_id = creds.owner_user_id.clone();
    let token = match creds
        .user_token
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        Some(token) => token.to_string(),
        None if configured.is_some() => {
            return Ok(ordered_auto_shared_providers(
                configured,
                &[],
                &owner_user_id,
            ));
        }
        None => bail!("本机节点缺少云端登录 token，请重新绑定"),
    };
    let url = format!(
        "{}/api/me/codex-vault/sharing",
        rt.cloud_http_url().trim_end_matches('/')
    );
    let value = match cloud_get(&url, &token).await {
        Ok(value) => value,
        Err(error) if configured.is_some() => {
            warn!("读取云端 Codex 共享授权失败，将仅尝试显式配置候选: {error:#}");
            return Ok(ordered_auto_shared_providers(
                configured,
                &[],
                &owner_user_id,
            ));
        }
        Err(error) => return Err(error),
    };
    let status: SharingStatusResponse =
        serde_json::from_value(value).context("云端共享授权状态响应格式不正确")?;
    Ok(ordered_auto_shared_providers(
        configured,
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
    ordered_auto_shared_providers(None, grants, owner_user_id)
        .into_iter()
        .next()
}

fn ordered_auto_shared_providers(
    configured: Option<AutoSharedProvider>,
    grants: &[SharingGrantSummary],
    owner_user_id: &str,
) -> Vec<AutoSharedProvider> {
    let mut providers = Vec::new();
    if let Some(provider) = configured {
        push_unique_shared_provider(&mut providers, provider);
    }
    for provider in grants
        .iter()
        .filter(|grant| grant_is_auto_share_candidate(grant, owner_user_id))
        .filter_map(AutoSharedProvider::from_grant)
    {
        push_unique_shared_provider(&mut providers, provider);
    }
    providers
}

fn push_unique_shared_provider(
    providers: &mut Vec<AutoSharedProvider>,
    provider: AutoSharedProvider,
) {
    if providers
        .iter()
        .any(|existing| same_shared_provider(existing, &provider))
    {
        return;
    }
    providers.push(provider);
}

fn same_shared_provider(left: &AutoSharedProvider, right: &AutoSharedProvider) -> bool {
    left.provider_user_id
        .as_deref()
        .zip(right.provider_user_id.as_deref())
        .is_some_and(|(left, right)| left == right)
        || left
            .provider_account
            .as_deref()
            .zip(right.provider_account.as_deref())
            .is_some_and(|(left, right)| left.eq_ignore_ascii_case(right))
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
#[path = "node_agent_codex_auth_switch_tests.rs"]
mod tests;
