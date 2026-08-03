//! HTTPS well-known challenge for App homepage-domain control proof.

use anyhow::{anyhow, bail, Result};
use chrono::{DateTime, Duration, Utc};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::time::Duration as StdDuration;

use crate::{
    open_commerce_developer_manifest_service::managed_app,
    open_commerce_developer_model::{
        DeveloperAppDomainChallengeCredential, DeveloperAppDomainChallengeState,
        OpenCommerceDeveloperApp,
    },
    open_commerce_service::OpenCommerceActor,
    store::Store,
};

const VERIFICATION_PATH: &str = "/.well-known/yilong-open-commerce-app-verification.txt";
const MAX_RESPONSE_BYTES: usize = 4 * 1024;

pub(crate) fn issue_challenge(
    store: &Store,
    project_id: &str,
    app_record_id: &str,
    expected_revision: i64,
    actor: &OpenCommerceActor<'_>,
) -> Result<DeveloperAppDomainChallengeCredential> {
    let current = managed_app(store, project_id, app_record_id, actor)?;
    if current.status != "active" {
        bail!("开发者应用已停用，请先重新启用");
    }
    if current.manifest_revision != expected_revision {
        bail!("App 资料已变化，请刷新后重试");
    }
    let homepage = current
        .homepage_url
        .as_deref()
        .ok_or_else(|| anyhow!("请先保存 HTTPS 应用主页"))?;
    let (verification_host, verification_url) = verification_endpoint(homepage)?;
    let token = format!("ocdv_{}", uuid::Uuid::new_v4().simple());
    let verification_content = format!("yilong-open-commerce-app-verification={token}");
    let expires_at = (Utc::now() + Duration::hours(24)).to_rfc3339();
    let app = store.issue_open_commerce_developer_app_domain_challenge(
        project_id,
        app_record_id,
        expected_revision,
        &verification_host,
        &sha256_hex(verification_content.as_bytes()),
        &expires_at,
    )?;
    store.record_open_commerce_audit(
        project_id,
        actor.user_id,
        Some(actor.app_id),
        "developer_app.domain_challenge_issued",
        "developer_app",
        &app.id,
        &json!({
            "app_id": app.app_id,
            "manifest_revision": app.manifest_revision,
            "verification_host": verification_host,
            "expires_at": expires_at,
        }),
    )?;
    Ok(DeveloperAppDomainChallengeCredential {
        schema: "open_commerce.developer_app_domain_challenge.v1",
        app,
        verification_url,
        verification_content,
        content_visible_once: true,
        expires_at,
    })
}

pub(crate) async fn verify_domain(
    store: &Store,
    project_id: &str,
    app_record_id: &str,
    actor: &OpenCommerceActor<'_>,
) -> Result<OpenCommerceDeveloperApp> {
    let app = managed_app(store, project_id, app_record_id, actor)?;
    if app.domain_verification_status == "verified"
        && app.domain_verification_revision == Some(app.manifest_revision)
    {
        return Ok(app);
    }
    let challenge = store
        .open_commerce_developer_app_domain_challenge(project_id, app_record_id)
        .map_err(|error| anyhow!("请先生成域名验证 challenge: {error}"))?;
    ensure_current_challenge(&app, &challenge)?;
    let verification_url = verification_url_for_host(&challenge.verification_host)?;
    if !allowed_hosts()
        .iter()
        .any(|allowed| allowed == &challenge.verification_host)
    {
        return verification_failed(
            store,
            &app,
            &challenge,
            actor,
            "host_not_allowed",
            anyhow!("域名未加入 OPEN_COMMERCE_APP_DOMAIN_ALLOWED_HOSTS 精确白名单"),
        );
    }
    let client = reqwest::Client::builder()
        .connect_timeout(StdDuration::from_secs(5))
        .timeout(StdDuration::from_secs(10))
        .redirect(reqwest::redirect::Policy::none())
        .build()?;
    let mut response = match client
        .get(&verification_url)
        .header("user-agent", "yilong-open-commerce-domain-verification/1.0")
        .send()
        .await
    {
        Ok(value) => value,
        Err(error) => {
            return verification_failed(
                store,
                &app,
                &challenge,
                actor,
                "endpoint_unreachable",
                anyhow!("域名验证地址不可达: {error}"),
            )
        }
    };
    if !response.status().is_success() {
        return verification_failed(
            store,
            &app,
            &challenge,
            actor,
            "endpoint_rejected",
            anyhow!("域名验证地址拒绝请求: HTTP {}", response.status()),
        );
    }
    let mut body = Vec::new();
    loop {
        let chunk = match response.chunk().await {
            Ok(value) => value,
            Err(error) => {
                return verification_failed(
                    store,
                    &app,
                    &challenge,
                    actor,
                    "response_unreadable",
                    anyhow!("读取域名验证响应失败: {error}"),
                )
            }
        };
        let Some(chunk) = chunk else { break };
        if body.len().saturating_add(chunk.len()) > MAX_RESPONSE_BYTES {
            return verification_failed(
                store,
                &app,
                &challenge,
                actor,
                "response_too_large",
                anyhow!("域名验证响应超过 4 KiB 限制"),
            );
        }
        body.extend_from_slice(&chunk);
    }
    let content = match std::str::from_utf8(&body) {
        Ok(value) => value.trim(),
        Err(_) => {
            return verification_failed(
                store,
                &app,
                &challenge,
                actor,
                "response_invalid_utf8",
                anyhow!("域名验证响应不是 UTF-8 文本"),
            )
        }
    };
    if sha256_hex(content.as_bytes()) != challenge.challenge_hash {
        return verification_failed(
            store,
            &app,
            &challenge,
            actor,
            "challenge_mismatch",
            anyhow!("固定验证地址未返回本次 challenge 内容"),
        );
    }
    let verified = store.verify_open_commerce_developer_app_domain(
        project_id,
        app_record_id,
        challenge.manifest_revision,
    )?;
    store.record_open_commerce_audit(
        project_id,
        actor.user_id,
        Some(actor.app_id),
        "developer_app.domain_verified",
        "developer_app",
        &verified.id,
        &json!({
            "app_id": verified.app_id,
            "manifest_revision": verified.manifest_revision,
            "verification_host": verified.domain_verification_host,
        }),
    )?;
    Ok(verified)
}

fn ensure_current_challenge(
    app: &OpenCommerceDeveloperApp,
    challenge: &DeveloperAppDomainChallengeState,
) -> Result<()> {
    if challenge.app_record_id != app.id
        || challenge.project_id != app.project_id
        || challenge.manifest_revision != app.manifest_revision
    {
        bail!("域名验证 challenge 未绑定当前 App 资料修订");
    }
    if !matches!(challenge.status.as_str(), "pending" | "failed") {
        bail!("域名验证 challenge 当前不可使用");
    }
    let expires_at = DateTime::parse_from_rfc3339(&challenge.expires_at)
        .map_err(|_| anyhow!("域名验证 challenge 到期时间无效"))?
        .with_timezone(&Utc);
    if expires_at <= Utc::now() {
        bail!("域名验证 challenge 已过期，请重新生成");
    }
    Ok(())
}

fn verification_endpoint(homepage: &str) -> Result<(String, String)> {
    let mut url = reqwest::Url::parse(homepage).map_err(|_| anyhow!("应用主页 URL 无效"))?;
    if url.scheme() != "https"
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
    {
        bail!("应用主页必须是无账号信息的 HTTPS URL");
    }
    if url.port_or_known_default() != Some(443) {
        bail!("应用主页域名验证仅支持标准 HTTPS 端口 443");
    }
    let host = url
        .host_str()
        .ok_or_else(|| anyhow!("应用主页缺少主机"))?
        .to_ascii_lowercase();
    url.set_path(VERIFICATION_PATH);
    url.set_query(None);
    url.set_fragment(None);
    Ok((host, url.to_string()))
}

fn verification_url_for_host(host: &str) -> Result<String> {
    let host = host.trim().to_ascii_lowercase();
    if host.is_empty() || host.contains('/') || host.contains('@') {
        bail!("域名验证主机无效");
    }
    Ok(format!("https://{host}{VERIFICATION_PATH}"))
}

fn allowed_hosts() -> Vec<String> {
    std::env::var("OPEN_COMMERCE_APP_DOMAIN_ALLOWED_HOSTS")
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_ascii_lowercase)
        .collect()
}

fn verification_failed(
    store: &Store,
    app: &OpenCommerceDeveloperApp,
    challenge: &DeveloperAppDomainChallengeState,
    actor: &OpenCommerceActor<'_>,
    error_code: &str,
    error: anyhow::Error,
) -> Result<OpenCommerceDeveloperApp> {
    let _ = store.record_open_commerce_developer_app_domain_failure(
        &app.project_id,
        &app.id,
        challenge.manifest_revision,
        error_code,
    );
    let _ = store.record_open_commerce_audit(
        &app.project_id,
        actor.user_id,
        Some(actor.app_id),
        "developer_app.domain_verification_failed",
        "developer_app",
        &app.id,
        &json!({
            "app_id": app.app_id,
            "manifest_revision": challenge.manifest_revision,
            "verification_host": challenge.verification_host,
            "error_code": error_code,
        }),
    );
    Err(error)
}

fn sha256_hex(value: &[u8]) -> String {
    hex::encode(Sha256::digest(value))
}
