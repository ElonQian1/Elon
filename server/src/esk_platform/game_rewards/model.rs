use serde::{Deserialize, Serialize};

pub type Result<T> = anyhow::Result<T>;
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("game rewards disabled")]
    Disabled,
    #[error("invalid reward evidence")]
    Invalid,
    #[error("unauthorized")]
    Unauthorized,
    #[error("conflicting or out-of-order evidence")]
    Conflict,
    #[error("insufficient realized profit")]
    Insufficient,
    #[error("reward record not found")]
    NotFound,
    #[error("reward ledger unavailable")]
    Unavailable,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Signed<T> {
    pub payload: T,
    pub signature_hex: String,
}

/// Cumulative net profit after costs, loss recovery and required reserves, in the exact
/// payout currency's base units. The named reconciler attests the external facts.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Settlement {
    pub schema: String,
    pub policy_digest: String,
    pub user_id: String,
    pub beneficiary: String,
    pub wallet_binding_digest: String,
    pub sequence: String,
    pub previous_report_digest: String,
    pub period_end_ms: String,
    pub cumulative_net_profit_units: String,
    pub reconciliation_digest: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct BudgetIntent {
    pub schema: String,
    pub policy_digest: String,
    pub allocation_hash: String,
    pub report_digest: String,
    pub user_id: String,
    pub beneficiary: String,
    pub mint_authority: String,
    pub creator: String,
    pub content_hash: String,
    pub license_hash: String,
    pub allow_modification: bool,
    pub allow_external_game: bool,
    pub opens_at_ms: String,
    pub direct_claim_after_ms: String,
    pub denominations: Vec<String>,
    pub tickets: Vec<String>,
}

/// An independent, operator-pinned observer signs only after checking the complete
/// funded Budget BCS against this intent and its successful final checkpoint.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct FundingEvidence {
    pub schema: String,
    pub policy_digest: String,
    pub intent_digest: String,
    pub allocation_hash: String,
    pub budget_id: String,
    pub transaction_digest: String,
    pub checkpoint: String,
    pub checkpoint_digest: String,
    pub checkpoint_time_ms: String,
    pub observed_at_ms: String,
    pub amount_units: String,
    pub beneficiary: String,
    pub initial_budget_verified: bool,
}

#[derive(Debug, Serialize)]
pub struct BudgetRecord {
    pub intent: BudgetIntent,
    pub intent_digest: String,
    pub amount_units: String,
    pub state: &'static str,
    pub funding: Option<FundingEvidence>,
    pub replayed: bool,
    pub offchain_payment_authorized: bool,
}

#[derive(Serialize)]
pub struct AccountView {
    pub schema: &'static str,
    pub policy_digest: String,
    pub asset_type: String,
    pub latest_report_digest: Option<String>,
    pub cumulative_net_profit_units: String,
    pub reserved_profit_units: String,
    pub available_profit_units: String,
    pub budgets: Vec<BudgetRecord>,
    pub offchain_payment_authorized: bool,
}
