//! Production Webhook feature gate and App eligibility policy.

use anyhow::{bail, Result};

use crate::{
    open_commerce_developer_credential_model::production_credentials_enabled,
    open_commerce_developer_model::OpenCommerceDeveloperApp,
    open_commerce_webhook_model::DeveloperWebhookSubscription, store::Store,
};

pub(crate) const PRODUCTION_WEBHOOK_ENV: &str = "OPEN_COMMERCE_PRODUCTION_WEBHOOKS_ENABLED";

pub(crate) fn production_webhooks_enabled() -> bool {
    std::env::var(PRODUCTION_WEBHOOK_ENV)
        .ok()
        .is_some_and(|value| matches!(value.trim(), "1" | "true" | "yes" | "enabled"))
}

pub(crate) fn normalize_environment(value: Option<&str>) -> Result<&'static str> {
    match value.map(str::trim).filter(|value| !value.is_empty()) {
        None | Some("sandbox") => Ok("sandbox"),
        Some("production") => Ok("production"),
        Some(_) => bail!("Webhook 环境只允许 sandbox 或 production"),
    }
}

pub(crate) fn ensure_eligible(
    store: &Store,
    app: &OpenCommerceDeveloperApp,
    environment: &str,
) -> Result<()> {
    if environment != "production" {
        return Ok(());
    }
    if !production_webhooks_enabled() {
        bail!("生产 Webhook 当前未启用");
    }
    if !production_credentials_enabled() {
        bail!("生产凭据入口未启用，不能使用生产 Webhook");
    }
    store.ensure_current_open_commerce_production_credential(&app.project_id, &app.id)
}

pub(crate) fn ensure_subscription_eligible(
    store: &Store,
    app: &OpenCommerceDeveloperApp,
    subscription: &DeveloperWebhookSubscription,
) -> Result<()> {
    if subscription.project_id != app.project_id || subscription.app_record_id != app.id {
        bail!("Webhook 订阅不属于当前开发者 App");
    }
    ensure_eligible(store, app, &subscription.environment)
}
