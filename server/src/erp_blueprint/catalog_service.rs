use anyhow::{bail, Result};
use std::collections::BTreeSet;

use crate::store::Store;

use super::{
    model::{
        CreateBlueprintRequest, ErpBlueprint, ErpCapabilityDefinition, EvolveBlueprintRequest,
        RequirementResolution, ResolveRequirementRequest,
    },
    proposal,
    validation::build_definition,
};

pub(crate) struct CapabilityCatalogSnapshot {
    pub capabilities: Vec<ErpCapabilityDefinition>,
    pub version: Option<String>,
    pub unreleased_capability_keys: Vec<String>,
}

pub(crate) fn evolve_blueprint(
    store: &Store,
    project_id: &str,
    blueprint_id: &str,
    request: EvolveBlueprintRequest,
) -> Result<ErpBlueprint> {
    if request.expected_revision < 1 {
        bail!("expected_revision 必须大于 0");
    }
    let blueprint = store.erp_blueprint(blueprint_id)?;
    if blueprint.definition.source_project_id != project_id {
        bail!("当前项目不是该 ERP 蓝图的维护项目");
    }
    let mut definition = blueprint.definition.clone();
    if let Some(name) = request.name {
        definition.name = name;
    }
    if let Some(description) = request.description {
        definition.description = description;
    }
    if let Some(threshold) = request.proposal_threshold {
        definition.proposal_threshold = threshold;
    }
    definition.modules.extend(request.add_modules);
    definition.capabilities.extend(request.add_capabilities);
    definition.themes.extend(request.add_themes);
    definition
        .extension_points
        .extend(request.add_extension_points);
    let next = build_definition(
        project_id,
        CreateBlueprintRequest {
            blueprint_key: definition.blueprint_key,
            name: definition.name,
            description: definition.description,
            modules: definition.modules,
            capabilities: definition.capabilities,
            themes: definition.themes,
            extension_points: definition.extension_points,
            proposal_threshold: definition.proposal_threshold,
        },
    )?;
    if next == blueprint.definition {
        bail!("蓝图演进请求没有产生任何变化");
    }
    store.update_erp_blueprint_definition(blueprint_id, request.expected_revision, &next)
}

pub(crate) fn catalog_for_project(
    store: &Store,
    project_id: &str,
    requested_instance_id: Option<&str>,
) -> Result<CapabilityCatalogSnapshot> {
    let blueprint = store
        .erp_blueprint_for_project(project_id)?
        .ok_or_else(|| anyhow::anyhow!("当前项目尚未关联 ERP 蓝图"))?;
    let version = if let Some(instance_id) = requested_instance_id {
        let instance = store.erp_instance(instance_id)?;
        if instance.project_id != project_id && blueprint.definition.source_project_id != project_id
        {
            bail!("不能读取其他商户实例的能力版本");
        }
        if instance.blueprint_id != blueprint.id {
            bail!("商户实例不属于当前蓝图");
        }
        Some(store.erp_blueprint_version(&instance.pinned_version_id)?)
    } else if let Some(instance) = store.erp_instance_for_project(project_id)? {
        Some(store.erp_blueprint_version(&instance.pinned_version_id)?)
    } else {
        store
            .list_erp_blueprint_versions(&blueprint.id)?
            .into_iter()
            .next()
    };
    let released_keys: BTreeSet<_> = version
        .as_ref()
        .map(|version| {
            version
                .manifest
                .capabilities
                .iter()
                .map(String::as_str)
                .collect()
        })
        .unwrap_or_default();
    let capabilities = blueprint
        .definition
        .capabilities
        .iter()
        .filter(|capability| released_keys.contains(capability.capability_key.as_str()))
        .cloned()
        .collect();
    let unreleased_capability_keys = blueprint
        .definition
        .capabilities
        .iter()
        .filter(|capability| !released_keys.contains(capability.capability_key.as_str()))
        .map(|capability| capability.capability_key.clone())
        .collect();
    Ok(CapabilityCatalogSnapshot {
        capabilities,
        version: version.map(|version| version.manifest.version),
        unreleased_capability_keys,
    })
}

pub(crate) fn search_capabilities(
    store: &Store,
    project_id: &str,
    query: &str,
    limit: usize,
) -> Result<CapabilityCatalogSnapshot> {
    let mut snapshot = catalog_for_project(store, project_id, None)?;
    let mut definition = store
        .erp_blueprint_for_project(project_id)?
        .ok_or_else(|| anyhow::anyhow!("当前项目尚未关联 ERP 蓝图"))?
        .definition;
    definition.capabilities = snapshot.capabilities;
    snapshot.capabilities = proposal::search_capabilities(&definition, query, limit);
    Ok(snapshot)
}

pub(crate) fn resolve_requirement(
    store: &Store,
    project_id: &str,
    request: ResolveRequirementRequest,
) -> Result<RequirementResolution> {
    let snapshot = catalog_for_project(store, project_id, request.instance_id.as_deref())?;
    let mut definition = store
        .erp_blueprint_for_project(project_id)?
        .ok_or_else(|| anyhow::anyhow!("当前项目尚未关联 ERP 蓝图"))?
        .definition;
    definition.capabilities = snapshot.capabilities;
    let mut resolution = proposal::resolve_requirement(&definition, request)?;
    resolution.catalog_version = snapshot.version;
    Ok(resolution)
}
