use anyhow::{anyhow, bail, Context, Result};
use rusqlite::{params, OptionalExtension, Row};
use sha2::{Digest, Sha256};

use crate::open_commerce_business_handoff_model::{
    normalize_handoff_completed_at, normalize_handoff_error_code, normalize_handoff_receipt_key,
    normalize_handoff_status, normalize_sha256, normalize_target_domain,
    normalize_target_reference, OpenCommerceBusinessHandoffReceipt,
    RecordBusinessHandoffReceiptRequest, BUSINESS_HANDOFF_AUTHORITY,
    BUSINESS_HANDOFF_RECEIPT_SCHEMA,
};
use crate::open_commerce_integration_model::INTEGRATION_STATUS_DISABLED;

use super::{new_id, now, Store};

pub(crate) struct RecordOpenCommerceBusinessHandoffReceipt<'a> {
    pub project_id: &'a str,
    pub actor_user_id: &'a str,
    pub actor_app_id: &'a str,
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
        self.conn()?
            .execute(
                "INSERT INTO open_commerce_business_handoff_receipts (
                    id, project_id, merchant_id, invocation_id, integration_id,
                    receipt_key, receipt_fingerprint, status, target_domain,
                    evidence_result_sha256, target_reference_sha256, error_code,
                    confirmed_by_user, assertion_authority, recorded_by_user_id,
                    recorded_by_app_id, completed_at, created_at
                 ) VALUES (
                    ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12,
                    ?13, ?14, ?15, ?16, ?17, ?18
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
                    BUSINESS_HANDOFF_AUTHORITY,
                    input.actor_user_id.trim(),
                    input.actor_app_id.trim(),
                    completed_at,
                    created_at
                ],
            )
            .map_err(map_handoff_conflict)?;
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
                        assertion_authority, recorded_by_user_id, recorded_by_app_id,
                        completed_at, created_at, receipt_fingerprint
                   FROM open_commerce_business_handoff_receipts
                  WHERE integration_id = ?1 AND receipt_key = ?2",
                params![integration_id.trim(), receipt_key.trim()],
                |row| Ok((handoff_from_row(row)?, row.get(17)?)),
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
        recorded_by_user_id: row.get(13)?,
        recorded_by_app_id: row.get(14)?,
        completed_at: row.get(15)?,
        created_at: row.get(16)?,
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
    completed_at: &str,
) -> String {
    let canonical = format!(
        "{integration_id}\n{invocation_id}\n{receipt_key}\n{status}\n{target_domain}\n\
         {evidence_result_sha256}\n{}\n{}\n{confirmed_by_user}\n{completed_at}",
        target_reference_sha256.unwrap_or_default(),
        error_code.unwrap_or_default()
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
            assertion_authority, recorded_by_user_id, recorded_by_app_id,
            completed_at, created_at
       FROM open_commerce_business_handoff_receipts";
