use crate::settlement::{Settlement, Signed};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub schema: String,
    pub policy_digest: String,
    pub source_id: String,
    pub source_public_key_hex: String,
    pub reconciler_public_key_hex: String,
    pub account_scope: String,
    pub user_id: String,
    pub beneficiary: String,
    pub wallet_binding_digest: String,
    pub asset_type: String,
    pub asset_decimals: u8,
    pub accounting_started_at_ms: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Environment {
    Live,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Basis {
    ParticipantCumulativeCashV1,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceKind {
    AccountOwnership,
    CashflowReconciliation,
    TradeCosts,
    ParticipantAllocation,
    ReserveCoverage,
}
pub const EVIDENCE_KINDS: [EvidenceKind; 5] = [
    EvidenceKind::AccountOwnership,
    EvidenceKind::CashflowReconciliation,
    EvidenceKind::TradeCosts,
    EvidenceKind::ParticipantAllocation,
    EvidenceKind::ReserveCoverage,
];

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvidenceRef {
    pub kind: EvidenceKind,
    pub sha256: String,
}

/// Cumulative amounts belong to one participant, one asset, one accounting origin.
/// A positive funding amount is income; a negative funding amount is expense.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Statement {
    pub schema: String,
    pub source_id: String,
    pub environment: Environment,
    pub basis: Basis,
    pub policy_digest: String,
    pub account_scope: String,
    pub user_id: String,
    pub beneficiary: String,
    pub wallet_binding_digest: String,
    pub asset_type: String,
    pub asset_decimals: u8,
    pub accounting_started_at_ms: String,
    pub sequence: String,
    pub previous_source_digest: String,
    pub period_start_ms: String,
    pub period_end_ms: String,
    pub cumulative_realized_trading_pnl_units: String,
    pub cumulative_trading_fees_units: String,
    pub cumulative_net_funding_units: String,
    pub cumulative_other_costs_units: String,
    pub held_reserve_units: String,
    pub principal_liability_units: String,
    pub unrealized_pnl_units: String,
    pub evidence: Vec<EvidenceRef>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Previous {
    pub statement: Signed<Statement>,
    pub settlement: Signed<Settlement>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub schema: String,
    pub statement: Signed<Statement>,
    pub previous: Option<Previous>,
}

#[derive(Debug, Serialize)]
pub struct Calculation {
    pub schema: &'static str,
    pub source_public_key_hex: String,
    pub source_digest: String,
    pub cumulative_realized_trading_pnl_units: String,
    pub cumulative_trading_fees_units: String,
    pub cumulative_net_funding_units: String,
    pub cumulative_other_costs_units: String,
    pub held_reserve_units: String,
    pub cumulative_net_profit_units: String,
    pub excluded_principal_liability_units: String,
    pub excluded_unrealized_pnl_units: String,
}

/// Deliberately not a Signed<Settlement>, and no Deserialize/capability semantics.
#[derive(Debug, Serialize)]
pub struct Candidate {
    pub schema: &'static str,
    pub statement: Signed<Statement>,
    pub calculation: Calculation,
    pub settlement: Settlement,
    pub settlement_signing_bytes_hex: String,
    pub settlement_signature_present: bool,
    pub external_facts_independently_verified: bool,
    pub offchain_payment_authorized: bool,
    pub funds_moved: bool,
}
