use serde::{Deserialize, Serialize};

pub(crate) const ECONOMY_SCHEMA: &str = "task_economy.shadow.v1";
pub(crate) const CURRENCY_CNY: &str = "CNY";
pub(crate) const INTENT_PENDING: &str = "pending";
pub(crate) const INTENT_POSTED: &str = "posted";
pub(crate) const INTENT_VOIDED: &str = "voided";
pub(crate) const RECEIPT_RECONCILED: &str = "reconciled";
pub(crate) const RECEIPT_VOIDED: &str = "voided";
pub(crate) const SUBJECT_TASK_ASSIGNMENT: &str = "task_assignment";
pub(crate) const SUBJECT_COMMERCE_INVOCATION: &str = "commerce_invocation";

#[derive(Debug, Clone, Serialize)]
pub(crate) struct TaskEconomyProjectSetting {
    pub project_id: String,
    pub enabled: bool,
    pub shadow_only: bool,
    pub updated_by_user_id: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct UpdateTaskEconomyProjectSettingRequest {
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct UsageReceipt {
    pub id: String,
    pub project_id: String,
    pub subject_type: String,
    pub subject_id: String,
    pub source_type: String,
    pub source_id: String,
    pub source_digest: String,
    pub consumer_user_id: String,
    pub provider_user_id: Option<String>,
    pub units: i64,
    pub amount_micros: i64,
    pub provider_amount_micros: i64,
    pub currency: String,
    pub billing_source: String,
    pub source_status: String,
    pub occurred_at: String,
    pub created_at: String,
}

pub(crate) struct CreateUsageReceipt<'a> {
    pub project_id: &'a str,
    pub subject_type: &'a str,
    pub subject_id: &'a str,
    pub source_type: &'a str,
    pub source_id: &'a str,
    pub source_digest: &'a str,
    pub consumer_user_id: &'a str,
    pub provider_user_id: Option<&'a str>,
    pub units: i64,
    pub amount_micros: i64,
    pub provider_amount_micros: i64,
    pub currency: &'a str,
    pub billing_source: &'a str,
    pub source_status: &'a str,
    pub occurred_at: &'a str,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct SettlementIntent {
    pub id: String,
    pub project_id: String,
    pub matter_id: Option<String>,
    pub assignment_id: Option<String>,
    pub payer_user_id: String,
    pub payee_user_id: Option<String>,
    pub idempotency_key: String,
    pub policy_version: String,
    pub policy_digest: String,
    pub status: String,
    pub shadow_only: bool,
    pub created_at: String,
    pub updated_at: String,
}

pub(crate) struct CreateSettlementIntent<'a> {
    pub project_id: &'a str,
    pub matter_id: Option<&'a str>,
    pub assignment_id: Option<&'a str>,
    pub payer_user_id: &'a str,
    pub payee_user_id: Option<&'a str>,
    pub idempotency_key: &'a str,
    pub policy_version: &'a str,
    pub policy_digest: &'a str,
    pub usage_receipt_id: &'a str,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct SettlementReceipt {
    pub id: String,
    pub project_id: String,
    pub intent_id: String,
    pub posting_key: String,
    pub status: String,
    pub compute_amount_micros: i64,
    pub provider_amount_micros: i64,
    pub platform_amount_micros: i64,
    pub outcome_reward_micros: i64,
    pub review_reward_micros: i64,
    pub currency: String,
    pub shadow_only: bool,
    pub accepted_matter_id: Option<String>,
    pub reason: String,
    pub created_at: String,
}

pub(crate) struct CreateSettlementReceipt<'a> {
    pub project_id: &'a str,
    pub intent_id: &'a str,
    pub posting_key: &'a str,
    pub status: &'a str,
    pub compute_amount_micros: i64,
    pub provider_amount_micros: i64,
    pub platform_amount_micros: i64,
    pub outcome_reward_micros: i64,
    pub review_reward_micros: i64,
    pub currency: &'a str,
    pub accepted_matter_id: Option<&'a str>,
    pub reason: &'a str,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct LedgerEntry {
    pub id: String,
    pub transaction_id: String,
    pub account_key: String,
    pub user_id: Option<String>,
    pub side: String,
    pub amount_micros: i64,
    pub currency: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct LedgerTransaction {
    pub id: String,
    pub project_id: String,
    pub settlement_receipt_id: String,
    pub posting_key: String,
    pub description: String,
    pub created_at: String,
    pub entries: Vec<LedgerEntry>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub(crate) struct TaskEconomyTotals {
    pub usage_receipts: usize,
    pub pending_intents: usize,
    pub posted_intents: usize,
    pub voided_intents: usize,
    pub settlement_receipts: usize,
    pub compute_amount_micros: i64,
    pub provider_amount_micros: i64,
    pub platform_amount_micros: i64,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct TaskEconomyOverview {
    pub schema: &'static str,
    pub project_id: String,
    pub runtime_enabled: bool,
    pub project_setting: TaskEconomyProjectSetting,
    pub shadow_only: bool,
    pub totals: TaskEconomyTotals,
    pub usage_receipts: Vec<UsageReceipt>,
    pub intents: Vec<SettlementIntent>,
    pub settlement_receipts: Vec<SettlementReceipt>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct SettlementReceiptDetail {
    pub receipt: SettlementReceipt,
    pub intent: SettlementIntent,
    pub usage_receipts: Vec<UsageReceipt>,
    pub ledger_transaction: Option<LedgerTransaction>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct SuiSettlementEnvelope {
    pub schema: &'static str,
    pub source_receipt_id: String,
    pub source_posting_key: String,
    pub project_object_key: String,
    pub intent_object_key: String,
    pub receipt_object_key: String,
    pub amount_micros: i64,
    pub provider_amount_micros: i64,
    pub platform_amount_micros: i64,
    pub currency: String,
    pub shadow_only: bool,
    pub ptb_steps: Vec<String>,
    pub network_submission: &'static str,
}
