use std::sync::Arc;

use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde_json::{json, Value};

use crate::{admin::check_auth, project_auth::json_error, types::AppState};

use super::{
    api::{domain_error, internal_error, require_paper_mode},
    format_esk_amount,
    model::{
        EskAllocationBatchMode, PaperAllocationBatchBody, PAPER_ALLOCATION_BATCH_CONFIRMATION,
    },
    prepare_paper_allocation_batch, EskAllocationBatchInput, EskAllocationBatchReceipt,
};

pub(crate) async fn create_paper_allocation_batch(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<PaperAllocationBatchBody>,
) -> Response {
    if !check_auth(&headers, &state.admin_token) {
        return json_error(StatusCode::UNAUTHORIZED, "无效的管理员令牌");
    }
    if let Some(response) = require_paper_mode() {
        return response;
    }

    let mode = body.mode;
    let expected_request_digest = body.expected_request_digest.clone();
    let confirmation = body.confirmation.clone();
    let input = match prepare_paper_allocation_batch(body) {
        Ok(input) => input,
        Err(error) => return domain_error(error),
    };

    match mode {
        EskAllocationBatchMode::DryRun => {
            match state.store.validate_esk_paper_allocation_batch(&input) {
                Ok(()) => Json(batch_view(&input, None, "validated")).into_response(),
                Err(error) => domain_error(error),
            }
        }
        EskAllocationBatchMode::Commit => {
            if confirmation != PAPER_ALLOCATION_BATCH_CONFIRMATION {
                return json_error(StatusCode::BAD_REQUEST, "ESK Paper 批次登记确认文本不匹配");
            }
            if expected_request_digest.as_deref() != Some(input.request_digest.as_str()) {
                return json_error(
                    StatusCode::BAD_REQUEST,
                    "ESK Paper 批次请求摘要与 dry-run 不匹配",
                );
            }
            match state.store.create_esk_paper_allocation_batch(&input) {
                Ok(receipt) => (
                    StatusCode::CREATED,
                    Json(batch_view(&input, Some(&receipt), "committed")),
                )
                    .into_response(),
                Err(error) => {
                    if error.to_string().contains("写入后不可见") {
                        internal_error("ESK Paper 批次写入后复核失败", error)
                    } else {
                        domain_error(error)
                    }
                }
            }
        }
    }
}

fn batch_view(
    input: &EskAllocationBatchInput,
    receipt: Option<&EskAllocationBatchReceipt>,
    status: &'static str,
) -> Value {
    let entries = input
        .entries
        .iter()
        .enumerate()
        .map(|(index, entry)| {
            let committed = receipt.and_then(|receipt| receipt.entries.get(index));
            json!({
                "ordinal": index,
                "entry_id": committed.map(|entry| entry.entry_id.as_str()),
                "user_id": entry.user_id,
                "amount": format_esk_amount(entry.amount_base_units),
                "amount_base_units": entry.amount_base_units.to_string(),
                "reference": entry.reference,
                "idempotency_key": entry.idempotency_key,
            })
        })
        .collect::<Vec<_>>();
    json!({
        "schema": "yilong.esk.paper_allocation_batch_receipt.v1",
        "batch_id": input.batch_id,
        "request_digest": input.request_digest,
        "mode": if receipt.is_some() { "commit" } else { "dry_run" },
        "status": status,
        "entry_count": input.entries.len(),
        "total": format_esk_amount(input.total_base_units),
        "total_base_units": input.total_base_units.to_string(),
        "created_at": receipt.map(|receipt| receipt.created_at.as_str()),
        "replayed": receipt.is_some_and(|receipt| receipt.replayed),
        "simulated": true,
        "funds_moved": false,
        "entries": entries,
    })
}
