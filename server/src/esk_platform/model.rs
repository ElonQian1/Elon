use serde::{Deserialize, Serialize};

pub(crate) const PREPARE_SCHEMA: &str = "yilong.esk.platform_allocation_input.v1";
pub(crate) const RECORD_CONFIRMATION: &str = "APPROVE AND RECORD PLATFORM ESK";
pub(crate) const CANCEL_CONFIRMATION: &str = "CANCEL PLATFORM ESK PREPARATION";

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct PaymentSource {
    pub namespace: String,
    pub network: String,
    pub asset_symbol: String,
    pub asset_reference: String,
    pub decimals: u32,
    pub reference_format: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PolicyBody {
    pub source: PaymentSource,
    pub issuance_limit_base_units: String,
}

#[derive(Debug, Clone)]
pub(crate) struct PlatformPolicy {
    pub source: PaymentSource,
    pub source_fingerprint: String,
    pub policy_digest: String,
    pub issuance_limit_base_units: i64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SaleTerms {
    pub sale_batch_id: String,
    pub payment_base_units_per_lot: String,
    pub esk_base_units_per_lot: String,
    pub disclosure_revision: String,
    pub terms_digest: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PrepareBody {
    pub schema: String,
    pub user_id: String,
    pub external_payment_reference: String,
    pub transfer_index: u32,
    pub payment_amount: String,
    pub amount: String,
    pub commercial_purpose: String,
    pub sale: SaleTerms,
    pub payment_evidence_digest: String,
    pub consent_digest: String,
    pub history_evidence_digest: String,
    pub history_complete: bool,
    pub review_reference: String,
}

/// Contains audit hashes, never raw payment references or credentials.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct PlatformAllocationInput {
    pub user_id: String,
    pub source_fingerprint: String,
    pub policy_digest: String,
    pub payment_key: String,
    pub payment_base_units: String,
    pub amount_base_units: i64,
    pub sale_terms_digest: String,
    pub payment_evidence_digest: String,
    pub consent_digest: String,
    pub history_evidence_digest: String,
    pub review_reference_digest: String,
    pub request_digest: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RecordBody {
    pub expected_request_digest: String,
    pub confirmation: String,
}

#[derive(Debug, Clone)]
pub(crate) struct PlatformAllocationRecord {
    pub allocation_id: String,
    pub input: PlatformAllocationInput,
    pub prepared_by: String,
    pub prepared_at: String,
    pub recorded_at: Option<String>,
    pub canceled_at: Option<String>,
    pub replayed: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct PlatformEntry {
    pub entry_id: String,
    pub allocation_id: String,
    pub amount_base_units: i64,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub(crate) struct PlatformAccount {
    pub total_base_units: i64,
    pub entry_count: i64,
    pub updated_at: Option<String>,
    pub entries: Vec<PlatformEntry>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PlatformError {
    InvalidInput,
    Disabled,
    InvalidPolicy,
    Unauthorized,
    UserUnavailable,
    Conflict,
    PolicyChanged,
    LimitExceeded,
    NotFound,
    CorruptLedger,
}

impl std::fmt::Display for PlatformError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::InvalidInput => "ESK_PLATFORM_INVALID_INPUT",
            Self::Disabled => "ESK_PLATFORM_WRITES_DISABLED",
            Self::InvalidPolicy => "ESK_PLATFORM_INVALID_POLICY",
            Self::Unauthorized => "ESK_PLATFORM_NOT_AUTHORIZED",
            Self::UserUnavailable => "ESK_PLATFORM_USER_UNAVAILABLE",
            Self::Conflict => "ESK_PLATFORM_PAYMENT_CONFLICT",
            Self::PolicyChanged => "ESK_PLATFORM_POLICY_CHANGED",
            Self::LimitExceeded => "ESK_PLATFORM_LIMIT_EXCEEDED",
            Self::NotFound => "ESK_PLATFORM_NOT_FOUND",
            Self::CorruptLedger => "ESK_PLATFORM_LEDGER_INCONSISTENT",
        })
    }
}

impl std::error::Error for PlatformError {}
