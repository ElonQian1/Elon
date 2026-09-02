use axum::{
    routing::{get, post},
    Router,
};
use std::sync::Arc;

use crate::types::AppState;

mod api;
mod batch;
mod batch_api;
mod model;
mod service;

pub(crate) use batch::prepare_paper_allocation_batch;
pub(crate) use model::{
    EskAccountLedger, EskAllocationBatchInput, EskAllocationBatchReceipt, EskAllocationInput,
    EskAllocationReceipt, EskAssetMode, EskSellbackInput, EskSellbackRecord, ESK_ASSET_ID,
    ESK_DECIMALS, ESK_NAME, ESK_SYMBOL,
};
pub(crate) use service::{format_esk_amount, parse_esk_amount};

pub(crate) fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/me/assets/esk", get(api::get_my_account))
        .route(
            "/api/me/assets/esk/sellback-requests",
            get(api::list_my_sellback_requests).post(api::create_my_sellback_request),
        )
        .route(
            "/api/me/assets/esk/sellback-requests/:request_id/cancel",
            post(api::cancel_my_sellback_request),
        )
        .route(
            "/api/admin/assets/esk/paper-allocations",
            post(api::create_paper_allocation),
        )
        .route(
            "/api/admin/assets/esk/paper-allocation-batches",
            post(batch_api::create_paper_allocation_batch),
        )
}

#[cfg(test)]
mod batch_tests;
#[cfg(test)]
mod tests;
