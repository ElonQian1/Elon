use anyhow::{bail, Result};
use rusqlite::{params, OptionalExtension, Transaction};

use crate::node_compute_sharing::endpoint_authority::NodeEndpointCredentialBinding;

use super::{normalize::NormalizedRegistrationRequest, LegacyNodeEndpointAuthority};
use crate::store::node_credentials::endpoint_authority::{
    current_node_endpoint_root_by_agent_on, current_node_endpoint_root_by_owner_install_on,
};

pub(super) fn endpoint_authority_at_start_on(
    transaction: &Transaction<'_>,
    request: &NormalizedRegistrationRequest<'_>,
) -> Result<Option<LegacyNodeEndpointAuthority>> {
    let existing = request
        .existing_agent_id
        .map(|agent_id| current_node_endpoint_root_by_agent_on(transaction, agent_id))
        .transpose()?
        .flatten();
    let proposed = current_node_endpoint_root_by_agent_on(transaction, request.proposed_agent_id)?;
    let owner_install = request
        .install_id
        .map(|install_id| {
            current_node_endpoint_root_by_owner_install_on(
                transaction,
                request.owner_user_id,
                install_id,
            )
        })
        .transpose()?
        .flatten();

    if let (Some(existing_agent_id), Some(current)) =
        (request.existing_agent_id, owner_install.as_ref())
    {
        if current.agent_id() != existing_agent_id {
            bail!("NODE_ENDPOINT_AUTHORITY_IDENTITY_CONFLICT");
        }
    }
    merge_endpoint_authority_candidates(
        request.owner_user_id,
        request.install_id,
        [existing, proposed, owner_install],
    )
}

pub(super) fn endpoint_authority_at_end_on(
    transaction: &Transaction<'_>,
    request: &NormalizedRegistrationRequest<'_>,
    final_agent_id: &str,
) -> Result<Option<LegacyNodeEndpointAuthority>> {
    let by_agent = current_node_endpoint_root_by_agent_on(transaction, final_agent_id)?;
    let by_owner_install = request
        .install_id
        .map(|install_id| {
            current_node_endpoint_root_by_owner_install_on(
                transaction,
                request.owner_user_id,
                install_id,
            )
        })
        .transpose()?
        .flatten();
    if by_owner_install
        .as_ref()
        .is_some_and(|current| current.agent_id() != final_agent_id)
    {
        bail!("NODE_ENDPOINT_AUTHORITY_IDENTITY_CONFLICT");
    }
    merge_endpoint_authority_candidates(
        request.owner_user_id,
        request.install_id,
        [by_agent, by_owner_install, None],
    )
}

fn merge_endpoint_authority_candidates(
    owner_user_id: &str,
    install_id: Option<&str>,
    candidates: [Option<NodeEndpointCredentialBinding>; 3],
) -> Result<Option<LegacyNodeEndpointAuthority>> {
    let mut current: Option<NodeEndpointCredentialBinding> = None;
    for candidate in candidates.into_iter().flatten() {
        if candidate.owner_user_id() != owner_user_id
            || install_id.is_some_and(|install_id| candidate.install_id() != install_id)
        {
            bail!("NODE_ENDPOINT_AUTHORITY_IDENTITY_CONFLICT");
        }
        if current
            .as_ref()
            .is_some_and(|current| current != &candidate)
        {
            bail!("NODE_ENDPOINT_AUTHORITY_IDENTITY_CONFLICT");
        }
        current = Some(candidate);
    }
    Ok(current.map(Into::into))
}

pub(super) fn require_legacy_endpoint_authority_absent_on(
    transaction: &Transaction<'_>,
    agent_id: &str,
    owner_user_id: Option<&str>,
    install_id: Option<&str>,
) -> Result<()> {
    let owner_user_id = owner_user_id
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let install_id = install_id.map(str::trim).filter(|value| !value.is_empty());
    let by_agent = current_node_endpoint_root_by_agent_on(transaction, agent_id)?;
    let by_owner_install = match (owner_user_id, install_id) {
        (Some(owner_user_id), Some(install_id)) => {
            current_node_endpoint_root_by_owner_install_on(transaction, owner_user_id, install_id)?
        }
        _ => None,
    };
    if let Some(current) = by_agent.as_ref() {
        if owner_user_id.is_some_and(|owner_user_id| current.owner_user_id() != owner_user_id)
            || install_id.is_some_and(|install_id| current.install_id() != install_id)
        {
            bail!("NODE_ENDPOINT_AUTHORITY_IDENTITY_CONFLICT");
        }
    }
    if let Some(current) = by_owner_install.as_ref() {
        if current.agent_id() != agent_id
            || by_agent
                .as_ref()
                .is_some_and(|by_agent| by_agent != current)
        {
            bail!("NODE_ENDPOINT_AUTHORITY_IDENTITY_CONFLICT");
        }
    }
    if by_agent.is_some() || by_owner_install.is_some() {
        bail!("NODE_ENDPOINT_AUTHORITY_REQUIRED");
    }
    Ok(())
}

pub(super) fn require_legacy_credential_current_on(
    transaction: &Transaction<'_>,
    agent_id: &str,
    expected_owner_user_id: Option<&str>,
    expected_install_id: Option<&str>,
    expected_database_secret_hash: Option<&str>,
    allow_install_enrichment: bool,
) -> Result<()> {
    let expected_owner_user_id = expected_owner_user_id
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let expected_install_id = expected_install_id
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let anchor = transaction
        .query_row(
            "SELECT owner_user_id, install_id
               FROM node_credentials
              WHERE agent_id=?1",
            params![agent_id],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?)),
        )
        .optional()?;
    let authoritative_owner_user_id = anchor
        .as_ref()
        .map(|(owner_user_id, _)| owner_user_id.as_str())
        .or(expected_owner_user_id);
    let authoritative_install_id = anchor
        .as_ref()
        .and_then(|(_, install_id)| install_id.as_deref())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .or(expected_install_id);

    // Endpoint roots always win. Secret material is not read until both root keys are absent.
    require_legacy_endpoint_authority_absent_on(
        transaction,
        agent_id,
        authoritative_owner_user_id,
        authoritative_install_id,
    )?;

    match (anchor, expected_database_secret_hash) {
        (Some((owner_user_id, install_id)), Some(expected_secret_hash)) => {
            let install_id = install_id
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty());
            let install_is_current = install_id == expected_install_id
                || (allow_install_enrichment && install_id.is_none());
            if Some(owner_user_id.as_str()) != expected_owner_user_id || !install_is_current {
                bail!("LEGACY_NODE_CREDENTIAL_IDENTITY_NOT_CURRENT");
            }
            let current_secret_hash = transaction.query_row(
                "SELECT secret_hash FROM node_credentials WHERE agent_id=?1",
                params![agent_id],
                |row| row.get::<_, String>(0),
            )?;
            if !constant_time_eq(
                current_secret_hash.as_bytes(),
                expected_secret_hash.as_bytes(),
            ) {
                bail!("LEGACY_NODE_CREDENTIAL_SECRET_NOT_CURRENT");
            }
        }
        (Some(_), None) => bail!("LEGACY_NODE_ENV_AUTHORITY_SHADOWED_BY_DATABASE"),
        (None, Some(_)) => bail!("LEGACY_NODE_DATABASE_AUTHORITY_NOT_CURRENT"),
        (None, None) => {}
    }
    Ok(())
}

pub(super) fn verify_legacy_secret_proof_on(
    transaction: &Transaction<'_>,
    agent_id: &str,
    expected_owner_user_id: &str,
    presented_secret_hash: &str,
) -> Result<()> {
    let anchor = transaction
        .query_row(
            "SELECT owner_user_id, install_id
               FROM node_credentials
              WHERE agent_id=?1",
            params![agent_id],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?)),
        )
        .optional()?
        .ok_or_else(|| anyhow::anyhow!("节点凭证不存在"))?;
    let install_id = anchor
        .1
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());

    // A durable endpoint root disables every legacy hash-proof surface, not just /agent/ws.
    require_legacy_endpoint_authority_absent_on(
        transaction,
        agent_id,
        Some(&anchor.0),
        install_id,
    )?;
    if anchor.0 != expected_owner_user_id {
        bail!("节点不属于当前用户");
    }
    let current_secret_hash = transaction.query_row(
        "SELECT secret_hash FROM node_credentials WHERE agent_id=?1",
        params![agent_id],
        |row| row.get::<_, String>(0),
    )?;
    if !constant_time_eq(
        current_secret_hash.as_bytes(),
        presented_secret_hash.as_bytes(),
    ) {
        bail!("节点 secret 不匹配");
    }
    Ok(())
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    left.iter()
        .zip(right.iter())
        .fold(0u8, |diff, (left, right)| diff | (left ^ right))
        == 0
}
