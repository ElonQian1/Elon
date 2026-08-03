use anyhow::{anyhow, bail, Result};
use rusqlite::{params, OptionalExtension};

use crate::open_commerce_developer_model::{
    DeveloperAppDomainChallengeState, OpenCommerceDeveloperApp,
};

use super::{now, Store};

impl Store {
    pub(crate) fn issue_open_commerce_developer_app_domain_challenge(
        &self,
        project_id: &str,
        app_record_id: &str,
        expected_revision: i64,
        verification_host: &str,
        challenge_hash: &str,
        expires_at: &str,
    ) -> Result<OpenCommerceDeveloperApp> {
        let changed = self.conn()?.execute(
            "UPDATE open_commerce_developer_apps
                SET domain_verification_status='pending',
                    domain_verification_host=?1,
                    domain_verification_revision=manifest_revision,
                    domain_verification_challenge_hash=?2,
                    domain_verification_expires_at=?3,
                    domain_verification_attempted_at=NULL,
                    domain_verified_at=NULL,
                    domain_verification_error_code=NULL, updated_at=?4
              WHERE project_id=?5 AND id=?6 AND manifest_revision=?7
                AND status='active' AND homepage_url IS NOT NULL",
            params![
                verification_host.trim(),
                challenge_hash.trim(),
                expires_at.trim(),
                now(),
                project_id.trim(),
                app_record_id.trim(),
                expected_revision,
            ],
        )?;
        if changed != 1 {
            bail!("App 资料已变化、主页缺失或应用已停用，请刷新后重试");
        }
        self.open_commerce_developer_app_for_project(project_id, app_record_id)
    }

    pub(crate) fn open_commerce_developer_app_domain_challenge(
        &self,
        project_id: &str,
        app_record_id: &str,
    ) -> Result<DeveloperAppDomainChallengeState> {
        self.conn()?
            .query_row(
                "SELECT id, project_id, domain_verification_revision,
                        domain_verification_host,
                        domain_verification_challenge_hash,
                        domain_verification_expires_at,
                        domain_verification_status
                   FROM open_commerce_developer_apps
                  WHERE project_id=?1 AND id=?2",
                params![project_id.trim(), app_record_id.trim()],
                |row| {
                    Ok(DeveloperAppDomainChallengeState {
                        app_record_id: row.get(0)?,
                        project_id: row.get(1)?,
                        manifest_revision: row.get(2)?,
                        verification_host: row.get(3)?,
                        challenge_hash: row.get(4)?,
                        expires_at: row.get(5)?,
                        status: row.get(6)?,
                    })
                },
            )
            .optional()?
            .ok_or_else(|| anyhow!("域名验证 challenge 不存在或已失效"))
    }

    pub(crate) fn record_open_commerce_developer_app_domain_failure(
        &self,
        project_id: &str,
        app_record_id: &str,
        manifest_revision: i64,
        error_code: &str,
    ) -> Result<OpenCommerceDeveloperApp> {
        let changed = self.conn()?.execute(
            "UPDATE open_commerce_developer_apps
                SET domain_verification_status='failed',
                    domain_verification_attempted_at=?1,
                    domain_verification_error_code=?2, updated_at=?1
              WHERE project_id=?3 AND id=?4
                AND domain_verification_revision=?5
                AND domain_verification_challenge_hash IS NOT NULL",
            params![
                now(),
                error_code.trim(),
                project_id.trim(),
                app_record_id.trim(),
                manifest_revision,
            ],
        )?;
        if changed != 1 {
            bail!("域名验证 challenge 已变化或失效");
        }
        self.open_commerce_developer_app_for_project(project_id, app_record_id)
    }

    pub(crate) fn verify_open_commerce_developer_app_domain(
        &self,
        project_id: &str,
        app_record_id: &str,
        manifest_revision: i64,
    ) -> Result<OpenCommerceDeveloperApp> {
        let timestamp = now();
        let changed = self.conn()?.execute(
            "UPDATE open_commerce_developer_apps
                SET domain_verification_status='verified',
                    domain_verification_attempted_at=?1,
                    domain_verified_at=?1,
                    domain_verification_error_code=NULL,
                    domain_verification_challenge_hash=NULL, updated_at=?1
              WHERE project_id=?2 AND id=?3
                AND manifest_revision=?4
                AND domain_verification_revision=?4
                AND status='active'
                AND domain_verification_challenge_hash IS NOT NULL",
            params![
                timestamp,
                project_id.trim(),
                app_record_id.trim(),
                manifest_revision,
            ],
        )?;
        if changed != 1 {
            bail!("域名验证 challenge 已变化、应用已停用或资料修订已更新");
        }
        self.open_commerce_developer_app_for_project(project_id, app_record_id)
    }
}
