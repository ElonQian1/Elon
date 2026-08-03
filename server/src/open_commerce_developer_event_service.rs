use anyhow::{anyhow, bail, Result};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use serde::{Deserialize, Serialize};

use crate::{
    open_commerce_developer_credential_model::AuthenticatedDeveloperCredential,
    open_commerce_developer_event_model::{
        DeveloperTerminalEventDetail, DeveloperTerminalEventPage, DeveloperTerminalEventQuery,
        DeveloperTerminalEventRecord, DeveloperTerminalEventSummary,
    },
    open_commerce_developer_model::OpenCommerceDeveloperApp,
    store::Store,
};

const CURSOR_VERSION: u8 = 2;
const DEFAULT_LIMIT: usize = 20;
const MAX_LIMIT: usize = 100;

#[derive(Debug, Serialize, Deserialize)]
struct DeveloperEventCursor {
    v: u8,
    app_id: String,
    #[serde(default)]
    environment: Option<String>,
    sequence: i64,
}

pub(crate) fn list_terminal_events(
    store: &Store,
    app: &OpenCommerceDeveloperApp,
    query: DeveloperTerminalEventQuery,
) -> Result<DeveloperTerminalEventPage> {
    list_terminal_events_for_credential(
        store,
        &AuthenticatedDeveloperCredential::sandbox(app.clone()),
        query,
    )
}

pub(crate) fn list_terminal_events_for_credential(
    store: &Store,
    credential: &AuthenticatedDeveloperCredential,
    query: DeveloperTerminalEventQuery,
) -> Result<DeveloperTerminalEventPage> {
    let app = &credential.app;
    let after_sequence =
        decode_cursor(query.cursor.as_deref(), &app.app_id, credential.environment)?;
    let limit = query.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);
    let mut records = store.list_open_commerce_developer_terminal_events(
        &app.owner_user_id,
        &app.app_id,
        credential.environment,
        after_sequence,
        limit + 1,
    )?;
    let has_more = records.len() > limit;
    if has_more {
        records.truncate(limit);
    }
    let checkpoint = records
        .last()
        .map(|record| record.sequence)
        .unwrap_or(after_sequence);
    let events = records
        .into_iter()
        .map(summary_from_record)
        .collect::<Result<Vec<_>>>()?;

    Ok(DeveloperTerminalEventPage {
        schema: "open_commerce.developer_terminal_events.v2",
        app_id: app.app_id.clone(),
        credential_environment: credential.environment.to_string(),
        events,
        next_cursor: (checkpoint > 0)
            .then(|| encode_cursor(&app.app_id, credential.environment, checkpoint))
            .transpose()?,
        has_more,
    })
}

pub(crate) fn terminal_event_detail(
    store: &Store,
    app: &OpenCommerceDeveloperApp,
    invocation_id: &str,
) -> Result<DeveloperTerminalEventDetail> {
    terminal_event_detail_for_credential(
        store,
        &AuthenticatedDeveloperCredential::sandbox(app.clone()),
        invocation_id,
    )
}

pub(crate) fn terminal_event_detail_for_credential(
    store: &Store,
    credential: &AuthenticatedDeveloperCredential,
    invocation_id: &str,
) -> Result<DeveloperTerminalEventDetail> {
    let app = &credential.app;
    let record = store
        .open_commerce_developer_terminal_event(
            &app.owner_user_id,
            &app.app_id,
            credential.environment,
            invocation_id,
        )?
        .ok_or_else(|| anyhow!("调用事件不存在或不属于当前应用"))?;
    let result = record.invocation.result.clone();
    Ok(DeveloperTerminalEventDetail {
        schema: "open_commerce.developer_terminal_event_detail.v2",
        event: summary_from_record(record)?,
        result,
    })
}

pub(crate) fn summary_from_record(
    record: DeveloperTerminalEventRecord,
) -> Result<DeveloperTerminalEventSummary> {
    let invocation = record.invocation;
    let event_type = match invocation.status.as_str() {
        "succeeded" => "invocation.succeeded",
        "failed" => "invocation.failed",
        _ => bail!("终态事件引用了非终态调用"),
    };
    let completed_at = invocation
        .completed_at
        .clone()
        .ok_or_else(|| anyhow!("终态调用缺少完成时间"))?;
    Ok(DeveloperTerminalEventSummary {
        schema: "open_commerce.developer_terminal_event.v2",
        event_id: invocation.id.clone(),
        event_type,
        invocation_id: invocation.id,
        merchant_id: invocation.merchant_id,
        capability_key: invocation.capability_key,
        credential_environment: invocation.credential_environment,
        credential_id: invocation.credential_id,
        idempotency_key: invocation.idempotency_key,
        status: invocation.status,
        result_available: invocation.result.is_some(),
        error_code: invocation.error_code,
        units: invocation.units,
        amount_micros: invocation.amount_micros,
        currency: invocation.currency,
        settlement_status: invocation.settlement_status,
        funds_moved: false,
        created_at: invocation.created_at,
        completed_at,
    })
}

fn decode_cursor(
    raw: Option<&str>,
    expected_app_id: &str,
    expected_environment: &str,
) -> Result<i64> {
    let Some(raw) = raw.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(0);
    };
    let bytes = URL_SAFE_NO_PAD
        .decode(raw.as_bytes())
        .map_err(|_| anyhow!("开发者事件游标无效"))?;
    let cursor: DeveloperEventCursor =
        serde_json::from_slice(&bytes).map_err(|_| anyhow!("开发者事件游标无效"))?;
    let environment_matches = match cursor.v {
        1 => expected_environment == "sandbox" && cursor.environment.is_none(),
        CURSOR_VERSION => cursor.environment.as_deref() == Some(expected_environment),
        _ => false,
    };
    if cursor.app_id != expected_app_id || cursor.sequence <= 0 || !environment_matches {
        bail!("开发者事件游标无效或不属于当前应用");
    }
    Ok(cursor.sequence)
}

fn encode_cursor(app_id: &str, environment: &str, sequence: i64) -> Result<String> {
    let bytes = serde_json::to_vec(&DeveloperEventCursor {
        v: CURSOR_VERSION,
        app_id: app_id.to_string(),
        environment: Some(environment.to_string()),
        sequence,
    })?;
    Ok(URL_SAFE_NO_PAD.encode(bytes))
}
