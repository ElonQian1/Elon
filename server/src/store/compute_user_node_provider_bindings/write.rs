use anyhow::{bail, ensure, Result};
use chrono::{DateTime, SecondsFormat, Utc};
use rusqlite::{params, TransactionBehavior};
use sha2::{Digest, Sha256};

use crate::{
    compute_federation::{
        provider::{PROVIDER_KIND_USER_NODE, PROVIDER_STATUS_REGISTERING},
        user_node_provider_binding::{
            build_user_node_provider_binding_receipt,
            canonical_user_node_provider_binding_request_digest, UserNodeProviderBindingMaterial,
            USER_NODE_PROVIDER_BINDING_CONFIRMATION,
        },
    },
    compute_plugin_sharing_directive::derive_compute_plugin_installation_identity_digest,
};

use super::{
    committed, current_user_node_provider_binding_on,
    read::{binding_by_idempotency_on, binding_by_node_on, binding_by_provider_on},
    CommittedUserNodeProviderBinding, Store, UserNodeProviderBindingDisposition,
};
use crate::store::{
    compute_provider_registry::{current_registered_provider_on, registered_provider_version_on},
    node_compute_plugin_sharing_rows::select_current_intent,
    node_credentials::current_node_endpoint_credential_source_for_user_node_provider_binding_on,
};

pub(super) fn bind(
    store: &Store,
    owner_user_id: &str,
    node_id: &str,
    provider_id: &str,
    idempotency_key: &str,
    confirmation: &str,
) -> Result<CommittedUserNodeProviderBinding> {
    exact_identifier("绑定所有者", owner_user_id, 160)?;
    exact_identifier("节点 ID", node_id, 160)?;
    exact_identifier("Provider ID", provider_id, 160)?;
    exact_identifier("幂等键", idempotency_key, 160)?;
    ensure!(
        confirmation == USER_NODE_PROVIDER_BINDING_CONFIRMATION,
        "建立节点 Provider 绑定前必须提交固定确认文本"
    );
    let idempotency_scope = idempotency_scope(owner_user_id, provider_id);
    let request_digest = canonical_user_node_provider_binding_request_digest(
        provider_id,
        node_id,
        owner_user_id,
        confirmation,
        &idempotency_scope,
        idempotency_key,
    )?;

    let mut connection = store.conn()?;
    let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
    if let Some(existing) =
        binding_by_idempotency_on(&transaction, &idempotency_scope, idempotency_key)?
    {
        let binding = existing.binding();
        if binding.request_digest() != request_digest
            || binding.provider_id() != provider_id
            || binding.node_id() != node_id
            || binding.owner_user_id() != owner_user_id
            || binding.confirmation() != confirmation
        {
            bail!("相同节点 Provider 绑定幂等键不能用于不同请求");
        }
        transaction.commit()?;
        return Ok(committed(
            existing,
            UserNodeProviderBindingDisposition::ExactReplay,
        ));
    }

    let provider = current_registered_provider_on(&transaction, provider_id)?
        .ok_or_else(|| anyhow::anyhow!("待绑定 Provider 不存在"))?;
    if provider.provider.provider_kind != PROVIDER_KIND_USER_NODE
        || provider.provider.status != PROVIDER_STATUS_REGISTERING
        || provider.provider.owner_account_id != owner_user_id
    {
        bail!("只有本人 owned registering user_node Provider 可以建立节点绑定");
    }
    let genesis = registered_provider_version_on(&transaction, provider_id, 1)?
        .ok_or_else(|| anyhow::anyhow!("待绑定 Provider 缺少 revision-1 genesis"))?;
    if genesis.provider.provider_kind != PROVIDER_KIND_USER_NODE
        || genesis.provider.status != PROVIDER_STATUS_REGISTERING
        || genesis.provider.owner_account_id != owner_user_id
        || genesis.provider.provider_id != provider_id
    {
        bail!("待绑定 Provider genesis 身份不符合 user_node 绑定要求");
    }

    let endpoint = current_node_endpoint_credential_source_for_user_node_provider_binding_on(
        &transaction,
        node_id,
        owner_user_id,
    )?
    .ok_or_else(|| anyhow::anyhow!("节点缺少本人当前 active endpoint credential"))?;
    let endpoint_binding = endpoint.binding();
    let consent = select_current_intent(&transaction, node_id)?
        .ok_or_else(|| anyhow::anyhow!("节点缺少当前插件共享同意"))?;
    let authorization = consent
        .authorization
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("节点插件共享同意缺少授权三元组"))?;
    let installation_identity_digest =
        derive_compute_plugin_installation_identity_digest(endpoint_binding.install_id())?;
    if consent.node_id != node_id
        || consent.owner_user_id != owner_user_id
        || !consent.plugin_runtime_requested
        || !consent.dispatchable
        || consent.installation_identity_digest != installation_identity_digest
        || authorization.revision != consent.policy_revision
        || authorization.digest != consent.policy_digest
    {
        bail!("节点当前插件共享同意与 endpoint 安装身份不一致");
    }
    if binding_by_provider_on(&transaction, provider_id)?.is_some() {
        bail!("该 Provider 已绑定其他节点安装身份");
    }
    if binding_by_node_on(&transaction, node_id)?.is_some() {
        bail!("该节点安装身份已绑定其他 Provider");
    }

    let provider_genesis_digest = genesis.provider_digest;
    let endpoint_installation_binding_digest =
        endpoint_binding.installation_binding_digest().to_string();
    let source_endpoint_credential_id = endpoint_binding.credential_id().to_string();
    let source_endpoint_credential_revision =
        i64::try_from(endpoint_binding.credential_revision())?;
    let source_endpoint_credential_digest = endpoint_binding.credential_digest().to_string();
    let source_consent_receipt_id = consent.consent_receipt_id.clone();
    let source_consent_policy_revision = consent.policy_revision;
    let source_consent_policy_digest = consent.policy_digest.clone();
    let source_authorization_ref = authorization.authorization_ref.clone();
    let source_authorization_revision = authorization.revision;
    let source_authorization_digest = authorization.digest.clone();
    let recorded_at = DateTime::parse_from_rfc3339(&crate::store::now())?
        .with_timezone(&Utc)
        .to_rfc3339_opts(SecondsFormat::Nanos, true);
    let receipt = build_user_node_provider_binding_receipt(UserNodeProviderBindingMaterial::new(
        provider_id.to_string(),
        provider_genesis_digest,
        node_id.to_string(),
        owner_user_id.to_string(),
        installation_identity_digest,
        endpoint_installation_binding_digest,
        source_endpoint_credential_id,
        source_endpoint_credential_revision,
        source_endpoint_credential_digest,
        source_consent_receipt_id,
        source_consent_policy_revision,
        source_consent_policy_digest,
        source_authorization_ref,
        source_authorization_revision,
        source_authorization_digest,
        idempotency_scope,
        idempotency_key.to_string(),
        recorded_at,
    )?)?;
    insert_on(&transaction, &receipt)?;
    let readback = binding_by_provider_on(&transaction, provider_id)?
        .ok_or_else(|| anyhow::anyhow!("节点 Provider 绑定写入后无法读取"))?;
    if readback != receipt {
        bail!("节点 Provider 绑定写入后的 canonical readback 不一致");
    }
    let final_authority = current_user_node_provider_binding_on(
        &transaction,
        provider_id,
        readback.binding_id(),
        owner_user_id,
    )?
    .ok_or_else(|| anyhow::anyhow!("节点 Provider 绑定写入后不再 current"))?;
    drop(final_authority);
    transaction.commit()?;
    Ok(committed(
        readback,
        UserNodeProviderBindingDisposition::Inserted,
    ))
}

fn insert_on(
    transaction: &rusqlite::Transaction<'_>,
    receipt: &crate::compute_federation::user_node_provider_binding::UserNodeProviderBindingReceiptV1,
) -> Result<()> {
    let binding = receipt.binding();
    let binding_json = receipt.binding_json()?;
    transaction.execute(
        "INSERT INTO compute_user_node_provider_bindings (
            binding_id, binding_schema, binding_digest, binding_json, binding_material_digest,
            canonicalization, digest_algorithm, provider_id, provider_genesis_policy_revision,
            provider_genesis_digest, node_id, owner_user_id, installation_identity_digest,
            endpoint_installation_binding_digest, source_endpoint_credential_id,
            source_endpoint_credential_revision, source_endpoint_credential_digest,
            source_consent_receipt_id, source_consent_policy_revision,
            source_consent_policy_digest, source_authorization_ref,
            source_authorization_revision, source_authorization_digest, confirmation,
            idempotency_scope, idempotency_key, request_digest, bound_at, recorded_at,
            binding_effect, provider_effect, capacity_effect, offer_effect, readiness_effect,
            route_effect, execution_effect, settlement_effect
         ) VALUES (
            ?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,
            ?20,?21,?22,?23,?24,?25,?26,?27,?28,?29,?30,?31,?32,?33,?34,?35,?36,?37
         )",
        params![
            receipt.binding_id(),
            receipt.schema(),
            receipt.binding_digest(),
            binding_json,
            receipt.binding_material_digest(),
            receipt.canonicalization(),
            receipt.digest_algorithm(),
            binding.provider_id(),
            binding.provider_genesis_policy_revision(),
            binding.provider_genesis_digest(),
            binding.node_id(),
            binding.owner_user_id(),
            binding.installation_identity_digest(),
            binding.endpoint_installation_binding_digest(),
            binding.source_endpoint_credential_id(),
            binding.source_endpoint_credential_revision(),
            binding.source_endpoint_credential_digest(),
            binding.source_consent_receipt_id(),
            binding.source_consent_policy_revision(),
            binding.source_consent_policy_digest(),
            binding.source_authorization_ref(),
            binding.source_authorization_revision(),
            binding.source_authorization_digest(),
            binding.confirmation(),
            binding.idempotency_scope(),
            binding.idempotency_key(),
            binding.request_digest(),
            binding.bound_at(),
            binding.recorded_at(),
            binding.binding_effect(),
            binding.provider_effect(),
            binding.capacity_effect(),
            binding.offer_effect(),
            binding.readiness_effect(),
            binding.route_effect(),
            binding.execution_effect(),
            binding.settlement_effect(),
        ],
    )?;
    Ok(())
}

fn idempotency_scope(owner_user_id: &str, provider_id: &str) -> String {
    let mut digest = Sha256::new();
    digest.update(b"ELON-COMPUTE-USER-NODE-PROVIDER-BINDING-IDEMPOTENCY-SCOPE-V1");
    digest.update([0]);
    digest.update(owner_user_id.as_bytes());
    digest.update([0]);
    digest.update(provider_id.as_bytes());
    format!(
        "user_node_provider_binding:{}",
        hex::encode(digest.finalize())
    )
}

fn exact_identifier(label: &str, value: &str, max: usize) -> Result<()> {
    if value.is_empty()
        || value.trim() != value
        || value.chars().count() > max
        || value.chars().any(char::is_control)
    {
        bail!("{label}不能为空、包含首尾空白/控制字符或超过 {max} 字符");
    }
    Ok(())
}
