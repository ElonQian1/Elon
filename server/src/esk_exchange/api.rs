use axum::{
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde_json::{json, Value};
use std::sync::Arc;

use crate::{
    admin::check_auth,
    esk_asset::EskAssetMode,
    project_auth::{auth_from_headers, json_error},
    types::AppState,
};

use super::{
    calculate_quote, format_amount,
    model::{
        CreateExchangeQuoteBody, EskExchangeConfig, EskExchangeExecutionInput, EskExchangeMode,
        EskExchangeQuoteInput, ExchangeListQuery, ExecuteExchangeBody, PaperUsdtCreditBody,
        EXCHANGE_CONFIRMATION, PAPER_USDT_CREDIT_CONFIRMATION, QUOTE_TTL_SECONDS,
    },
    parse_amount, EskExchangeDirection, EskExchangeExecutionRecord, EskExchangeQuoteRecord,
    PaperUsdtCreditInput,
};

pub(crate) async fn get_my_exchange_account(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(_) => return json_error(StatusCode::UNAUTHORIZED, "未登录"),
    };
    let exchange = match state.store.esk_exchange_account_ledger(&user.id) {
        Ok(value) => value,
        Err(error) => return internal_error("读取 USDT Paper 余额失败", error),
    };
    let esk = match state.store.esk_account_ledger(&user.id) {
        Ok(value) => value,
        Err(error) => return internal_error("读取 ESK Paper 余额失败", error),
    };
    let mode = EskExchangeMode::from_env();
    let config = EskExchangeConfig::from_env().ok();
    let enabled = mode == EskExchangeMode::Paper
        && EskAssetMode::from_env() == EskAssetMode::Paper
        && config.is_some();
    let available_esk = esk
        .total_base_units
        .saturating_sub(esk.reserved_base_units)
        .max(0);
    Json(json!({
        "schema": "yilong.esk.paper_exchange_account.v1",
        "mode": mode.label(),
        "enabled": enabled,
        "simulated": true,
        "funds_moved": false,
        "on_chain_settlement": false,
        "trading_mode": "paper",
        "balances": {
            "esk": amount_view(esk.total_base_units, available_esk, esk.revision, esk.updated_at),
            "usdt": amount_view(exchange.usdt_units, exchange.usdt_units, exchange.entry_count, exchange.updated_at),
        },
        "pricing": config.as_ref().map(|value| json!({
            "usdt_per_esk": format_amount(value.price_units),
            "price_base_units": value.price_units.to_string(),
            "fee_bps": value.fee_bps,
            "fee_percent": format!("{}.{:02}%", value.fee_bps / 100, value.fee_bps % 100),
            "config_revision": value.revision,
            "quote_ttl_seconds": QUOTE_TTL_SECONDS,
        })),
        "status_message": exchange_status(mode, enabled),
    }))
    .into_response()
}

pub(crate) async fn create_my_exchange_quote(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<CreateExchangeQuoteBody>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(_) => return json_error(StatusCode::UNAUTHORIZED, "未登录"),
    };
    let config = match require_exchange_config() {
        Ok(value) => value,
        Err(response) => return response,
    };
    let (source, _) = body.direction.assets();
    let input_units = match parse_amount(&body.input_amount, &format!("{source} 兑换金额")) {
        Ok(value) => value,
        Err(error) => return json_error(StatusCode::BAD_REQUEST, error),
    };
    let (gross_output_units, fee_units, net_output_units) = match calculate_quote(
        body.direction,
        input_units,
        config.price_units,
        config.fee_bps,
    ) {
        Ok(value) => value,
        Err(error) => return json_error(StatusCode::BAD_REQUEST, error),
    };
    let input = EskExchangeQuoteInput {
        user_id: user.id,
        direction: body.direction,
        input_units,
        price_units: config.price_units,
        fee_bps: config.fee_bps,
        config_revision: config.revision,
        gross_output_units,
        fee_units,
        net_output_units,
    };
    match state.store.create_esk_exchange_quote(&input) {
        Ok(quote) => (StatusCode::CREATED, Json(quote_view(&quote))).into_response(),
        Err(error) => domain_error(error),
    }
}

pub(crate) async fn execute_my_exchange(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<ExecuteExchangeBody>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(_) => return json_error(StatusCode::UNAUTHORIZED, "未登录"),
    };
    let config = match require_exchange_config() {
        Ok(value) => value,
        Err(response) => return response,
    };
    if body.confirmation != EXCHANGE_CONFIRMATION {
        return json_error(StatusCode::BAD_REQUEST, "Paper 兑换确认文本不匹配");
    }
    let input = EskExchangeExecutionInput {
        user_id: user.id,
        quote_id: match bounded(&body.quote_id, "报价 ID", 160) {
            Ok(value) => value,
            Err(response) => return response,
        },
        idempotency_key: match bounded(&body.idempotency_key, "幂等键", 160) {
            Ok(value) => value,
            Err(response) => return response,
        },
        config_revision: config.revision,
    };
    match state.store.execute_esk_exchange(&input) {
        Ok(record) => {
            let status = if record.replayed {
                StatusCode::OK
            } else {
                StatusCode::CREATED
            };
            (status, Json(execution_view(&record))).into_response()
        }
        Err(error) => domain_error(error),
    }
}

pub(crate) async fn list_my_exchanges(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(query): Query<ExchangeListQuery>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(_) => return json_error(StatusCode::UNAUTHORIZED, "未登录"),
    };
    match state.store.list_esk_exchanges(&user.id, query.limit) {
        Ok(records) => Json(json!({
            "schema": "yilong.esk.paper_exchange_execution_list.v1",
            "simulated": true,
            "funds_moved": false,
            "on_chain_settlement": false,
            "trading_mode": "paper",
            "executions": records.iter().map(execution_view).collect::<Vec<_>>(),
        }))
        .into_response(),
        Err(error) => internal_error("读取 Paper 兑换流水失败", error),
    }
}

pub(crate) async fn create_paper_usdt_credit(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<PaperUsdtCreditBody>,
) -> Response {
    if !check_auth(&headers, &state.admin_token) {
        return json_error(StatusCode::UNAUTHORIZED, "无效的管理员令牌");
    }
    if let Err(response) = require_exchange_mode() {
        return response;
    }
    if body.confirmation != PAPER_USDT_CREDIT_CONFIRMATION {
        return json_error(StatusCode::BAD_REQUEST, "USDT Paper 登记确认文本不匹配");
    }
    let input = PaperUsdtCreditInput {
        user_id: match bounded(&body.user_id, "用户 ID", 160) {
            Ok(value) => value,
            Err(response) => return response,
        },
        amount_units: match parse_amount(&body.amount, "USDT Paper 登记金额") {
            Ok(value) => value,
            Err(error) => return json_error(StatusCode::BAD_REQUEST, error),
        },
        reference: match bounded(&body.reference, "登记引用", 240) {
            Ok(value) => value,
            Err(response) => return response,
        },
        idempotency_key: match bounded(&body.idempotency_key, "幂等键", 160) {
            Ok(value) => value,
            Err(response) => return response,
        },
    };
    match state.store.create_paper_usdt_credit(&input) {
        Ok(receipt) => (
            StatusCode::CREATED,
            Json(json!({
                "schema": "yilong.usdt.paper_credit_receipt.v1",
                "credit_id": receipt.credit_id,
                "user_id": receipt.user_id,
                "amount": format_amount(receipt.amount_units),
                "amount_base_units": receipt.amount_units.to_string(),
                "reference": receipt.reference,
                "created_at": receipt.created_at,
                "replayed": receipt.replayed,
                "simulated": true,
                "funds_moved": false,
                "on_chain_settlement": false,
                "trading_mode": "paper",
            })),
        )
            .into_response(),
        Err(error) => domain_error(error),
    }
}

fn amount_view(total: i64, available: i64, revision: i64, updated_at: Option<String>) -> Value {
    json!({
        "total": format_amount(total), "available": format_amount(available),
        "total_base_units": total.to_string(), "available_base_units": available.to_string(),
        "revision": revision, "updated_at": updated_at,
    })
}

fn quote_view(quote: &EskExchangeQuoteRecord) -> Value {
    let direction =
        EskExchangeDirection::from_label(&quote.direction).expect("stored exchange direction");
    let (source, target) = direction.assets();
    json!({
        "schema": "yilong.esk.paper_exchange_quote.v1",
        "quote_id": quote.quote_id, "direction": quote.direction,
        "input_asset": source, "output_asset": target,
        "input_amount": format_amount(quote.input_units), "input_base_units": quote.input_units.to_string(),
        "gross_output_amount": format_amount(quote.gross_output_units), "gross_output_base_units": quote.gross_output_units.to_string(),
        "fee_asset": target, "fee_amount": format_amount(quote.fee_units), "fee_base_units": quote.fee_units.to_string(),
        "net_output_amount": format_amount(quote.net_output_units), "net_output_base_units": quote.net_output_units.to_string(),
        "usdt_per_esk": format_amount(quote.price_units), "price_base_units": quote.price_units.to_string(),
        "fee_bps": quote.fee_bps, "config_revision": quote.config_revision,
        "created_at": quote.created_at, "expires_at": quote.expires_at,
        "simulated": true, "funds_moved": false, "on_chain_settlement": false, "trading_mode": "paper",
    })
}

fn execution_view(record: &EskExchangeExecutionRecord) -> Value {
    json!({
        "schema": "yilong.esk.paper_exchange_execution.v1",
        "execution_id": record.execution_id, "executed_at": record.executed_at,
        "replayed": record.replayed, "quote": quote_view(&record.quote),
        "simulated": true, "funds_moved": false, "on_chain_settlement": false, "trading_mode": "paper",
    })
}

fn require_exchange_mode() -> Result<(), Response> {
    match (EskExchangeMode::from_env(), EskAssetMode::from_env()) {
        (EskExchangeMode::Paper, EskAssetMode::Paper) => Ok(()),
        (EskExchangeMode::Invalid, _) | (_, EskAssetMode::Invalid) => Err(json_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "ESK/USDT Paper 兑换模式配置无效，已失败关闭",
        )),
        _ => Err(json_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "ESK/USDT Paper 兑换尚未启用",
        )),
    }
}

fn require_exchange_config() -> Result<EskExchangeConfig, Response> {
    require_exchange_mode()?;
    EskExchangeConfig::from_env().map_err(|error| {
        json_error(
            StatusCode::SERVICE_UNAVAILABLE,
            format!("{error}，兑换已失败关闭"),
        )
    })
}

fn exchange_status(mode: EskExchangeMode, enabled: bool) -> &'static str {
    if enabled {
        "Paper 模拟兑换已启用；报价和流水均未上链，也不会移动真实 USDT 或 ESK。"
    } else if mode == EskExchangeMode::Invalid {
        "Paper 兑换配置无效，写入已失败关闭。"
    } else {
        "Paper 兑换尚未启用；当前只显示已登记余额。"
    }
}

fn bounded(value: &str, label: &str, max: usize) -> Result<String, Response> {
    let value = value.trim();
    if value.is_empty() || value.chars().count() > max || value.chars().any(char::is_control) {
        return Err(json_error(StatusCode::BAD_REQUEST, format!("{label}无效")));
    }
    Ok(value.to_string())
}

fn domain_error(error: anyhow::Error) -> Response {
    let message = error.to_string();
    let status = if message.contains("超过")
        || message.contains("已经")
        || message.contains("过期")
        || message.contains("幂等键")
    {
        StatusCode::CONFLICT
    } else if message.contains("不存在") {
        StatusCode::NOT_FOUND
    } else if message.contains("无效") || message.contains("必须") || message.contains("过小")
    {
        StatusCode::BAD_REQUEST
    } else {
        tracing::warn!(error = %message, "ESK exchange request failed");
        return json_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "ESK/USDT Paper 兑换操作失败",
        );
    };
    json_error(status, message)
}

fn internal_error(context: &'static str, error: anyhow::Error) -> Response {
    tracing::warn!(error = %error, context, "ESK exchange storage failed");
    json_error(StatusCode::INTERNAL_SERVER_ERROR, context)
}
