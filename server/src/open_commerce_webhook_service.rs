use anyhow::{bail, Result};

use crate::{
    open_commerce_developer_model::OpenCommerceDeveloperApp,
    open_commerce_webhook_model::{
        CreateDeveloperWebhookRequest, DeveloperWebhookCredential, DeveloperWebhookDelivery,
        DeveloperWebhookSubscription,
    },
    store::Store,
};

pub(crate) fn ensure_owned_app(
    store: &Store,
    project_id: &str,
    app_record_id: &str,
    owner_user_id: &str,
    require_active: bool,
) -> Result<OpenCommerceDeveloperApp> {
    let app = store.open_commerce_developer_app_for_project(project_id, app_record_id)?;
    if app.owner_user_id != owner_user_id.trim() {
        bail!("当前用户不能管理该开发者 App 的 Webhook");
    }
    if require_active && app.status != "active" {
        bail!("开发者 App 已停用");
    }
    Ok(app)
}

pub(crate) fn create_webhook(
    store: &Store,
    app: &OpenCommerceDeveloperApp,
    request: CreateDeveloperWebhookRequest,
) -> Result<DeveloperWebhookCredential> {
    let callback_url = crate::open_commerce_webhook_security::validate_webhook_callback_url(
        &request.callback_url,
    )?;
    let signing_key_id = crate::open_commerce_webhook_security::webhook_master_key_id()?;
    let subscription =
        store.create_open_commerce_developer_webhook(app, &callback_url, &signing_key_id)?;
    let signing_secret = match crate::open_commerce_webhook_security::derive_webhook_signing_secret(
        &subscription.id,
        subscription.signing_secret_version,
    ) {
        Ok(secret) => secret,
        Err(error) => {
            let _ = store.set_open_commerce_developer_webhook_enabled(
                &app.project_id,
                &app.id,
                &subscription.id,
                false,
            );
            return Err(error);
        }
    };
    Ok(DeveloperWebhookCredential {
        schema: "open_commerce.developer_webhook_credential.v1",
        subscription,
        signing_secret,
        signing_secret_visible_once: true,
    })
}

pub(crate) fn rotate_webhook_secret(
    store: &Store,
    app: &OpenCommerceDeveloperApp,
    subscription_id: &str,
) -> Result<DeveloperWebhookCredential> {
    let current =
        store.open_commerce_developer_webhook_for_app(&app.project_id, &app.id, subscription_id)?;
    let next_version = current
        .signing_secret_version
        .checked_add(1)
        .ok_or_else(|| anyhow::anyhow!("Webhook 签名密钥版本已耗尽"))?;
    let signing_key_id = crate::open_commerce_webhook_security::webhook_master_key_id()?;
    let signing_secret = crate::open_commerce_webhook_security::derive_webhook_signing_secret(
        &current.id,
        next_version,
    )?;
    let subscription = store.rotate_open_commerce_developer_webhook_secret(
        &app.project_id,
        &app.id,
        &current.id,
        current.signing_secret_version,
        next_version,
        &signing_key_id,
    )?;
    Ok(DeveloperWebhookCredential {
        schema: "open_commerce.developer_webhook_credential.v1",
        subscription,
        signing_secret,
        signing_secret_visible_once: true,
    })
}

pub(crate) fn list_webhooks(
    store: &Store,
    app: &OpenCommerceDeveloperApp,
) -> Result<Vec<DeveloperWebhookSubscription>> {
    store.list_open_commerce_developer_webhooks(&app.project_id, &app.id)
}

pub(crate) fn set_webhook_enabled(
    store: &Store,
    app: &OpenCommerceDeveloperApp,
    subscription_id: &str,
    enabled: bool,
) -> Result<DeveloperWebhookSubscription> {
    if enabled {
        let subscription = store.open_commerce_developer_webhook_for_app(
            &app.project_id,
            &app.id,
            subscription_id,
        )?;
        let current_key_id = crate::open_commerce_webhook_security::webhook_master_key_id()?;
        if subscription.signing_key_id != current_key_id {
            bail!("Webhook 签名主密钥已变化，请创建新订阅");
        }
    }
    store.set_open_commerce_developer_webhook_enabled(
        &app.project_id,
        &app.id,
        subscription_id,
        enabled,
    )
}

pub(crate) fn list_deliveries(
    store: &Store,
    app: &OpenCommerceDeveloperApp,
    subscription_id: &str,
) -> Result<Vec<DeveloperWebhookDelivery>> {
    store.list_open_commerce_developer_webhook_deliveries(
        &app.project_id,
        &app.id,
        subscription_id,
        50,
    )
}
