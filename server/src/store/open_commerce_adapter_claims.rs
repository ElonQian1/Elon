use anyhow::{anyhow, Context, Result};
use chrono::{Duration, Utc};
use rusqlite::{params, OptionalExtension, Row, TransactionBehavior};
use sha2::{Digest, Sha256};

use crate::{
    open_commerce_adapter_claim_model::{
        OpenCommerceAdapterHandoffClaim, ADAPTER_HANDOFF_CLAIM_SCHEMA,
    },
    open_commerce_adapter_model::{
        OpenCommerceAdapterCredential, ADAPTER_HANDOFF_CLAIM_SCOPE, ADAPTER_HANDOFF_SCOPE,
    },
};

use super::{new_id, now, Store};

impl Store {
    pub(crate) fn list_project_open_commerce_adapter_handoff_claims(
        &self,
        project_id: &str,
        limit: usize,
    ) -> Result<Vec<OpenCommerceAdapterHandoffClaim>> {
        let conn = self.conn()?;
        let mut statement = conn.prepare(&format!(
            "{CLAIM_SELECT} WHERE claim.project_id=?1
             ORDER BY claim.created_at DESC LIMIT ?2"
        ))?;
        let claims = statement
            .query_map(
                params![project_id.trim(), limit.clamp(1, 200) as i64],
                claim_from_row,
            )?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(claims)
    }

    pub(crate) fn list_open_commerce_adapter_handoff_candidate_ids(
        &self,
        credential: &OpenCommerceAdapterCredential,
        limit: usize,
    ) -> Result<Vec<String>> {
        let conn = self.conn()?;
        let mut statement = conn.prepare(
            "SELECT i.id
               FROM open_commerce_invocation_terminal_events e
               JOIN open_commerce_invocations i ON i.id=e.invocation_id
              WHERE i.project_id=?1
                AND i.merchant_id=?2
                AND i.status='succeeded'
                AND i.result_json IS NOT NULL
                AND NOT EXISTS (
                    SELECT 1
                      FROM open_commerce_business_handoff_receipts h
                     WHERE h.id=(
                       SELECT h2.id
                         FROM open_commerce_business_handoff_receipts h2
                        WHERE h2.project_id=i.project_id
                          AND h2.merchant_id=i.merchant_id
                          AND h2.invocation_id=i.id
                        ORDER BY h2.completed_at DESC, h2.created_at DESC, h2.id DESC
                        LIMIT 1
                     )
                       AND h.status IN ('applied', 'ignored')
                )
                AND NOT EXISTS (
                    SELECT 1
                      FROM open_commerce_business_handoff_claims c
                     WHERE c.invocation_id=i.id
                       AND c.status='active'
                       AND julianday(c.lease_expires_at) > julianday(?3)
                )
                AND NOT EXISTS (
                    SELECT 1
                      FROM open_commerce_business_handoff_claims cooldown
                     WHERE cooldown.invocation_id=i.id
                       AND cooldown.retry_not_before IS NOT NULL
                       AND julianday(cooldown.retry_not_before) > julianday(?3)
                )
                AND NOT EXISTS (
                    SELECT 1
                      FROM open_commerce_business_handoff_claims suspended
                     WHERE suspended.invocation_id=i.id
                       AND suspended.retry_suspended_at IS NOT NULL
                       AND suspended.retry_resumed_at IS NULL
                )
              ORDER BY COALESCE((
                         SELECT julianday(MAX(previous.created_at))
                           FROM open_commerce_business_handoff_claims previous
                          WHERE previous.invocation_id=i.id
                       ), 0) ASC,
                       e.seq ASC
              LIMIT ?4",
        )?;
        let ids = statement
            .query_map(
                params![
                    credential.project_id,
                    credential.merchant_id,
                    now(),
                    limit.clamp(1, 50) as i64
                ],
                |row| row.get(0),
            )?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(ids)
    }

    pub(crate) fn try_claim_open_commerce_adapter_handoff(
        &self,
        credential: &OpenCommerceAdapterCredential,
        invocation_id: &str,
        lease_seconds: i64,
    ) -> Result<Option<(OpenCommerceAdapterHandoffClaim, String)>> {
        let lease_token = new_claim_token();
        let timestamp = now();
        let lease_expires_at = (Utc::now() + Duration::seconds(lease_seconds)).to_rfc3339();
        let lease_deadline_at = (Utc::now() + Duration::hours(1)).to_rfc3339();
        let claim_id = new_id("handoffclaim");
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        tx.execute(
            "UPDATE open_commerce_business_handoff_claims
                SET status='expired', updated_at=?1
              WHERE project_id=?2 AND merchant_id=?3 AND status='active'
                AND julianday(lease_expires_at) <= julianday(?1)",
            params![timestamp, credential.project_id, credential.merchant_id],
        )?;
        let eligible: bool = tx.query_row(
            "SELECT EXISTS(
               SELECT 1
                 FROM open_commerce_invocations i
                 JOIN open_commerce_adapter_credentials c
                   ON c.id=?4 AND c.integration_id=?5
                 JOIN open_commerce_integrations n ON n.id=c.integration_id
                WHERE i.id=?1 AND i.project_id=?2 AND i.merchant_id=?3
                  AND i.status='succeeded' AND i.result_json IS NOT NULL
                  AND c.status='active' AND c.credential_version=?6
                  AND julianday(c.expires_at) > julianday(?7)
                  AND n.status<>'disabled'
                  AND instr(c.scopes_json, ?8) > 0
                  AND instr(c.scopes_json, ?9) > 0
                  AND NOT EXISTS (
                    SELECT 1 FROM open_commerce_business_handoff_claims active_claim
                     WHERE active_claim.invocation_id=i.id AND active_claim.status='active'
                  )
                  AND NOT EXISTS (
                    SELECT 1 FROM open_commerce_business_handoff_claims cooldown
                     WHERE cooldown.invocation_id=i.id
                       AND cooldown.retry_not_before IS NOT NULL
                       AND julianday(cooldown.retry_not_before) > julianday(?7)
                  )
                  AND NOT EXISTS (
                    SELECT 1 FROM open_commerce_business_handoff_claims suspended
                     WHERE suspended.invocation_id=i.id
                       AND suspended.retry_suspended_at IS NOT NULL
                       AND suspended.retry_resumed_at IS NULL
                  )
                  AND NOT EXISTS (
                    SELECT 1 FROM open_commerce_business_handoff_receipts h
                     WHERE h.id=(
                       SELECT h2.id FROM open_commerce_business_handoff_receipts h2
                        WHERE h2.project_id=i.project_id
                          AND h2.merchant_id=i.merchant_id
                          AND h2.invocation_id=i.id
                        ORDER BY h2.completed_at DESC, h2.created_at DESC, h2.id DESC
                        LIMIT 1
                     ) AND h.status IN ('applied', 'ignored')
                  )
             )",
            params![
                invocation_id.trim(),
                credential.project_id,
                credential.merchant_id,
                credential.id,
                credential.integration_id,
                credential.credential_version,
                timestamp,
                scope_fragment(ADAPTER_HANDOFF_CLAIM_SCOPE),
                scope_fragment(ADAPTER_HANDOFF_SCOPE),
            ],
            |row| row.get(0),
        )?;
        if !eligible {
            tx.commit()?;
            return Ok(None);
        }
        let attempt_no: i64 = tx.query_row(
            "SELECT COALESCE(MAX(attempt_no), 0) + 1
               FROM open_commerce_business_handoff_claims
              WHERE invocation_id=?1",
            params![invocation_id.trim()],
            |row| row.get(0),
        )?;
        let inserted = tx.execute(
            "INSERT INTO open_commerce_business_handoff_claims (
               id, project_id, merchant_id, invocation_id, integration_id,
               adapter_credential_id, adapter_credential_version, attempt_no,
               status, lease_token_hash, lease_token_hint, lease_expires_at,
               lease_deadline_at,
               release_reason_code, released_at, completion_status, retry_not_before,
               retry_suspended_at, retry_suspension_reason, retry_resumed_at,
               retry_resumed_by_user_id,
               completed_receipt_id, created_at, updated_at
             ) VALUES (
               ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 'active', ?9, ?10, ?11, ?12,
               NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, ?13, ?13
             )",
            params![
                claim_id,
                credential.project_id,
                credential.merchant_id,
                invocation_id.trim(),
                credential.integration_id,
                credential.id,
                credential.credential_version,
                attempt_no,
                claim_token_hash(&lease_token),
                token_hint(&lease_token),
                lease_expires_at,
                lease_deadline_at,
                timestamp,
            ],
        );
        if let Err(error) = inserted {
            if error.to_string().contains("UNIQUE constraint failed") {
                tx.rollback()?;
                return Ok(None);
            }
            return Err(error.into());
        }
        tx.commit()?;
        drop(conn);
        let claim = self.open_commerce_adapter_handoff_claim(&claim_id)?;
        Ok(Some((claim, lease_token)))
    }

    pub(crate) fn verify_open_commerce_adapter_handoff_claim(
        &self,
        credential: &OpenCommerceAdapterCredential,
        claim_id: &str,
        lease_token: &str,
    ) -> Result<OpenCommerceAdapterHandoffClaim> {
        self.conn()?
            .query_row(
                &format!(
                    "{CLAIM_SELECT}
                     JOIN open_commerce_adapter_credentials credential
                       ON credential.id=claim.adapter_credential_id
                     JOIN open_commerce_integrations integration
                       ON integration.id=claim.integration_id
                    WHERE claim.id=?1
                      AND claim.lease_token_hash=?2
                      AND claim.adapter_credential_id=?3
                      AND claim.adapter_credential_version=?4
                      AND claim.integration_id=?5
                      AND claim.status IN ('active', 'completed')
                      AND (claim.status='completed'
                        OR julianday(claim.lease_expires_at) > julianday(?6))
                      AND credential.status='active'
                      AND credential.credential_version=?4
                      AND julianday(credential.expires_at) > julianday(?6)
                      AND integration.status<>'disabled'
                      AND instr(credential.scopes_json, ?7) > 0
                      AND instr(credential.scopes_json, ?8) > 0"
                ),
                params![
                    claim_id.trim(),
                    claim_token_hash(lease_token.trim()),
                    credential.id,
                    credential.credential_version,
                    credential.integration_id,
                    now(),
                    scope_fragment(ADAPTER_HANDOFF_CLAIM_SCOPE),
                    scope_fragment(ADAPTER_HANDOFF_SCOPE),
                ],
                claim_from_row,
            )
            .map_err(|error| {
                anyhow!(error).context("衔接任务租约无效、已过期、已被重新领取或机器凭据已失效")
            })
    }

    pub(crate) fn release_open_commerce_adapter_handoff_claim(
        &self,
        credential: &OpenCommerceAdapterCredential,
        claim_id: &str,
        lease_token: &str,
        reason_code: &str,
    ) -> Result<OpenCommerceAdapterHandoffClaim> {
        let timestamp = now();
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let changed = tx.execute(
            "UPDATE open_commerce_business_handoff_claims AS claim
                SET status='expired', release_reason_code=?1, released_at=?2,
                    completion_status=NULL, retry_not_before=NULL,
                    retry_suspended_at=NULL, retry_suspension_reason=NULL,
                    retry_resumed_at=NULL, retry_resumed_by_user_id=NULL,
                    updated_at=?2
              WHERE claim.id=?3
                AND claim.lease_token_hash=?4
                AND claim.adapter_credential_id=?5
                AND claim.adapter_credential_version=?6
                AND claim.integration_id=?7
                AND claim.status='active'
                AND julianday(claim.lease_expires_at) > julianday(?2)
                AND EXISTS (
                  SELECT 1
                    FROM open_commerce_adapter_credentials credential
                    JOIN open_commerce_integrations integration
                      ON integration.id=credential.integration_id
                   WHERE credential.id=claim.adapter_credential_id
                     AND credential.status='active'
                     AND credential.credential_version=claim.adapter_credential_version
                     AND julianday(credential.expires_at) > julianday(?2)
                     AND integration.id=claim.integration_id
                     AND integration.status<>'disabled'
                     AND instr(credential.scopes_json, ?8) > 0
                     AND instr(credential.scopes_json, ?9) > 0
                )",
            params![
                reason_code,
                timestamp,
                claim_id.trim(),
                claim_token_hash(lease_token.trim()),
                credential.id,
                credential.credential_version,
                credential.integration_id,
                scope_fragment(ADAPTER_HANDOFF_CLAIM_SCOPE),
                scope_fragment(ADAPTER_HANDOFF_SCOPE),
            ],
        )?;
        if changed != 1 {
            return Err(anyhow!(
                "衔接任务租约无效、已完成、已过期、已被释放或机器凭据已失效"
            ));
        }
        tx.commit()?;
        drop(conn);
        self.open_commerce_adapter_handoff_claim(claim_id)
    }

    pub(crate) fn renew_open_commerce_adapter_handoff_claim(
        &self,
        credential: &OpenCommerceAdapterCredential,
        claim_id: &str,
        lease_token: &str,
        extend_seconds: i64,
    ) -> Result<OpenCommerceAdapterHandoffClaim> {
        let timestamp = now();
        let requested_expires_at = (Utc::now() + Duration::seconds(extend_seconds)).to_rfc3339();
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let changed = tx.execute(
            "UPDATE open_commerce_business_handoff_claims AS claim
                SET lease_expires_at=CASE
                      WHEN julianday(?1) < julianday(claim.lease_deadline_at) THEN ?1
                      ELSE claim.lease_deadline_at
                    END,
                    updated_at=?2
              WHERE claim.id=?3
                AND claim.lease_token_hash=?4
                AND claim.adapter_credential_id=?5
                AND claim.adapter_credential_version=?6
                AND claim.integration_id=?7
                AND claim.status='active'
                AND julianday(claim.lease_expires_at) > julianday(?2)
                AND julianday(claim.lease_deadline_at) > julianday(?2)
                AND EXISTS (
                  SELECT 1
                    FROM open_commerce_adapter_credentials credential
                    JOIN open_commerce_integrations integration
                      ON integration.id=credential.integration_id
                   WHERE credential.id=claim.adapter_credential_id
                     AND credential.status='active'
                     AND credential.credential_version=claim.adapter_credential_version
                     AND julianday(credential.expires_at) > julianday(?2)
                     AND integration.id=claim.integration_id
                     AND integration.status<>'disabled'
                     AND instr(credential.scopes_json, ?8) > 0
                     AND instr(credential.scopes_json, ?9) > 0
                )",
            params![
                requested_expires_at,
                timestamp,
                claim_id.trim(),
                claim_token_hash(lease_token.trim()),
                credential.id,
                credential.credential_version,
                credential.integration_id,
                scope_fragment(ADAPTER_HANDOFF_CLAIM_SCOPE),
                scope_fragment(ADAPTER_HANDOFF_SCOPE),
            ],
        )?;
        if changed != 1 {
            return Err(anyhow!(
                "衔接任务租约无效、已完成、已过期、已被释放、达到最长处理期限或机器凭据已失效"
            ));
        }
        tx.commit()?;
        drop(conn);
        self.open_commerce_adapter_handoff_claim(claim_id)
    }

    pub(crate) fn resume_open_commerce_adapter_handoff_retry(
        &self,
        project_id: &str,
        claim_id: &str,
        resumed_by_user_id: &str,
    ) -> Result<OpenCommerceAdapterHandoffClaim> {
        let timestamp = now();
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let changed = tx.execute(
            "UPDATE open_commerce_business_handoff_claims AS claim
                SET retry_resumed_at=?1, retry_resumed_by_user_id=?2,
                    retry_not_before=NULL, updated_at=?1
              WHERE claim.id=?3 AND claim.project_id=?4
                AND claim.status='completed'
                AND claim.completion_status='rejected'
                AND claim.retry_suspended_at IS NOT NULL
                AND claim.retry_resumed_at IS NULL
                AND NOT EXISTS (
                  SELECT 1 FROM open_commerce_business_handoff_claims newer
                   WHERE newer.invocation_id=claim.invocation_id
                     AND newer.attempt_no > claim.attempt_no
                )",
            params![
                timestamp,
                resumed_by_user_id.trim(),
                claim_id.trim(),
                project_id.trim(),
            ],
        )?;
        if changed != 1 {
            return Err(anyhow!(
                "该租约不存在、并非当前暂停重试项或已经由其他编辑者重新排队"
            ));
        }
        tx.commit()?;
        drop(conn);
        self.open_commerce_adapter_handoff_claim(claim_id)
    }

    pub(crate) fn open_commerce_adapter_handoff_claim(
        &self,
        claim_id: &str,
    ) -> Result<OpenCommerceAdapterHandoffClaim> {
        self.conn()?
            .query_row(
                &format!("{CLAIM_SELECT} WHERE claim.id=?1"),
                params![claim_id.trim()],
                claim_from_row,
            )
            .map_err(|error| anyhow!(error).context("衔接任务租约不存在"))
    }
}

pub(crate) fn claim_token_hash(value: &str) -> String {
    hex::encode(Sha256::digest(value.as_bytes()))
}

fn new_claim_token() -> String {
    format!(
        "oc_claim_{}{}",
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple()
    )
}

fn token_hint(value: &str) -> String {
    format!("...{}", &value[value.len().saturating_sub(6)..])
}

fn scope_fragment(scope: &str) -> String {
    format!("\"{scope}\"")
}

fn claim_from_row(row: &Row<'_>) -> rusqlite::Result<OpenCommerceAdapterHandoffClaim> {
    Ok(OpenCommerceAdapterHandoffClaim {
        schema: ADAPTER_HANDOFF_CLAIM_SCHEMA,
        id: row.get(0)?,
        project_id: row.get(1)?,
        merchant_id: row.get(2)?,
        invocation_id: row.get(3)?,
        integration_id: row.get(4)?,
        adapter_credential_id: row.get(5)?,
        adapter_credential_version: row.get(6)?,
        attempt_no: row.get(7)?,
        status: row.get(8)?,
        lease_token_hint: row.get(9)?,
        lease_expires_at: row.get(10)?,
        lease_deadline_at: row.get(11)?,
        release_reason_code: row.get(12)?,
        released_at: row.get(13)?,
        completion_status: row.get(14)?,
        retry_not_before: row.get(15)?,
        retry_suspended_at: row.get(16)?,
        retry_suspension_reason: row.get(17)?,
        retry_resumed_at: row.get(18)?,
        completed_receipt_id: row.get(19)?,
        created_at: row.get(20)?,
        updated_at: row.get(21)?,
    })
}

const CLAIM_SELECT: &str =
    "SELECT claim.id, claim.project_id, claim.merchant_id, claim.invocation_id,
            claim.integration_id, claim.adapter_credential_id,
            claim.adapter_credential_version, claim.attempt_no,
            CASE WHEN claim.status='expired' AND claim.released_at IS NOT NULL
                 THEN 'released'
                 WHEN claim.status='active'
                       AND julianday(claim.lease_expires_at) <= julianday('now')
                 THEN 'expired' ELSE claim.status END AS effective_status,
            claim.lease_token_hint, claim.lease_expires_at, claim.lease_deadline_at,
            claim.release_reason_code, claim.released_at,
            claim.completion_status, claim.retry_not_before,
            claim.retry_suspended_at, claim.retry_suspension_reason,
            claim.retry_resumed_at, claim.completed_receipt_id,
            claim.created_at, claim.updated_at
       FROM open_commerce_business_handoff_claims claim";
