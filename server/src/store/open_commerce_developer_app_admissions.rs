//! Persistence and guarded transitions for developer-App admission review.

use anyhow::{anyhow, bail, Result};
use rusqlite::{params, OptionalExtension, Row};

use crate::open_commerce_developer_admission_model::DeveloperAppAdmission;

use super::{new_id, now, Store};

impl Store {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn submit_open_commerce_developer_app_admission(
        &self,
        project_id: &str,
        app_record_id: &str,
        manifest_revision: i64,
        organization_name: &str,
        jurisdiction: &str,
        registration_id: &str,
        attested_at: &str,
    ) -> Result<DeveloperAppAdmission> {
        let mut conn = self.conn()?;
        let tx = conn.transaction()?;
        let eligible: bool = tx.query_row(
            "SELECT EXISTS(
                SELECT 1 FROM open_commerce_developer_apps
                 WHERE id=?1 AND project_id=?2 AND status='active'
                   AND manifest_status='approved' AND manifest_revision=?3
                   AND domain_verification_status='verified'
                   AND domain_verification_revision=?3
             )",
            params![app_record_id.trim(), project_id.trim(), manifest_revision],
            |row| row.get(0),
        )?;
        if !eligible {
            bail!("App 当前资料、域名证明或审核状态不满足准入申请条件");
        }
        let current = tx
            .query_row(
                &format!("{ADMISSION_SELECT} WHERE app_record_id=?1"),
                params![app_record_id.trim()],
                admission_from_row,
            )
            .optional()?;
        if current.as_ref().is_some_and(|value| {
            value.manifest_revision == manifest_revision
                && matches!(value.status.as_str(), "submitted" | "approved")
        }) {
            bail!("当前资料修订已提交准入审查或已经获准");
        }
        let id = current
            .as_ref()
            .map(|value| value.id.clone())
            .unwrap_or_else(|| new_id("admission"));
        let created_at = current
            .as_ref()
            .map(|value| value.created_at.clone())
            .unwrap_or_else(now);
        let timestamp = now();
        tx.execute(
            "INSERT INTO open_commerce_developer_app_admissions (
                id, app_record_id, project_id, manifest_revision,
                organization_name, jurisdiction, registration_id, attested_at,
                status, requested_at, reviewed_at, reviewed_by_user_id,
                review_note, risk_tier, suspended_at, created_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8,
                       'submitted', ?9, NULL, NULL, NULL, NULL, NULL, ?10, ?9)
             ON CONFLICT(app_record_id) DO UPDATE SET
                project_id=excluded.project_id,
                manifest_revision=excluded.manifest_revision,
                organization_name=excluded.organization_name,
                jurisdiction=excluded.jurisdiction,
                registration_id=excluded.registration_id,
                attested_at=excluded.attested_at,
                status='submitted', requested_at=excluded.requested_at,
                reviewed_at=NULL, reviewed_by_user_id=NULL, review_note=NULL,
                risk_tier=NULL, suspended_at=NULL, updated_at=excluded.updated_at",
            params![
                id,
                app_record_id.trim(),
                project_id.trim(),
                manifest_revision,
                organization_name,
                jurisdiction,
                registration_id,
                attested_at,
                timestamp,
                created_at,
            ],
        )?;
        tx.commit()?;
        drop(conn);
        self.open_commerce_developer_app_admission(app_record_id)?
            .ok_or_else(|| anyhow!("准入申请保存后不可读取"))
    }

    pub(crate) fn open_commerce_developer_app_admission(
        &self,
        app_record_id: &str,
    ) -> Result<Option<DeveloperAppAdmission>> {
        self.conn()?
            .query_row(
                &format!("{ADMISSION_SELECT} WHERE app_record_id=?1"),
                params![app_record_id.trim()],
                admission_from_row,
            )
            .optional()
            .map_err(Into::into)
    }

    pub(crate) fn list_reviewable_open_commerce_developer_app_admissions(
        &self,
        limit: usize,
    ) -> Result<Vec<DeveloperAppAdmission>> {
        let limit = limit.clamp(1, 100) as i64;
        let conn = self.conn()?;
        let mut stmt = conn.prepare(&format!(
            "{ADMISSION_SELECT} WHERE status IN ('submitted', 'approved')
               AND EXISTS (
                   SELECT 1 FROM open_commerce_developer_apps app
                    WHERE app.id=open_commerce_developer_app_admissions.app_record_id
                      AND app.status='active'
                      AND app.manifest_status='approved'
                      AND app.manifest_revision=open_commerce_developer_app_admissions.manifest_revision
                      AND app.domain_verification_status='verified'
                      AND app.domain_verification_revision=app.manifest_revision
               )
             ORDER BY CASE status WHEN 'submitted' THEN 0 ELSE 1 END,
                      requested_at ASC LIMIT ?1"
        ))?;
        stmt.query_map(params![limit], admission_from_row)?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    pub(crate) fn review_open_commerce_developer_app_admission(
        &self,
        app_record_id: &str,
        manifest_revision: i64,
        reviewer_user_id: &str,
        decision: &str,
        risk_tier: Option<&str>,
        note: Option<&str>,
    ) -> Result<DeveloperAppAdmission> {
        let timestamp = now();
        let expected_status = if decision == "suspended" {
            "approved"
        } else {
            "submitted"
        };
        let changed = self.conn()?.execute(
            "UPDATE open_commerce_developer_app_admissions
                SET status=?1, reviewed_at=?2, reviewed_by_user_id=?3,
                    review_note=?4,
                    risk_tier=CASE WHEN ?1='approved' THEN ?5
                                   WHEN ?1='changes_requested' THEN NULL
                                   ELSE risk_tier END,
                    suspended_at=CASE WHEN ?1='suspended' THEN ?2 ELSE NULL END,
                    updated_at=?2
              WHERE app_record_id=?6 AND manifest_revision=?7 AND status=?8
                AND EXISTS (
                    SELECT 1 FROM open_commerce_developer_apps app
                     WHERE app.id=open_commerce_developer_app_admissions.app_record_id
                       AND app.status='active'
                       AND app.manifest_status='approved'
                       AND app.manifest_revision=?7
                       AND app.domain_verification_status='verified'
                       AND app.domain_verification_revision=?7
                )",
            params![
                decision,
                timestamp,
                reviewer_user_id.trim(),
                note,
                risk_tier,
                app_record_id.trim(),
                manifest_revision,
                expected_status,
            ],
        )?;
        if changed != 1 {
            bail!("准入申请已变化、已被处理或不再满足当前资料条件");
        }
        self.open_commerce_developer_app_admission(app_record_id)?
            .ok_or_else(|| anyhow!("准入审核结果不可读取"))
    }
}

fn admission_from_row(row: &Row<'_>) -> rusqlite::Result<DeveloperAppAdmission> {
    Ok(DeveloperAppAdmission {
        schema: "open_commerce.developer_app_admission.v1",
        id: row.get(0)?,
        app_record_id: row.get(1)?,
        project_id: row.get(2)?,
        manifest_revision: row.get(3)?,
        organization_name: row.get(4)?,
        jurisdiction: row.get(5)?,
        registration_id: row.get(6)?,
        attested_at: row.get(7)?,
        status: row.get(8)?,
        requested_at: row.get(9)?,
        reviewed_at: row.get(10)?,
        reviewed_by_user_id: row.get(11)?,
        review_note: row.get(12)?,
        risk_tier: row.get(13)?,
        suspended_at: row.get(14)?,
        production_credential_issued: false,
        network_access_enabled: false,
        created_at: row.get(15)?,
        updated_at: row.get(16)?,
    })
}

const ADMISSION_SELECT: &str = "SELECT id, app_record_id, project_id, manifest_revision,
            organization_name, jurisdiction, registration_id, attested_at,
            status, requested_at, reviewed_at, reviewed_by_user_id,
            review_note, risk_tier, suspended_at, created_at, updated_at
       FROM open_commerce_developer_app_admissions";
