use anyhow::{bail, Result};
use rusqlite::Transaction;

use crate::node_compute_sharing::endpoint_authority::{
    derive_node_endpoint_installation_binding_digest, NodeEndpointCredentialBinding,
    NodeEndpointOwnerCredentialMutationRequest,
    NodeEndpointOwnerReauthenticationConsumptionEnvelope, PresentedNodeEndpointCredentialSecret,
    VerifiedSecureOwnerApiTransport,
};

use super::{
    super::{credentials, owner_reauthentication},
    current_account::CurrentOwnerAccountSource,
};

pub(super) struct ReplayedOwnerCredentialMutation {
    consumption: NodeEndpointOwnerReauthenticationConsumptionEnvelope,
    consumption_digest: String,
    committed: NodeEndpointCredentialBinding,
    result_is_current: bool,
}

impl ReplayedOwnerCredentialMutation {
    pub(super) fn consumption(&self) -> &NodeEndpointOwnerReauthenticationConsumptionEnvelope {
        &self.consumption
    }

    pub(super) fn consumption_digest(&self) -> &str {
        &self.consumption_digest
    }

    pub(super) fn committed(&self) -> &NodeEndpointCredentialBinding {
        &self.committed
    }

    pub(super) fn result_is_current(&self) -> bool {
        self.result_is_current
    }
}

pub(super) fn read_exact_on(
    transaction: &Transaction<'_>,
    account: &CurrentOwnerAccountSource,
    request: &NodeEndpointOwnerCredentialMutationRequest,
    transport: &VerifiedSecureOwnerApiTransport,
    presented: Option<&PresentedNodeEndpointCredentialSecret>,
) -> Result<Option<ReplayedOwnerCredentialMutation>> {
    let Some(stored) = owner_reauthentication::consumption_rows::by_owner_mutation_request_on(
        transaction,
        account.owner_user_id(),
        request.credential_mutation_request_id(),
    )?
    else {
        return Ok(None);
    };
    let (_, mutation_digest) = request.canonical_json_and_digest()?;
    transport.validate_for_mutation(
        request.authorization_action(),
        request.agent_id(),
        &mutation_digest,
    )?;
    let envelope = stored.envelope();
    let source = owner_reauthentication::receipt_by_id_on(
        transaction,
        envelope.reauthentication_receipt_id(),
    )?
    .ok_or_else(|| anyhow::anyhow!("NODE_ENDPOINT_OWNER_REAUTHENTICATION_SOURCE_MISSING"))?;
    if source.1 != envelope.reauthentication_digest()
        || source.0.owner_user_id() != account.owner_user_id()
        || source.0.authorization_action() != request.authorization_action()
        || source.0.authorization_issuance_request_id()
            != request.authorization_issuance_request_id()
        || source.0.credential_mutation_request_id() != request.credential_mutation_request_id()
        || source.0.credential_mutation_request_digest() != mutation_digest
        || source.0.agent_id() != request.agent_id()
        || source.0.install_id() != request.install_id()
        || !expected_matches(request, &source.0)
    {
        bail!("NODE_ENDPOINT_OWNER_CREDENTIAL_MUTATION_REPLAY_MISMATCH");
    }
    verify_rotation_possession_on(transaction, request, &source.0, presented)?;
    let committed = NodeEndpointCredentialBinding::from_store_readback(
        envelope.current_credential_id().to_string(),
        source.0.agent_id().to_string(),
        source.0.owner_user_id().to_string(),
        source.0.install_id().to_string(),
        derive_node_endpoint_installation_binding_digest(
            source.0.agent_id(),
            source.0.owner_user_id(),
            source.0.install_id(),
        )?,
        envelope.current_credential_revision(),
        envelope.current_credential_digest().to_string(),
        envelope.current_credential_status().to_string(),
    )?;
    let current = credentials::current_binding_by_agent_on(transaction, request.agent_id())?;
    let result_is_current = current.as_ref().is_some_and(|value| {
        value.credential_id() == committed.credential_id()
            && value.credential_revision() == committed.credential_revision()
            && value.credential_digest() == committed.credential_digest()
            && value.status() == committed.status()
    });
    let consumption_digest = stored.consumption_digest().to_string();
    let consumption = stored.into_envelope();
    Ok(Some(ReplayedOwnerCredentialMutation {
        consumption,
        consumption_digest,
        committed,
        result_is_current,
    }))
}

fn verify_rotation_possession_on(
    transaction: &Transaction<'_>,
    request: &NodeEndpointOwnerCredentialMutationRequest,
    source: &crate::node_compute_sharing::endpoint_authority::NodeEndpointOwnerReauthenticationEnvelope,
    presented: Option<&PresentedNodeEndpointCredentialSecret>,
) -> Result<()> {
    if request.authorization_action() != "credential_rotation" {
        if presented.is_some() {
            bail!("NODE_ENDPOINT_OWNER_CREDENTIAL_POSSESSION_SHAPE_INVALID");
        }
        return Ok(());
    }
    let expected = request
        .expected()
        .ok_or_else(|| anyhow::anyhow!("NODE_ENDPOINT_OWNER_EXPECTED_CREDENTIAL_MISSING"))?;
    let presented =
        presented.ok_or_else(|| anyhow::anyhow!("NODE_ENDPOINT_CREDENTIAL_POSSESSION_REQUIRED"))?;
    let historical = NodeEndpointCredentialBinding::from_store_readback(
        expected.credential_id().to_string(),
        source.agent_id().to_string(),
        source.owner_user_id().to_string(),
        source.install_id().to_string(),
        derive_node_endpoint_installation_binding_digest(
            source.agent_id(),
            source.owner_user_id(),
            source.install_id(),
        )?,
        expected.credential_revision(),
        expected.credential_digest().to_string(),
        "active".to_string(),
    )?;
    credentials::verify_bound_secret_on(transaction, &historical, presented)
}

fn expected_matches(
    request: &NodeEndpointOwnerCredentialMutationRequest,
    source: &crate::node_compute_sharing::endpoint_authority::NodeEndpointOwnerReauthenticationEnvelope,
) -> bool {
    match request.expected() {
        None => {
            source.expected_credential_id().is_none()
                && source.expected_credential_revision().is_none()
                && source.expected_credential_digest().is_none()
        }
        Some(expected) => {
            source.expected_credential_id() == Some(expected.credential_id())
                && source.expected_credential_revision() == Some(expected.credential_revision())
                && source.expected_credential_digest() == Some(expected.credential_digest())
        }
    }
}
