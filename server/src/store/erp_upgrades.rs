use anyhow::{anyhow, bail, Result};
use rusqlite::{params, OptionalExtension, Row};

use crate::erp_blueprint::model::{
    ErpCompatibilityReport, ErpExtensionRef, ErpInstanceConfiguration, ErpUpgradeAdoptionEvidence,
    ErpUpgradeCampaign,
};

use super::{new_id, now, Store};

impl Store {
    pub(crate) fn create_erp_upgrade_campaign(
        &self,
        instance_id: &str,
        from_version_id: &str,
        target_version_id: &str,
        report: &ErpCompatibilityReport,
        instance_revision: i64,
        from_configuration: &ErpInstanceConfiguration,
        target_configuration: &ErpInstanceConfiguration,
        private_extensions: &[ErpExtensionRef],
        actor_user_id: &str,
    ) -> Result<ErpUpgradeCampaign> {
        if let Some(existing) = self.find_erp_upgrade_campaign(
            instance_id,
            from_version_id,
            target_version_id,
            instance_revision,
        )? {
            return Ok(existing);
        }
        let id = new_id("erp_upgrade");
        let timestamp = now();
        let status = if report.compatible {
            "ready"
        } else {
            "blocked"
        };
        self.conn()?.execute(
            "INSERT INTO erp_upgrade_campaigns (
               id, instance_id, from_version_id, target_version_id, status,
               compatibility_json, private_extensions_snapshot_json, created_by,
               instance_revision, from_configuration_json, target_configuration_json,
               created_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?12)",
            params![
                id,
                instance_id.trim(),
                from_version_id.trim(),
                target_version_id.trim(),
                status,
                serde_json::to_string(report)?,
                serde_json::to_string(private_extensions)?,
                actor_user_id.trim(),
                instance_revision,
                serde_json::to_string(from_configuration)?,
                serde_json::to_string(target_configuration)?,
                timestamp,
            ],
        )?;
        self.erp_upgrade_campaign(&id)
    }

    fn find_erp_upgrade_campaign(
        &self,
        instance_id: &str,
        from_version_id: &str,
        target_version_id: &str,
        instance_revision: i64,
    ) -> Result<Option<ErpUpgradeCampaign>> {
        self.conn()?
            .query_row(
                &format!(
                    "{UPGRADE_SELECT}
                     WHERE instance_id=?1 AND from_version_id=?2 AND target_version_id=?3
                       AND instance_revision=?4 AND status IN ('ready', 'blocked')
                     ORDER BY created_at DESC LIMIT 1"
                ),
                params![
                    instance_id.trim(),
                    from_version_id.trim(),
                    target_version_id.trim(),
                    instance_revision,
                ],
                upgrade_from_row,
            )
            .optional()
            .map_err(Into::into)
    }

    pub(crate) fn erp_upgrade_campaign(&self, campaign_id: &str) -> Result<ErpUpgradeCampaign> {
        self.conn()?
            .query_row(
                &format!("{UPGRADE_SELECT} WHERE id=?1"),
                params![campaign_id.trim()],
                upgrade_from_row,
            )
            .map_err(|error| anyhow!(error).context("ERP 升级活动不存在"))
    }

    pub(crate) fn list_erp_upgrade_campaigns_for_instance(
        &self,
        instance_id: &str,
    ) -> Result<Vec<ErpUpgradeCampaign>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(&format!(
            "{UPGRADE_SELECT} WHERE instance_id=?1 ORDER BY created_at DESC"
        ))?;
        let result = stmt
            .query_map(params![instance_id.trim()], upgrade_from_row)?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into);
        result
    }

    pub(crate) fn decide_erp_upgrade_campaign(
        &self,
        campaign_id: &str,
        action: &str,
        reason: &str,
        actor_user_id: &str,
        adoption_evidence: Option<&ErpUpgradeAdoptionEvidence>,
    ) -> Result<ErpUpgradeCampaign> {
        let campaign = self.erp_upgrade_campaign(campaign_id)?;
        let (next, expected_version, next_version, expected_revision, next_configuration) =
            match action {
                "adopt" if campaign.status == "ready" => (
                    "adopted",
                    campaign.from_version_id.as_str(),
                    campaign.target_version_id.as_str(),
                    campaign.instance_revision,
                    &campaign.target_configuration,
                ),
                "rollback" if campaign.status == "adopted" => (
                    "rolled_back",
                    campaign.target_version_id.as_str(),
                    campaign.from_version_id.as_str(),
                    campaign
                        .adopted_instance_revision
                        .ok_or_else(|| anyhow!("升级活动缺少采用后的实例修订号"))?,
                    &campaign.from_configuration,
                ),
                "adopt" => bail!("只有兼容检查通过的 ready 活动可以采用"),
                "rollback" => bail!("只有已采用的升级可以回滚"),
                _ => bail!("升级操作只能是 adopt 或 rollback"),
            };
        let mut conn = self.conn()?;
        let tx = conn.transaction()?;
        let (
            current_theme,
            current_modules_raw,
            current_plugins_raw,
            current_private_raw,
            current_revision,
        ): (String, String, String, String, i64) = tx.query_row(
            "SELECT theme_key, enabled_modules_json, plugins_json,
                    private_extensions_json, configuration_revision
               FROM erp_instances WHERE id=?1",
            params![campaign.instance_id],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ))
            },
        )?;
        let current_private: Vec<ErpExtensionRef> = serde_json::from_str(&current_private_raw)?;
        if current_private != campaign.private_extensions_snapshot {
            bail!("私有扩展清单已变化，必须先人工核对再继续升级操作");
        }
        let current_configuration = ErpInstanceConfiguration {
            theme_key: current_theme,
            enabled_modules: serde_json::from_str(&current_modules_raw)?,
            plugins: serde_json::from_str(&current_plugins_raw)?,
        };
        let expected_configuration = if action == "adopt" {
            &campaign.from_configuration
        } else {
            &campaign.target_configuration
        };
        if current_revision != expected_revision || &current_configuration != expected_configuration
        {
            bail!("实例配置已在兼容检查后变化，必须重新检查后再操作");
        }
        let updated_instance = tx.execute(
            "UPDATE erp_instances
                SET pinned_version_id=?1, theme_key=?2, enabled_modules_json=?3,
                    plugins_json=?4, configuration_revision=configuration_revision+1,
                    updated_at=?5
              WHERE id=?6 AND pinned_version_id=?7 AND configuration_revision=?8
                AND status='active'",
            params![
                next_version,
                next_configuration.theme_key,
                serde_json::to_string(&next_configuration.enabled_modules)?,
                serde_json::to_string(&next_configuration.plugins)?,
                now(),
                campaign.instance_id,
                expected_version,
                expected_revision,
            ],
        )?;
        if updated_instance == 0 {
            bail!("实例版本已变化或实例不可升级");
        }
        let updated_campaign = tx.execute(
            "UPDATE erp_upgrade_campaigns
             SET status=?1, decided_by=?2, rollback_reason=?3, updated_at=?4
                 , adopted_instance_revision=?5, adoption_evidence_json=?6
             WHERE id=?7 AND status=?8",
            params![
                next,
                actor_user_id.trim(),
                if next == "rolled_back" {
                    Some(reason.trim())
                } else {
                    None
                },
                now(),
                if next == "adopted" {
                    Some(expected_revision + 1)
                } else {
                    campaign.adopted_instance_revision
                },
                if next == "adopted" {
                    adoption_evidence.map(serde_json::to_string).transpose()?
                } else {
                    campaign
                        .adoption_evidence
                        .as_ref()
                        .map(serde_json::to_string)
                        .transpose()?
                },
                campaign_id.trim(),
                campaign.status,
            ],
        )?;
        if updated_campaign == 0 {
            bail!("升级活动状态已变化");
        }
        tx.commit()?;
        drop(conn);
        self.erp_upgrade_campaign(campaign_id)
    }
}

const UPGRADE_SELECT: &str = "SELECT id, instance_id, from_version_id,
 target_version_id, status, compatibility_json, private_extensions_snapshot_json,
 instance_revision, adopted_instance_revision, from_configuration_json,
 target_configuration_json, adoption_evidence_json, created_by, decided_by,
 rollback_reason, created_at, updated_at FROM erp_upgrade_campaigns";

fn upgrade_from_row(row: &Row<'_>) -> rusqlite::Result<ErpUpgradeCampaign> {
    Ok(ErpUpgradeCampaign {
        id: row.get(0)?,
        instance_id: row.get(1)?,
        from_version_id: row.get(2)?,
        target_version_id: row.get(3)?,
        status: row.get(4)?,
        compatibility: decode(row, 5)?,
        private_extensions_snapshot: decode(row, 6)?,
        instance_revision: row.get(7)?,
        adopted_instance_revision: row.get(8)?,
        from_configuration: decode(row, 9)?,
        target_configuration: decode(row, 10)?,
        adoption_evidence: decode_optional(row, 11)?,
        created_by: row.get(12)?,
        decided_by: row.get(13)?,
        rollback_reason: row.get(14)?,
        created_at: row.get(15)?,
        updated_at: row.get(16)?,
    })
}

fn decode_optional<T: serde::de::DeserializeOwned>(
    row: &Row<'_>,
    index: usize,
) -> rusqlite::Result<Option<T>> {
    let raw: Option<String> = row.get(index)?;
    raw.map(|value| {
        serde_json::from_str(&value).map_err(|error| {
            rusqlite::Error::FromSqlConversionFailure(
                value.len(),
                rusqlite::types::Type::Text,
                Box::new(error),
            )
        })
    })
    .transpose()
}

fn decode<T: serde::de::DeserializeOwned>(row: &Row<'_>, index: usize) -> rusqlite::Result<T> {
    let raw: String = row.get(index)?;
    serde_json::from_str(&raw).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(
            raw.len(),
            rusqlite::types::Type::Text,
            Box::new(error),
        )
    })
}
