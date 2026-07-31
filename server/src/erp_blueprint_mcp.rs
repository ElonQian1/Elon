use anyhow::{bail, Result};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::{
    erp_blueprint::{
        catalog_service, instance_service,
        model::{
            PrepareUpgradeRequest, ResolveRequirementRequest, SubmitFeatureSignalRequest,
            UpdateErpInstanceRequest,
        },
        service,
    },
    project_auth::can_edit,
    store::Store,
};

#[derive(Debug, Deserialize)]
struct SearchArgs {
    query: String,
    #[serde(default = "default_limit")]
    limit: usize,
}

#[derive(Debug, Deserialize)]
struct SignalArgs {
    instance_id: String,
    #[serde(flatten)]
    request: SubmitFeatureSignalRequest,
}

#[derive(Debug, Deserialize)]
struct UpgradeArgs {
    instance_id: String,
    target_version: String,
}

#[derive(Debug, Deserialize)]
struct UpdateInstanceArgs {
    instance_id: String,
    #[serde(flatten)]
    request: UpdateErpInstanceRequest,
}

pub(crate) fn handles(name: &str) -> bool {
    name.starts_with("erp_")
}

pub(crate) fn call_tool(
    store: &Store,
    project_id: &str,
    user_id: &str,
    project_role: &str,
    name: &str,
    arguments: Value,
) -> Result<Value> {
    match name {
        "erp_get_overview" => {
            ensure_empty(&arguments, name)?;
            Ok(serde_json::to_value(service::overview(store, project_id)?)?)
        }
        "erp_search_capabilities" => {
            let args: SearchArgs = decode(arguments, name)?;
            let snapshot =
                catalog_service::search_capabilities(store, project_id, &args.query, args.limit)?;
            Ok(json!({
                "schema":"yilong.erp.capability_catalog.v1",
                "catalog_version":snapshot.version,
                "capabilities":snapshot.capabilities
            }))
        }
        "erp_resolve_requirement" => {
            let request: ResolveRequirementRequest = decode(arguments, name)?;
            Ok(serde_json::to_value(service::resolve_requirement(
                store, project_id, request,
            )?)?)
        }
        "erp_submit_feature_signal" => {
            ensure_write(project_role)?;
            let args: SignalArgs = decode(arguments, name)?;
            Ok(serde_json::to_value(service::submit_signal(
                store,
                project_id,
                &args.instance_id,
                user_id,
                args.request,
            )?)?)
        }
        "erp_update_instance_configuration" => {
            ensure_write(project_role)?;
            let args: UpdateInstanceArgs = decode(arguments, name)?;
            Ok(serde_json::to_value(
                instance_service::update_configuration(
                    store,
                    project_id,
                    &args.instance_id,
                    args.request,
                )?,
            )?)
        }
        "erp_prepare_upgrade_check" => {
            ensure_write(project_role)?;
            let args: UpgradeArgs = decode(arguments, name)?;
            Ok(serde_json::to_value(service::prepare_upgrade(
                store,
                project_id,
                &args.instance_id,
                user_id,
                PrepareUpgradeRequest {
                    target_version: args.target_version,
                },
            )?)?)
        }
        _ => bail!("未知 ERP MCP 工具：{name}"),
    }
}

fn ensure_write(role: &str) -> Result<()> {
    if can_edit(role) {
        Ok(())
    } else {
        bail!("当前项目只有查看权限，不能提交信号或准备升级")
    }
}

fn decode<T: for<'de> Deserialize<'de>>(arguments: Value, name: &str) -> Result<T> {
    serde_json::from_value(arguments).map_err(|error| anyhow::anyhow!("{name} 参数错误：{error}"))
}

fn ensure_empty(arguments: &Value, name: &str) -> Result<()> {
    if arguments.as_object().is_some_and(|value| value.is_empty()) {
        Ok(())
    } else {
        bail!("{name} 不接受参数")
    }
}

fn default_limit() -> usize {
    20
}
