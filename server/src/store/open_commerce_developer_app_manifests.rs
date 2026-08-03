use anyhow::{anyhow, bail, Result};
use rusqlite::{params, OptionalExtension};

use crate::open_commerce_developer_model::OpenCommerceDeveloperApp;

use super::{now, Store};

impl Store {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn update_open_commerce_developer_app_manifest(
        &self,
        project_id: &str,
        app_record_id: &str,
        expected_revision: i64,
        homepage_url: Option<&str>,
        privacy_policy_url: Option<&str>,
        terms_url: Option<&str>,
        support_email: Option<&str>,
        requested_scopes: &[String],
    ) -> Result<OpenCommerceDeveloperApp> {
        let scopes_json = serde_json::to_string(requested_scopes)?;
        let timestamp = now();
        let mut conn = self.conn()?;
        let tx = conn.transaction()?;
        let changed = tx.execute(
            "UPDATE open_commerce_developer_apps
                SET homepage_url=?1, privacy_policy_url=?2, terms_url=?3,
                    support_email=?4, requested_scopes_json=?5,
                    manifest_status='draft', manifest_revision=manifest_revision+1,
                    submitted_at=NULL, reviewed_at=NULL,
                    reviewed_by_user_id=NULL, review_note=NULL,
                    domain_verification_status='pending',
                    domain_verification_host=NULL,
                    domain_verification_revision=NULL,
                    domain_verification_challenge_hash=NULL,
                    domain_verification_expires_at=NULL,
                    domain_verification_attempted_at=NULL,
                    domain_verified_at=NULL,
                    domain_verification_error_code=NULL, updated_at=?6
              WHERE project_id=?7 AND id=?8 AND manifest_revision=?9
                AND status='active'",
            params![
                homepage_url,
                privacy_policy_url,
                terms_url,
                support_email,
                scopes_json,
                timestamp,
                project_id.trim(),
                app_record_id.trim(),
                expected_revision,
            ],
        )?;
        if changed != 1 {
            bail!("App 资料已变化或应用已停用，请刷新后重试");
        }
        tx.execute(
            "UPDATE open_commerce_developer_app_admissions
                SET status='changes_requested', reviewed_at=?1,
                    reviewed_by_user_id=NULL,
                    review_note='manifest_revision_changed', risk_tier=NULL,
                    suspended_at=NULL, updated_at=?1
              WHERE app_record_id=?2 AND status IN ('submitted', 'approved')",
            params![timestamp, app_record_id.trim()],
        )?;
        super::open_commerce_developer_credentials::revoke_active_production_credentials(
            &tx,
            app_record_id,
            "manifest_revision_changed",
            &timestamp,
        )?;
        tx.commit()?;
        drop(conn);
        self.open_commerce_developer_app_for_project(project_id, app_record_id)
    }

    pub(crate) fn submit_open_commerce_developer_app_manifest(
        &self,
        project_id: &str,
        app_record_id: &str,
        expected_revision: i64,
    ) -> Result<OpenCommerceDeveloperApp> {
        let timestamp = now();
        let changed = self.conn()?.execute(
            "UPDATE open_commerce_developer_apps
                SET manifest_status='submitted', submitted_at=?1,
                    reviewed_at=NULL, reviewed_by_user_id=NULL,
                    review_note=NULL, updated_at=?1
              WHERE project_id=?2 AND id=?3 AND manifest_revision=?4
                AND status='active'
                AND manifest_status IN ('draft', 'changes_requested')",
            params![
                timestamp,
                project_id.trim(),
                app_record_id.trim(),
                expected_revision,
            ],
        )?;
        if changed != 1 {
            bail!("App 资料已变化、已提交或应用已停用，请刷新后重试");
        }
        self.open_commerce_developer_app_for_project(project_id, app_record_id)
    }

    pub(crate) fn review_open_commerce_developer_app_manifest(
        &self,
        app_record_id: &str,
        expected_revision: i64,
        reviewer_user_id: &str,
        decision: &str,
        note: Option<&str>,
    ) -> Result<OpenCommerceDeveloperApp> {
        let timestamp = now();
        let changed = self.conn()?.execute(
            "UPDATE open_commerce_developer_apps
                SET manifest_status=?1, reviewed_at=?2,
                    reviewed_by_user_id=?3, review_note=?4, updated_at=?2
              WHERE id=?5 AND manifest_revision=?6 AND status='active'
                AND manifest_status='submitted'",
            params![
                decision,
                timestamp,
                reviewer_user_id.trim(),
                note,
                app_record_id.trim(),
                expected_revision,
            ],
        )?;
        if changed != 1 {
            bail!("待审资料已变化、已被处理或应用已停用，请刷新后重试");
        }
        self.open_commerce_developer_app_by_record_id(app_record_id)
    }

    pub(crate) fn list_submitted_open_commerce_developer_app_manifests(
        &self,
        limit: usize,
    ) -> Result<Vec<OpenCommerceDeveloperApp>> {
        let limit = limit.clamp(1, 100) as i64;
        let conn = self.conn()?;
        let mut stmt = conn.prepare(&format!(
            "{} WHERE manifest_status='submitted' ORDER BY submitted_at ASC LIMIT ?1",
            super::open_commerce_developer_apps::APP_SELECT
        ))?;
        let apps = stmt
            .query_map(
                params![limit],
                super::open_commerce_developer_apps::app_from_row,
            )?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(anyhow::Error::from)?;
        Ok(apps)
    }

    pub(crate) fn open_commerce_developer_app_by_record_id(
        &self,
        app_record_id: &str,
    ) -> Result<OpenCommerceDeveloperApp> {
        self.conn()?
            .query_row(
                &format!(
                    "{} WHERE id=?1",
                    super::open_commerce_developer_apps::APP_SELECT
                ),
                params![app_record_id.trim()],
                super::open_commerce_developer_apps::app_from_row,
            )
            .optional()?
            .ok_or_else(|| anyhow!("开发者应用不存在"))
    }
}
