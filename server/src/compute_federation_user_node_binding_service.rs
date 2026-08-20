use anyhow::{anyhow, ensure, Result};
use serde::{Deserialize, Serialize};

use crate::{
    compute_federation::user_node_provider_binding::USER_NODE_PROVIDER_BINDING_SCHEMA_V1,
    store::{
        compute_user_node_provider_bindings::{
            UserNodeProviderBindingDisposition, UserNodeProviderBindingInspection,
        },
        Store,
    },
};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct BindMyUserNodeProviderRequest {
    pub node_id: String,
    pub idempotency_key: String,
    pub confirmation: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct MyUserNodeProviderBindingView {
    pub schema: &'static str,
    pub binding_id: String,
    pub binding_digest: String,
    pub provider_id: String,
    pub node_id: String,
    pub replayed: bool,
    pub current: bool,
    pub current_blocker: Option<&'static str>,
    pub activation_effect: &'static str,
}

pub(crate) fn bind_for_user(
    store: &Store,
    owner_user_id: &str,
    provider_id: &str,
    request: BindMyUserNodeProviderRequest,
) -> Result<MyUserNodeProviderBindingView> {
    let committed = store.bind_user_node_provider(
        owner_user_id,
        &request.node_id,
        provider_id,
        &request.idempotency_key,
        &request.confirmation,
    )?;
    let replayed = matches!(
        committed.disposition(),
        UserNodeProviderBindingDisposition::ExactReplay
    );
    let inspection = store
        .inspect_user_node_provider_binding_for_owner(owner_user_id, provider_id)?
        .ok_or_else(|| anyhow!("节点 Provider 绑定提交后无法读取"))?;
    ensure!(
        inspection.receipt().binding_id() == committed.receipt().binding_id(),
        "节点 Provider 绑定提交后的 committed/readback 身份不一致"
    );
    Ok(binding_view(inspection, replayed))
}

pub(crate) fn get_for_user(
    store: &Store,
    owner_user_id: &str,
    provider_id: &str,
) -> Result<MyUserNodeProviderBindingView> {
    let inspection = store
        .inspect_user_node_provider_binding_for_owner(owner_user_id, provider_id)?
        .ok_or_else(|| anyhow!("节点 Provider 尚未建立安装身份绑定"))?;
    Ok(binding_view(inspection, false))
}

fn binding_view(
    inspection: UserNodeProviderBindingInspection,
    replayed: bool,
) -> MyUserNodeProviderBindingView {
    let receipt = inspection.receipt();
    let binding = receipt.binding();
    MyUserNodeProviderBindingView {
        schema: USER_NODE_PROVIDER_BINDING_SCHEMA_V1,
        binding_id: receipt.binding_id().to_string(),
        binding_digest: receipt.binding_digest().to_string(),
        provider_id: binding.provider_id().to_string(),
        node_id: binding.node_id().to_string(),
        replayed,
        current: inspection.current(),
        current_blocker: inspection.current_blocker(),
        activation_effect: "none",
    }
}
