use anyhow::{bail, Result};
use serde::Serialize;

use crate::{
    open_commerce_directory_model::DIRECTORY_STATUS_PUBLISHED,
    open_commerce_model::{
        ACCESS_OWNER_ONLY, CAPABILITY_STATUS_ACTIVE, HANDLER_MERCHANT_RUNTIME,
        MERCHANT_STATUS_ACTIVE,
    },
    open_commerce_runtime_model::RUNTIME_STATUS_ACTIVE,
    store::Store,
};

use super::materialization;

const READINESS_SCHEMA: &str = "yilong.erp.open_commerce_readiness.v1";
const MATERIALIZATION_READY: &str = "accepted_verified";

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ErpOpenCommerceReadiness {
    pub schema: &'static str,
    pub project_id: String,
    pub instance_id: String,
    pub overall_state: &'static str,
    pub erp_onboarding_ready: bool,
    pub consumer_invocation_ready: bool,
    pub consumer_discovery_ready: bool,
    pub materialization: MaterializationReadiness,
    pub merchant_selection: MerchantSelection,
    pub runtime: Option<RuntimeReadiness>,
    pub active_runtime_capability_keys: Vec<String>,
    pub directory: Option<DirectoryReadiness>,
    pub blockers: Vec<ReadinessBlocker>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct MaterializationReadiness {
    pub state: String,
    pub recoverable: bool,
    pub blockers: Vec<String>,
    pub next_action: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct MerchantSelection {
    pub status: &'static str,
    pub selected: Option<MerchantSummary>,
    pub candidates: Vec<MerchantSummary>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct MerchantSummary {
    pub id: String,
    pub display_name: String,
    pub status: String,
    pub node_mode: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct RuntimeReadiness {
    pub status: String,
    pub manifest_sha256: Option<String>,
    pub last_verified_at: Option<String>,
    pub last_error_code: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct DirectoryReadiness {
    pub status: String,
    pub revision: i64,
    pub published_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ReadinessBlocker {
    pub code: &'static str,
    pub scope: &'static str,
    pub message: String,
    pub next_action: String,
}

pub(crate) fn get(
    store: &Store,
    project_id: &str,
    instance_id: &str,
    merchant_id: Option<&str>,
) -> Result<ErpOpenCommerceReadiness> {
    let instance = store.erp_instance(instance_id)?;
    if instance.project_id != project_id {
        bail!("只有商户实例所属项目可以读取开放商业就绪度");
    }

    let materialization = materialization::status(store, project_id, instance_id)?;
    let erp_onboarding_ready = materialization.state == MATERIALIZATION_READY;
    let materialization_summary = MaterializationReadiness {
        state: materialization.state,
        recoverable: materialization.recoverable,
        blockers: materialization.blockers,
        next_action: materialization.next_action,
    };

    let merchants = store.list_project_open_commerce_merchants(project_id)?;
    let candidates = merchants
        .iter()
        .map(|detail| merchant_summary(&detail.merchant))
        .collect::<Vec<_>>();
    let (selection_status, selected) = select_merchant(&merchants, merchant_id)?;

    let mut blockers = Vec::new();
    if !erp_onboarding_ready {
        blockers.push(ReadinessBlocker {
            code: "erp_materialization_not_verified",
            scope: "erp_onboarding",
            message: format!(
                "ERP 实例物化状态为 {}，尚未形成已验收证据",
                materialization_summary.state
            ),
            next_action: materialization_summary.next_action.clone(),
        });
    }

    let Some(selected) = selected else {
        blockers.push(selection_blocker(selection_status));
        return Ok(response(
            project_id,
            instance_id,
            erp_onboarding_ready,
            false,
            false,
            materialization_summary,
            MerchantSelection {
                status: selection_status,
                selected: None,
                candidates,
            },
            None,
            Vec::new(),
            None,
            blockers,
        ));
    };

    let merchant = &selected.merchant;
    if merchant.status != MERCHANT_STATUS_ACTIVE {
        blockers.push(ReadinessBlocker {
            code: "merchant_inactive",
            scope: "consumer_invocation",
            message: format!("商户节点当前状态为 {}", merchant.status),
            next_action: "由项目编辑者启用商户节点后重新检查。".into(),
        });
    }

    let runtime_binding = store
        .list_project_open_commerce_runtime_bindings(project_id)?
        .into_iter()
        .find(|binding| binding.merchant_id == merchant.id);
    match runtime_binding.as_ref() {
        None => blockers.push(ReadinessBlocker {
            code: "runtime_binding_missing",
            scope: "consumer_invocation",
            message: "商户尚未配置可验证的运行时绑定".into(),
            next_action: "配置商户后端 HTTPS 运行地址和服务端密钥引用，再执行运行时验证。".into(),
        }),
        Some(binding) if binding.status != RUNTIME_STATUS_ACTIVE => {
            blockers.push(ReadinessBlocker {
                code: "runtime_binding_not_active",
                scope: "consumer_invocation",
                message: format!("商户运行时当前状态为 {}", binding.status),
                next_action: "修复运行时连通性或清单问题，并重新执行验证。".into(),
            });
        }
        Some(_) => {}
    }

    let mut active_runtime_capability_keys = selected
        .capabilities
        .iter()
        .filter(|capability| {
            capability.status == CAPABILITY_STATUS_ACTIVE
                && capability.handler_type == HANDLER_MERCHANT_RUNTIME
                && capability.access_level != ACCESS_OWNER_ONLY
        })
        .map(|capability| capability.capability_key.clone())
        .collect::<Vec<_>>();
    active_runtime_capability_keys.sort();
    if active_runtime_capability_keys.is_empty() {
        blockers.push(ReadinessBlocker {
            code: "merchant_runtime_capability_missing",
            scope: "consumer_invocation",
            message: "商户没有面向消费者 AI 的有效 merchant_runtime 能力".into(),
            next_action: "发布至少一项 public 或 authorized 的 merchant_runtime 能力。".into(),
        });
    }

    let publication = store.open_commerce_directory_publication(&merchant.id)?;
    if publication
        .as_ref()
        .is_none_or(|value| value.status != DIRECTORY_STATUS_PUBLISHED)
    {
        blockers.push(ReadinessBlocker {
            code: "directory_not_published",
            scope: "consumer_discovery",
            message: "商户尚未发布到开放商业目录".into(),
            next_action: "审核公开资料和能力后，由商户主动发布目录条目。".into(),
        });
    }

    let consumer_invocation_ready = merchant.status == MERCHANT_STATUS_ACTIVE
        && runtime_binding
            .as_ref()
            .is_some_and(|binding| binding.status == RUNTIME_STATUS_ACTIVE)
        && !active_runtime_capability_keys.is_empty();
    let consumer_discovery_ready = consumer_invocation_ready
        && publication
            .as_ref()
            .is_some_and(|value| value.status == DIRECTORY_STATUS_PUBLISHED);

    Ok(response(
        project_id,
        instance_id,
        erp_onboarding_ready,
        consumer_invocation_ready,
        consumer_discovery_ready,
        materialization_summary,
        MerchantSelection {
            status: selection_status,
            selected: Some(merchant_summary(merchant)),
            candidates,
        },
        runtime_binding.map(|binding| RuntimeReadiness {
            status: binding.status,
            manifest_sha256: binding.manifest_sha256,
            last_verified_at: binding.last_verified_at,
            last_error_code: binding.last_error_code,
        }),
        active_runtime_capability_keys,
        publication.map(|value| DirectoryReadiness {
            status: value.status,
            revision: value.revision,
            published_at: value.published_at,
        }),
        blockers,
    ))
}

fn select_merchant<'a>(
    merchants: &'a [crate::open_commerce_model::OpenCommerceMerchantDetail],
    merchant_id: Option<&str>,
) -> Result<(
    &'static str,
    Option<&'a crate::open_commerce_model::OpenCommerceMerchantDetail>,
)> {
    if let Some(merchant_id) = merchant_id.map(str::trim).filter(|value| !value.is_empty()) {
        let selected = merchants
            .iter()
            .find(|detail| detail.merchant.id == merchant_id)
            .ok_or_else(|| anyhow::anyhow!("当前项目中不存在指定的商户节点"))?;
        return Ok(("selected_explicit", Some(selected)));
    }
    match merchants {
        [] => Ok(("merchant_missing", None)),
        [selected] => Ok(("selected_implicit", Some(selected))),
        _ => Ok(("selection_required", None)),
    }
}

fn merchant_summary(
    merchant: &crate::open_commerce_model::OpenCommerceMerchant,
) -> MerchantSummary {
    MerchantSummary {
        id: merchant.id.clone(),
        display_name: merchant.display_name.clone(),
        status: merchant.status.clone(),
        node_mode: merchant.node_mode.clone(),
    }
}

fn selection_blocker(status: &str) -> ReadinessBlocker {
    if status == "selection_required" {
        ReadinessBlocker {
            code: "merchant_selection_required",
            scope: "consumer_invocation",
            message: "当前项目存在多个商户节点，无法安全推断 ERP 实例对应哪一个节点".into(),
            next_action: "调用时显式提供 merchant_id；本接口不会自动创建永久绑定。".into(),
        }
    } else {
        ReadinessBlocker {
            code: "merchant_missing",
            scope: "consumer_invocation",
            message: "当前项目尚未创建开放商业商户节点".into(),
            next_action: "先在当前 ERP 项目创建商户节点，再配置运行时和对外能力。".into(),
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn response(
    project_id: &str,
    instance_id: &str,
    erp_onboarding_ready: bool,
    consumer_invocation_ready: bool,
    consumer_discovery_ready: bool,
    materialization: MaterializationReadiness,
    merchant_selection: MerchantSelection,
    runtime: Option<RuntimeReadiness>,
    active_runtime_capability_keys: Vec<String>,
    directory: Option<DirectoryReadiness>,
    blockers: Vec<ReadinessBlocker>,
) -> ErpOpenCommerceReadiness {
    let overall_state = match (erp_onboarding_ready, consumer_discovery_ready) {
        (true, true) => "ready",
        (false, true) => "consumer_ready_erp_pending",
        (true, false) => "erp_ready_commerce_pending",
        (false, false) => "blocked",
    };
    ErpOpenCommerceReadiness {
        schema: READINESS_SCHEMA,
        project_id: project_id.into(),
        instance_id: instance_id.into(),
        overall_state,
        erp_onboarding_ready,
        consumer_invocation_ready,
        consumer_discovery_ready,
        materialization,
        merchant_selection,
        runtime,
        active_runtime_capability_keys,
        directory,
        blockers,
    }
}
