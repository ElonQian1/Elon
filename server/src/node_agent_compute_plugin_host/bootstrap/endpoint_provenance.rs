use anyhow::{bail, Result};
use homecli_proto::NodeEndpointPlanningBootstrapSessionBindingV1Fields;

use crate::node_agent_endpoint_session::ValidatedEndpointSessionProvenance;

use super::{account::ComputePluginBootstrapAccountBinding, ComputePluginBootstrap};

const MAX_IJSON_SAFE_INTEGER: u64 = 9_007_199_254_740_991;

#[derive(Clone, PartialEq, Eq)]
pub(super) struct ComputePluginEndpointCredentialProvenance {
    agent_id: String,
    owner_user_id: String,
    install_id: String,
    credential_id: String,
    credential_revision: u64,
    credential_digest: String,
}

#[derive(Clone, PartialEq, Eq)]
struct ComputePluginEndpointSessionProvenance {
    credential: ComputePluginEndpointCredentialProvenance,
    installation_binding_digest: String,
    session_id: String,
    session_generation: u64,
    authentication_receipt_id: String,
    authentication_digest: String,
    server_instance_id: String,
    agent_version: String,
    capability_set_digest: String,
    authenticated_at: String,
    expires_at: String,
}

#[derive(Clone, PartialEq, Eq)]
pub(super) struct BoundComputePluginEndpointSessionProvenance {
    provenance: ComputePluginEndpointSessionProvenance,
    witness_id: String,
}

/// Linear process-local proof that Bootstrap is still bound to one exact endpoint socket.
///
/// It deliberately implements neither `Clone`, `Debug`, nor serialization. The endpoint loop must
/// retain it and revalidate both this witness and `EndpointSessionLease` around every stage.
pub(crate) struct ComputePluginEndpointSessionWitness {
    bound: BoundComputePluginEndpointSessionProvenance,
}

impl ComputePluginBootstrap {
    pub(crate) fn bind_endpoint_session_provenance(
        &self,
        validated: ValidatedEndpointSessionProvenance<'_>,
    ) -> Result<ComputePluginEndpointSessionWitness> {
        let provenance = ComputePluginEndpointSessionProvenance::new(validated)?;
        let mut state = self.state.lock().map_err(|_| {
            self.invalidate_policy_binding_intents_after_poison();
            anyhow::anyhow!("COMPUTE_PLUGIN_BOOTSTRAP_STATE_POISONED")
        })?;
        let Some(installation) = state.installation.as_ref() else {
            bail!("COMPUTE_PLUGIN_ENDPOINT_INSTALLATION_UNAVAILABLE");
        };
        if installation.install_id() != provenance.credential.install_id {
            bail!("COMPUTE_PLUGIN_ENDPOINT_INSTALLATION_MISMATCH");
        }

        let credential_replaced = state
            .endpoint_credential
            .as_ref()
            .is_none_or(|current| current != &provenance.credential);
        if credential_replaced {
            let account = ComputePluginBootstrapAccountBinding {
                node_id: provenance.credential.agent_id.clone(),
                owner_user_id: provenance.credential.owner_user_id.clone(),
            };
            self.replace_account_authority(
                &mut state,
                Some(account),
                Some(provenance.credential.clone()),
            );
        } else if state.account.as_ref().is_none_or(|account| {
            account.node_id != provenance.credential.agent_id
                || account.owner_user_id != provenance.credential.owner_user_id
        }) {
            self.replace_account_authority(&mut state, None, None);
            bail!("COMPUTE_PLUGIN_ENDPOINT_ACCOUNT_BINDING_MISMATCH");
        }

        let bound = BoundComputePluginEndpointSessionProvenance {
            provenance,
            witness_id: uuid::Uuid::new_v4().to_string(),
        };
        state.endpoint_session = Some(bound.clone());
        Ok(ComputePluginEndpointSessionWitness { bound })
    }

    pub(crate) fn require_endpoint_session_provenance(
        &self,
        witness: &ComputePluginEndpointSessionWitness,
    ) -> Result<()> {
        let state = self.state.lock().map_err(|_| {
            self.invalidate_policy_binding_intents_after_poison();
            anyhow::anyhow!("COMPUTE_PLUGIN_BOOTSTRAP_STATE_POISONED")
        })?;
        if state.endpoint_session.as_ref() != Some(&witness.bound) {
            bail!("COMPUTE_PLUGIN_ENDPOINT_SESSION_PROVENANCE_STALE");
        }
        Ok(())
    }

    pub(crate) fn release_endpoint_session_provenance(
        &self,
        witness: ComputePluginEndpointSessionWitness,
    ) {
        let mut state = match self.state.lock() {
            Ok(state) => state,
            Err(_) => {
                self.invalidate_policy_binding_intents_after_poison();
                return;
            }
        };
        if state.endpoint_session.as_ref() == Some(&witness.bound) {
            state.endpoint_session = None;
        }
    }
}

impl ComputePluginEndpointSessionProvenance {
    fn new(validated: ValidatedEndpointSessionProvenance<'_>) -> Result<Self> {
        let fields = validated.into_accepted_fields();
        if !bounded_identifier(&fields.agent_id, 160)
            || !bounded_identifier(&fields.owner_user_id, 160)
            || !bounded_identifier(&fields.install_id, 512)
            || !sha256_digest(&fields.installation_binding_digest)
            || !bounded_identifier(&fields.credential_id, 160)
            || !positive_safe_integer(fields.credential_revision)
            || !sha256_digest(&fields.credential_digest)
            || !bounded_identifier(&fields.session_id, 160)
            || !positive_safe_integer(fields.session_generation)
            || !bounded_identifier(&fields.authentication_receipt_id, 160)
            || !sha256_digest(&fields.authentication_digest)
            || !bounded_identifier(&fields.server_instance_id, 160)
            || !bounded_identifier(&fields.agent_version, 160)
            || !sha256_digest(&fields.capability_set_digest)
            || !bounded_timestamp(&fields.authenticated_at)
            || !bounded_timestamp(&fields.expires_at)
        {
            bail!("COMPUTE_PLUGIN_ENDPOINT_SESSION_PROVENANCE_INVALID");
        }
        Ok(Self {
            credential: ComputePluginEndpointCredentialProvenance {
                agent_id: fields.agent_id,
                owner_user_id: fields.owner_user_id,
                install_id: fields.install_id,
                credential_id: fields.credential_id,
                credential_revision: fields.credential_revision,
                credential_digest: fields.credential_digest,
            },
            installation_binding_digest: fields.installation_binding_digest,
            session_id: fields.session_id,
            session_generation: fields.session_generation,
            authentication_receipt_id: fields.authentication_receipt_id,
            authentication_digest: fields.authentication_digest,
            server_instance_id: fields.server_instance_id,
            agent_version: fields.agent_version,
            capability_set_digest: fields.capability_set_digest,
            authenticated_at: fields.authenticated_at,
            expires_at: fields.expires_at,
        })
    }
}

impl ComputePluginEndpointSessionWitness {
    pub(crate) fn node_id(&self) -> &str {
        &self.bound.provenance.credential.agent_id
    }

    pub(crate) fn owner_user_id(&self) -> &str {
        &self.bound.provenance.credential.owner_user_id
    }

    pub(crate) fn require_session_binding(
        &self,
        binding: &NodeEndpointPlanningBootstrapSessionBindingV1Fields,
    ) -> Result<()> {
        let provenance = &self.bound.provenance;
        let credential = &provenance.credential;
        if binding.agent_id != credential.agent_id
            || binding.owner_user_id != credential.owner_user_id
            || binding.install_id != credential.install_id
            || binding.installation_binding_digest != provenance.installation_binding_digest
            || binding.credential_id != credential.credential_id
            || binding.credential_revision != credential.credential_revision
            || binding.credential_digest != credential.credential_digest
            || binding.session_id != provenance.session_id
            || binding.session_generation != provenance.session_generation
            || binding.authentication_receipt_id != provenance.authentication_receipt_id
            || binding.authentication_digest != provenance.authentication_digest
            || binding.server_instance_id != provenance.server_instance_id
            || binding.agent_version != provenance.agent_version
            || binding.capability_set_digest != provenance.capability_set_digest
            || binding.authenticated_at != provenance.authenticated_at
            || binding.expires_at != provenance.expires_at
        {
            bail!("COMPUTE_PLUGIN_ENDPOINT_SESSION_BINDING_MISMATCH");
        }
        Ok(())
    }
}

fn bounded_identifier(value: &str, max_bytes: usize) -> bool {
    !value.is_empty()
        && value == value.trim()
        && value.len() <= max_bytes
        && !value.chars().any(|character| character.is_control())
}

fn positive_safe_integer(value: u64) -> bool {
    (1..=MAX_IJSON_SAFE_INTEGER).contains(&value)
}

fn bounded_timestamp(value: &str) -> bool {
    (20..=64).contains(&value.len())
        && value == value.trim()
        && !value.chars().any(|character| character.is_control())
}

fn sha256_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}
