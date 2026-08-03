use rusqlite::Row;

use crate::open_commerce_webhook_model::{
    DeveloperWebhookDelivery, DeveloperWebhookSubscription, DEVELOPER_WEBHOOK_DELIVERY_SCHEMA,
    DEVELOPER_WEBHOOK_SUBSCRIPTION_SCHEMA,
};

pub(super) const SUBSCRIPTION_SELECT: &str =
    "SELECT id, project_id, app_record_id, app_id, callback_url, signing_key_id,
            signing_secret_version, status, verification_status, verification_attempted_at,
            verification_error_code, verified_at, consecutive_failures,
            last_delivery_at, last_error_code, created_at, updated_at, disabled_at
       FROM open_commerce_developer_webhook_subscriptions";

pub(super) const DELIVERY_SELECT: &str =
    "SELECT id, subscription_id, invocation_id, event_sequence, event_type,
            status, attempt_count, manual_retry_count, next_attempt_at,
            response_status, error_code, created_at, last_attempt_at,
            last_manual_retry_at, delivered_at
       FROM open_commerce_developer_webhook_deliveries";

pub(super) fn subscription_from_row(
    row: &Row<'_>,
) -> rusqlite::Result<DeveloperWebhookSubscription> {
    Ok(DeveloperWebhookSubscription {
        schema: DEVELOPER_WEBHOOK_SUBSCRIPTION_SCHEMA,
        id: row.get(0)?,
        project_id: row.get(1)?,
        app_record_id: row.get(2)?,
        app_id: row.get(3)?,
        callback_url: row.get(4)?,
        signing_key_id: row.get(5)?,
        signing_secret_version: row.get(6)?,
        status: row.get(7)?,
        verification_status: row.get(8)?,
        verification_attempted_at: row.get(9)?,
        verification_error_code: row.get(10)?,
        verified_at: row.get(11)?,
        consecutive_failures: row.get(12)?,
        last_delivery_at: row.get(13)?,
        last_error_code: row.get(14)?,
        created_at: row.get(15)?,
        updated_at: row.get(16)?,
        disabled_at: row.get(17)?,
    })
}

pub(super) fn delivery_from_row(row: &Row<'_>) -> rusqlite::Result<DeveloperWebhookDelivery> {
    Ok(DeveloperWebhookDelivery {
        schema: DEVELOPER_WEBHOOK_DELIVERY_SCHEMA,
        id: row.get(0)?,
        subscription_id: row.get(1)?,
        invocation_id: row.get(2)?,
        event_sequence: row.get(3)?,
        event_type: row.get(4)?,
        status: row.get(5)?,
        attempt_count: row.get(6)?,
        manual_retry_count: row.get(7)?,
        next_attempt_at: row.get(8)?,
        response_status: row.get(9)?,
        error_code: row.get(10)?,
        created_at: row.get(11)?,
        last_attempt_at: row.get(12)?,
        last_manual_retry_at: row.get(13)?,
        delivered_at: row.get(14)?,
    })
}
