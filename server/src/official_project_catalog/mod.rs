use anyhow::{bail, Context, Result};
use chrono::Utc;
use rusqlite::{params, OptionalExtension};
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashSet;

use crate::{
    erp_blueprint::{
        model::{
            CreateBlueprintRequest, CreateBlueprintVersionRequest, ErpBlueprintDefinition,
            ErpReleaseManifest, BLUEPRINT_SCHEMA, RELEASE_SCHEMA,
        },
        service,
    },
    store::Store,
};

const CATALOG_JSON: &str = include_str!("catalog.json");
const CATALOG_SCHEMA: &str = "yilong.official_project_catalog.v1";

#[derive(Debug, Deserialize)]
struct OfficialProjectCatalog {
    schema: String,
    projects: Vec<OfficialProjectDefinition>,
}

#[derive(Debug, Deserialize)]
struct OfficialProjectDefinition {
    id: String,
    name: String,
    display_name: String,
    description: String,
    repo_url: String,
    branch: String,
    landing: Value,
    blueprint: Option<ErpBlueprintDefinition>,
    release: Option<ErpReleaseManifest>,
}

pub(crate) fn ensure(store: &Store) -> Result<bool> {
    let owner_user_id = {
        let conn = store.conn()?;
        conn.query_row(
            "SELECT created_by FROM projects WHERE id='elon-self' AND status != 'deleted'",
            [],
            |row| row.get::<_, String>(0),
        )
        .optional()?
    };
    let Some(owner_user_id) = owner_user_id else {
        tracing::warn!("一龙自身项目尚未初始化，暂不登记官方目录项目");
        return Ok(false);
    };

    let catalog = parse_catalog()?;
    for project in &catalog.projects {
        ensure_project(store, &owner_user_id, project)?;
        ensure_landing(store, &owner_user_id, project)?;
        if project.blueprint.is_some() {
            ensure_blueprint(store, &owner_user_id, project)?;
        }
    }
    Ok(true)
}

fn parse_catalog() -> Result<OfficialProjectCatalog> {
    let catalog: OfficialProjectCatalog =
        serde_json::from_str(CATALOG_JSON).context("官方项目目录 JSON 无效")?;
    if catalog.schema != CATALOG_SCHEMA {
        bail!("官方项目目录 schema 不受支持: {}", catalog.schema);
    }
    let mut ids = HashSet::new();
    for project in &catalog.projects {
        if project.id.trim().is_empty() || project.name.trim().is_empty() {
            bail!("官方项目目录包含空项目标识或名称");
        }
        if !ids.insert(project.id.as_str()) {
            bail!("官方项目目录包含重复项目: {}", project.id);
        }
        match (&project.blueprint, &project.release) {
            (Some(blueprint), Some(release)) => {
                if blueprint.schema != BLUEPRINT_SCHEMA || blueprint.source_project_id != project.id
                {
                    bail!("官方项目 {} 的蓝图来源不匹配", project.id);
                }
                if release.schema != RELEASE_SCHEMA
                    || release.blueprint_key != blueprint.blueprint_key
                {
                    bail!("官方项目 {} 的发布清单与蓝图不匹配", project.id);
                }
            }
            (None, None) => {}
            _ => bail!("官方项目 {} 必须同时提供蓝图和发布清单", project.id),
        }
    }
    Ok(catalog)
}

fn ensure_project(
    store: &Store,
    owner_user_id: &str,
    project: &OfficialProjectDefinition,
) -> Result<()> {
    let now = Utc::now().to_rfc3339();
    let conn = store.conn()?;
    let existing: Option<(String, String)> = conn
        .query_row(
            "SELECT created_by, source_type FROM projects WHERE id=?1",
            params![project.id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;
    if let Some((created_by, source_type)) = existing.as_ref() {
        if created_by != owner_user_id {
            bail!("官方项目 {} 的标识已被其他用户占用", project.id);
        }
        if !matches!(
            source_type.as_str(),
            "official_catalog" | "local_path" | "pc_managed"
        ) {
            bail!("官方项目 {} 的来源类型不兼容: {source_type}", project.id);
        }
    }

    let tx = conn.unchecked_transaction()?;
    tx.execute(
        "INSERT OR IGNORE INTO projects (
           id, name, display_name, description, workspace_key, template, source_type,
           status, created_by, is_public, join_mode, repo_url, branch, created_at, updated_at
         ) VALUES (?1, ?2, ?3, ?4, ?1, 'local', 'official_catalog', 'active', ?5, 1,
                   'readonly', ?6, ?7, ?8, ?8)",
        params![
            project.id,
            project.name,
            project.display_name,
            project.description,
            owner_user_id,
            project.repo_url,
            project.branch,
            now
        ],
    )?;
    tx.execute(
        "UPDATE projects
            SET name=?1, display_name=?2, description=?3, is_public=1, join_mode='readonly',
                repo_url=?4, branch=?5, status='active', updated_at=?6
          WHERE id=?7
            AND (name IS NOT ?1 OR display_name IS NOT ?2 OR description IS NOT ?3
                 OR is_public != 1 OR join_mode != 'readonly' OR repo_url IS NOT ?4
                 OR branch IS NOT ?5 OR status != 'active')",
        params![
            project.name,
            project.display_name,
            project.description,
            project.repo_url,
            project.branch,
            now,
            project.id
        ],
    )?;
    tx.execute(
        "INSERT INTO project_members (project_id, user_id, role, created_at)
         VALUES (?1, ?2, 'owner', ?3)
         ON CONFLICT(project_id, user_id) DO UPDATE SET role='owner'",
        params![project.id, owner_user_id, now],
    )?;
    tx.commit()?;
    Ok(())
}

fn ensure_landing(
    store: &Store,
    owner_user_id: &str,
    project: &OfficialProjectDefinition,
) -> Result<()> {
    let desired = crate::project_landing::normalize_landing_snapshot(&project.landing)
        .ok_or_else(|| anyhow::anyhow!("官方项目 {} 的首页为空", project.id))?;
    if store
        .project_landing_snapshot(owner_user_id, &project.id)?
        .as_ref()
        != Some(&desired)
    {
        store.update_project_landing_snapshot(owner_user_id, &project.id, &project.landing)?;
    }
    Ok(())
}

fn ensure_blueprint(
    store: &Store,
    owner_user_id: &str,
    project: &OfficialProjectDefinition,
) -> Result<()> {
    let definition = project
        .blueprint
        .as_ref()
        .with_context(|| format!("官方项目 {} 缺少蓝图", project.id))?;
    let release = project
        .release
        .as_ref()
        .with_context(|| format!("官方项目 {} 缺少发布清单", project.id))?;
    let blueprint = match store.erp_blueprint_for_project(&project.id)? {
        Some(blueprint) => {
            if blueprint.definition.blueprint_key != definition.blueprint_key
                || blueprint.definition.source_project_id != project.id
            {
                bail!("官方项目 {} 已绑定不兼容蓝图", project.id);
            }
            blueprint
        }
        None => service::create_blueprint(
            store,
            &project.id,
            owner_user_id,
            CreateBlueprintRequest {
                blueprint_key: definition.blueprint_key.clone(),
                name: definition.name.clone(),
                description: definition.description.clone(),
                modules: definition.modules.clone(),
                capabilities: definition.capabilities.clone(),
                themes: definition.themes.clone(),
                extension_points: definition.extension_points.clone(),
                proposal_threshold: definition.proposal_threshold,
            },
        )?,
    };
    if store
        .list_erp_blueprint_versions(&blueprint.id)?
        .iter()
        .any(|version| version.manifest.version == release.version)
    {
        return Ok(());
    }
    service::publish_version(
        store,
        &project.id,
        &blueprint.id,
        owner_user_id,
        CreateBlueprintVersionRequest {
            manifest: release.clone(),
        },
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn temp_store() -> Store {
        let path = std::env::temp_dir().join(format!(
            "elon_official_catalog_{}.db",
            Uuid::new_v4().simple()
        ));
        Store::open(&path).expect("official catalog test store should open")
    }

    #[test]
    fn official_catalog_is_valid_idempotent_and_installable() {
        let catalog = parse_catalog().unwrap();
        assert!(!catalog.projects.is_empty());
        let store = temp_store();
        let owner = store
            .create_user("catalog-owner@example.com", "secret1", None, None)
            .unwrap();

        for project in &catalog.projects {
            ensure_project(&store, &owner.id, project).unwrap();
            ensure_landing(&store, &owner.id, project).unwrap();
            if project.blueprint.is_some() {
                ensure_blueprint(&store, &owner.id, project).unwrap();
            }
            ensure_project(&store, &owner.id, project).unwrap();
            ensure_landing(&store, &owner.id, project).unwrap();
            if project.blueprint.is_some() {
                ensure_blueprint(&store, &owner.id, project).unwrap();
            }

            let public_project = store.get_public_project(&project.id).unwrap();
            assert_eq!(
                public_project.display_name.as_deref(),
                Some(project.display_name.as_str())
            );
            let expected_install_action = project.blueprint.as_ref().map(|_| "erp_blueprint");
            assert_eq!(
                public_project
                    .install_action
                    .as_ref()
                    .map(|action| action.kind),
                expected_install_action
            );
        }

        let quant = catalog
            .projects
            .iter()
            .find(|project| project.id == "yilong-quant")
            .expect("一龙量化交易必须登记在官方项目目录");
        assert!(quant.blueprint.is_none());
    }
}
