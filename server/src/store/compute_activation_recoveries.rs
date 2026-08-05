use anyhow::{anyhow, bail, Result};
use rusqlite::{params, Connection, OptionalExtension, Row, TransactionBehavior};

use crate::{
    compute_federation::{
        capacity::ComputeCapacityPoolStatus,
        provider::{ComputeProvider, PROVIDER_STATUS_ACTIVE, PROVIDER_STATUS_QUARANTINED},
    },
    compute_federation_activation_recovery_model::{
        ComputeActivationRecoveryApplicationReceipt, ComputeActivationRecoveryPlan,
        ComputeActivationRecoveryPlanReceipt, ComputeActivationRecoveryReviewReceipt,
        RECOVERY_APPLICATION_SCHEMA, RECOVERY_PLAN_SCHEMA, RECOVERY_REVIEW_SCHEMA,
    },
};

use super::{
    compute_activation_quarantines::{
        audit_quarantine_on, quarantine_by_request_on, StoredQuarantine,
    },
    compute_capacity_pool_lifecycle::{
        transition_compute_capacity_pool_status_on, TransitionComputeCapacityPoolStatus,
    },
    compute_capacity_pool_queries::current_capacity_pool_on,
    compute_provider_registry::{
        current_registered_provider_on, register_compute_provider_on,
        registered_provider_version_on, validate_compute_provider_contract,
    },
    new_id, now, Store,
};

#[derive(Debug, Clone)]
pub(crate) struct PrepareComputeActivationRecoveryPlan {
    pub request_id: String,
    pub expected_quarantine_digest: String,
    pub target_provider: ComputeProvider,
    pub remediation_summary: String,
    pub evidence_refs: Vec<String>,
    pub idempotency_scope: String,
    pub idempotency_key: String,
    pub prepared_by_user_id: String,
}

#[derive(Debug, Clone)]
pub(crate) struct ReviewComputeActivationRecoveryPlan {
    pub request_id: String,
    pub expected_plan_digest: String,
    pub review_note: Option<String>,
    pub idempotency_scope: String,
    pub idempotency_key: String,
    pub reviewed_by_user_id: String,
}

#[derive(Debug, Clone)]
pub(crate) struct ApplyComputeActivationRecoveryPlan {
    pub request_id: String,
    pub expected_plan_digest: String,
    pub idempotency_scope: String,
    pub idempotency_key: String,
    pub applied_by_user_id: String,
}

mod integrity;
mod operations;

use integrity::*;

// Storage decoding and digest audit helpers are kept below; this module is split by domain,
// while each helper re-audits immutable JSON before returning it to a caller.

#[derive(Debug, Clone)]
struct StoredRecoveryReview {
    recovery_review_id: String,
    recovery_plan_id: String,
    request_id: String,
    plan_digest: String,
    prepared_by_user_id: String,
    reviewed_by_user_id: String,
    review_note: Option<String>,
    request_digest: String,
    review_digest: String,
    reviewed_at: String,
}

impl StoredRecoveryReview {
    fn into_receipt(self, replayed: bool) -> ComputeActivationRecoveryReviewReceipt {
        ComputeActivationRecoveryReviewReceipt {
            schema: RECOVERY_REVIEW_SCHEMA,
            recovery_review_id: self.recovery_review_id,
            recovery_plan_id: self.recovery_plan_id,
            request_id: self.request_id,
            plan_digest: self.plan_digest,
            prepared_by_user_id: self.prepared_by_user_id,
            reviewed_by_user_id: self.reviewed_by_user_id,
            review_note: self.review_note,
            request_digest: self.request_digest,
            review_digest: self.review_digest,
            reviewed_at: self.reviewed_at,
            replayed,
            recovery_effect: "none",
        }
    }
}

#[derive(Debug, Clone)]
struct StoredRecoveryApplication {
    recovery_application_id: String,
    recovery_plan_id: String,
    recovery_review_id: String,
    quarantine_id: String,
    request_id: String,
    provider_id: String,
    pool_id: String,
    plan_digest: String,
    review_digest: String,
    recovered_provider_policy_revision: i64,
    recovered_provider_digest: String,
    capacity_epoch: i64,
    pool_lifecycle_event_id: String,
    application_digest: String,
    applied_by_user_id: String,
    applied_at: String,
}

impl StoredRecoveryApplication {
    fn into_receipt(self, replayed: bool) -> ComputeActivationRecoveryApplicationReceipt {
        ComputeActivationRecoveryApplicationReceipt {
            schema: RECOVERY_APPLICATION_SCHEMA,
            recovery_application_id: self.recovery_application_id,
            recovery_plan_id: self.recovery_plan_id,
            recovery_review_id: self.recovery_review_id,
            quarantine_id: self.quarantine_id,
            request_id: self.request_id,
            provider_id: self.provider_id,
            pool_id: self.pool_id,
            plan_digest: self.plan_digest,
            review_digest: self.review_digest,
            recovered_provider_policy_revision: self.recovered_provider_policy_revision,
            recovered_provider_digest: self.recovered_provider_digest,
            capacity_epoch: self.capacity_epoch,
            pool_lifecycle_event_id: self.pool_lifecycle_event_id,
            application_digest: self.application_digest,
            applied_by_user_id: self.applied_by_user_id,
            applied_at: self.applied_at,
            replayed,
            provider_effect: "active",
            pool_effect: "active",
            offer_effect: "none_active_offers_required",
            node_effect: "none",
            money_effect: "none",
        }
    }
}

fn plan_receipt(
    plan: ComputeActivationRecoveryPlan,
    replayed: bool,
) -> ComputeActivationRecoveryPlanReceipt {
    ComputeActivationRecoveryPlanReceipt {
        plan,
        replayed,
        provider_effect: "none",
        pool_effect: "none",
        offer_effect: "none",
    }
}

fn validate_recovery_dependencies(
    conn: &Connection,
    quarantine: &StoredQuarantine,
    input: &PrepareComputeActivationRecoveryPlan,
    target_digest: &str,
) -> Result<()> {
    if quarantine.quarantine_digest != input.expected_quarantine_digest {
        bail!("激活隔离摘要已变化");
    }
    let provider = current_registered_provider_on(conn, &quarantine.provider_id)?
        .ok_or_else(|| anyhow!("当前 Provider 不存在"))?;
    let pool = current_capacity_pool_on(conn, &quarantine.pool_id)?
        .ok_or_else(|| anyhow!("当前 CapacityPool 不存在"))?;
    if provider.provider.status != PROVIDER_STATUS_QUARANTINED
        || provider.provider.policy_revision != quarantine.quarantined_provider_policy_revision
        || provider.provider_digest != quarantine.quarantined_provider_digest
        || pool.status != ComputeCapacityPoolStatus::Quarantined
        || pool.provider_id != quarantine.provider_id
        || pool.binding.capacity_epoch != quarantine.capacity_epoch
    {
        bail!("恢复计划依赖的 quarantined Provider 或 Pool 已变化");
    }
    if input.target_provider.provider_id != quarantine.provider_id
        || input.target_provider.owner_account_id != provider.provider.owner_account_id
        || input.target_provider.provider_kind != provider.provider.provider_kind
        || input.target_provider.created_at != provider.provider.created_at
        || input.target_provider.policy_revision != provider.provider.policy_revision + 1
        || input.target_provider.status != PROVIDER_STATUS_ACTIVE
        || target_digest.is_empty()
    {
        bail!("恢复计划目标 Provider 身份、状态或 revision 无效");
    }
    validate_compute_provider_contract(&input.target_provider)
}

fn validate_apply_dependencies(
    conn: &Connection,
    plan: &ComputeActivationRecoveryPlan,
) -> Result<()> {
    let quarantine = quarantine_by_request_on(conn, &plan.request_id)?
        .ok_or_else(|| anyhow!("恢复计划引用的隔离回执不存在"))?;
    audit_quarantine_on(conn, &quarantine)?;
    let provider = current_registered_provider_on(conn, &plan.provider_id)?
        .ok_or_else(|| anyhow!("当前 Provider 不存在"))?;
    let pool = current_capacity_pool_on(conn, &plan.pool_id)?
        .ok_or_else(|| anyhow!("当前 CapacityPool 不存在"))?;
    let active_offers: i64 = conn.query_row(
        "SELECT COUNT(*) FROM compute_offers WHERE provider_id=?1 AND status='active'",
        params![plan.provider_id],
        |row| row.get(0),
    )?;
    if quarantine.quarantine_digest != plan.expected_quarantine_digest
        || provider.provider.status != PROVIDER_STATUS_QUARANTINED
        || provider.provider.policy_revision != plan.expected_provider_policy_revision
        || provider.provider_digest != plan.expected_provider_digest
        || pool.status != ComputeCapacityPoolStatus::Quarantined
        || pool.binding.capacity_epoch != plan.expected_capacity_epoch
        || pool.binding.pool_revision != plan.expected_pool_revision
        || pool.binding.pool_digest != plan.expected_pool_digest
        || active_offers != 0
    {
        bail!("恢复依赖已变化，或仍存在必须先退场的 active Offer");
    }
    Ok(())
}

fn audit_recovery_review_on(conn: &Connection, review: &StoredRecoveryReview) -> Result<()> {
    let plan = recovery_plan_by_id_on(conn, &review.recovery_plan_id)?
        .ok_or_else(|| anyhow!("恢复复核引用的计划不存在"))?;
    let input = ReviewComputeActivationRecoveryPlan {
        request_id: review.request_id.clone(),
        expected_plan_digest: review.plan_digest.clone(),
        review_note: review.review_note.clone(),
        idempotency_scope: String::new(),
        idempotency_key: String::new(),
        reviewed_by_user_id: review.reviewed_by_user_id.clone(),
    };
    if review.request_digest != review_request_digest(&input)?
        || review.review_digest
            != recovery_review_digest(
                &review.recovery_review_id,
                &plan,
                &input,
                &review.request_digest,
                &review.reviewed_at,
            )?
        || review.plan_digest != plan.plan_digest
        || review.prepared_by_user_id != plan.prepared_by_user_id
        || review.reviewed_by_user_id == plan.prepared_by_user_id
    {
        bail!("恢复计划复核回执审计失败");
    }
    Ok(())
}

fn audit_recovery_application_on(
    conn: &Connection,
    stored: &StoredRecoveryApplication,
) -> Result<()> {
    let plan = recovery_plan_by_id_on(conn, &stored.recovery_plan_id)?
        .ok_or_else(|| anyhow!("恢复应用引用的计划不存在"))?;
    let review = recovery_review_by_plan_on(conn, &stored.recovery_plan_id)?
        .ok_or_else(|| anyhow!("恢复应用引用的复核不存在"))?;
    audit_recovery_review_on(conn, &review)?;
    let provider = registered_provider_version_on(
        conn,
        &stored.provider_id,
        stored.recovered_provider_policy_revision,
    )?
    .ok_or_else(|| anyhow!("恢复后的 Provider 历史版本不存在"))?;
    let event_matches = conn.query_row("SELECT 1 FROM compute_capacity_pool_lifecycle_events WHERE event_id=?1 AND pool_id=?2 AND capacity_epoch=?3 AND previous_status='quarantined' AND target_status='active' AND subject_kind='compute_activation_recovery_plan' AND subject_id=?4 AND request_digest=?5 AND occurred_at=?6", params![stored.pool_lifecycle_event_id, stored.pool_id, stored.capacity_epoch, stored.recovery_plan_id, stored.plan_digest, stored.applied_at], |row| row.get::<_, i64>(0)).optional()?.is_some();
    let expected = recovery_application_digest(
        &stored.recovery_application_id,
        &plan,
        &review,
        &stored.pool_lifecycle_event_id,
        &stored.applied_by_user_id,
        &stored.applied_at,
    )?;
    if plan.status != "applied"
        || plan.recovery_plan_id != stored.recovery_plan_id
        || plan.quarantine_id != stored.quarantine_id
        || plan.request_id != stored.request_id
        || plan.provider_id != stored.provider_id
        || plan.pool_id != stored.pool_id
        || plan.plan_digest != stored.plan_digest
        || review.recovery_review_id != stored.recovery_review_id
        || review.review_digest != stored.review_digest
        || plan.target_provider_policy_revision != stored.recovered_provider_policy_revision
        || plan.target_provider_digest != stored.recovered_provider_digest
        || plan.expected_capacity_epoch != stored.capacity_epoch
        || provider.provider.status != PROVIDER_STATUS_ACTIVE
        || provider.provider_digest != stored.recovered_provider_digest
        || !event_matches
        || expected != stored.application_digest
    {
        bail!("恢复应用回执与历史 Provider、Pool、计划或复核不一致");
    }
    Ok(())
}

fn recovery_plan_by_id_on(
    conn: &Connection,
    id: &str,
) -> Result<Option<ComputeActivationRecoveryPlan>> {
    conn.query_row(PLAN_SELECT, params![id], recovery_plan_from_row)
        .optional()?
        .map(audit_recovery_plan)
        .transpose()
}
fn current_recovery_plan_on(
    conn: &Connection,
    request_id: &str,
) -> Result<Option<ComputeActivationRecoveryPlan>> {
    let id = conn.query_row("SELECT recovery_plan_id FROM compute_activation_recovery_plans WHERE request_id=?1 AND status IN ('prepared','applied') ORDER BY CASE status WHEN 'prepared' THEN 0 ELSE 1 END, prepared_at DESC LIMIT 1", params![request_id], |row| row.get::<_, String>(0)).optional()?;
    id.map(|value| recovery_plan_by_id_on(conn, &value))
        .transpose()
        .map(Option::flatten)
}
fn recovery_plan_by_idempotency_on(
    conn: &Connection,
    scope: &str,
    key: &str,
) -> Result<Option<ComputeActivationRecoveryPlan>> {
    let id = conn.query_row("SELECT recovery_plan_id FROM compute_activation_recovery_plans WHERE idempotency_scope=?1 AND idempotency_key=?2", params![scope,key], |row| row.get::<_,String>(0)).optional()?;
    id.map(|value| recovery_plan_by_id_on(conn, &value))
        .transpose()
        .map(Option::flatten)
}

fn audit_recovery_plan(stored: StoredRecoveryPlan) -> Result<ComputeActivationRecoveryPlan> {
    let target: ComputeProvider = serde_json::from_str(&stored.target_provider_json)?;
    validate_compute_provider_contract(&target)?;
    let target_digest = digest_bytes(stored.target_provider_json.as_bytes());
    let refs: Vec<String> = serde_json::from_str(&stored.evidence_refs_json)?;
    let input = stored.as_input(target.clone(), refs.clone());
    if target.provider_id != stored.provider_id
        || target.policy_revision != stored.target_provider_policy_revision
        || target.status != PROVIDER_STATUS_ACTIVE
        || target_digest != stored.target_provider_digest
        || digest_json(&serde_json::json!({"endpoint":target.endpoint,"adapter":target.adapter}))?
            != stored.routing_digest
        || digest_json(&refs)? != stored.evidence_refs_digest
        || recovery_plan_digest_from_stored(&stored, &input)? != stored.plan_digest
    {
        bail!("恢复计划规范摘要审计失败");
    }
    Ok(stored.into_plan(target, refs))
}

#[derive(Debug, Clone)]
struct StoredRecoveryPlan {
    recovery_plan_id: String,
    quarantine_id: String,
    application_id: String,
    request_id: String,
    provider_id: String,
    pool_id: String,
    expected_quarantine_digest: String,
    expected_provider_policy_revision: i64,
    expected_provider_digest: String,
    expected_capacity_epoch: i64,
    expected_pool_revision: i64,
    expected_pool_digest: String,
    target_provider_policy_revision: i64,
    target_provider_digest: String,
    target_provider_json: String,
    routing_digest: String,
    remediation_summary: String,
    evidence_refs_json: String,
    evidence_refs_digest: String,
    status: String,
    plan_digest: String,
    prepared_by_user_id: String,
    prepared_at: String,
    applied_at: Option<String>,
    superseded_at: Option<String>,
}
impl StoredRecoveryPlan {
    fn as_input(
        &self,
        target_provider: ComputeProvider,
        evidence_refs: Vec<String>,
    ) -> PrepareComputeActivationRecoveryPlan {
        PrepareComputeActivationRecoveryPlan {
            request_id: self.request_id.clone(),
            expected_quarantine_digest: self.expected_quarantine_digest.clone(),
            target_provider,
            remediation_summary: self.remediation_summary.clone(),
            evidence_refs,
            idempotency_scope: String::new(),
            idempotency_key: String::new(),
            prepared_by_user_id: self.prepared_by_user_id.clone(),
        }
    }
    fn into_plan(
        self,
        target_provider: ComputeProvider,
        evidence_refs: Vec<String>,
    ) -> ComputeActivationRecoveryPlan {
        ComputeActivationRecoveryPlan {
            schema: RECOVERY_PLAN_SCHEMA,
            recovery_plan_id: self.recovery_plan_id,
            quarantine_id: self.quarantine_id,
            application_id: self.application_id,
            request_id: self.request_id,
            provider_id: self.provider_id,
            pool_id: self.pool_id,
            expected_quarantine_digest: self.expected_quarantine_digest,
            expected_provider_policy_revision: self.expected_provider_policy_revision,
            expected_provider_digest: self.expected_provider_digest,
            expected_capacity_epoch: self.expected_capacity_epoch,
            expected_pool_revision: self.expected_pool_revision,
            expected_pool_digest: self.expected_pool_digest,
            target_provider_policy_revision: self.target_provider_policy_revision,
            target_provider_digest: self.target_provider_digest,
            target_provider,
            routing_digest: self.routing_digest,
            remediation_summary: self.remediation_summary,
            evidence_refs,
            evidence_refs_digest: self.evidence_refs_digest,
            status: self.status,
            plan_digest: self.plan_digest,
            prepared_by_user_id: self.prepared_by_user_id,
            prepared_at: self.prepared_at,
            applied_at: self.applied_at,
            superseded_at: self.superseded_at,
        }
    }
}

fn recovery_plan_from_row(row: &Row<'_>) -> rusqlite::Result<StoredRecoveryPlan> {
    Ok(StoredRecoveryPlan {
        recovery_plan_id: row.get(0)?,
        quarantine_id: row.get(1)?,
        application_id: row.get(2)?,
        request_id: row.get(3)?,
        provider_id: row.get(4)?,
        pool_id: row.get(5)?,
        expected_quarantine_digest: row.get(6)?,
        expected_provider_policy_revision: row.get(7)?,
        expected_provider_digest: row.get(8)?,
        expected_capacity_epoch: row.get(9)?,
        expected_pool_revision: row.get(10)?,
        expected_pool_digest: row.get(11)?,
        target_provider_policy_revision: row.get(12)?,
        target_provider_digest: row.get(13)?,
        target_provider_json: row.get(14)?,
        routing_digest: row.get(15)?,
        remediation_summary: row.get(16)?,
        evidence_refs_json: row.get(17)?,
        evidence_refs_digest: row.get(18)?,
        status: row.get(19)?,
        plan_digest: row.get(20)?,
        prepared_by_user_id: row.get(21)?,
        prepared_at: row.get(22)?,
        applied_at: row.get(23)?,
        superseded_at: row.get(24)?,
    })
}
const PLAN_SELECT:&str="SELECT recovery_plan_id, quarantine_id, application_id, request_id, provider_id, pool_id, expected_quarantine_digest, expected_provider_policy_revision, expected_provider_digest, expected_capacity_epoch, expected_pool_revision, expected_pool_digest, target_provider_policy_revision, target_provider_digest, target_provider_json, routing_digest, remediation_summary, evidence_refs_json, evidence_refs_digest, status, plan_digest, prepared_by_user_id, prepared_at, applied_at, superseded_at FROM compute_activation_recovery_plans WHERE recovery_plan_id=?1";

fn recovery_review_by_plan_on(conn: &Connection, id: &str) -> Result<Option<StoredRecoveryReview>> {
    recovery_review_on(conn, "WHERE recovery_plan_id=?1", params![id])
}
fn recovery_review_by_idempotency_on(
    conn: &Connection,
    scope: &str,
    key: &str,
) -> Result<Option<StoredRecoveryReview>> {
    recovery_review_on(
        conn,
        "WHERE idempotency_scope=?1 AND idempotency_key=?2",
        params![scope, key],
    )
}
fn recovery_review_on<P: rusqlite::Params>(
    conn: &Connection,
    filter: &str,
    p: P,
) -> Result<Option<StoredRecoveryReview>> {
    conn.query_row(&format!("SELECT recovery_review_id,recovery_plan_id,request_id,plan_digest,prepared_by_user_id,reviewed_by_user_id,review_note,request_digest,review_digest,reviewed_at FROM compute_activation_recovery_reviews {filter}"),p,|r|Ok(StoredRecoveryReview{recovery_review_id:r.get(0)?,recovery_plan_id:r.get(1)?,request_id:r.get(2)?,plan_digest:r.get(3)?,prepared_by_user_id:r.get(4)?,reviewed_by_user_id:r.get(5)?,review_note:r.get(6)?,request_digest:r.get(7)?,review_digest:r.get(8)?,reviewed_at:r.get(9)?})).optional().map_err(Into::into)
}

fn recovery_application_by_plan_on(
    conn: &Connection,
    id: &str,
) -> Result<Option<StoredRecoveryApplication>> {
    recovery_application_on(conn, "WHERE recovery_plan_id=?1", params![id])
}
fn recovery_application_by_request_on(
    conn: &Connection,
    id: &str,
) -> Result<Option<StoredRecoveryApplication>> {
    recovery_application_on(conn, "WHERE request_id=?1", params![id])
}
fn recovery_application_by_idempotency_on(
    conn: &Connection,
    scope: &str,
    key: &str,
) -> Result<Option<StoredRecoveryApplication>> {
    recovery_application_on(
        conn,
        "WHERE idempotency_scope=?1 AND idempotency_key=?2",
        params![scope, key],
    )
}
fn recovery_application_on<P: rusqlite::Params>(
    conn: &Connection,
    filter: &str,
    p: P,
) -> Result<Option<StoredRecoveryApplication>> {
    conn.query_row(&format!("SELECT recovery_application_id,recovery_plan_id,recovery_review_id,quarantine_id,request_id,provider_id,pool_id,plan_digest,review_digest,recovered_provider_policy_revision,recovered_provider_digest,capacity_epoch,pool_lifecycle_event_id,application_digest,applied_by_user_id,applied_at FROM compute_activation_recovery_applications {filter}"),p,|r|Ok(StoredRecoveryApplication{recovery_application_id:r.get(0)?,recovery_plan_id:r.get(1)?,recovery_review_id:r.get(2)?,quarantine_id:r.get(3)?,request_id:r.get(4)?,provider_id:r.get(5)?,pool_id:r.get(6)?,plan_digest:r.get(7)?,review_digest:r.get(8)?,recovered_provider_policy_revision:r.get(9)?,recovered_provider_digest:r.get(10)?,capacity_epoch:r.get(11)?,pool_lifecycle_event_id:r.get(12)?,application_digest:r.get(13)?,applied_by_user_id:r.get(14)?,applied_at:r.get(15)?})).optional().map_err(Into::into)
}
