use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::store::Store;

use super::super::{
    model::{CreateErpInstanceRequest, ErpInstance},
    service as blueprint_service,
};

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct CreateMarketplaceInstanceRequest {
    #[serde(default)]
    pub project_name: String,
    #[serde(default)]
    pub target_project_id: Option<String>,
    #[serde(default)]
    pub industry: Option<String>,
    #[serde(default)]
    pub theme_key: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct MarketplaceInstanceResult {
    pub schema: &'static str,
    pub source_project_id: String,
    pub instance: ErpInstance,
    pub target_route: String,
}

pub(crate) fn create_instance(
    store: &Store,
    source_project_id: &str,
    actor_user_id: &str,
    request: CreateMarketplaceInstanceRequest,
) -> Result<MarketplaceInstanceResult> {
    let public_project = store.get_public_project(source_project_id)?;
    if public_project
        .install_action
        .as_ref()
        .map(|action| action.kind)
        != Some("erp_blueprint")
    {
        bail!("该公开项目没有可安装的 ERP 蓝图");
    }

    let blueprint = store
        .erp_blueprint_for_project(source_project_id)?
        .filter(|blueprint| blueprint.definition.source_project_id == source_project_id)
        .ok_or_else(|| anyhow::anyhow!("该公开项目没有 ERP 蓝图"))?;
    let version = store
        .list_erp_blueprint_versions(&blueprint.id)?
        .into_iter()
        .next()
        .ok_or_else(|| anyhow::anyhow!("该 ERP 蓝图还没有已发布版本"))?;
    let theme_key = request
        .theme_key
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .or_else(|| blueprint.definition.themes.first().cloned())
        .ok_or_else(|| anyhow::anyhow!("该 ERP 蓝图没有可用主题"))?;
    let industry = request
        .industry
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("local_retail")
        .to_string();

    let instance = blueprint_service::create_instance(
        store,
        source_project_id,
        &blueprint.id,
        actor_user_id,
        CreateErpInstanceRequest {
            instance_key: format!("merchant.{}", Uuid::new_v4().simple()),
            project_name: request.project_name,
            target_project_id: request.target_project_id,
            version: version.manifest.version,
            industry,
            theme_key,
            enabled_modules: Vec::new(),
            plugins: Vec::new(),
            private_extensions: Vec::new(),
        },
    )?;
    let target_route = format!(
        "/projects/{}?tab=openCommerce&commerce=erp",
        instance.project_id
    );

    Ok(MarketplaceInstanceResult {
        schema: "yilong.erp.marketplace_instance.v1",
        source_project_id: source_project_id.to_string(),
        instance,
        target_route,
    })
}
