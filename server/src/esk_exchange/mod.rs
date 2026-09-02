use axum::{
    routing::{get, post},
    Router,
};
use std::sync::Arc;

use crate::types::AppState;

mod amount;
mod api;
mod model;
mod quote;

pub(crate) use amount::{format_amount, parse_amount};
pub(crate) use model::{
    EskExchangeAccountLedger, EskExchangeConfig, EskExchangeDirection, EskExchangeExecutionInput,
    EskExchangeExecutionRecord, EskExchangeMode, EskExchangeQuoteInput, EskExchangeQuoteRecord,
    PaperUsdtCreditInput, PaperUsdtCreditReceipt,
};
pub(crate) use quote::calculate_quote;

pub(crate) fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/api/me/assets/esk/exchange-account",
            get(api::get_my_exchange_account),
        )
        .route(
            "/api/me/assets/esk/exchange-quotes",
            post(api::create_my_exchange_quote),
        )
        .route(
            "/api/me/assets/esk/exchanges",
            get(api::list_my_exchanges).post(api::execute_my_exchange),
        )
        .route(
            "/api/admin/assets/usdt/paper-credits",
            post(api::create_paper_usdt_credit),
        )
}

#[cfg(test)]
mod tests;
