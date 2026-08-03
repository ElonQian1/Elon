use anyhow::Result;
use chrono::Utc;

use crate::{
    open_commerce_developer_credential_model::production_credentials_enabled,
    open_commerce_developer_model::OpenCommerceDeveloperApp,
    open_commerce_production_webhook::production_webhooks_enabled,
    open_commerce_webhook_health_model::{
        DeveloperWebhookEnvironmentHealth, DeveloperWebhookHealthSummary,
        WEBHOOK_HEALTH_ACTION_REQUIRED, WEBHOOK_HEALTH_ATTENTION, WEBHOOK_HEALTH_HEALTHY,
        WEBHOOK_HEALTH_IDLE, WEBHOOK_HEALTH_PROCESSING,
    },
    store::Store,
};

pub(crate) fn health_summary(
    store: &Store,
    app: &OpenCommerceDeveloperApp,
) -> Result<DeveloperWebhookHealthSummary> {
    let production_webhooks_enabled = production_webhooks_enabled();
    let production_credentials_enabled = production_credentials_enabled();
    let production_credential_eligible =
        store.has_current_open_commerce_production_credential(&app.project_id, &app.id)?;
    let production_ready = production_webhooks_enabled
        && production_credentials_enabled
        && production_credential_eligible;
    let production_blocker_code = production_blocker(
        production_webhooks_enabled,
        production_credentials_enabled,
        production_credential_eligible,
    );
    let mut environments =
        store.open_commerce_developer_webhook_environment_health(&app.project_id, &app.id)?;
    for environment in &mut environments {
        environment.status = environment_status(environment, production_ready).to_string();
    }
    Ok(DeveloperWebhookHealthSummary {
        schema: "open_commerce.developer_webhook_health.v1",
        app_record_id: app.id.clone(),
        app_id: app.app_id.clone(),
        production_webhooks_enabled,
        production_credentials_enabled,
        production_credential_eligible,
        production_ready,
        production_blocker_code,
        environments,
        generated_at: Utc::now().to_rfc3339(),
    })
}

fn production_blocker(
    webhooks_enabled: bool,
    credentials_enabled: bool,
    credential_eligible: bool,
) -> Option<String> {
    if !webhooks_enabled {
        Some("production_webhooks_disabled".to_string())
    } else if !credentials_enabled {
        Some("production_credentials_disabled".to_string())
    } else if !credential_eligible {
        Some("production_credential_unavailable".to_string())
    } else {
        None
    }
}

fn environment_status(
    health: &DeveloperWebhookEnvironmentHealth,
    production_ready: bool,
) -> &'static str {
    if health.environment == "production"
        && health.active_subscription_count > 0
        && !production_ready
    {
        WEBHOOK_HEALTH_ACTION_REQUIRED
    } else if health.dead_delivery_count > 0 {
        WEBHOOK_HEALTH_ACTION_REQUIRED
    } else if health.retry_delivery_count > 0 {
        WEBHOOK_HEALTH_ATTENTION
    } else if health.pending_delivery_count > 0 || health.delivering_delivery_count > 0 {
        WEBHOOK_HEALTH_PROCESSING
    } else if health.active_subscription_count > 0 {
        WEBHOOK_HEALTH_HEALTHY
    } else {
        WEBHOOK_HEALTH_IDLE
    }
}
