use anyhow::{bail, Result};

use super::amount::{format_esk_amount, parse_esk_amount};
use super::model::{
    EskAccountLedger, EskAccountView, EskAssetIdentityView, EskAssetMode, EskBalanceView,
    EskSellbackPolicyView, EskSellbackRecord, EskSellbackView, ESK_ASSET_ID, ESK_DECIMALS,
    ESK_NAME, ESK_SYMBOL,
};

pub(crate) fn account_view(mode: EskAssetMode, ledger: EskAccountLedger) -> EskAccountView {
    let available = ledger
        .total_base_units
        .saturating_sub(ledger.reserved_base_units)
        .max(0);
    let enabled = mode.writes_enabled();
    let status_message = match mode {
        EskAssetMode::Paper => "Paper 测试登记，尚未上链；可用额已扣除卖回和量化 Paper 分配申请占用，两类申请都不代表成交、入金或收益。",
        EskAssetMode::Disabled => "ESK 资产写入尚未启用；当前仅展示已登记事实。",
        EskAssetMode::Invalid => "ESK 资产配置无效，写入已失败关闭。",
    };
    EskAccountView {
        schema: "yilong.esk.asset_account.v2",
        mode: mode.label(),
        enabled,
        simulated: true,
        funds_moved: false,
        asset: EskAssetIdentityView {
            asset_id: ESK_ASSET_ID,
            symbol: ESK_SYMBOL,
            name: ESK_NAME,
            decimals: ESK_DECIMALS,
            issuance_mode: "paper_recorded",
            chain_status: "not_deployed",
            contract_address: None,
        },
        balance: EskBalanceView {
            total: format_esk_amount(ledger.total_base_units),
            available: format_esk_amount(available),
            reserved_for_sellback: format_esk_amount(ledger.sellback_reserved_base_units),
            reserved_for_quant: format_esk_amount(ledger.quant_reserved_base_units),
            reserved_total: format_esk_amount(ledger.reserved_base_units),
            total_base_units: ledger.total_base_units.to_string(),
            available_base_units: available.to_string(),
            sellback_reserved_base_units: ledger.sellback_reserved_base_units.to_string(),
            quant_reserved_base_units: ledger.quant_reserved_base_units.to_string(),
            reserved_base_units: ledger.reserved_base_units.to_string(),
            revision: ledger.revision,
            updated_at: ledger.updated_at,
        },
        sellback: EskSellbackPolicyView {
            application_only: true,
            request_enabled: enabled && available > 0,
            settlement_enabled: false,
            pricing_status: "not_defined",
        },
        status_message,
    }
}

pub(crate) fn sellback_view(record: EskSellbackRecord) -> EskSellbackView {
    EskSellbackView {
        request_id: record.request_id,
        amount: format_esk_amount(record.amount_base_units),
        amount_base_units: record.amount_base_units.to_string(),
        status: record.status,
        revision: record.revision,
        submitted_at: record.submitted_at,
        updated_at: record.updated_at,
        simulated: true,
        funds_moved: false,
        replayed: record.replayed,
    }
}

pub(crate) fn validate_bounded_label(value: &str, label: &str, max_chars: usize) -> Result<String> {
    let value = value.trim();
    let length = value.chars().count();
    if length == 0 || length > max_chars || value.chars().any(char::is_control) {
        bail!("{label} 必须为 1..={max_chars} 个可见字符");
    }
    Ok(value.to_string())
}
