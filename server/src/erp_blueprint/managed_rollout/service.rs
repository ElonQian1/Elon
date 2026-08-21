use anyhow::{bail, Result};

use crate::{erp_blueprint::model::ErpInstance, store::Store};

use super::{
    model::{CreateManagedRolloutPlanRequest, ManagedRolloutPlan},
    validation::compile_payload,
};

pub(crate) fn create_plan(
    store: &Store,
    project_id: &str,
    instance_id: &str,
    actor_user_id: &str,
    request: CreateManagedRolloutPlanRequest,
) -> Result<ManagedRolloutPlan> {
    if !request.merchant_confirmed {
        bail!("商户未确认，不能生成托管发布计划");
    }
    if request.expected_configuration_revision < 1 {
        bail!("expected_configuration_revision 必须大于 0");
    }
    let instance = owned_active_instance(store, project_id, instance_id)?;
    if instance.configuration_revision != request.expected_configuration_revision {
        bail!("ERP 实例配置已经变化，请刷新后重新生成计划");
    }
    let merchant_id = instance
        .open_commerce_merchant_id
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("ERP 实例尚未绑定开放商业商户"))?;
    store.open_commerce_merchant_for_project(project_id, merchant_id)?;
    let version = store.erp_blueprint_version(&instance.pinned_version_id)?;
    if version.status != "published" || version.id != instance.pinned_version_id {
        bail!("ERP 实例固定版本不是当前可用的已发布版本");
    }
    let payload = compile_payload(&instance, &version, merchant_id, request)?;
    store.create_managed_rollout_plan(project_id, actor_user_id, &payload)
}

pub(crate) fn list_plans(
    store: &Store,
    project_id: &str,
    instance_id: &str,
    limit: usize,
) -> Result<Vec<ManagedRolloutPlan>> {
    owned_active_instance(store, project_id, instance_id)?;
    store.list_managed_rollout_plans(project_id, instance_id, limit)
}

pub(crate) fn get_plan(
    store: &Store,
    project_id: &str,
    instance_id: &str,
    rollout_id: &str,
) -> Result<ManagedRolloutPlan> {
    owned_active_instance(store, project_id, instance_id)?;
    store.managed_rollout_plan(project_id, instance_id, rollout_id)
}

fn owned_active_instance(
    store: &Store,
    project_id: &str,
    instance_id: &str,
) -> Result<ErpInstance> {
    let instance = store.erp_instance(instance_id)?;
    if instance.project_id != project_id {
        bail!("托管发布计划只能访问当前项目的 ERP 实例");
    }
    if instance.status != "active" {
        bail!("已归档 ERP 实例不能生成或读取托管发布计划");
    }
    Ok(instance)
}
