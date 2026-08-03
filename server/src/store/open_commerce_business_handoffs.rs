use anyhow::{anyhow, bail, Context, Result};
use chrono::{Duration, Utc};
use rusqlite::{params, OptionalExtension, Row, TransactionBehavior};
use sha2::{Digest, Sha256};

use crate::open_commerce_adapter_model::{ADAPTER_HANDOFF_CLAIM_SCOPE, ADAPTER_HANDOFF_SCOPE};
use crate::open_commerce_business_handoff_model::{
    normalize_handoff_completed_at, normalize_handoff_error_code, normalize_handoff_receipt_key,
    normalize_handoff_status, normalize_sha256, normalize_target_domain,
    normalize_target_reference, OpenCommerceBusinessHandoffReceipt,
    RecordBusinessHandoffReceiptRequest, BUSINESS_HANDOFF_ADAPTER_AUTHORITY,
    BUSINESS_HANDOFF_AUTHORITY, BUSINESS_HANDOFF_RECEIPT_SCHEMA,
};
use crate::open_commerce_integration_model::INTEGRATION_STATUS_DISABLED;
use crate::open_commerce_merchant_evidence_model::MerchantTerminalInvocationRecord;

use super::{
    new_id, now, open_commerce_invocations::INVOCATION_SELECT,
    open_commerce_merchant_evidence::merchant_record_from_row, Store,
};

pub(crate) struct RecordOpenCommerceBusinessHandoffReceipt<'a> {
    pub project_id: &'a str,
    pub actor_user_id: &'a str,
    pub actor_app_id: &'a str,
    pub assertion_authority: &'a str,
    pub adapter_credential_id: Option<&'a str>,
    pub adapter_credential_version: Option<i64>,
    pub adapter_claim: Option<AdapterClaimReceiptProof<'a>>,
    pub request: RecordBusinessHandoffReceiptRequest,
}

pub(crate) struct AdapterClaimReceiptProof<'a> {
    pub claim_id: &'a str,
    pub lease_token: &'a str,
}

impl Store {
    pub(crate) fn record_open_commerce_business_handoff_receipt(
        &self,
        input: RecordOpenCommerceBusinessHandoffReceipt<'_>,
    ) -> Result<(OpenCommerceBusinessHandoffReceipt, bool)> {
        let integration = self.open_commerce_integration_for_project(
            input.project_id,
            &input.request.integration_id,
        )?;
        if integration.merchant_id != input.request.merchant_id.trim() {
            bail!("数据接入与业务证据不属于同一商户");
        }
        if integration.status == INTEGRATION_STATUS_DISABLED {
            bail!("数据接入已停用，不能记录业务衔接回执");
        }
        validate_authority(self, &input, &integration.id)?;

        let receipt_key = normalize_handoff_receipt_key(&input.request.receipt_key)?;
        let status = normalize_handoff_status(&input.request.status)?;
        let target_domain = normalize_target_domain(&input.request.target_domain)?;
        let evidence_result_sha256 =
            normalize_sha256(&input.request.evidence_result_sha256, "业务证据摘要")?;
        let target_reference =
            normalize_target_reference(input.request.target_reference.as_deref())?;
        let target_reference_sha256 = target_reference
            .as_deref()
            .map(|value| hex::encode(Sha256::digest(value.as_bytes())));
        let error_code = normalize_handoff_error_code(input.request.error_code.as_deref())?;
        let completed_at = normalize_handoff_completed_at(&input.request.completed_at)?;
        let fingerprint = receipt_fingerprint(
            &integration.id,
            &input.request.invocation_id,
            &receipt_key,
            &status,
            &target_domain,
            &evidence_result_sha256,
            target_reference_sha256.as_deref(),
            error_code.as_deref(),
            input.request.confirmed_by_user,
            input.assertion_authority,
            input.adapter_credential_id,
            input.adapter_credential_version,
            input.adapter_claim.as_ref().map(|claim| claim.claim_id),
            &completed_at,
        );

        if let Some(claim) = input.adapter_claim.as_ref() {
            return self.record_claimed_open_commerce_business_handoff_receipt(
                &input,
                claim,
                &integration.id,
                &receipt_key,
                &fingerprint,
                &status,
                &target_domain,
                &evidence_result_sha256,
                target_reference_sha256.as_deref(),
                error_code.as_deref(),
                &completed_at,
            );
        }

        if let Some((existing, stored_fingerprint)) =
            self.find_open_commerce_business_handoff_receipt(&integration.id, &receipt_key)?
        {
            if stored_fingerprint != fingerprint {
                bail!("相同业务衔接回执键不能用于不同结果");
            }
            return Ok((existing, false));
        }

        let id = new_id("handoff");
        let created_at = now();
        let inserted = self
            .conn()?
            .execute(
                "INSERT INTO open_commerce_business_handoff_receipts (
                    id, project_id, merchant_id, invocation_id, integration_id,
                    receipt_key, receipt_fingerprint, status, target_domain,
                    evidence_result_sha256, target_reference_sha256, error_code,
                    confirmed_by_user, assertion_authority, adapter_credential_id,
                    adapter_credential_version, recorded_by_user_id, recorded_by_app_id,
                    completed_at, created_at, adapter_claim_id
                 ) SELECT
                    ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12,
                    ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, NULL
                  WHERE ?14 <> 'adapter_token_authenticated'
                     OR EXISTS (
                        SELECT 1 FROM open_commerce_adapter_credentials c
                        JOIN open_commerce_integrations n ON n.id=c.integration_id
                         WHERE c.id=?15 AND c.integration_id=?5 AND c.status='active'
                            AND c.credential_version=?16
                            AND julianday(c.expires_at) > julianday(?20)
                            AND n.status<>'disabled'
                            AND instr(c.scopes_json, ?21) > 0
                     )",
                params![
                    id,
                    input.project_id.trim(),
                    input.request.merchant_id.trim(),
                    input.request.invocation_id.trim(),
                    integration.id,
                    receipt_key,
                    fingerprint,
                    status,
                    target_domain,
                    evidence_result_sha256,
                    target_reference_sha256,
                    error_code,
                    input.request.confirmed_by_user,
                    input.assertion_authority,
                    input.adapter_credential_id,
                    input.adapter_credential_version,
                    input.actor_user_id.trim(),
                    input.actor_app_id.trim(),
                    completed_at,
                    created_at,
                    scope_fragment(ADAPTER_HANDOFF_SCOPE)
                ],
            )
            .map_err(map_handoff_conflict)?;
        if inserted == 0 {
            bail!("适配器凭据已撤销或轮换，请重新鉴权后提交回执");
        }
        Ok((
            self.open_commerce_business_handoff_receipt(input.project_id, &id)?,
            true,
        ))
    }

    #[allow(clippy::too_many_arguments)]
    fn record_claimed_open_commerce_business_handoff_receipt(
        &self,
        input: &RecordOpenCommerceBusinessHandoffReceipt<'_>,
        claim: &AdapterClaimReceiptProof<'_>,
        integration_id: &str,
        receipt_key: &str,
        fingerprint: &str,
        status: &str,
        target_domain: &str,
        evidence_result_sha256: &str,
        target_reference_sha256: Option<&str>,
        error_code: Option<&str>,
        completed_at: &str,
    ) -> Result<(OpenCommerceBusinessHandoffReceipt, bool)> {
        let credential_id = input
            .adapter_credential_id
            .ok_or_else(|| anyhow!("租约回执必须绑定机器凭据"))?;
        let credential_version = input
            .adapter_credential_version
            .ok_or_else(|| anyhow!("租约回执必须绑定机器凭据版本"))?;
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let existing = tx
            .query_row(
                "SELECT id, project_id, merchant_id, invocation_id, integration_id,
                        receipt_key, status, target_domain, evidence_result_sha256,
                        target_reference_sha256, error_code, confirmed_by_user,
                        assertion_authority, adapter_credential_id, adapter_credential_version,
                        recorded_by_user_id, recorded_by_app_id, completed_at, created_at,
                        adapter_claim_id, receipt_fingerprint
                   FROM open_commerce_business_handoff_receipts
                  WHERE integration_id=?1 AND receipt_key=?2",
                params![integration_id, receipt_key],
                |row| Ok((handoff_from_row(row)?, row.get::<_, String>(20)?)),
            )
            .optional()?;
        if let Some((existing, stored_fingerprint)) = existing {
            if stored_fingerprint != fingerprint
                || existing.adapter_claim_id.as_deref() != Some(claim.claim_id)
            {
                bail!("相同业务衔接回执键不能用于不同结果");
            }
            let replay_valid: bool = tx.query_row(
                "SELECT EXISTS(
                   SELECT 1 FROM open_commerce_business_handoff_claims c
                    WHERE c.id=?1 AND c.lease_token_hash=?2
                      AND c.status='completed' AND c.completed_receipt_id=?3
                      AND c.adapter_credential_id=?4
                      AND c.adapter_credential_version=?5
                 )",
                params![
                    claim.claim_id,
                    super::open_commerce_adapter_claims::claim_token_hash(claim.lease_token),
                    existing.id,
                    credential_id,
                    credential_version,
                ],
                |row| row.get(0),
            )?;
            if !replay_valid {
                bail!("已完成租约的重放凭据不匹配");
            }
            tx.commit()?;
            return Ok((existing, false));
        }

        let id = new_id("handoff");
        let created_at = now();
        let lease_token_hash =
            super::open_commerce_adapter_claims::claim_token_hash(claim.lease_token);
        let inserted = tx
            .execute(
                "INSERT INTO open_commerce_business_handoff_receipts (
                    id, project_id, merchant_id, invocation_id, integration_id,
                    receipt_key, receipt_fingerprint, status, target_domain,
                    evidence_result_sha256, target_reference_sha256, error_code,
                    confirmed_by_user, assertion_authority, adapter_credential_id,
                    adapter_credential_version, recorded_by_user_id, recorded_by_app_id,
                    completed_at, created_at, adapter_claim_id
                 ) SELECT
                    ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12,
                    ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21
                  WHERE EXISTS (
                    SELECT 1
                      FROM open_commerce_business_handoff_claims claim
                      JOIN open_commerce_adapter_credentials credential
                        ON credential.id=claim.adapter_credential_id
                      JOIN open_commerce_integrations integration
                        ON integration.id=claim.integration_id
                     WHERE claim.id=?21 AND claim.lease_token_hash=?22
                       AND claim.status='active'
                       AND claim.project_id=?2 AND claim.merchant_id=?3
                       AND claim.invocation_id=?4 AND claim.integration_id=?5
                       AND claim.adapter_credential_id=?15
                       AND claim.adapter_credential_version=?16
                       AND julianday(claim.lease_expires_at) > julianday(?20)
                       AND credential.status='active'
                       AND credential.credential_version=?16
                       AND julianday(credential.expires_at) > julianday(?20)
                       AND integration.status<>'disabled'
                       AND instr(credential.scopes_json, ?23) > 0
                       AND instr(credential.scopes_json, ?24) > 0
                  )",
                params![
                    id,
                    input.project_id.trim(),
                    input.request.merchant_id.trim(),
                    input.request.invocation_id.trim(),
                    integration_id,
                    receipt_key,
                    fingerprint,
                    status,
                    target_domain,
                    evidence_result_sha256,
                    target_reference_sha256,
                    error_code,
                    input.request.confirmed_by_user,
                    input.assertion_authority,
                    credential_id,
                    credential_version,
                    input.actor_user_id.trim(),
                    input.actor_app_id.trim(),
                    completed_at,
                    created_at,
                    claim.claim_id,
                    lease_token_hash,
                    scope_fragment(ADAPTER_HANDOFF_CLAIM_SCOPE),
                    scope_fragment(ADAPTER_HANDOFF_SCOPE),
                ],
            )
            .map_err(map_handoff_conflict)?;
        if inserted == 0 {
            bail!("衔接任务租约已过期、被替换或机器凭据已失效");
        }
        let attempt_no: i64 = tx.query_row(
            "SELECT attempt_no FROM open_commerce_business_handoff_claims WHERE id=?1",
            params![claim.claim_id],
            |row| row.get(0),
        )?;
        let retry_suspended = status == "rejected" && attempt_no >= 6;
        let retry_not_before = if status == "rejected" && !retry_suspended {
            let exponent = (attempt_no.saturating_sub(1)).clamp(0, 5) as u32;
            let delay_seconds = 30_i64.saturating_mul(1_i64 << exponent).min(900);
            Some((Utc::now() + Duration::seconds(delay_seconds)).to_rfc3339())
        } else {
            None
        };
        let retry_suspended_at = retry_suspended.then(|| created_at.clone());
        let retry_suspension_reason = retry_suspended.then(|| "max_rejected_attempts".to_string());
        let completed = tx.execute(
            "UPDATE open_commerce_business_handoff_claims
                SET status='completed', completed_receipt_id=?1,
                    completion_status=?2, retry_not_before=?3,
                    retry_suspended_at=?4, retry_suspension_reason=?5,
                    retry_resumed_at=NULL, retry_resumed_by_user_id=NULL,
                    updated_at=?6
              WHERE id=?7 AND status='active' AND lease_token_hash=?8
                AND adapter_credential_id=?9 AND adapter_credential_version=?10",
            params![
                id,
                status,
                retry_not_before,
                retry_suspended_at,
                retry_suspension_reason,
                created_at,
                claim.claim_id,
                lease_token_hash,
                credential_id,
                credential_version,
            ],
        )?;
        if completed != 1 {
            bail!("衔接任务租约完成状态发生并发冲突");
        }
        tx.commit()?;
        drop(conn);
        Ok((
            self.open_commerce_business_handoff_receipt(input.project_id, &id)?,
            true,
        ))
    }

    pub(crate) fn list_open_commerce_business_handoff_receipts(
        &self,
        project_id: &str,
        merchant_id: &str,
        limit: usize,
    ) -> Result<Vec<OpenCommerceBusinessHandoffReceipt>> {
        self.open_commerce_merchant_for_project(project_id, merchant_id)?;
        let conn = self.conn()?;
        let mut statement = conn.prepare(&format!(
            "{HANDOFF_SELECT}
             WHERE project_id = ?1 AND merchant_id = ?2
             ORDER BY created_at DESC LIMIT ?3"
        ))?;
        let receipts = statement
            .query_map(
                params![
                    project_id.trim(),
                    merchant_id.trim(),
                    limit.clamp(1, 200) as i64
                ],
                handoff_from_row,
            )?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(receipts)
    }

    pub(crate) fn list_open_commerce_business_handoff_queue_records(
        &self,
        project_id: &str,
        merchant_id: &str,
        state: Option<&str>,
        limit: usize,
    ) -> Result<Vec<MerchantTerminalInvocationRecord>> {
        self.open_commerce_merchant_for_project(project_id, merchant_id)?;
        let conn = self.conn()?;
        let mut statement = conn.prepare(&format!(
            "SELECT i.*, e.seq
               FROM open_commerce_invocation_terminal_events e
               JOIN ({INVOCATION_SELECT}) i ON i.id = e.invocation_id
               LEFT JOIN open_commerce_business_handoff_receipts h
                 ON h.id = (
                    SELECT h2.id
                      FROM open_commerce_business_handoff_receipts h2
                     WHERE h2.project_id = i.project_id
                       AND h2.merchant_id = i.merchant_id
                       AND h2.invocation_id = i.id
                     ORDER BY h2.completed_at DESC, h2.created_at DESC, h2.id DESC
                     LIMIT 1
                 )
              WHERE i.project_id = ?1
                AND i.merchant_id = ?2
                AND i.result_json IS NOT NULL
                AND (h.id IS NULL OR h.status = 'rejected')
                AND (
                    ?3 = ''
                    OR (?3 = 'pending' AND h.id IS NULL)
                    OR (?3 = 'retry_required' AND h.status = 'rejected')
                )
              ORDER BY e.seq DESC
              LIMIT ?4"
        ))?;
        let records = statement
            .query_map(
                params![
                    project_id.trim(),
                    merchant_id.trim(),
                    state.unwrap_or_default(),
                    limit.clamp(1, 201) as i64
                ],
                merchant_record_from_row,
            )?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(records)
    }

    pub(crate) fn latest_open_commerce_business_handoff_receipt(
        &self,
        project_id: &str,
        merchant_id: &str,
        invocation_id: &str,
    ) -> Result<Option<OpenCommerceBusinessHandoffReceipt>> {
        self.conn()?
            .query_row(
                &format!(
                    "{HANDOFF_SELECT}
                     WHERE project_id = ?1 AND merchant_id = ?2 AND invocation_id = ?3
                     ORDER BY completed_at DESC, created_at DESC, id DESC
                     LIMIT 1"
                ),
                params![project_id.trim(), merchant_id.trim(), invocation_id.trim()],
                handoff_from_row,
            )
            .optional()
            .map_err(Into::into)
    }

    fn open_commerce_business_handoff_receipt(
        &self,
        project_id: &str,
        receipt_id: &str,
    ) -> Result<OpenCommerceBusinessHandoffReceipt> {
        self.conn()?
            .query_row(
                &format!("{HANDOFF_SELECT} WHERE project_id = ?1 AND id = ?2"),
                params![project_id.trim(), receipt_id.trim()],
                handoff_from_row,
            )
            .map_err(|error| anyhow!(error).context("业务衔接回执不存在"))
    }

    fn find_open_commerce_business_handoff_receipt(
        &self,
        integration_id: &str,
        receipt_key: &str,
    ) -> Result<Option<(OpenCommerceBusinessHandoffReceipt, String)>> {
        self.conn()?
            .query_row(
                "SELECT id, project_id, merchant_id, invocation_id, integration_id,
                        receipt_key, status, target_domain, evidence_result_sha256,
                        target_reference_sha256, error_code, confirmed_by_user,
                        assertion_authority, adapter_credential_id, adapter_credential_version,
                        recorded_by_user_id, recorded_by_app_id, completed_at, created_at,
                        adapter_claim_id, receipt_fingerprint
                   FROM open_commerce_business_handoff_receipts
                  WHERE integration_id = ?1 AND receipt_key = ?2",
                params![integration_id.trim(), receipt_key.trim()],
                |row| Ok((handoff_from_row(row)?, row.get(20)?)),
            )
            .optional()
            .map_err(Into::into)
    }
}

fn handoff_from_row(row: &Row<'_>) -> rusqlite::Result<OpenCommerceBusinessHandoffReceipt> {
    Ok(OpenCommerceBusinessHandoffReceipt {
        schema: BUSINESS_HANDOFF_RECEIPT_SCHEMA,
        id: row.get(0)?,
        project_id: row.get(1)?,
        merchant_id: row.get(2)?,
        invocation_id: row.get(3)?,
        integration_id: row.get(4)?,
        receipt_key: row.get(5)?,
        status: row.get(6)?,
        target_domain: row.get(7)?,
        evidence_result_sha256: row.get(8)?,
        target_reference_sha256: row.get(9)?,
        error_code: row.get(10)?,
        confirmed_by_user: row.get(11)?,
        assertion_authority: row.get(12)?,
        adapter_credential_id: row.get(13)?,
        adapter_credential_version: row.get(14)?,
        recorded_by_user_id: row.get(15)?,
        recorded_by_app_id: row.get(16)?,
        completed_at: row.get(17)?,
        created_at: row.get(18)?,
        adapter_claim_id: row.get(19)?,
        funds_moved: false,
    })
}

#[allow(clippy::too_many_arguments)]
fn receipt_fingerprint(
    integration_id: &str,
    invocation_id: &str,
    receipt_key: &str,
    status: &str,
    target_domain: &str,
    evidence_result_sha256: &str,
    target_reference_sha256: Option<&str>,
    error_code: Option<&str>,
    confirmed_by_user: bool,
    assertion_authority: &str,
    adapter_credential_id: Option<&str>,
    adapter_credential_version: Option<i64>,
    adapter_claim_id: Option<&str>,
    completed_at: &str,
) -> String {
    let canonical = format!(
        "{integration_id}\n{invocation_id}\n{receipt_key}\n{status}\n{target_domain}\n\
         {evidence_result_sha256}\n{}\n{}\n{confirmed_by_user}\n{assertion_authority}\n{}\n{}\n{}\n{completed_at}",
        target_reference_sha256.unwrap_or_default(),
        error_code.unwrap_or_default(),
        adapter_credential_id.unwrap_or_default(),
        adapter_credential_version.unwrap_or_default(),
        adapter_claim_id.unwrap_or_default()
    );
    hex::encode(Sha256::digest(canonical.as_bytes()))
}

fn map_handoff_conflict(error: rusqlite::Error) -> anyhow::Error {
    if error.to_string().contains("UNIQUE constraint failed") {
        anyhow!("业务衔接回执发生并发冲突，请读取已有回执")
    } else {
        anyhow!(error)
    }
}

const HANDOFF_SELECT: &str = "SELECT id, project_id, merchant_id, invocation_id, integration_id,
            receipt_key, status, target_domain, evidence_result_sha256,
            target_reference_sha256, error_code, confirmed_by_user,
            assertion_authority, adapter_credential_id, adapter_credential_version,
            recorded_by_user_id, recorded_by_app_id,
            completed_at, created_at, adapter_claim_id
       FROM open_commerce_business_handoff_receipts";

fn validate_authority(
    store: &Store,
    input: &RecordOpenCommerceBusinessHandoffReceipt<'_>,
    integration_id: &str,
) -> Result<()> {
    match input.assertion_authority {
        BUSINESS_HANDOFF_AUTHORITY => {
            if !input.request.confirmed_by_user
                || input.adapter_credential_id.is_some()
                || input.adapter_credential_version.is_some()
                || input.adapter_claim.is_some()
            {
                bail!("人工衔接回执必须由用户确认且不能绑定适配器凭据");
            }
        }
        BUSINESS_HANDOFF_ADAPTER_AUTHORITY => {
            if input.request.confirmed_by_user {
                bail!("适配器衔接回执不能伪装成人工确认");
            }
            let credential_id = input
                .adapter_credential_id
                .ok_or_else(|| anyhow!("适配器衔接回执必须绑定机器凭据"))?;
            let credential_version = input
                .adapter_credential_version
                .ok_or_else(|| anyhow!("适配器衔接回执必须绑定机器凭据版本"))?;
            let credential = store
                .open_commerce_adapter_credential_for_project(input.project_id, credential_id)?;
            if credential.integration_id != integration_id
                || credential.status != "active"
                || credential.credential_version != credential_version
            {
                bail!("适配器凭据与当前数据接入不匹配或已撤销");
            }
            if input.adapter_claim.is_some()
                && !credential
                    .scopes
                    .iter()
                    .any(|scope| scope == ADAPTER_HANDOFF_CLAIM_SCOPE)
            {
                bail!("适配器凭据未获得 business_handoff.claim 权限");
            }
        }
        _ => bail!("业务衔接回执权威类型不受支持"),
    }
    Ok(())
}

fn scope_fragment(scope: &str) -> String {
    format!("\"{scope}\"")
}
