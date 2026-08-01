use anyhow::{anyhow, bail, Result};
use rusqlite::{params, OptionalExtension, Row};

use crate::erp_blueprint::model::{
    ErpBlueprint, ErpBlueprintDefinition, ErpBlueprintVersion, ErpExtensionRef, ErpInstance,
    ErpReleaseManifest, UpdateErpInstanceRequest,
};

use super::{new_id, now, Store};

impl Store {
    pub(crate) fn create_erp_blueprint(
        &self,
        definition: ErpBlueprintDefinition,
        actor_user_id: &str,
    ) -> Result<ErpBlueprint> {
        let id = new_id("erp_blueprint");
        let timestamp = now();
        self.conn()?
            .execute(
                "INSERT INTO erp_blueprints (
                   id, blueprint_key, source_project_id, name, description,
                   proposal_threshold, definition_json, status, created_by, created_at, updated_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'active', ?8, ?9, ?9)",
                params![
                    id,
                    definition.blueprint_key,
                    definition.source_project_id,
                    definition.name,
                    definition.description,
                    definition.proposal_threshold,
                    serde_json::to_string(&definition)?,
                    actor_user_id.trim(),
                    timestamp,
                ],
            )
            .map_err(|error| anyhow!(error).context("该项目或 blueprint_key 已登记为 ERP 蓝图"))?;
        self.erp_blueprint(&id)
    }

    pub(crate) fn erp_blueprint(&self, blueprint_id: &str) -> Result<ErpBlueprint> {
        self.conn()?
            .query_row(
                &format!("{BLUEPRINT_SELECT} WHERE id=?1"),
                params![blueprint_id.trim()],
                blueprint_from_row,
            )
            .map_err(|error| anyhow!(error).context("ERP 蓝图不存在"))
    }

    pub(crate) fn erp_blueprint_for_project(
        &self,
        project_id: &str,
    ) -> Result<Option<ErpBlueprint>> {
        self.conn()?
            .query_row(
                &format!(
                    "{BLUEPRINT_SELECT}
                     WHERE source_project_id=?1 OR id=(SELECT blueprint_id FROM erp_instances WHERE project_id=?1)"
                ),
                params![project_id.trim()],
                blueprint_from_row,
            )
            .optional()
            .map_err(Into::into)
    }

    pub(crate) fn create_erp_blueprint_version(
        &self,
        blueprint_id: &str,
        manifest: &ErpReleaseManifest,
        manifest_sha256: &str,
        actor_user_id: &str,
    ) -> Result<ErpBlueprintVersion> {
        let id = new_id("erp_version");
        let timestamp = now();
        self.conn()?
            .execute(
                "INSERT INTO erp_blueprint_versions (
                   id, blueprint_id, version, manifest_json, manifest_sha256,
                   status, created_by, created_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, 'published', ?6, ?7)",
                params![
                    id,
                    blueprint_id.trim(),
                    manifest.version,
                    serde_json::to_string(manifest)?,
                    manifest_sha256,
                    actor_user_id.trim(),
                    timestamp,
                ],
            )
            .map_err(|error| anyhow!(error).context("该蓝图版本已存在；已发布清单不可覆盖"))?;
        self.erp_blueprint_version(&id)
    }

    pub(crate) fn update_erp_blueprint_definition(
        &self,
        blueprint_id: &str,
        expected_revision: i64,
        definition: &ErpBlueprintDefinition,
    ) -> Result<ErpBlueprint> {
        let updated = self.conn()?.execute(
            "UPDATE erp_blueprints
                SET name=?1, description=?2, proposal_threshold=?3,
                    definition_json=?4, definition_revision=definition_revision+1,
                    updated_at=?5
              WHERE id=?6 AND status='active' AND definition_revision=?7",
            params![
                definition.name,
                definition.description,
                definition.proposal_threshold,
                serde_json::to_string(definition)?,
                now(),
                blueprint_id.trim(),
                expected_revision,
            ],
        )?;
        if updated == 0 {
            bail!("蓝图已被其他维护者修改或已归档，请刷新后重试");
        }
        self.erp_blueprint(blueprint_id)
    }

    pub(crate) fn erp_blueprint_version(&self, version_id: &str) -> Result<ErpBlueprintVersion> {
        self.conn()?
            .query_row(
                &format!("{VERSION_SELECT} WHERE id=?1"),
                params![version_id.trim()],
                version_from_row,
            )
            .map_err(|error| anyhow!(error).context("ERP 蓝图版本不存在"))
    }

    pub(crate) fn erp_blueprint_version_by_name(
        &self,
        blueprint_id: &str,
        version: &str,
    ) -> Result<ErpBlueprintVersion> {
        self.conn()?
            .query_row(
                &format!(
                    "{VERSION_SELECT} WHERE blueprint_id=?1 AND version=?2 AND status='published'"
                ),
                params![blueprint_id.trim(), version.trim()],
                version_from_row,
            )
            .map_err(|error| anyhow!(error).context("当前蓝图没有该已发布版本"))
    }

    pub(crate) fn list_erp_blueprint_versions(
        &self,
        blueprint_id: &str,
    ) -> Result<Vec<ErpBlueprintVersion>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(&format!(
            "{VERSION_SELECT} WHERE blueprint_id=?1 ORDER BY created_at DESC"
        ))?;
        let mut result = stmt
            .query_map(params![blueprint_id.trim()], version_from_row)?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(anyhow::Error::from)?;
        result.sort_by(|left, right| {
            super::super::erp_blueprint::validation::version_cmp(
                &right.manifest.version,
                &left.manifest.version,
            )
        });
        Ok(result)
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn create_erp_instance(
        &self,
        instance_key: &str,
        project_id: &str,
        blueprint_id: &str,
        version_id: &str,
        industry: &str,
        theme_key: &str,
        enabled_modules: &[String],
        plugins: &[ErpExtensionRef],
        private_extensions: &[ErpExtensionRef],
        onboarding_mode: &str,
        actor_user_id: &str,
    ) -> Result<ErpInstance> {
        let id = new_id("erp_instance");
        let timestamp = now();
        self.conn()?
            .execute(
                "INSERT INTO erp_instances (
                   id, instance_key, project_id, blueprint_id, pinned_version_id,
                   industry, theme_key, enabled_modules_json, plugins_json,
                   private_extensions_json, onboarding_mode, status, created_by, created_at, updated_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, 'active', ?12, ?13, ?13)",
                params![
                    id,
                    instance_key.trim(),
                    project_id.trim(),
                    blueprint_id.trim(),
                    version_id.trim(),
                    industry.trim(),
                    theme_key.trim(),
                    serde_json::to_string(enabled_modules)?,
                    serde_json::to_string(plugins)?,
                    serde_json::to_string(private_extensions)?,
                    onboarding_mode.trim(),
                    actor_user_id.trim(),
                    timestamp,
                ],
            )
            .map_err(|error| anyhow!(error).context("实例标识或项目已绑定 ERP 实例"))?;
        self.erp_instance(&id)
    }

    pub(crate) fn erp_instance(&self, instance_id: &str) -> Result<ErpInstance> {
        self.conn()?
            .query_row(
                &format!("{INSTANCE_SELECT} WHERE i.id=?1"),
                params![instance_id.trim()],
                instance_from_row,
            )
            .map_err(|error| anyhow!(error).context("ERP 商户实例不存在"))
    }

    pub(crate) fn erp_instance_by_key(&self, instance_key: &str) -> Result<Option<ErpInstance>> {
        self.conn()?
            .query_row(
                &format!("{INSTANCE_SELECT} WHERE i.instance_key=?1"),
                params![instance_key.trim()],
                instance_from_row,
            )
            .optional()
            .map_err(Into::into)
    }

    pub(crate) fn erp_instance_for_project(&self, project_id: &str) -> Result<Option<ErpInstance>> {
        self.conn()?
            .query_row(
                &format!("{INSTANCE_SELECT} WHERE i.project_id=?1"),
                params![project_id.trim()],
                instance_from_row,
            )
            .optional()
            .map_err(Into::into)
    }

    pub(crate) fn list_erp_instances(&self, blueprint_id: &str) -> Result<Vec<ErpInstance>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(&format!(
            "{INSTANCE_SELECT} WHERE i.blueprint_id=?1 ORDER BY i.created_at DESC"
        ))?;
        let result = stmt
            .query_map(params![blueprint_id.trim()], instance_from_row)?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into);
        result
    }

    pub(crate) fn update_erp_instance_configuration(
        &self,
        instance_id: &str,
        request: &UpdateErpInstanceRequest,
    ) -> Result<ErpInstance> {
        let updated = self.conn()?.execute(
            "UPDATE erp_instances
                SET theme_key=?1, enabled_modules_json=?2, plugins_json=?3,
                    private_extensions_json=?4,
                    configuration_revision=configuration_revision+1, updated_at=?5
              WHERE id=?6 AND status='active' AND configuration_revision=?7",
            params![
                request.theme_key,
                serde_json::to_string(&request.enabled_modules)?,
                serde_json::to_string(&request.plugins)?,
                serde_json::to_string(&request.private_extensions)?,
                now(),
                instance_id.trim(),
                request.expected_revision,
            ],
        )?;
        if updated == 0 {
            bail!("实例配置已变化或实例已归档，请刷新后重试");
        }
        self.erp_instance(instance_id)
    }
}

const BLUEPRINT_SELECT: &str =
    "SELECT id, definition_json, definition_revision, status, created_by, created_at, updated_at FROM erp_blueprints";
const VERSION_SELECT: &str = "SELECT id, blueprint_id, manifest_json, manifest_sha256, status, created_by, created_at FROM erp_blueprint_versions";
const INSTANCE_SELECT: &str = "SELECT i.id, i.instance_key, i.project_id, i.blueprint_id,
 i.pinned_version_id, v.version, i.industry, i.theme_key, i.enabled_modules_json,
 i.plugins_json, i.private_extensions_json, i.configuration_revision, i.bootstrap_matter_id,
 i.onboarding_mode, i.status, i.created_by, i.created_at, i.updated_at
 FROM erp_instances i JOIN erp_blueprint_versions v ON v.id=i.pinned_version_id";

fn blueprint_from_row(row: &Row<'_>) -> rusqlite::Result<ErpBlueprint> {
    Ok(ErpBlueprint {
        id: row.get(0)?,
        definition: decode(row, 1)?,
        definition_revision: row.get(2)?,
        status: row.get(3)?,
        created_by: row.get(4)?,
        created_at: row.get(5)?,
        updated_at: row.get(6)?,
    })
}

fn version_from_row(row: &Row<'_>) -> rusqlite::Result<ErpBlueprintVersion> {
    Ok(ErpBlueprintVersion {
        id: row.get(0)?,
        blueprint_id: row.get(1)?,
        manifest: decode(row, 2)?,
        manifest_sha256: row.get(3)?,
        status: row.get(4)?,
        created_by: row.get(5)?,
        created_at: row.get(6)?,
    })
}

fn instance_from_row(row: &Row<'_>) -> rusqlite::Result<ErpInstance> {
    Ok(ErpInstance {
        id: row.get(0)?,
        instance_key: row.get(1)?,
        project_id: row.get(2)?,
        blueprint_id: row.get(3)?,
        pinned_version_id: row.get(4)?,
        pinned_version: row.get(5)?,
        industry: row.get(6)?,
        theme_key: row.get(7)?,
        enabled_modules: decode(row, 8)?,
        plugins: decode(row, 9)?,
        private_extensions: decode(row, 10)?,
        configuration_revision: row.get(11)?,
        bootstrap_matter_id: row.get(12)?,
        onboarding_mode: row.get(13)?,
        status: row.get(14)?,
        created_by: row.get(15)?,
        created_at: row.get(16)?,
        updated_at: row.get(17)?,
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
