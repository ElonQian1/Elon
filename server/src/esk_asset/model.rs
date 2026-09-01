use serde::{Deserialize, Serialize};

pub(crate) const ESK_ASSET_ID: &str = "esk";
pub(crate) const ESK_SYMBOL: &str = "ESK";
pub(crate) const ESK_NAME: &str = "一龙 ESK";
pub(crate) const ESK_DECIMALS: u32 = 6;
pub(crate) const ESK_SCALE: i64 = 1_000_000;
pub(crate) const PAPER_ALLOCATION_CONFIRMATION: &str = "RECORD PAPER ESK";
pub(crate) const SELLBACK_CANCEL_CONFIRMATION: &str = "CANCEL ESK SELLBACK REQUEST";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EskAssetMode {
    Disabled,
    Paper,
    Invalid,
}

impl EskAssetMode {
    pub(crate) fn from_env() -> Self {
        Self::from_value(std::env::var("ESK_ASSET_MODE").ok().as_deref())
    }

    pub(crate) fn from_value(value: Option<&str>) -> Self {
        match value.map(str::trim).filter(|value| !value.is_empty()) {
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

    pub(crate) fn writes_enabled(self) -> bool {
        matches!(self, Self::Paper)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EskAccountLedger {
    pub total_base_units: i64,
    pub reserved_base_units: i64,
    pub revision: i64,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EskAllocationInput {
    pub user_id: String,
    pub amount_base_units: i64,
    pub reference: String,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(crate) struct EskAllocationReceipt {
    pub entry_id: String,
    pub user_id: String,
    pub amount_base_units: i64,
    pub reference: String,
    pub idempotency_key: String,
    pub created_at: String,
    pub replayed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EskSellbackInput {
    pub user_id: String,
    pub amount_base_units: i64,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(crate) struct EskSellbackRecord {
    pub request_id: String,
    pub user_id: String,
    pub amount_base_units: i64,
    pub status: String,
    pub revision: i64,
    pub submitted_at: String,
    pub updated_at: String,
    pub replayed: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PaperAllocationBody {
    pub user_id: String,
    pub amount: String,
    pub reference: String,
    pub idempotency_key: String,
    pub confirmation: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CreateSellbackBody {
    pub amount: String,
    pub idempotency_key: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CancelSellbackBody {
    pub confirmation: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct SellbackListQuery {
    #[serde(default = "default_limit")]
    pub limit: usize,
}

fn default_limit() -> usize {
    20
}

#[derive(Debug, Serialize)]
pub(crate) struct EskAssetIdentityView {
    pub asset_id: &'static str,
    pub symbol: &'static str,
    pub name: &'static str,
    pub decimals: u32,
    pub issuance_mode: &'static str,
    pub chain_status: &'static str,
    pub contract_address: Option<String>,
}

#[derive(Debug, Serialize)]
pub(crate) struct EskBalanceView {
    pub total: String,
    pub available: String,
    pub reserved_for_sellback: String,
    pub total_base_units: String,
    pub available_base_units: String,
    pub reserved_base_units: String,
    pub revision: i64,
    pub updated_at: Option<String>,
}

#[derive(Debug, Serialize)]
pub(crate) struct EskSellbackPolicyView {
    pub application_only: bool,
    pub request_enabled: bool,
    pub settlement_enabled: bool,
    pub pricing_status: &'static str,
}

#[derive(Debug, Serialize)]
pub(crate) struct EskAccountView {
    pub schema: &'static str,
    pub mode: &'static str,
    pub enabled: bool,
    pub simulated: bool,
    pub funds_moved: bool,
    pub asset: EskAssetIdentityView,
    pub balance: EskBalanceView,
    pub sellback: EskSellbackPolicyView,
    pub status_message: &'static str,
}

#[derive(Debug, Serialize)]
pub(crate) struct EskSellbackView {
    pub request_id: String,
    pub amount: String,
    pub amount_base_units: String,
    pub status: String,
    pub revision: i64,
    pub submitted_at: String,
    pub updated_at: String,
    pub simulated: bool,
    pub funds_moved: bool,
    pub replayed: bool,
}
