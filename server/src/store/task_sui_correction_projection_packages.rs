use anyhow::{anyhow, bail, Result};
use rusqlite::{params, OptionalExtension, Row};

use crate::task_settlement::model::{
    SUI_INTEGRITY_CONFLICT, SUI_INTEGRITY_VERIFIED, SUI_NETWORK_NOT_SUBMITTED,
};
use crate::task_settlement::sui_correction_model::{
    CreateSuiCorrectionProjectionPackage, SuiCorrectionProjectionEnvelope,
    SuiCorrectionProjectionPackage,
};

use super::{new_id, now, Store};

impl Store {
    pub(crate) fn create_task_sui_correction_projection_package(
        &self,
        input: CreateSuiCorrectionProjectionPackage<'_>,
    ) -> Result<SuiCorrectionProjectionPackage> {
        let timestamp = now();
        let id = new_id("sui_correction_projection");
        let conn = self.conn()?;
        conn.execute(
            "INSERT OR IGNORE INTO task_sui_correction_projection_packages (
               id, project_id, correction_id, reversal_receipt_id,
               replacement_receipt_id, target_network, package_schema,
               projection_digest, source_bundle_digest, envelope_json,
               integrity_status, network_submission, submission_attempts,
               last_error, created_by_user_id, verified_at, created_at, updated_at
             ) VALUES (
               ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10,
               'verified', 'not_submitted', 0, NULL, ?11, ?12, ?12, ?12
             )",
            params![
                id,
                input.project_id.trim(),
                input.correction_id.trim(),
                input.reversal_receipt_id.trim(),
                input.replacement_receipt_id.trim(),
                input.target_network.trim(),
                input.package_schema.trim(),
                input.projection_digest.trim(),
                input.source_bundle_digest.trim(),
                input.envelope_json,
                input.created_by_user_id.trim(),
                timestamp,
            ],
        )?;
        let package = select_by_key(
            &conn,
            input.project_id,
            input.correction_id,
            input.target_network,
            input.package_schema,
        )?
        .ok_or_else(|| anyhow!("Sui 纠正投影包写入后无法读取"))?;
        if package.reversal_receipt_id != input.reversal_receipt_id.trim()
            || package.replacement_receipt_id != input.replacement_receipt_id.trim()
            || package.projection_digest != input.projection_digest.trim()
            || package.source_bundle_digest != input.source_bundle_digest.trim()
            || serde_json::to_string(&package.envelope)? != input.envelope_json
        {
            bail!("同一 Sui 纠正投影包幂等键对应的内容发生冲突");
        }
        Ok(package)
    }

    pub(crate) fn list_task_sui_correction_projection_packages(
        &self,
        project_id: &str,
        limit: usize,
    ) -> Result<Vec<SuiCorrectionProjectionPackage>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(&format!(
            "{} WHERE project_id=?1 ORDER BY created_at DESC LIMIT ?2",
            projection_select()
        ))?;
        let rows = stmt.query_map(
            params![project_id.trim(), limit.clamp(1, 500) as i64],
            read_projection,
        )?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    pub(crate) fn task_sui_correction_projection_package(
        &self,
        project_id: &str,
        projection_id: &str,
    ) -> Result<Option<SuiCorrectionProjectionPackage>> {
        let conn = self.conn()?;
        conn.query_row(
            &format!("{} WHERE project_id=?1 AND id=?2", projection_select()),
            params![project_id.trim(), projection_id.trim()],
            read_projection,
        )
        .optional()
        .map_err(Into::into)
    }

    pub(crate) fn update_task_sui_correction_projection_integrity(
        &self,
        project_id: &str,
        projection_id: &str,
        integrity_status: &str,
        last_error: Option<&str>,
    ) -> Result<SuiCorrectionProjectionPackage> {
        let integrity_status = match integrity_status.trim() {
            SUI_INTEGRITY_VERIFIED => SUI_INTEGRITY_VERIFIED,
            SUI_INTEGRITY_CONFLICT => SUI_INTEGRITY_CONFLICT,
            _ => bail!("未知 Sui 纠正投影完整性状态"),
        };
        let timestamp = now();
        let conn = self.conn()?;
        let changed = conn.execute(
            "UPDATE task_sui_correction_projection_packages
                SET integrity_status=?3, last_error=?4,
                    verified_at=?5, updated_at=?5
              WHERE project_id=?1 AND id=?2 AND network_submission=?6",
            params![
                project_id.trim(),
                projection_id.trim(),
                integrity_status,
                clean_error(last_error),
                timestamp,
                SUI_NETWORK_NOT_SUBMITTED,
            ],
        )?;
        if changed == 0 {
            bail!("Sui 纠正投影包不存在，或已进入网络提交生命周期，不能由链下复核改写");
        }
        drop(conn);
        self.task_sui_correction_projection_package(project_id, projection_id)?
            .ok_or_else(|| anyhow!("Sui 纠正投影包复核后无法读取"))
    }
}

fn projection_select() -> &'static str {
    "SELECT id, project_id, correction_id, reversal_receipt_id,
            replacement_receipt_id, target_network, package_schema,
            projection_digest, source_bundle_digest, envelope_json,
            integrity_status, network_submission, submission_attempts,
            last_error, created_by_user_id, verified_at, created_at, updated_at
       FROM task_sui_correction_projection_packages"
}

fn select_by_key(
    conn: &rusqlite::Connection,
    project_id: &str,
    correction_id: &str,
    target_network: &str,
    package_schema: &str,
) -> Result<Option<SuiCorrectionProjectionPackage>> {
    conn.query_row(
        &format!(
            "{} WHERE project_id=?1 AND correction_id=?2
                AND target_network=?3 AND package_schema=?4",
            projection_select()
        ),
        params![
            project_id.trim(),
            correction_id.trim(),
            target_network.trim(),
            package_schema.trim(),
        ],
        read_projection,
    )
    .optional()
    .map_err(Into::into)
}

fn read_projection(row: &Row<'_>) -> rusqlite::Result<SuiCorrectionProjectionPackage> {
    let envelope_json: String = row.get(9)?;
    let envelope = serde_json::from_str::<SuiCorrectionProjectionEnvelope>(&envelope_json)
        .map_err(|error| {
            rusqlite::Error::FromSqlConversionFailure(
                9,
                rusqlite::types::Type::Text,
                Box::new(error),
            )
        })?;
    let integrity_status: String = row.get(10)?;
    Ok(SuiCorrectionProjectionPackage {
        id: row.get(0)?,
        project_id: row.get(1)?,
        correction_id: row.get(2)?,
        reversal_receipt_id: row.get(3)?,
        replacement_receipt_id: row.get(4)?,
        target_network: row.get(5)?,
        package_schema: row.get(6)?,
        projection_digest: row.get(7)?,
        source_bundle_digest: row.get(8)?,
        envelope,
        submission_readiness: if integrity_status == SUI_INTEGRITY_VERIFIED {
            "adapter_required".to_string()
        } else {
            "integrity_conflict".to_string()
        },
        integrity_status,
        network_submission: row.get(11)?,
        submission_attempts: row.get(12)?,
        last_error: row.get(13)?,
        created_by_user_id: row.get(14)?,
        verified_at: row.get(15)?,
        created_at: row.get(16)?,
        updated_at: row.get(17)?,
    })
}

fn clean_error(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.chars().take(512).collect())
}
