use rusqlite::Row;

use crate::open_commerce_webhook_model::{
    DeveloperWebhookDelivery, DeveloperWebhookSubscription, DEVELOPER_WEBHOOK_DELIVERY_SCHEMA,
    DEVELOPER_WEBHOOK_SUBSCRIPTION_SCHEMA,
};

pub(super) const SUBSCRIPTION_SELECT: &str =
    "SELECT id, project_id, app_record_id, app_id, environment, callback_url, signing_key_id,
            signing_secret_version, deliver_on_succeeded, deliver_on_failed,
            status, verification_status, verification_attempted_at,
            verification_error_code, verified_at, consecutive_failures,
            last_delivery_at, last_error_code, created_at, updated_at, disabled_at
       FROM open_commerce_developer_webhook_subscriptions";

pub(super) const DELIVERY_SELECT: &str =
    "SELECT id, subscription_id, invocation_id, event_sequence, event_type,
            enqueue_source, status, attempt_count, manual_retry_count, next_attempt_at,
            response_status, error_code, created_at, last_attempt_at,
            last_manual_retry_at, history_replay_requested_at, delivered_at
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
        environment: row.get(4)?,
        callback_url: row.get(5)?,
        signing_key_id: row.get(6)?,
        signing_secret_version: row.get(7)?,
        deliver_on_succeeded: row.get(8)?,
        deliver_on_failed: row.get(9)?,
        status: row.get(10)?,
        verification_status: row.get(11)?,
        verification_attempted_at: row.get(12)?,
        verification_error_code: row.get(13)?,
        verified_at: row.get(14)?,
        consecutive_failures: row.get(15)?,
        last_delivery_at: row.get(16)?,
        last_error_code: row.get(17)?,
        created_at: row.get(18)?,
        updated_at: row.get(19)?,
        disabled_at: row.get(20)?,
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
        enqueue_source: row.get(5)?,
        status: row.get(6)?,
        attempt_count: row.get(7)?,
        manual_retry_count: row.get(8)?,
        next_attempt_at: row.get(9)?,
        response_status: row.get(10)?,
        error_code: row.get(11)?,
        created_at: row.get(12)?,
        last_attempt_at: row.get(13)?,
        last_manual_retry_at: row.get(14)?,
        history_replay_requested_at: row.get(15)?,
        delivered_at: row.get(16)?,
    })
}
