use anyhow::{bail, Result};

use crate::{
    open_commerce_developer_model::OpenCommerceDeveloperApp,
    open_commerce_webhook_model::{
        CreateDeveloperWebhookRequest, DeveloperWebhookCredential, DeveloperWebhookDelivery,
        DeveloperWebhookHistoryReplayResult, DeveloperWebhookSubscription,
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
    let environment = crate::open_commerce_production_webhook::normalize_environment(
        request.environment.as_deref(),
    )?;
    crate::open_commerce_production_webhook::ensure_eligible(store, app, environment)?;
    let callback_url = crate::open_commerce_webhook_security::validate_webhook_callback_url(
        &request.callback_url,
    )?;
    let deliver_on_succeeded = request.deliver_on_succeeded.unwrap_or(true);
    let deliver_on_failed = request.deliver_on_failed.unwrap_or(true);
    if !deliver_on_succeeded && !deliver_on_failed {
        bail!("Webhook 至少需要订阅一种终态事件");
    }
    let signing_key_id = crate::open_commerce_webhook_security::webhook_master_key_id()?;
    let subscription = store.create_open_commerce_developer_webhook(
        app,
        &callback_url,
        &signing_key_id,
        environment,
        deliver_on_succeeded,
        deliver_on_failed,
    )?;
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
    crate::open_commerce_production_webhook::ensure_subscription_eligible(store, app, &current)?;
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
        crate::open_commerce_production_webhook::ensure_subscription_eligible(
            store,
            app,
            &subscription,
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

pub(crate) fn retry_delivery(
    store: &Store,
    app: &OpenCommerceDeveloperApp,
    subscription_id: &str,
    delivery_id: &str,
) -> Result<DeveloperWebhookDelivery> {
    let subscription =
        store.open_commerce_developer_webhook_for_app(&app.project_id, &app.id, subscription_id)?;
    crate::open_commerce_production_webhook::ensure_subscription_eligible(
        store,
        app,
        &subscription,
    )?;
    store.retry_open_commerce_developer_webhook_delivery(
        &app.project_id,
        &app.id,
        subscription_id,
        delivery_id,
    )
}

pub(crate) fn acknowledge_dead_letter(
    store: &Store,
    app: &OpenCommerceDeveloperApp,
    subscription_id: &str,
    delivery_id: &str,
    acknowledged_by_user_id: &str,
    reason: &str,
) -> Result<DeveloperWebhookDelivery> {
    let reason = reason.trim();
    if reason.chars().count() < 4
        || reason.chars().count() > 500
        || reason.chars().any(char::is_control)
    {
        bail!("死信处理原因必须为 4 至 500 个有效字符");
    }
    store.acknowledge_open_commerce_developer_webhook_dead_letter(
        &app.project_id,
        &app.id,
        subscription_id,
        delivery_id,
        acknowledged_by_user_id,
        reason,
    )
}

pub(crate) fn replay_history(
    store: &Store,
    app: &OpenCommerceDeveloperApp,
    subscription_id: &str,
    after_sequence: i64,
    limit: usize,
) -> Result<DeveloperWebhookHistoryReplayResult> {
    if after_sequence < 0 {
        bail!("Webhook 历史补发起始序号不能为负数");
    }
    if !(1..=100).contains(&limit) {
        bail!("Webhook 单次历史补发数量必须在 1 到 100 之间");
    }
    let subscription =
        store.open_commerce_developer_webhook_for_app(&app.project_id, &app.id, subscription_id)?;
    crate::open_commerce_production_webhook::ensure_subscription_eligible(
        store,
        app,
        &subscription,
    )?;
    store.replay_open_commerce_developer_webhook_history(
        &app.project_id,
        &app.id,
        subscription_id,
        after_sequence,
        limit,
    )
}
