use anyhow::{anyhow, bail, Result};

use super::model::{
    EskAccountLedger, EskAccountView, EskAssetIdentityView, EskAssetMode, EskBalanceView,
    EskSellbackPolicyView, EskSellbackRecord, EskSellbackView, ESK_ASSET_ID, ESK_DECIMALS,
    ESK_NAME, ESK_SCALE, ESK_SYMBOL,
};

pub(crate) fn parse_esk_amount(value: &str) -> Result<i64> {
    let value = value.trim();
    if value.is_empty() || value.starts_with('+') || value.starts_with('-') {
        bail!("ESK 金额必须是正数十进制字符串");
    }
    let mut parts = value.split('.');
    let whole = parts.next().unwrap_or_default();
    let fractional = parts.next();
    if parts.next().is_some()
        || whole.is_empty()
        || !whole.bytes().all(|byte| byte.is_ascii_digit())
    {
        bail!("ESK 金额格式无效");
    }
    let fractional = fractional.unwrap_or_default();
    if (value.contains('.') && fractional.is_empty())
        || fractional.len() > ESK_DECIMALS as usize
        || !fractional.bytes().all(|byte| byte.is_ascii_digit())
    {
        bail!("ESK 金额最多支持六位小数");
    }
    let whole = whole
        .parse::<i64>()
        .map_err(|_| anyhow!("ESK 金额超出范围"))?;
    let padded = format!("{fractional:0<width$}", width = ESK_DECIMALS as usize);
    let fraction = if padded.is_empty() {
        0
    } else {
        padded
            .parse::<i64>()
            .map_err(|_| anyhow!("ESK 金额格式无效"))?
    };
    let base_units = whole
        .checked_mul(ESK_SCALE)
        .and_then(|value| value.checked_add(fraction))
        .ok_or_else(|| anyhow!("ESK 金额超出范围"))?;
    if base_units <= 0 {
        bail!("ESK 金额必须大于 0");
    }
    Ok(base_units)
}

pub(crate) fn format_esk_amount(base_units: i64) -> String {
    let sign = if base_units < 0 { "-" } else { "" };
    let absolute = base_units.unsigned_abs();
    let scale = ESK_SCALE as u64;
    format!(
        "{sign}{}.{:0width$}",
        absolute / scale,
        absolute % scale,
        width = ESK_DECIMALS as usize
    )
}

pub(crate) fn account_view(mode: EskAssetMode, ledger: EskAccountLedger) -> EskAccountView {
    let available = ledger
        .total_base_units
        .saturating_sub(ledger.reserved_base_units)
        .max(0);
    let enabled = mode.writes_enabled();
    let status_message = match mode {
        EskAssetMode::Paper => "Paper 测试登记，尚未上链；卖回仅提交申请，不代表成交或付款。",
        EskAssetMode::Disabled => "ESK 资产写入尚未启用；当前仅展示已登记事实。",
        EskAssetMode::Invalid => "ESK 资产配置无效，写入已失败关闭。",
    };
    EskAccountView {
        schema: "yilong.esk.asset_account.v1",
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
            reserved_for_sellback: format_esk_amount(ledger.reserved_base_units),
            total_base_units: ledger.total_base_units.to_string(),
            available_base_units: available.to_string(),
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
