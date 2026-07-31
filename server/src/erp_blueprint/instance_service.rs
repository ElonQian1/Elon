use anyhow::{bail, Result};

use crate::store::Store;

use super::{
    model::{ErpInstance, UpdateErpInstanceRequest},
    validation::validate_instance_configuration,
};

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
