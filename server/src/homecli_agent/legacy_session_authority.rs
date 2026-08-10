use std::{collections::HashMap, sync::Arc};

use anyhow::{anyhow, Result};
use homecli_proto::{NodeDevRuntimeProfile, NodeHardwareProfile};
use sha2::Digest as _;

use crate::types::AppState;

use super::DURABLE_CLI_COMPLETION_PROTO_VERSION;

pub(super) struct AuthorizedLegacySessionRegistration {
    pub(super) owner_user_id: Option<String>,
    pub(super) install_id: Option<String>,
    pub(super) credential_proof: LegacySessionCredentialProof,
}

pub(super) enum LegacySessionCredentialProof {
    EnvironmentOnly,
    Database { secret_hash: String },
}

impl LegacySessionCredentialProof {
    pub(super) fn database_secret_hash(&self) -> Option<&str> {
        match self {
            Self::EnvironmentOnly => None,
            Self::Database { secret_hash } => Some(secret_hash),
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn authenticate_and_prepare(
    state: &Arc<AppState>,
    secrets: &HashMap<String, String>,
    presented_token: &str,
    agent_id: &str,
    version: &str,
    proto_version: u32,
    allowed_clis: &[String],
    claimed_owner_user_id: Option<&str>,
    device_name: Option<&str>,
    claimed_install_id: Option<&str>,
    hardware: Option<&NodeHardwareProfile>,
    dev_runtime: Option<&NodeDevRuntimeProfile>,
) -> Result<AuthorizedLegacySessionRegistration> {
    // The Store reads a DB secret candidate only after rejecting both the agent root and the
    // authoritative owner/install root. Environment secrets are compared only after that gate.
    let legacy_auth = state
        .store
        .legacy_node_websocket_auth_candidate(agent_id, claimed_owner_user_id, claimed_install_id)
        .map_err(|error| anyhow!("legacy endpoint authority gate for {agent_id}: {error}"))?;
    // A durable DB anchor owns its namespace. Environment secrets are accepted only when no DB
    // row exists, so an operator override cannot silently downgrade a registered credential.
    let credential_proof = if let Some(expected_hash) = legacy_auth.database_secret_hash() {
        let presented_hash = hex::encode(sha2::Sha256::digest(presented_token.as_bytes()));
        if !constant_time_eq(expected_hash.as_bytes(), presented_hash.as_bytes()) {
            return Err(anyhow!("auth failed for agent_id={agent_id}"));
        }
        LegacySessionCredentialProof::Database {
            secret_hash: expected_hash.to_string(),
        }
    } else {
        let expected = secrets
            .get(agent_id)
            .ok_or_else(|| anyhow!("auth failed for agent_id={agent_id}"))?;
        if !constant_time_eq(expected.as_bytes(), presented_token.as_bytes()) {
            return Err(anyhow!("auth failed for agent_id={agent_id}"));
        }
        LegacySessionCredentialProof::EnvironmentOnly
    };

    let database_bound = legacy_auth.is_database_bound();
    let stored_owner_user_id = legacy_auth.owner_user_id().map(str::to_string);
    if let (Some(claimed), Some(stored)) = (claimed_owner_user_id, stored_owner_user_id.as_deref())
    {
        if claimed != stored {
            return Err(anyhow!(
                "registered owner does not match credential owner for agent_id={agent_id}"
            ));
        }
    }
    if proto_version >= DURABLE_CLI_COMPLETION_PROTO_VERSION {
        let claimed_install_id = claimed_install_id.ok_or_else(|| {
            anyhow!("durable completion protocol requires install_id for agent_id={agent_id}")
        })?;
        if let Some(stored_install_id) = legacy_auth
            .install_id()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            if claimed_install_id != stored_install_id {
                return Err(anyhow!(
                    "registered install_id does not match credential installation for agent_id={agent_id}"
                ));
            }
        }
    }

    let resolved_owner_user_id =
        stored_owner_user_id.or_else(|| claimed_owner_user_id.map(str::to_string));
    let resolved_install_id = legacy_auth
        .install_id()
        .map(str::to_string)
        .or_else(|| claimed_install_id.map(str::to_string));
    if proto_version >= DURABLE_CLI_COMPLETION_PROTO_VERSION
        && resolved_owner_user_id
            .as_deref()
            .map(str::trim)
            .is_none_or(str::is_empty)
    {
        return Err(anyhow!(
            "durable completion protocol requires an authoritative owner for agent_id={agent_id}"
        ));
    }

    state
        .agent_manager
        .with_endpoint_authority_write_fence(|| -> Result<()> {
            state
                .store
                .require_legacy_node_websocket_preparation_current(
                    agent_id,
                    resolved_owner_user_id.as_deref(),
                    resolved_install_id.as_deref(),
                    credential_proof.database_secret_hash(),
                )?;
            if let Some(owner) = &resolved_owner_user_id {
                if let Err(error) = state.store.update_node_credential_registration_info(
                    agent_id,
                    owner,
                    claimed_install_id,
                    device_name,
                ) {
                    if proto_version >= DURABLE_CLI_COMPLETION_PROTO_VERSION && database_bound {
                        return Err(anyhow!(
                            "failed to bind durable node installation for agent_id={agent_id}: {error}"
                        ));
                    }
                    tracing::warn!(%agent_id, %error, "failed to update node registration info");
                }
            }
            if proto_version >= DURABLE_CLI_COMPLETION_PROTO_VERSION && database_bound {
                let verified = state
                    .store
                    .get_node_credential(agent_id)
                    .map_err(|error| anyhow!("verify node installation for {agent_id}: {error}"))?
                    .ok_or_else(|| anyhow!("node credential disappeared for agent_id={agent_id}"))?;
                if Some(verified.owner_user_id.as_str()) != resolved_owner_user_id.as_deref()
                    || verified.install_id.as_deref() != claimed_install_id
                {
                    return Err(anyhow!(
                        "durable node owner/install binding was not persisted for agent_id={agent_id}"
                    ));
                }
            }
            if let (Some(owner), Some(hardware)) = (&resolved_owner_user_id, hardware) {
                if let Err(error) =
                    state
                        .store
                        .upsert_node_hardware_snapshot(agent_id, owner, device_name, hardware)
                {
                    tracing::warn!(%agent_id, %error, "failed to update node hardware snapshot");
                }
            }
            if let Some(owner) = &resolved_owner_user_id {
                if let Err(error) = state.store.record_node_handshake(
                    agent_id,
                    owner,
                    version,
                    allowed_clis,
                    dev_runtime,
                ) {
                    tracing::warn!(%agent_id, %error, "failed to record node handshake");
                }
            }
            Ok(())
        })
        .await?;

    Ok(AuthorizedLegacySessionRegistration {
        owner_user_id: resolved_owner_user_id,
        install_id: resolved_install_id,
        credential_proof,
    })
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
