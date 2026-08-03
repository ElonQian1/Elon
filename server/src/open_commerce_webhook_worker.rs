use chrono::Utc;
use reqwest::StatusCode;
use std::{sync::Arc, time::Duration};

use crate::{
    open_commerce_webhook_model::{DeveloperWebhookDeliveryClaim, DeveloperWebhookEnvelope},
    types::AppState,
};

const MAX_DELIVERIES_PER_TICK: usize = 10;

pub(crate) fn spawn(state: Arc<AppState>) {
    if crate::open_commerce_webhook_security::webhook_master_key_id().is_err() {
        tracing::warn!(
            "developer webhook worker disabled: OPEN_COMMERCE_WEBHOOK_MASTER_SECRET is unavailable"
        );
        return;
    }
    tokio::spawn(async move {
        let worker_id = format!("webhook-worker:{}", uuid::Uuid::new_v4().simple());
        let mut interval = tokio::time::interval(Duration::from_secs(2));
        loop {
            interval.tick().await;
            for _ in 0..MAX_DELIVERIES_PER_TICK {
                let claim = match state
                    .store
                    .claim_open_commerce_developer_webhook_delivery(&worker_id)
                {
                    Ok(Some(claim)) => claim,
                    Ok(None) => break,
                    Err(error) => {
                        tracing::warn!(error = %error, "failed to claim developer webhook delivery");
                        break;
                    }
                };
                deliver(&state, claim).await;
            }
        }
    });
}

async fn deliver(state: &AppState, claim: DeveloperWebhookDeliveryClaim) {
    let current_key_id = match crate::open_commerce_webhook_security::webhook_master_key_id() {
        Ok(value) => value,
        Err(_) => {
            fail(
                state,
                &claim,
                None,
                "webhook_master_key_unavailable",
                None,
                true,
            );
            return;
        }
    };
    if current_key_id != claim.signing_key_id {
        fail(
            state,
            &claim,
            None,
            "webhook_signing_key_changed",
            None,
            true,
        );
        return;
    }
    let record = match state.store.open_commerce_developer_terminal_event(
        &claim.owner_user_id,
        &claim.app_id,
        "sandbox",
        &claim.delivery.invocation_id,
    ) {
        Ok(Some(record)) => record,
        _ => {
            fail(
                state,
                &claim,
                None,
                "webhook_event_unavailable",
                None,
                false,
            );
            return;
        }
    };
    let event = match crate::open_commerce_developer_event_service::summary_from_record(record) {
        Ok(event) => event,
        Err(_) => {
            fail(state, &claim, None, "webhook_event_invalid", None, false);
            return;
        }
    };
    let emitted_at = Utc::now();
    let envelope = DeveloperWebhookEnvelope {
        schema: "open_commerce.developer_webhook_event.v1",
        delivery_id: claim.delivery.id.clone(),
        subscription_id: claim.delivery.subscription_id.clone(),
        app_id: claim.app_id.clone(),
        emitted_at: emitted_at.to_rfc3339(),
        event,
    };
    let body = match serde_json::to_vec(&envelope) {
        Ok(body) => body,
        Err(_) => {
            fail(state, &claim, None, "webhook_payload_invalid", None, false);
            return;
        }
    };
    let secret = match crate::open_commerce_webhook_security::derive_webhook_signing_secret(
        &claim.delivery.subscription_id,
        claim.signing_secret_version,
    ) {
        Ok(secret) => secret,
        Err(_) => {
            fail(
                state,
                &claim,
                None,
                "webhook_signing_key_unavailable",
                None,
                true,
            );
            return;
        }
    };
    let timestamp = emitted_at.timestamp().to_string();
    let signature =
        match crate::open_commerce_webhook_security::sign_webhook(&secret, &timestamp, &body) {
            Ok(signature) => signature,
            Err(_) => {
                fail(state, &claim, None, "webhook_signing_failed", None, false);
                return;
            }
        };
    let callback_url = match crate::open_commerce_webhook_security::validate_webhook_callback_url(
        &claim.callback_url,
    ) {
        Ok(value) => value,
        Err(_) => {
            fail(state, &claim, None, "webhook_target_invalid", None, true);
            return;
        }
    };
    let target = match crate::open_commerce_outbound_security::pinned_public_https_target(
        &callback_url,
        Duration::from_secs(5),
        Duration::from_secs(10),
    )
    .await
    {
        Ok(value) => value,
        Err(error) => {
            tracing::warn!(error = %error, delivery_id = %claim.delivery.id, "developer webhook target rejected");
            fail(state, &claim, None, "webhook_target_unsafe", None, true);
            return;
        }
    };
    let response = target
        .client
        .post(&target.url)
        .header("content-type", "application/json")
        .header("user-agent", "yilong-open-commerce-webhook/1.0")
        .header("x-yilong-webhook-id", &claim.delivery.id)
        .header("x-yilong-webhook-timestamp", &timestamp)
        .header("x-yilong-webhook-signature", format!("v1={signature}"))
        .body(body)
        .send()
        .await;
    match response {
        Ok(response) if response.status().is_success() => {
            if let Err(error) = state
                .store
                .complete_open_commerce_developer_webhook_delivery(
                    &claim,
                    response.status().as_u16() as i64,
                )
            {
                tracing::warn!(error = %error, delivery_id = %claim.delivery.id, "failed to complete webhook delivery");
            }
        }
        Ok(response) => {
            let status = response.status();
            let retryable = status == StatusCode::REQUEST_TIMEOUT
                || status == StatusCode::TOO_MANY_REQUESTS
                || status.is_server_error();
            let force_disable = status == StatusCode::GONE;
            fail(
                state,
                &claim,
                Some(status.as_u16() as i64),
                if force_disable {
                    "webhook_endpoint_gone"
                } else if retryable {
                    "webhook_endpoint_retryable"
                } else {
                    "webhook_endpoint_rejected"
                },
                retryable.then(|| retry_delay_seconds(claim.delivery.attempt_count)),
                force_disable,
            );
        }
        Err(error) => {
            tracing::debug!(error = %error, delivery_id = %claim.delivery.id, "developer webhook network failure");
            fail(
                state,
                &claim,
                None,
                "webhook_network_error",
                Some(retry_delay_seconds(claim.delivery.attempt_count)),
                false,
            );
        }
    }
}

fn fail(
    state: &AppState,
    claim: &DeveloperWebhookDeliveryClaim,
    response_status: Option<i64>,
    error_code: &str,
    retry_after_seconds: Option<i64>,
    force_disable: bool,
) {
    if let Err(error) = state.store.fail_open_commerce_developer_webhook_delivery(
        claim,
        response_status,
        error_code,
        retry_after_seconds,
        force_disable,
    ) {
        tracing::warn!(error = %error, delivery_id = %claim.delivery.id, "failed to record webhook delivery failure");
    }
}

fn retry_delay_seconds(attempt_count: i64) -> i64 {
    let exponent = attempt_count.saturating_sub(1).clamp(0, 8) as u32;
    (10_i64.saturating_mul(2_i64.saturating_pow(exponent))).min(3600)
}
