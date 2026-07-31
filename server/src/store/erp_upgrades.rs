use anyhow::{anyhow, bail, Result};
use rusqlite::{params, Row};

use crate::erp_blueprint::model::{ErpCompatibilityReport, ErpExtensionRef, ErpUpgradeCampaign};

use super::{new_id, now, Store};

impl Store {
    pub(crate) fn create_erp_upgrade_campaign(
        &self,
        instance_id: &str,
        from_version_id: &str,
        target_version_id: &str,
        report: &ErpCompatibilityReport,
        private_extensions: &[ErpExtensionRef],
        actor_user_id: &str,
    ) -> Result<ErpUpgradeCampaign> {
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
               created_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9)",
            params![
                id,
                instance_id.trim(),
                from_version_id.trim(),
                target_version_id.trim(),
                status,
                serde_json::to_string(report)?,
                serde_json::to_string(private_extensions)?,
                actor_user_id.trim(),
                timestamp,
            ],
        )?;
        self.erp_upgrade_campaign(&id)
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
    ) -> Result<ErpUpgradeCampaign> {
        let campaign = self.erp_upgrade_campaign(campaign_id)?;
        let (next, expected_version, next_version) = match action {
            "adopt" if campaign.status == "ready" => (
                "adopted",
                campaign.from_version_id.as_str(),
                campaign.target_version_id.as_str(),
            ),
            "rollback" if campaign.status == "adopted" => (
                "rolled_back",
                campaign.target_version_id.as_str(),
                campaign.from_version_id.as_str(),
            ),
            "adopt" => bail!("只有兼容检查通过的 ready 活动可以采用"),
            "rollback" => bail!("只有已采用的升级可以回滚"),
            _ => bail!("升级操作只能是 adopt 或 rollback"),
        };
        let mut conn = self.conn()?;
        let tx = conn.transaction()?;
        let current_private_raw: String = tx.query_row(
            "SELECT private_extensions_json FROM erp_instances WHERE id=?1",
            params![campaign.instance_id],
            |row| row.get(0),
        )?;
        let current_private: Vec<ErpExtensionRef> = serde_json::from_str(&current_private_raw)?;
        if current_private != campaign.private_extensions_snapshot {
            bail!("私有扩展清单已变化，必须先人工核对再继续升级操作");
        }
        let updated_instance = tx.execute(
            "UPDATE erp_instances SET pinned_version_id=?1, updated_at=?2
             WHERE id=?3 AND pinned_version_id=?4 AND status='active'",
            params![next_version, now(), campaign.instance_id, expected_version],
        )?;
        if updated_instance == 0 {
            bail!("实例版本已变化或实例不可升级");
        }
        let updated_campaign = tx.execute(
            "UPDATE erp_upgrade_campaigns
             SET status=?1, decided_by=?2, rollback_reason=?3, updated_at=?4
             WHERE id=?5 AND status=?6",
            params![
                next,
                actor_user_id.trim(),
                if next == "rolled_back" {
                    Some(reason.trim())
                } else {
                    None
                },
                now(),
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
 created_by, decided_by, rollback_reason, created_at, updated_at FROM erp_upgrade_campaigns";

fn upgrade_from_row(row: &Row<'_>) -> rusqlite::Result<ErpUpgradeCampaign> {
    Ok(ErpUpgradeCampaign {
        id: row.get(0)?,
        instance_id: row.get(1)?,
        from_version_id: row.get(2)?,
        target_version_id: row.get(3)?,
        status: row.get(4)?,
        compatibility: decode(row, 5)?,
        private_extensions_snapshot: decode(row, 6)?,
        created_by: row.get(7)?,
        decided_by: row.get(8)?,
        rollback_reason: row.get(9)?,
        created_at: row.get(10)?,
        updated_at: row.get(11)?,
    })
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
