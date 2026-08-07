use crate::store_schema::apply_migrations;
use anyhow::{anyhow, Result};
use rusqlite::{params, Connection, OptionalExtension};
use std::path::Path;
use std::sync::{Mutex, MutexGuard};
mod account_identities;
#[cfg(test)]
mod account_identities_tests;
pub(crate) use account_identities::*;
mod account_security;
mod account_security_events;
mod account_security_support;
#[cfg(test)]
mod account_security_tests;
pub(crate) use account_security::*;
pub(crate) use account_security_events::*;
mod admin_stats;
mod admin_stats_quotas;
mod ai_resource_policies;
mod billing;
mod billing_alerts;
mod billing_pay;
mod billing_price_rules;
#[cfg(test)]
mod billing_reservation_tests;
mod billing_reservations;
mod build_quota;
mod codex_vault;
mod codex_vault_dispatch_authorization;
pub(crate) mod codex_vault_emergency;
mod codex_vault_emergency_delivery_guard;
mod codex_vault_emergency_lease_guard;
mod codex_vault_emergency_usage_attach;
mod codex_vault_sharing_health;
#[cfg(test)]
mod codex_vault_sharing_regression_tests;
pub(crate) mod codex_vault_usage_estimation;
mod common;
mod compute_activation_applications;
mod compute_activation_lifecycle;
mod compute_activation_plan_dependencies;
mod compute_activation_plan_reviews;
mod compute_activation_plans;
mod compute_activation_quarantines;
mod compute_activation_recoveries;
mod compute_activation_requests;
mod compute_attempt_aborts;
mod compute_attempt_activations;
mod compute_attempt_consumer_reviews;
mod compute_attempt_execution_receipts;
mod compute_attempt_finalizations;
mod compute_attempt_leases;
mod compute_attempt_platform_observations;
mod compute_attempt_settlement_challenge_resolutions;
mod compute_attempt_settlement_challenges;
mod compute_attempt_settlement_corrections;
mod compute_attempt_settlement_releases;
mod compute_attempt_settlements;
mod compute_attempt_terminals;
mod compute_attempt_usage;
mod compute_attempt_verifications;
mod compute_broker_reservation;
mod compute_capacity_audit;
mod compute_capacity_bucket_queries;
mod compute_capacity_claim_activation;
mod compute_capacity_claim_return;
mod compute_capacity_claim_rows;
mod compute_capacity_claim_transitions;
mod compute_capacity_claims;
mod compute_capacity_expiry_recovery;
mod compute_capacity_history_queries;
mod compute_capacity_ledger;
mod compute_capacity_pool_epoch;
mod compute_capacity_pool_guards;
mod compute_capacity_pool_lifecycle;
mod compute_capacity_pool_queries;
mod compute_capacity_posting;
mod compute_capacity_registry;
mod compute_capacity_request_digest;
mod compute_capacity_rows;
mod compute_capacity_supply_queries;
mod compute_capacity_supply_withdrawal;
mod compute_job_contract_validation;
mod compute_job_registry;
mod compute_metering;
mod compute_offer_contract_validation;
mod compute_offer_lifecycle;
mod compute_offer_owner_queries;
mod compute_offer_publications;
mod compute_offer_registry;
mod compute_offer_terminal;
mod compute_platform_settlement_account_view;
mod compute_price_snapshot_registry;
mod compute_price_snapshot_validation;
mod compute_provider_registry;
mod compute_quote_candidates;
mod compute_reservation_contract_validation;
mod compute_reservation_registry;
mod compute_settlement_account_views;
mod compute_settlement_release_batch_runs;
mod compute_settlement_release_candidates;
mod compute_settlement_withdrawal_requests;
mod compute_settlement_withdrawal_terminals;
mod conversation_forks;
mod conversations;
pub(crate) mod default_joint_projects;
mod erp_blueprints;
mod erp_proposals;
mod erp_upgrades;
mod external_app_tool_executions;
mod external_apps;
#[cfg(test)]
mod external_apps_tests;
mod friend_messages;
mod friends;
mod group_ai;
mod group_ai_flow;
mod group_ai_governance;
mod group_summary;
#[cfg(test)]
mod group_summary_tests;
mod groups;
mod join_requests;
mod message_recall;
mod native_sessions;
mod node_cli_completion_receipts;
mod node_compute_replay;
mod node_compute_runs;
mod node_compute_sharing;
mod node_compute_sharing_health;
mod node_credentials;
mod node_hardware;
mod node_ledger;
#[cfg(test)]
mod node_payout_tests;
mod node_payouts;
mod node_public_dev;
mod open_commerce_action_confirmations;
mod open_commerce_adapter_claims;
mod open_commerce_adapter_credentials;
mod open_commerce_app_activity_health;
mod open_commerce_app_blocks;
mod open_commerce_authorization_requests;
mod open_commerce_business_handoffs;
mod open_commerce_capabilities;
mod open_commerce_capability_sources;
mod open_commerce_consumer_data_requests;
mod open_commerce_consumer_portability;
mod open_commerce_consumer_portability_adoptions;
mod open_commerce_consumer_portability_imports;
mod open_commerce_consumer_portability_merges;
mod open_commerce_consumer_portability_trust;
mod open_commerce_consumer_preferences;
mod open_commerce_consumer_receipts;
mod open_commerce_consumer_relationships;
mod open_commerce_consumer_vault;
mod open_commerce_data_erasure_evidence;
mod open_commerce_data_request_followups;
mod open_commerce_developer_app_admissions;
mod open_commerce_developer_app_domains;
mod open_commerce_developer_app_manifests;
mod open_commerce_developer_apps;
mod open_commerce_developer_credentials;
mod open_commerce_developer_events;
mod open_commerce_developer_webhook_dead_letters;
mod open_commerce_developer_webhook_health;
mod open_commerce_developer_webhook_history;
mod open_commerce_developer_webhook_replays;
mod open_commerce_developer_webhook_rows;
mod open_commerce_developer_webhook_secret;
mod open_commerce_developer_webhook_verification;
mod open_commerce_developer_webhooks;
mod open_commerce_directory;
mod open_commerce_grant_budgets;
mod open_commerce_grants;
mod open_commerce_integrations;
mod open_commerce_invocation_recovery;
mod open_commerce_invocations;
mod open_commerce_merchant_evidence;
mod open_commerce_merchant_identity;
mod open_commerce_merchants;
mod open_commerce_portability_reauthorization;
mod open_commerce_production_webhooks;
mod open_commerce_rate_limits;
mod open_commerce_runtime_bindings;
pub(crate) use open_commerce_action_confirmations::CreateOpenCommerceActionConfirmation;
pub(crate) use open_commerce_business_handoffs::{
    AdapterClaimReceiptProof, RecordOpenCommerceBusinessHandoffReceipt,
};
pub(crate) use open_commerce_integrations::RecordOpenCommerceSyncReceipt;
pub(crate) use open_commerce_invocations::{
    OpenCommerceInvocationProvenance, OpenCommerceInvocationStart,
};
mod pc_project_binding;
mod project_android_device_leases;
mod project_android_devices;
mod project_branding;
mod project_dev_profiles;
mod project_execution_sessions;
mod project_identities;
mod project_invites;
mod project_landing_snapshots;
mod project_landing_upload_tokens;
mod project_member_audit;
mod project_member_conversations;
mod project_member_moderation;
mod project_module_completion;
mod project_module_queries;
#[cfg(test)]
mod project_module_tests;
mod project_module_types;
mod project_module_workspaces;
mod project_ops;
mod project_query;
pub(crate) mod project_releases;
mod project_roles;
mod project_runtime_permissions;
mod project_space;
mod project_storage;
mod project_workspace_health_snapshots;
mod projects;
mod projects_members;
mod realtime_close_events;
#[cfg(test)]
mod realtime_close_events_tests;
pub(crate) mod route_c_budget;
mod social_ai_messages;
mod social_ai_pending;
mod social_ai_selected;
mod store_types;
mod store_types_project;
mod system_projects;
mod task_completion_replay;
mod task_recovery;
mod task_settlement_correction_posting;
mod task_settlement_correction_rows;
mod task_settlement_corrections;
mod task_settlement_disputes;
mod task_settlement_rows;
mod task_settlements;
mod task_start_sync;
mod task_sui_correction_projection_packages;
mod task_sui_preflight_adapters;
mod task_sui_preflight_job_leases;
mod task_sui_preflight_jobs;
mod task_sui_preflight_reports;
mod task_sui_projection_packages;
#[cfg(test)]
mod task_title_tests;
mod tasks;
pub(crate) mod token_usage;
mod token_usage_stats;
#[cfg(test)]
mod token_usage_tests;
mod ui_route_learning;
mod user_archive;
mod user_memories;
mod user_presence;
mod user_progression;
mod users;
mod workspace_tasks;
pub use admin_stats::{
    estimate_cost_cny, AdminAccountingAuditRow, AdminDayRow, AdminFeatureRow, AdminModelRow,
    AdminPlatformSummary, AdminTrendRow, AdminUserDetail, AdminUserUsageRow, UserQuota,
};
pub use billing::{AdminBalanceRow, AdminBillingEventRow, BillingEvent, RechargeRecord};
pub use billing_alerts::BillingAlertRow;
pub use billing_price_rules::{BillingPriceRule, BillingPriceRuleUpsert, BillingPriceSnapshot};
pub use billing_reservations::{
    ActiveBillingReservation, BillingReservationOutcome, BillingReservationRequest,
};
pub use codex_vault::{CodexVaultRecord, CodexVaultSlotRecord};
pub(crate) use codex_vault_emergency_delivery_guard::CodexVaultEmergencyCredentialDeliveryClaim;
use common::{
    account_columns, clean_optional, hash_password, hash_token, new_id, normalize_account, now,
    password_needs_rehash, safe_external_id, validate_password, verify_password,
};
pub(crate) use compute_activation_applications::ApplyComputeActivationPlan;
pub(crate) use compute_activation_lifecycle::SupersedeComputeActivationEvidenceRequest;
pub(crate) use compute_activation_plan_reviews::{
    ComputeActivationPlanReviewReceipt, ReviewComputeActivationPlan,
};
pub(crate) use compute_activation_plans::PrepareComputeActivationPlan;
pub(crate) use compute_activation_quarantines::QuarantineComputeActivationApplication;
pub(crate) use compute_activation_recoveries::{
    ApplyComputeActivationRecoveryPlan, PrepareComputeActivationRecoveryPlan,
    ReviewComputeActivationRecoveryPlan, SupersedeComputeActivationRecoveryPlan,
};
pub(crate) use compute_activation_requests::{
    ReviewComputeActivationEvidenceRequest, SubmitComputeActivationEvidenceRequest,
};
pub(crate) use compute_attempt_aborts::{AbortComputeAttemptRequest, ComputeAttemptAbortReceipt};
pub(crate) use compute_attempt_activations::{
    ActivateComputeAttemptRequest, ComputeAttemptActivationReceipt,
};
pub(crate) use compute_attempt_consumer_reviews::{
    ComputeAttemptConsumerReviewReceipt, ReviewComputeAttemptTerminalCandidateRequest,
};
pub(crate) use compute_attempt_execution_receipts::{
    ComputeAttemptExecutionReceiptEnvelope, ComputePendingExecutionReceiptCandidate,
    IssueComputeAttemptExecutionReceiptRequest,
};
pub(crate) use compute_attempt_finalizations::{
    ComputeAttemptFinalizationReceipt, ComputePendingAttemptFinalizationCandidate,
    FinalizeComputeAttemptRequest,
};
pub(crate) use compute_attempt_leases::{
    ComputeAttemptLeaseRenewalReceipt, ComputeAttemptLeaseStateReceipt,
    RenewComputeAttemptLeaseRequest,
};
pub(crate) use compute_attempt_platform_observations::{
    ComputeAttemptPlatformObservationReceipt, ComputeObservedUsageInput,
    ComputePendingPlatformObservationCandidate, ObserveComputeAttemptTerminalCandidateRequest,
};
pub(crate) use compute_attempt_settlement_challenge_resolutions::{
    ComputeSettlementChallengeHistoryItem, ComputeSettlementChallengeResolutionReceipt,
    ResolveComputeSettlementChallengeRequest,
};
pub(crate) use compute_attempt_settlement_challenges::{
    ComputePendingSettlementChallengeCandidate, ComputeSettlementChallengeReceipt,
    OpenComputeSettlementChallengeRequest,
};
pub(crate) use compute_attempt_settlement_corrections::{
    ComputePendingSettlementCorrectionCandidate, ComputeSettlementCorrectionReceipt,
    CorrectComputeAttemptSettlementRequest,
};
pub(crate) use compute_attempt_settlement_releases::{
    ComputeSettlementReleaseReceipt, ReleaseComputeAttemptSettlementRequest,
};
pub(crate) use compute_attempt_settlements::{
    ComputeAttemptSettlementReceipt, ComputePendingAttemptSettlementCandidate,
    ComputeSettlementLifecycleHistoryItem, SettleComputeAttemptRequest,
};
pub(crate) use compute_attempt_terminals::{
    ComputeAttemptTerminalCandidateReceipt, ComputeDeclaredResultArtifactInput,
    DeclareComputeAttemptTerminalCandidateRequest,
};
pub(crate) use compute_attempt_usage::{
    ComputeAttemptUsageDeclarationReceipt, ComputeAttemptUsageTemplateReceipt,
    ComputeDeclaredUsageInput, DeclareComputeAttemptUsageRequest,
};
pub(crate) use compute_attempt_verifications::{
    ComputeAttemptVerificationDecisionReceipt, ComputePendingAttemptVerificationCandidate,
    DecideComputeAttemptVerificationRequest,
};
pub(crate) use compute_broker_reservation::{
    ComputeBrokerFinishAction, ComputeBrokerFinishReceipt, ComputeBrokerReservationReceipt,
    FinishComputeBrokerRequest, ReserveComputeBrokerRequest,
};
pub(crate) use compute_capacity_audit::{
    stable_compute_capacity_pool_audit_digest, ComputeCapacityBucketAudit,
    ComputeCapacityDerivedBalance, ComputeCapacityPoolAuditReport,
};
pub(crate) use compute_capacity_bucket_queries::ComputeCapacityBucketRead;
pub(crate) use compute_capacity_claim_transitions::{
    ComputeCapacityClaimTerminalAction, FinishComputeCapacityClaim,
    FinishComputeCapacityClaimReceipt,
};
pub(crate) use compute_capacity_claims::{
    HoldComputeCapacityClaim, HoldComputeCapacityClaimLine, HoldComputeCapacityClaimReceipt,
};
pub(crate) use compute_capacity_expiry_recovery::{
    ComputeCapacityExpiryRecoveryItem, ComputeCapacityExpiryRecoveryReport,
};
pub(crate) use compute_capacity_history_queries::{
    ComputeCapacityLedgerHistoryLeg, ComputeCapacityLedgerHistoryPage,
    ComputeCapacityLedgerHistoryTransaction,
};
pub(crate) use compute_capacity_ledger::{
    AddComputeCapacitySupply, AddComputeCapacitySupplyLine, ComputeCapacityLedgerWriteReceipt,
};
pub(crate) use compute_capacity_pool_epoch::{
    ComputeCapacityPoolEpochReceipt, RolloverComputeCapacityPoolEpoch,
};
pub(crate) use compute_capacity_pool_lifecycle::{
    ComputeCapacityPoolStatusReceipt, TransitionComputeCapacityPoolStatus,
};
pub(crate) use compute_capacity_supply_withdrawal::{
    WithdrawComputeCapacitySupply, WithdrawComputeCapacitySupplyLine,
};
pub(crate) use compute_job_registry::ComputeJobRegistrationReceipt;
pub use compute_metering::ComputeMeterEvent;
pub(crate) use compute_offer_contract_validation::{compute_offer_digest, compute_sku_digest};
pub(crate) use compute_offer_lifecycle::DrainComputeOffer;
pub(crate) use compute_offer_publications::PublishComputeOfferDraft;
pub(crate) use compute_offer_registry::ComputeOfferRegistrationReceipt;
pub(crate) use compute_offer_terminal::TerminateComputeOffer;
pub(crate) use compute_platform_settlement_account_view::ComputePlatformSettlementAccountView;
pub(crate) use compute_price_snapshot_registry::ComputePriceSnapshotRegistrationReceipt;
pub(crate) use compute_price_snapshot_validation::compute_price_snapshot_digest;
pub(crate) use compute_provider_registry::validate_compute_provider_contract;
pub(crate) use compute_provider_registry::ComputeProviderRegistrationReceipt;
pub(crate) use compute_quote_candidates::ComputeJobQuoteCandidatePage;
pub(crate) use compute_reservation_registry::ComputeReservationRegistrationReceipt;
pub(crate) use compute_settlement_account_views::{
    ComputeSettlementAccountView, ComputeSettlementWithdrawalQueuePage,
};
pub(crate) use compute_settlement_release_batch_runs::{
    ComputeSettlementReleaseBatchFailure, ComputeSettlementReleaseBatchHistoryPage,
    ComputeSettlementReleaseBatchReport, StartComputeSettlementReleaseBatch,
};
pub(crate) use compute_settlement_release_candidates::{
    ComputeSettlementReleaseCandidate, ComputeSettlementReleaseCandidatePage,
};
pub(crate) use compute_settlement_withdrawal_requests::{
    ComputeSettlementWithdrawalRequestReceipt, CreateComputeSettlementWithdrawalRequest,
};
pub(crate) use compute_settlement_withdrawal_terminals::{
    ComputeSettlementWithdrawalTerminalReceipt, TerminalizeComputeSettlementWithdrawalRequest,
};
pub(crate) use external_app_tool_executions::{
    AdminExternalAppToolExecutionSummary, ExternalAppToolExecutionWrite,
};
pub use node_cli_completion_receipts::{
    NodeCliCompletionIngestOutcome, NodeCliCompletionReceipt, NodeCliCompletionReceiptInput,
};
pub use node_compute_replay::{
    LocalOfflineNodeComputeRunClaim, LocalOfflineNodeComputeRunClaimOutcome,
    NodeComputeReplayBinding, NodeComputeReplayExpectation,
};
pub use node_compute_runs::{
    NodeComputeRun, NodeComputeRunFinish, NodeComputeRunStart, NodeQualityScore,
};
pub use node_compute_sharing::{
    NodeComputeSharingPolicy, NodeComputeSharingStatus, UpdateNodeComputeSharingPolicy,
};
pub use node_compute_sharing_health::NodeComputeSharingRuntimeHealth;
pub use node_ledger::{NodeBalance, NodeCredential, NodeTransaction, SettleParams};
pub use node_payouts::CreateNodePayout;
pub(crate) use project_android_device_leases::{
    AcquireAndroidDeviceLease, ProjectAndroidDeviceLease,
};
pub(crate) use project_android_devices::ProjectAndroidDevice;
pub use project_dev_profiles::ProjectDevProfile;
pub use project_execution_sessions::{
    ProjectExecutionSession, ProjectExecutionSessionFinish, ProjectExecutionSessionStart,
};
pub(crate) use project_module_types::{
    CreateUiTunerContextArtifact, ProjectModuleCheckpoint, ProjectModuleContextArtifact,
    ProjectModuleConversation, ProjectModuleMemory, ProjectModuleWorkspace, UiTunerWorkspaceBundle,
    UI_TUNER_MODULE_KEY,
};
pub use project_roles::{
    PERMISSION_INVITE_MEMBERS, PERMISSION_MANAGE_MEMBERS, PERMISSION_MANAGE_PROJECT_SETTINGS,
    PERMISSION_MANAGE_ROLES, PERMISSION_MODERATE_MEMBERS, PERMISSION_SEND_MESSAGES,
    PERMISSION_VIEW_AUDIT_LOG, PERMISSION_VIEW_MEMBERS,
};
pub use project_space::{
    CHANNEL_PERMISSION_MANAGE, CHANNEL_PERMISSION_SEND, CHANNEL_PERMISSION_START_AI,
    CHANNEL_PERMISSION_VIEW,
};
pub use project_workspace_health_snapshots::ProjectWorkspaceHealthSnapshotWrite;
pub use realtime_close_events::RealtimeCloseMetricRow;
pub(crate) use social_ai_messages::{
    SocialAiHistoryMessage, SOCIAL_AI_DISPLAY_NAME, SOCIAL_AI_FRIEND_ACCOUNT,
    SOCIAL_AI_FRIEND_NAME, SOCIAL_AI_FRIEND_PREVIEW, SOCIAL_AI_USER_ID,
};
pub(crate) use social_ai_pending::SocialAiPendingMention;
pub use store_types::*;
pub use store_types_project::JoinRequestRecord;
pub use store_types_project::*;
pub(crate) use system_projects::{
    is_system_project_name, is_system_project_source_type, system_project_key_for_source_type,
    CHAT_MEMORY_PROJECT_NAME, PHONE_CONTROL_PROJECT_NAME,
};
pub(crate) use task_completion_replay::is_automatic_communication_failure;
pub use task_completion_replay::{PcCliTaskCompletionApply, PcCliTaskCompletionOutcome};
pub use task_start_sync::{PcLocalTaskStartApply, PcLocalTaskStartOutcome};
pub use token_usage::{
    TokenUsageAccountingResult, TokenUsageBillingCharge, TokenUsageRecord, UsageDayRow,
    UsageFeatureRow, UsageModeRow, UsageQuota, UsageStats, UsageTotals,
};
pub(crate) use ui_route_learning::{UiLearnedRoute, UiRouteLearningEntry, UiRouteLearningSource};
pub use user_memories::{
    UserMemory, MEMORY_SCOPE_CHAT, MEMORY_SCOPE_GLOBAL, MEMORY_SCOPE_PHONE_CONTROL,
    MEMORY_SCOPE_PROJECT,
};
pub use user_progression::UserProgressionLedger;
pub struct Store {
    conn: Mutex<Connection>,
}
const MAX_TASK_EVENTS_PER_TASK: i64 = 1000;

impl Store {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let conn = Connection::open(path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        conn.pragma_update(None, "busy_timeout", 5000)?;
        apply_migrations(&conn)?;

        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    pub fn ensure_device_user(&self, user_id: &str) -> Result<PublicUser> {
        let id = safe_external_id(user_id, "default");
        let now = now();
        let conn = self.conn()?;
        conn.execute(
            "INSERT OR IGNORE INTO users (
                id, phone, email, password_hash, nickname, role, status, created_at, updated_at
             )
             VALUES (?1, ?2, NULL, 'device-user', 'APK 用户', 'user', 'active', ?3, ?3)",
            params![
                id,
                format!("device-{}", safe_external_id(user_id, "default")),
                now
            ],
        )?;

        let user = conn.query_row(
            "SELECT id, phone, email, nickname, role, status FROM users WHERE id = ?1",
            params![safe_external_id(user_id, "default")],
            |row| {
                let phone: Option<String> = row.get(1)?;
                let email: Option<String> = row.get(2)?;
                Ok(PublicUser {
                    id: row.get(0)?,
                    account: email
                        .or(phone)
                        .unwrap_or_else(|| row.get(0).unwrap_or_default()),
                    nickname: row.get(3)?,
                    role: row.get(4)?,
                    status: row.get(5)?,
                    avatar_data_url: None,
                })
            },
        )?;
        drop(conn);
        Ok(user)
    }

    pub(crate) fn conn(&self) -> Result<MutexGuard<'_, Connection>> {
        self.conn.lock().map_err(|_| anyhow!("数据库连接锁已损坏"))
    }
}

mod project_helpers;
use self::project_helpers::{
    apply_effective_project_summary_role, find_owner_project_by_name,
    find_owner_project_by_workspace_path, find_project_by_id_for_user, project_summary_from_row,
    update_external_project_binding,
};
