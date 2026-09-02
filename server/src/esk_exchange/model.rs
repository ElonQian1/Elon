use anyhow::{anyhow, bail, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::amount::parse_amount;

pub(crate) const EXCHANGE_CONFIRMATION: &str = "CONFIRM PAPER ESK USDT EXCHANGE";
pub(crate) const PAPER_USDT_CREDIT_CONFIRMATION: &str = "RECORD PAPER USDT CREDIT";
pub(crate) const QUOTE_TTL_SECONDS: i64 = 60;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EskExchangeMode {
    Disabled,
    Paper,
    Invalid,
}

impl EskExchangeMode {
    pub(crate) fn from_env() -> Self {
        match std::env::var("ESK_PAPER_EXCHANGE_MODE")
            .ok()
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            None | Some("disabled") => Self::Disabled,
            Some("paper") => Self::Paper,
            Some(_) => Self::Invalid,
        }
    }

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Disabled => "disabled",
            Self::Paper => "paper",
            Self::Invalid => "invalid",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EskExchangeConfig {
    pub price_units: i64,
    pub fee_bps: u16,
    pub revision: String,
}

impl EskExchangeConfig {
    pub(crate) fn from_env() -> Result<Self> {
        let raw_price = std::env::var("ESK_PAPER_USDT_PER_ESK")
            .map_err(|_| anyhow!("ESK Paper 兑换价格尚未配置"))?;
        let price_units = parse_amount(&raw_price, "ESK Paper 价格")?;
        let raw_fee = std::env::var("ESK_PAPER_EXCHANGE_FEE_BPS")
            .map_err(|_| anyhow!("ESK Paper 兑换手续费尚未配置"))?;
        let fee_bps = raw_fee
            .trim()
            .parse::<u16>()
            .map_err(|_| anyhow!("ESK Paper 兑换手续费必须是整数基点"))?;
        if fee_bps > 1_000 {
            bail!("ESK Paper 兑换手续费必须在 0..=1000 基点之间");
        }
        let revision = hex::encode(Sha256::digest(
            format!("esk-paper-exchange-v1:{price_units}:{fee_bps}").as_bytes(),
        ));
        Ok(Self {
            price_units,
            fee_bps,
            revision,
        })
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum EskExchangeDirection {
    UsdtToEsk,
    EskToUsdt,
}

impl EskExchangeDirection {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::UsdtToEsk => "usdt_to_esk",
            Self::EskToUsdt => "esk_to_usdt",
        }
    }

    pub(crate) fn from_label(value: &str) -> Result<Self> {
        match value {
            "usdt_to_esk" => Ok(Self::UsdtToEsk),
            "esk_to_usdt" => Ok(Self::EskToUsdt),
            _ => bail!("兑换方向无效"),
        }
    }

    pub(crate) fn assets(self) -> (&'static str, &'static str) {
        match self {
            Self::UsdtToEsk => ("USDT", "ESK"),
            Self::EskToUsdt => ("ESK", "USDT"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EskExchangeAccountLedger {
    pub usdt_units: i64,
    pub entry_count: i64,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PaperUsdtCreditInput {
    pub user_id: String,
    pub amount_units: i64,
    pub reference: String,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(crate) struct PaperUsdtCreditReceipt {
    pub credit_id: String,
    pub user_id: String,
    pub amount_units: i64,
    pub reference: String,
    pub idempotency_key: String,
    pub created_at: String,
    pub replayed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EskExchangeQuoteInput {
    pub user_id: String,
    pub direction: EskExchangeDirection,
    pub input_units: i64,
    pub price_units: i64,
    pub fee_bps: u16,
    pub config_revision: String,
    pub gross_output_units: i64,
    pub fee_units: i64,
    pub net_output_units: i64,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(crate) struct EskExchangeQuoteRecord {
    pub quote_id: String,
    pub user_id: String,
    pub direction: String,
    pub input_units: i64,
    pub price_units: i64,
    pub fee_bps: u16,
    pub config_revision: String,
    pub gross_output_units: i64,
    pub fee_units: i64,
    pub net_output_units: i64,
    pub created_at: String,
    pub expires_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EskExchangeExecutionInput {
    pub user_id: String,
    pub quote_id: String,
    pub idempotency_key: String,
    pub config_revision: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(crate) struct EskExchangeExecutionRecord {
    pub execution_id: String,
    pub quote: EskExchangeQuoteRecord,
    pub idempotency_key: String,
    pub executed_at: String,
    pub replayed: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CreateExchangeQuoteBody {
    pub direction: EskExchangeDirection,
    pub input_amount: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExecuteExchangeBody {
    pub quote_id: String,
    pub idempotency_key: String,
    pub confirmation: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ExchangeListQuery {
    #[serde(default = "default_limit")]
    pub limit: usize,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PaperUsdtCreditBody {
    pub user_id: String,
    pub amount: String,
    pub reference: String,
    pub idempotency_key: String,
    pub confirmation: String,
}

fn default_limit() -> usize {
    20
}
