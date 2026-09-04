use serde::{Deserialize, Serialize};

pub(crate) const POLICY_SCHEMA: &str = "yilong.esk.platform_sellback_policy.v1";
pub(crate) const SUBMIT_SCHEMA: &str = "yilong.esk.platform_sellback_submit.v1";
pub(crate) const CANCEL_SCHEMA: &str = "yilong.esk.platform_sellback_cancel.v1";
pub(crate) const LOOKUP_SCHEMA: &str = "yilong.esk.platform_sellback_lookup.v1";
pub(crate) const MAX_PAGE_SIZE: usize = 20;
pub(crate) const SUBMIT_CONFIRMATION: &str = "SUBMIT PLATFORM ESK SELLBACK REQUEST";
pub(crate) const CANCEL_CONFIRMATION: &str = "CANCEL PLATFORM ESK SELLBACK REQUEST";

/// Explicit operator-approved candidate policy; no commercial default values.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SellbackPolicyBody {
    pub schema: String,
    pub revision: String,
    pub approval_digest: String,
    pub source_fingerprint: String,
    pub eligible_user_ids: Vec<String>,
    pub min_request_base_units: String,
    pub max_request_base_units: String,
    pub max_open_requests_per_user: String,
    pub max_reserved_base_units_per_user: String,
    pub max_reserved_base_units_global: String,
    pub hold_mode: String,
    pub cancel_mode: String,
    pub expiry_mode: String,
    pub participation_effect: String,
    pub disabled_account_recovery_text: String,
    pub terms_text: String,
    pub terms_digest: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SellbackPolicy {
    pub body: SellbackPolicyBody,
    pub policy_digest: String,
}

/// Invalid current configuration must not disable old reads, cancels or exact replays.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum SellbackConfiguration {
    Disabled,
    Invalid,
    Enabled(SellbackPolicy),
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SellbackSubmitBody {
    pub schema: String,
    pub idempotency_key: String,
    pub amount_base_units: String,
    pub expected_snapshot_digest: String,
    pub policy_digest: String,
    pub terms_digest: String,
    pub confirmation: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SellbackSubmitInput {
    pub idempotency_key: String,
    pub amount_base_units: i64,
    pub expected_snapshot_digest: String,
    pub policy_digest: String,
    pub terms_digest: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SellbackCancelBody {
    pub schema: String,
    pub confirmation: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SellbackLookupBody {
    pub schema: String,
    pub idempotency_key: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SellbackCursor {
    pub snapshot_digest: String,
    pub after_request_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SellbackRecord {
    pub request_id: String,
    pub user_id: String,
    pub input: SellbackSubmitInput,
    pub policy: SellbackPolicy,
    pub request_digest: String,
    pub created_at: String,
    pub canceled_at: Option<String>,
    pub cancel_event_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SellbackAvailability {
    pub new_requests_enabled: bool,
    /// A fixed category, never a configuration exception or private eligible-user list.
    pub reason: String,
    pub policy: Option<SellbackPolicy>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SellbackSummary {
    pub snapshot_digest: String,
    pub total_base_units: i64,
    pub reserved_base_units: i64,
    pub available_base_units: i64,
    pub open_request_count: i64,
    pub request_count: i64,
    pub availability: SellbackAvailability,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SellbackPage {
    pub summary: SellbackSummary,
    pub requests: Vec<SellbackRecord>,
    pub range_start: i64,
    pub range_end: i64,
    pub has_more: bool,
    pub next_cursor: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SellbackResult {
    pub request: SellbackRecord,
    pub summary: SellbackSummary,
    pub replayed: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SellbackError {
    Unauthorized,
    InvalidInput,
    Disabled,
    Ineligible,
    PolicyChanged,
    Conflict,
    SnapshotChanged,
    LimitExceeded,
    InsufficientAvailable,
    NotFound,
    Corrupt,
}

impl std::fmt::Display for SellbackError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Unauthorized => "ESK_PLATFORM_SELLBACK_UNAUTHORIZED",
            Self::InvalidInput => "ESK_PLATFORM_SELLBACK_INVALID_INPUT",
            Self::Disabled => "ESK_PLATFORM_SELLBACK_DISABLED",
            Self::Ineligible => "ESK_PLATFORM_SELLBACK_INELIGIBLE",
            Self::PolicyChanged => "ESK_PLATFORM_SELLBACK_POLICY_CHANGED",
            Self::Conflict => "ESK_PLATFORM_SELLBACK_CONFLICT",
            Self::SnapshotChanged => "ESK_PLATFORM_SELLBACK_SNAPSHOT_CHANGED",
            Self::LimitExceeded => "ESK_PLATFORM_SELLBACK_LIMIT_EXCEEDED",
            Self::InsufficientAvailable => "ESK_PLATFORM_SELLBACK_INSUFFICIENT_AVAILABLE",
            Self::NotFound => "ESK_PLATFORM_SELLBACK_NOT_FOUND",
            Self::Corrupt => "ESK_PLATFORM_SELLBACK_CORRUPT",
        })
    }
}

impl std::error::Error for SellbackError {}
