use anyhow::{anyhow, bail, Context, Result};
use rusqlite::{params, OptionalExtension, Row};
use sha2::{Digest, Sha256};

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
    pub request: RecordBusinessHandoffReceiptRequest,
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
            &completed_at,
        );

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
                    completed_at, created_at
                 ) SELECT
                    ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12,
                    ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20
                  WHERE ?14 <> 'adapter_token_authenticated'
                     OR EXISTS (
                        SELECT 1 FROM open_commerce_adapter_credentials c
                         WHERE c.id=?15 AND c.integration_id=?5 AND c.status='active'
                           AND c.credential_version=?16
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
                    created_at
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
                        receipt_fingerprint
                   FROM open_commerce_business_handoff_receipts
                  WHERE integration_id = ?1 AND receipt_key = ?2",
                params![integration_id.trim(), receipt_key.trim()],
                |row| Ok((handoff_from_row(row)?, row.get(19)?)),
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
    completed_at: &str,
) -> String {
    let canonical = format!(
        "{integration_id}\n{invocation_id}\n{receipt_key}\n{status}\n{target_domain}\n\
         {evidence_result_sha256}\n{}\n{}\n{confirmed_by_user}\n{assertion_authority}\n{}\n{}\n{completed_at}",
        target_reference_sha256.unwrap_or_default(),
        error_code.unwrap_or_default(),
        adapter_credential_id.unwrap_or_default(),
        adapter_credential_version.unwrap_or_default()
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
            completed_at, created_at
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
        }
        _ => bail!("业务衔接回执权威类型不受支持"),
    }
    Ok(())
}
