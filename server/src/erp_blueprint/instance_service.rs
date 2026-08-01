use anyhow::{bail, Result};

use crate::{
    project_auth::can_edit,
    store::{is_system_project_source_type, Store},
};

use super::{
    model::{ErpInstance, UpdateErpInstanceRequest},
    validation::validate_instance_configuration,
};

pub(crate) const ONBOARDING_NEW_PROJECT: &str = "new_project";
pub(crate) const ONBOARDING_EXISTING_PROJECT: &str = "existing_project";

pub(crate) struct ErpInstanceTarget {
    pub project_id: String,
    pub onboarding_mode: &'static str,
    pub cleanup_on_failure: bool,
}

pub(crate) fn resolve_instance_target(
    store: &Store,
    actor_user_id: &str,
    blueprint_project_id: &str,
    project_name: &str,
    target_project_id: Option<&str>,
) -> Result<ErpInstanceTarget> {
    if let Some(target_project_id) = target_project_id.map(str::trim).filter(|id| !id.is_empty()) {
        if target_project_id == blueprint_project_id {
            bail!("蓝图维护项目不能同时作为商户实例项目");
        }
        let access = store
            .get_project_access(actor_user_id, target_project_id)
            .map_err(|_| anyhow::anyhow!("目标项目不存在或当前用户无权访问"))?;
        if !can_edit(&access.role) {
            bail!("只有目标项目的 owner、admin 或 editor 可以将其纳入 ERP");
        }
        if is_system_project_source_type(&access.source_type) {
            bail!("系统归档项目不能纳入 ERP");
        }
        if store.erp_instance_for_project(target_project_id)?.is_some() {
            bail!("目标项目已经绑定 ERP 商户实例");
        }
        if store
            .erp_blueprint_for_project(target_project_id)?
            .is_some()
        {
            bail!("ERP 蓝图维护项目不能作为商户实例项目");
        }
        return Ok(ErpInstanceTarget {
            project_id: target_project_id.to_string(),
            onboarding_mode: ONBOARDING_EXISTING_PROJECT,
            cleanup_on_failure: false,
        });
    }

    let project_name = project_name.trim();
    if project_name.is_empty() || project_name.chars().count() > 120 {
        bail!("新建商户项目名称不能为空且不能超过 120 个字符");
    }
    let created = store.create_project(
        actor_user_id,
        project_name,
        Some("由一龙官方 ERP 蓝图创建的独立商户项目"),
        Some("android"),
    )?;
    if created.reused_existing {
        bail!("同名项目已经存在；请选择“纳入现有项目”或使用新的项目名称");
    }
    Ok(ErpInstanceTarget {
        project_id: created.project.id,
        onboarding_mode: ONBOARDING_NEW_PROJECT,
        cleanup_on_failure: true,
    })
}

pub(crate) fn update_configuration(
    store: &Store,
    project_id: &str,
    instance_id: &str,
    request: UpdateErpInstanceRequest,
) -> Result<ErpInstance> {
    if !request.merchant_confirmed {
        bail!("商户未确认，不能修改实例配置和私有扩展登记");
    }
    if request.expected_revision < 1 {
        bail!("expected_revision 必须大于 0");
    }
    let instance = store.erp_instance(instance_id)?;
    if instance.project_id != project_id {
        bail!("只有商户实例所属项目可以修改实例配置");
    }
    if instance.status != "active" {
        bail!("已归档实例不能修改配置");
    }
    let blueprint = store.erp_blueprint(&instance.blueprint_id)?;
    let version = store.erp_blueprint_version(&instance.pinned_version_id)?;
    validate_instance_configuration(
        &blueprint.definition,
        &version.manifest,
        &request.theme_key,
        &request.enabled_modules,
        &request.plugins,
        &request.private_extensions,
    )?;
    if instance.theme_key == request.theme_key
        && instance.enabled_modules == request.enabled_modules
        && instance.plugins == request.plugins
        && instance.private_extensions == request.private_extensions
    {
        bail!("实例配置没有变化");
    }
    store.update_erp_instance_configuration(instance_id, &request)
}
