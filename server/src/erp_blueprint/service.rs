use anyhow::{bail, Result};
use serde_json::json;

use crate::group_ai::types::ProjectAiBot;
use crate::store::Store;

use super::{
    catalog_service, compatibility, matter_bridge,
    model::{
        CreateBlueprintRequest, CreateBlueprintVersionRequest, CreateErpInstanceRequest,
        DecideProposalRequest, DecideUpgradeRequest, ErpBlueprint, ErpBlueprintVersion,
        ErpFeatureProposal, ErpFeatureSignal, ErpInstance, ErpInstanceConfiguration,
        ErpProjectOverview, ErpUpgradeAdoptionEvidence, ErpUpgradeCampaign, PrepareUpgradeRequest,
        RequirementResolution, ResolveRequirementRequest, SubmitFeatureSignalRequest,
        BLUEPRINT_SCHEMA,
    },
    validation::{
        build_definition, manifest_hash, normalize_key, stable_need_key,
        validate_instance_configuration, validate_release, validate_signal, version_is_newer,
    },
};

pub(crate) fn overview(store: &Store, project_id: &str) -> Result<ErpProjectOverview> {
    let blueprint = store.erp_blueprint_for_project(project_id)?;
    let Some(blueprint) = blueprint else {
        return Ok(empty_overview());
    };
    let versions = store.list_erp_blueprint_versions(&blueprint.id)?;
    let current_instance = store.erp_instance_for_project(project_id)?;
    let is_blueprint_project = blueprint.definition.source_project_id == project_id;
    let instances = if is_blueprint_project {
        store.list_erp_instances(&blueprint.id)?
    } else {
        current_instance.clone().into_iter().collect()
    };
    let proposals = store.list_erp_feature_proposals(&blueprint.id)?;
    let upgrades = if let Some(instance) = current_instance.as_ref() {
        store.list_erp_upgrade_campaigns_for_instance(&instance.id)?
    } else {
        Vec::new()
    };
    let catalog = catalog_service::catalog_for_project(store, project_id, None)?;
    Ok(ErpProjectOverview {
        schema: "yilong.erp.project_overview.v1",
        capability_catalog: catalog.capabilities,
        catalog_version: catalog.version,
        unreleased_capability_keys: catalog.unreleased_capability_keys,
        blueprint: Some(blueprint),
        versions,
        instance: current_instance,
        instances,
        proposals,
        upgrades,
        boundaries: boundaries(),
    })
}

pub(crate) fn create_blueprint(
    store: &Store,
    project_id: &str,
    actor_user_id: &str,
    request: CreateBlueprintRequest,
) -> Result<ErpBlueprint> {
    let definition = build_definition(project_id, request)?;
    store.create_erp_blueprint(definition, actor_user_id)
}

pub(crate) fn publish_version(
    store: &Store,
    project_id: &str,
    blueprint_id: &str,
    actor_user_id: &str,
    request: CreateBlueprintVersionRequest,
) -> Result<ErpBlueprintVersion> {
    let blueprint = owned_blueprint(store, project_id, blueprint_id)?;
    validate_release(&blueprint.definition, &request.manifest)?;
    let versions = store.list_erp_blueprint_versions(blueprint_id)?;
    match (
        versions.first(),
        request.manifest.previous_version.as_deref(),
    ) {
        (None, None) => {}
        (None, Some(_)) => bail!("首个蓝图版本不能声明 previous_version"),
        (Some(_), None) => bail!("非首个蓝图版本必须声明 previous_version"),
        (Some(latest), Some(previous)) => {
            if latest.manifest.version != previous {
                bail!(
                    "previous_version 必须是当前最新发布版本 {}",
                    latest.manifest.version
                );
            }
        }
    }
    let hash = manifest_hash(&request.manifest)?;
    store.create_erp_blueprint_version(blueprint_id, &request.manifest, &hash, actor_user_id)
}

pub(crate) fn create_instance(
    store: &Store,
    project_id: &str,
    blueprint_id: &str,
    actor_user_id: &str,
    request: CreateErpInstanceRequest,
) -> Result<ErpInstance> {
    let blueprint = owned_blueprint(store, project_id, blueprint_id)?;
    let instance_key = normalize_key(&request.instance_key, "instance_key")?;
    let project_name = request.project_name.trim();
    if project_name.is_empty() || project_name.chars().count() > 120 {
        bail!("商户项目名称不能为空且不能超过 120 个字符");
    }
    let industry = normalize_key(&request.industry, "industry")?;
    let theme_key = normalize_key(&request.theme_key, "theme_key")?;
    let version = store.erp_blueprint_version_by_name(blueprint_id, &request.version)?;
    if !blueprint.definition.themes.contains(&theme_key) {
        bail!("主题未在蓝图中声明");
    }
    let enabled_modules = if request.enabled_modules.is_empty() {
        version
            .manifest
            .modules
            .iter()
            .map(|module| module.module_key.clone())
            .collect::<Vec<_>>()
    } else {
        request
            .enabled_modules
            .into_iter()
            .map(|module| normalize_key(&module, "enabled_module"))
            .collect::<Result<Vec<_>>>()?
    };
    let configuration = validate_instance_configuration(
        &blueprint.definition,
        &version.manifest,
        &theme_key,
        &enabled_modules,
        &request.plugins,
        &request.private_extensions,
    )?;
    if store.erp_instance_by_key(&instance_key)?.is_some() {
        bail!("instance_key 已被其他商户实例使用");
    }
    let created = store.create_project(
        actor_user_id,
        project_name,
        Some("由一龙官方 ERP 蓝图创建的独立商户项目"),
        Some("android"),
    )?;
    if created.reused_existing {
        bail!("同名项目已经存在；为避免误绑定或覆盖，请为商户实例使用新的项目名称");
    }
    let result = store.create_erp_instance(
        &instance_key,
        &created.project.id,
        blueprint_id,
        &version.id,
        &industry,
        &configuration.theme_key,
        &configuration.enabled_modules,
        &configuration.plugins,
        &request.private_extensions,
        actor_user_id,
    );
    if result.is_err() {
        store
            .purge_project_records(actor_user_id, &created.project.id)
            .map_err(|cleanup| anyhow::anyhow!("ERP 实例登记失败且空项目清理失败：{cleanup}"))?;
    }
    result
}

pub(crate) fn resolve_requirement(
    store: &Store,
    project_id: &str,
    request: ResolveRequirementRequest,
) -> Result<RequirementResolution> {
    catalog_service::resolve_requirement(store, project_id, request)
}

pub(crate) fn submit_signal(
    store: &Store,
    project_id: &str,
    instance_id: &str,
    actor_user_id: &str,
    request: SubmitFeatureSignalRequest,
) -> Result<ErpFeatureSignal> {
    validate_signal(&request)?;
    let instance = store.erp_instance(instance_id)?;
    if instance.project_id != project_id {
        bail!("需求信号只能由所属商户项目提交");
    }
    if instance.status != "active" {
        bail!("已归档实例不能提交需求信号");
    }
    if normalize_key(&request.industry, "industry")? != instance.industry {
        bail!("需求信号行业必须与商户实例登记行业一致");
    }
    let need_key = match request.need_key.as_deref() {
        Some(value) => normalize_key(value, "need_key")?,
        None => stable_need_key(&request.requirement_summary),
    };
    store.upsert_erp_feature_signal(
        &instance.blueprint_id,
        instance_id,
        &need_key,
        &request,
        actor_user_id,
    )
}

pub(crate) fn decide_proposal(
    store: &Store,
    project_id: &str,
    proposal_id: &str,
    actor_user_id: &str,
    request: DecideProposalRequest,
    bots: &[ProjectAiBot],
) -> Result<(ErpFeatureProposal, Option<String>)> {
    let proposal = store.erp_feature_proposal(proposal_id)?;
    let blueprint = owned_blueprint(store, project_id, &proposal.blueprint_id)?;
    if request.decision == "accepted"
        && proposal.support_count < blueprint.definition.proposal_threshold
    {
        bail!(
            "提案仅获 {} 个独立商户支持，尚未达到阈值 {}",
            proposal.support_count,
            blueprint.definition.proposal_threshold
        );
    }
    let decided = store.decide_erp_feature_proposal(
        proposal_id,
        &request.decision,
        &request.note,
        actor_user_id,
    )?;
    if request.create_matter && request.decision == "accepted" {
        let matter = matter_bridge::create_proposal_matter(
            store,
            &blueprint,
            &decided,
            actor_user_id,
            bots,
        )?;
        return Ok((store.erp_feature_proposal(proposal_id)?, Some(matter.id)));
    }
    Ok((decided, None))
}

pub(crate) fn create_proposal_matter(
    store: &Store,
    project_id: &str,
    proposal_id: &str,
    actor_user_id: &str,
    bots: &[ProjectAiBot],
) -> Result<(ErpFeatureProposal, String)> {
    let proposal = store.erp_feature_proposal(proposal_id)?;
    let blueprint = owned_blueprint(store, project_id, &proposal.blueprint_id)?;
    let matter =
        matter_bridge::create_proposal_matter(store, &blueprint, &proposal, actor_user_id, bots)?;
    Ok((store.erp_feature_proposal(proposal_id)?, matter.id))
}

pub(crate) fn create_instance_bootstrap_matter(
    store: &Store,
    project_id: &str,
    instance_id: &str,
    actor_user_id: &str,
    bots: &[ProjectAiBot],
) -> Result<(ErpInstance, String)> {
    let instance = store.erp_instance(instance_id)?;
    if instance.project_id != project_id {
        bail!("只有商户实例所属项目可以创建初始化 Matter");
    }
    if instance.status != "active" {
        bail!("已归档实例不能创建初始化 Matter");
    }
    let blueprint = store.erp_blueprint(&instance.blueprint_id)?;
    let version = store.erp_blueprint_version(&instance.pinned_version_id)?;
    let matter = matter_bridge::create_bootstrap_matter(
        store,
        &blueprint,
        &version,
        &instance,
        actor_user_id,
        bots,
    )?;
    Ok((store.erp_instance(instance_id)?, matter.id))
}

pub(crate) fn prepare_upgrade(
    store: &Store,
    project_id: &str,
    instance_id: &str,
    actor_user_id: &str,
    request: PrepareUpgradeRequest,
) -> Result<ErpUpgradeCampaign> {
    let instance = store.erp_instance(instance_id)?;
    let blueprint = store.erp_blueprint(&instance.blueprint_id)?;
    if instance.project_id != project_id && blueprint.definition.source_project_id != project_id {
        bail!("只有商户实例或蓝图维护项目可以准备升级");
    }
    if instance.status != "active" {
        bail!("已归档实例不能准备升级");
    }
    let target =
        store.erp_blueprint_version_by_name(&instance.blueprint_id, &request.target_version)?;
    if target.id == instance.pinned_version_id {
        bail!("实例已经固定在目标版本");
    }
    if !version_is_newer(&target.manifest.version, &instance.pinned_version) {
        bail!("升级目标必须高于实例当前版本；历史版本只能通过既有升级活动回滚");
    }
    let preparation = compatibility::prepare(&instance, &target);
    let from_configuration = ErpInstanceConfiguration {
        theme_key: instance.theme_key.clone(),
        enabled_modules: instance.enabled_modules.clone(),
        plugins: instance.plugins.clone(),
    };
    store.create_erp_upgrade_campaign(
        instance_id,
        &instance.pinned_version_id,
        &target.id,
        &preparation.report,
        instance.configuration_revision,
        &from_configuration,
        &preparation.target_configuration,
        &instance.private_extensions,
        actor_user_id,
    )
}

pub(crate) fn decide_upgrade(
    store: &Store,
    project_id: &str,
    campaign_id: &str,
    actor_user_id: &str,
    request: DecideUpgradeRequest,
) -> Result<ErpUpgradeCampaign> {
    let campaign = store.erp_upgrade_campaign(campaign_id)?;
    let instance = store.erp_instance(&campaign.instance_id)?;
    if instance.project_id != project_id {
        bail!("只有商户实例所属项目可以确认采用或回滚；蓝图维护项目只能准备兼容检查");
    }
    if !request.merchant_confirmed {
        bail!("商户未确认，不能采用或回滚升级");
    }
    if request.action == "rollback" && request.reason.trim().is_empty() {
        bail!("回滚必须填写原因");
    }
    let adoption_evidence = if request.action == "adopt" {
        if !request.execution_attested {
            bail!("采用升级前必须确认真实开发、迁移和验证流程已经完成");
        }
        let verification_summary = request.verification_summary.trim();
        if verification_summary.chars().count() < 8 || verification_summary.chars().count() > 500 {
            bail!("采用升级必须填写 8 到 500 个字符的验证摘要");
        }
        if let Some(commit) = request.deployed_commit.as_deref() {
            if !(7..=64).contains(&commit.len())
                || !commit.chars().all(|value| value.is_ascii_hexdigit())
            {
                bail!("deployed_commit 必须是 7 到 64 位十六进制提交标识");
            }
        }
        Some(ErpUpgradeAdoptionEvidence {
            execution_attested: true,
            verification_summary: verification_summary.to_string(),
            deployed_commit: request.deployed_commit,
        })
    } else {
        None
    };
    store.decide_erp_upgrade_campaign(
        campaign_id,
        &request.action,
        &request.reason,
        actor_user_id,
        adoption_evidence.as_ref(),
    )
}

fn owned_blueprint(store: &Store, project_id: &str, blueprint_id: &str) -> Result<ErpBlueprint> {
    let blueprint = store.erp_blueprint(blueprint_id)?;
    if blueprint.definition.source_project_id != project_id {
        bail!("当前项目不是该 ERP 蓝图的维护项目");
    }
    Ok(blueprint)
}

fn empty_overview() -> ErpProjectOverview {
    ErpProjectOverview {
        schema: "yilong.erp.project_overview.v1",
        blueprint: None,
        versions: vec![],
        instance: None,
        instances: vec![],
        proposals: vec![],
        upgrades: vec![],
        capability_catalog: vec![],
        catalog_version: None,
        unreleased_capability_keys: vec![],
        boundaries: boundaries(),
    }
}

fn boundaries() -> serde_json::Value {
    json!({
        "raw_merchant_data_uploaded": false,
        "private_source_uploaded": false,
        "ai_can_accept_or_merge": false,
        "ai_can_publish_or_upgrade": false,
        "v1_executes_git_or_deploy": false,
        "schema": BLUEPRINT_SCHEMA,
    })
}
