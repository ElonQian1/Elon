use serde::Serialize;

/// A minimal formal-asset view shared by independently authorized clients.
/// Internal account IDs, payment evidence and write inputs never enter this DTO.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct DelegatedAssetPage {
    pub schema: &'static str,
    pub subject: String,
    pub client_id: String,
    pub expires_at: String,
    pub asset: DelegatedAssetIdentity,
    pub balance: DelegatedAssetBalance,
    pub snapshot_digest: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub progress: Option<DelegatedAssetProgress>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct DelegatedAssetIdentity {
    pub asset_id: &'static str,
    pub symbol: &'static str,
    pub decimals: u8,
    pub source: &'static str,
    pub simulated: bool,
    pub chain_status: &'static str,
    pub funds_moved: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct DelegatedAssetBalance {
    pub total_base_units: String,
    pub reserved_base_units: String,
    pub available_base_units: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct DelegatedAssetProgress {
    pub request_count: String,
    pub open_count: String,
    pub range_start: String,
    pub range_end: String,
    pub requests: Vec<DelegatedSellbackRow>,
    pub has_more: bool,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct DelegatedSellbackRow {
    pub request_id: String,
    pub amount_base_units: String,
    pub status: &'static str,
    pub created_at: String,
    pub canceled_at: Option<String>,
}
