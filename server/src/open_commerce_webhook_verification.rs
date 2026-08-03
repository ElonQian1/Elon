use anyhow::{anyhow, bail, Result};
use chrono::{Duration, Utc};
use serde_json::Value;
use std::time::Duration as StdDuration;

use crate::{
    open_commerce_developer_model::OpenCommerceDeveloperApp,
    open_commerce_webhook_model::{
        DeveloperWebhookSubscription, DeveloperWebhookVerificationEnvelope,
    },
    store::Store,
};

const MAX_VERIFICATION_RESPONSE_BYTES: usize = 16 * 1024;

pub(crate) async fn verify_endpoint(
    store: &Store,
    app: &OpenCommerceDeveloperApp,
    subscription: &DeveloperWebhookSubscription,
) -> Result<DeveloperWebhookSubscription> {
    if subscription.project_id != app.project_id || subscription.app_record_id != app.id {
        bail!("Webhook 订阅不属于当前开发者 App");
    }
    crate::open_commerce_production_webhook::ensure_subscription_eligible(
        store,
        app,
        subscription,
    )?;
    if subscription.verification_status == "verified" {
        return Ok(subscription.clone());
    }
    let callback_url = match crate::open_commerce_webhook_security::validate_webhook_callback_url(
        &subscription.callback_url,
    ) {
        Ok(value) => value,
        Err(error) => {
            return verification_failed(store, app, subscription, "callback_invalid", error)
        }
    };
    let current_key_id = match crate::open_commerce_webhook_security::webhook_master_key_id() {
        Ok(value) => value,
        Err(error) => {
            return verification_failed(store, app, subscription, "signing_key_unavailable", error)
        }
    };
    if current_key_id != subscription.signing_key_id {
        return verification_failed(
            store,
            app,
            subscription,
            "signing_key_changed",
            anyhow!("Webhook 签名主密钥已变化，请创建新订阅"),
        );
    }
    let signing_secret = match crate::open_commerce_webhook_security::derive_webhook_signing_secret(
        &subscription.id,
        subscription.signing_secret_version,
    ) {
        Ok(value) => value,
        Err(error) => {
            return verification_failed(
                store,
                app,
                subscription,
                "signing_secret_unavailable",
                error,
            )
        }
    };
    let issued_at = Utc::now();
    let challenge = format!("whch_{}", uuid::Uuid::new_v4().simple());
    let envelope = DeveloperWebhookVerificationEnvelope {
        schema: "open_commerce.developer_webhook_verification.v2",
        subscription_id: subscription.id.clone(),
        environment: subscription.environment.clone(),
        challenge: challenge.clone(),
        issued_at: issued_at.to_rfc3339(),
        expires_at: (issued_at + Duration::minutes(5)).to_rfc3339(),
    };
    let body = serde_json::to_vec(&envelope)?;
    let timestamp = issued_at.timestamp().to_string();
    let signature =
        crate::open_commerce_webhook_security::sign_webhook(&signing_secret, &timestamp, &body)?;
    let target = match crate::open_commerce_outbound_security::pinned_public_https_target(
        &callback_url,
        StdDuration::from_secs(5),
        StdDuration::from_secs(10),
    )
    .await
    {
        Ok(value) => value,
        Err(error) => {
            return verification_failed(store, app, subscription, "callback_unsafe", error)
        }
    };
    let mut response = match target
        .client
        .post(&target.url)
        .header("content-type", "application/json")
        .header(
            "user-agent",
            "yilong-open-commerce-webhook-verification/1.0",
        )
        .header(
            "x-yilong-webhook-id",
            format!("verification:{}", subscription.id),
        )
        .header("x-yilong-webhook-timestamp", &timestamp)
        .header("x-yilong-webhook-signature", format!("v1={signature}"))
        .body(body)
        .send()
        .await
    {
        Ok(response) => response,
        Err(error) => {
            return verification_failed(
                store,
                app,
                subscription,
                "endpoint_unreachable",
                anyhow!("Webhook 回调端点不可达: {error}"),
            )
        }
    };
    if !response.status().is_success() {
        return verification_failed(
            store,
            app,
            subscription,
            "endpoint_rejected",
            anyhow!("Webhook 回调端点拒绝验证: HTTP {}", response.status()),
        );
    }
    let mut response_body = Vec::new();
    loop {
        let chunk = match response.chunk().await {
            Ok(chunk) => chunk,
            Err(error) => {
                return verification_failed(
                    store,
                    app,
                    subscription,
                    "response_unreadable",
                    anyhow!("读取 Webhook 验证响应失败: {error}"),
                )
            }
        };
        let Some(chunk) = chunk else { break };
        if response_body.len().saturating_add(chunk.len()) > MAX_VERIFICATION_RESPONSE_BYTES {
            return verification_failed(
                store,
                app,
                subscription,
                "response_too_large",
                anyhow!("Webhook 验证响应超过 16 KiB 限制"),
            );
        }
        response_body.extend_from_slice(&chunk);
    }
    let response_value: Value = match serde_json::from_slice(&response_body) {
        Ok(value) => value,
        Err(error) => {
            return verification_failed(
                store,
                app,
                subscription,
                "response_invalid_json",
                anyhow!("Webhook 验证响应不是有效 JSON: {error}"),
            )
        }
    };
    if response_value.get("challenge").and_then(Value::as_str) != Some(challenge.as_str()) {
        return verification_failed(
            store,
            app,
            subscription,
            "challenge_mismatch",
            anyhow!("Webhook 验证响应未原样返回 challenge"),
        );
    }
    store.verify_open_commerce_developer_webhook(&app.project_id, &app.id, &subscription.id)
}

fn verification_failed(
    store: &Store,
    app: &OpenCommerceDeveloperApp,
    subscription: &DeveloperWebhookSubscription,
    error_code: &str,
    error: anyhow::Error,
) -> Result<DeveloperWebhookSubscription> {
    let _ = store.record_open_commerce_developer_webhook_verification_failure(
        &app.project_id,
        &app.id,
        &subscription.id,
        error_code,
    );
    Err(error)
}
