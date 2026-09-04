use axum::{
    routing::{get, post},
    Router,
};
use std::sync::Arc;

use crate::types::AppState;

mod amount;
mod api;
mod batch;
mod batch_api;
#[path = "../esk_exchange/mod.rs"]
pub(crate) mod exchange;
mod model;
#[path = "../esk_platform/mod.rs"]
pub(crate) mod platform;
mod quant_allocation;
mod quant_allocation_api;
mod service;

pub(crate) use amount::{format_esk_amount, parse_esk_amount};
pub(crate) use batch::prepare_paper_allocation_batch;
#[cfg(test)]
pub(crate) use model::override_esk_asset_mode_for_test;
pub(crate) use model::{
    EskAccountLedger, EskAllocationBatchInput, EskAllocationBatchReceipt, EskAllocationInput,
    EskAllocationReceipt, EskAssetMode, EskSellbackInput, EskSellbackRecord, ESK_ASSET_ID,
    ESK_DECIMALS, ESK_NAME, ESK_SYMBOL,
};
pub(crate) use quant_allocation::{
    EskQuantAllocationInput, EskQuantAllocationReceiptInput, EskQuantAllocationRecord,
    ESK_QUANT_RISK_DISCLOSURE_REVISION,
};

pub(crate) fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .merge(platform::routes())
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
            "/api/me/assets/esk/quant-allocation-requests",
            get(quant_allocation_api::list_my_requests)
                .post(quant_allocation_api::create_my_request),
        )
        .route(
            "/api/me/assets/esk/quant-allocation-requests/:request_id/cancel",
            post(quant_allocation_api::cancel_my_request),
        )
        .route(
            "/api/me/assets/esk/quant-allocation-receipts",
            post(quant_allocation_api::apply_my_receipt),
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
mod quant_allocation_tests;
#[cfg(test)]
mod tests;
