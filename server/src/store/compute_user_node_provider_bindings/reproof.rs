use anyhow::{bail, Result};
use rusqlite::{params, OptionalExtension, Transaction};

use crate::compute_federation::provider::{PROVIDER_KIND_USER_NODE, PROVIDER_STATUS_REGISTERING};

use super::{
    current_authority, read::binding_by_provider_on, CurrentUserNodeProviderBindingAuthority,
};
use crate::store::{
    compute_provider_registry::{current_registered_provider_on, registered_provider_version_on},
    node_compute_plugin_sharing_rows::select_current_intent,
    node_credentials::current_node_endpoint_credential_for_user_node_provider_binding_on,
};

pub(in crate::store) fn current_user_node_provider_binding_on<'tx, 'conn>(
    transaction: &'tx Transaction<'conn>,
    provider_id: &str,
    binding_id: &str,
    expected_owner_user_id: &str,
) -> Result<Option<CurrentUserNodeProviderBindingAuthority<'tx, 'conn>>> {
    let Some(receipt) = binding_by_provider_on(transaction, provider_id)? else {
        return Ok(None);
    };
    let binding = receipt.binding();
    if receipt.binding_id() != binding_id || binding.owner_user_id() != expected_owner_user_id {
        return Ok(None);
    }

    let genesis = registered_provider_version_on(
        transaction,
        provider_id,
        binding.provider_genesis_policy_revision(),
    )?
    .ok_or_else(|| anyhow::anyhow!("节点 Provider 绑定的 Provider genesis 已缺失"))?;
    if genesis.provider_digest != binding.provider_genesis_digest()
        || genesis.provider.provider_id != provider_id
        || genesis.provider.provider_kind != PROVIDER_KIND_USER_NODE
        || genesis.provider.owner_account_id != expected_owner_user_id
        || genesis.provider.status != PROVIDER_STATUS_REGISTERING
    {
        bail!("节点 Provider 绑定的 Provider genesis 审计失败");
    }

    let Some(provider) = current_registered_provider_on(transaction, provider_id)? else {
        return Ok(None);
    };
    if provider.provider.provider_kind != PROVIDER_KIND_USER_NODE
        || provider.provider.owner_account_id != expected_owner_user_id
    {
        return Ok(None);
    }
    require_provider_lineage_on(
        transaction,
        provider_id,
        expected_owner_user_id,
        provider.provider.policy_revision,
    )?;

    let Some(endpoint) = current_node_endpoint_credential_for_user_node_provider_binding_on(
        transaction,
        binding.node_id(),
        expected_owner_user_id,
        binding.source_endpoint_credential_id(),
        binding.source_endpoint_credential_revision(),
        binding.source_endpoint_credential_digest(),
        binding.endpoint_installation_binding_digest(),
    )?
    else {
        return Ok(None);
    };
    let endpoint_binding = endpoint.binding();
    if endpoint_binding.agent_id() != binding.node_id() {
        return Ok(None);
    }

    require_initial_consent_source_on(transaction, binding)?;
    let Some(consent) = select_current_intent(transaction, binding.node_id())? else {
        return Ok(None);
    };
    let consent_current = consent.node_id == binding.node_id()
        && consent.owner_user_id == expected_owner_user_id
        && consent.installation_identity_digest == binding.installation_identity_digest()
        && consent.plugin_runtime_requested
        && consent.policy_revision >= binding.source_consent_policy_revision()
        && consent.authorization.as_ref().is_some_and(|authorization| {
            authorization.revision == consent.policy_revision
                && authorization.digest == consent.policy_digest
        });
    if !consent_current {
        return Ok(None);
    }

    Ok(Some(current_authority(
        receipt, provider, endpoint, consent,
    )))
}

pub(in crate::store) fn current_user_node_provider_binding_by_digest_on<'tx, 'conn>(
    transaction: &'tx Transaction<'conn>,
    provider_id: &str,
    expected_binding_digest: &str,
    expected_owner_user_id: &str,
) -> Result<Option<CurrentUserNodeProviderBindingAuthority<'tx, 'conn>>> {
    let Some(receipt) = binding_by_provider_on(transaction, provider_id)? else {
        return Ok(None);
    };
    if receipt.binding_digest() != expected_binding_digest {
        return Ok(None);
    }
    current_user_node_provider_binding_on(
        transaction,
        provider_id,
        receipt.binding_id(),
        expected_owner_user_id,
    )
}

pub(in crate::store) fn require_user_node_provider_activation_binding_on(
    transaction: &Transaction<'_>,
    provider_id: &str,
    binding_id: &str,
    expected_owner_user_id: &str,
    expected_provider_policy_revision: i64,
    expected_provider_digest: &str,
) -> Result<()> {
    let provider = current_registered_provider_on(transaction, provider_id)?
        .ok_or_else(|| anyhow::anyhow!("激活证据申请引用的 Provider 不存在"))?;
    if provider.provider.provider_kind != PROVIDER_KIND_USER_NODE {
        return Ok(());
    }
    if provider.provider.status != PROVIDER_STATUS_REGISTERING {
        bail!("只有 registering user_node Provider 可以提交激活证据申请");
    }
    let authority = current_user_node_provider_binding_on(
        transaction,
        provider_id,
        binding_id,
        expected_owner_user_id,
    )?
    .ok_or_else(|| anyhow::anyhow!("user_node 激活证据缺少当前节点安装绑定"))?;
    if authority.provider().provider.policy_revision != expected_provider_policy_revision
        || authority.provider().provider_digest != expected_provider_digest
    {
        bail!("user_node 激活证据的 Provider 版本已变化");
    }
    Ok(())
}

fn require_initial_consent_source_on(
    transaction: &Transaction<'_>,
    binding: &crate::compute_federation::user_node_provider_binding::UserNodeProviderBindingMaterial,
) -> Result<()> {
    let source = transaction
        .query_row(
            "SELECT 1 FROM node_compute_plugin_sharing_consents
              WHERE receipt_id=?1 AND node_id=?2 AND owner_user_id=?3
                AND installation_identity_digest=?4 AND policy_revision=?5
                AND policy_digest=?6 AND plugin_runtime_requested=1
                AND authorization_ref=?7 AND authorization_revision=?8
                AND authorization_digest=?9",
            params![
                binding.source_consent_receipt_id(),
                binding.node_id(),
                binding.owner_user_id(),
                binding.installation_identity_digest(),
                binding.source_consent_policy_revision(),
                binding.source_consent_policy_digest(),
                binding.source_authorization_ref(),
                binding.source_authorization_revision(),
                binding.source_authorization_digest(),
            ],
            |_| Ok(()),
        )
        .optional()?
        .is_some();
    if !source {
        bail!("节点 Provider 绑定的初始插件同意来源已缺失");
    }
    Ok(())
}

fn require_provider_lineage_on(
    transaction: &Transaction<'_>,
    provider_id: &str,
    owner_user_id: &str,
    current_policy_revision: i64,
) -> Result<()> {
    let exact_count = transaction.query_row(
        "SELECT COUNT(*) FROM compute_provider_versions lineage
          WHERE lineage.provider_id=?1
            AND lineage.policy_revision BETWEEN 1 AND ?2
            AND json_extract(lineage.provider_json,'$.schema')='compute_federation.provider.v1'
            AND json_extract(lineage.provider_json,'$.provider_id')=?1
            AND json_extract(lineage.provider_json,'$.provider_kind')='user_node'
            AND json_extract(lineage.provider_json,'$.owner_account_id')=?3
            AND json_extract(lineage.provider_json,'$.policy_revision')=
                lineage.policy_revision",
        params![provider_id, current_policy_revision, owner_user_id],
        |row| row.get::<_, i64>(0),
    )?;
    if exact_count != current_policy_revision {
        bail!("节点 Provider 绑定的 Provider revision lineage 不连续");
    }
    Ok(())
}
