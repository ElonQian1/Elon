use anyhow::Result;
use serde::Deserialize;

use crate::node_compute_sharing::endpoint_authority::{
    ExpectedNodeEndpointCredential, NodeEndpointOwnerCredentialMutationRequest,
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::node_endpoint_transport) struct IssueCredentialRequest {
    authorization_issuance_request_id: String,
    credential_mutation_request_id: String,
    agent_id: String,
    install_id: String,
    password: String,
    confirm_issue: bool,
}

impl IssueCredentialRequest {
    pub(super) fn into_parts(self) -> Result<(NodeEndpointOwnerCredentialMutationRequest, String)> {
        let request = NodeEndpointOwnerCredentialMutationRequest::issue(
            self.authorization_issuance_request_id,
            self.credential_mutation_request_id,
            self.agent_id,
            self.install_id,
            self.confirm_issue,
        )?;
        Ok((request, self.password))
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::node_endpoint_transport) struct RotateCredentialRequest {
    authorization_issuance_request_id: String,
    credential_mutation_request_id: String,
    install_id: String,
    expected_credential: ExpectedCredentialRequest,
    password: String,
    confirm_rotation: bool,
}

impl RotateCredentialRequest {
    pub(super) fn into_parts(
        self,
        agent_id: String,
    ) -> Result<(NodeEndpointOwnerCredentialMutationRequest, String)> {
        let request = NodeEndpointOwnerCredentialMutationRequest::rotate(
            self.authorization_issuance_request_id,
            self.credential_mutation_request_id,
            agent_id,
            self.install_id,
            self.expected_credential.into_domain()?,
            self.confirm_rotation,
        )?;
        Ok((request, self.password))
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::node_endpoint_transport) struct RecoverCredentialRequest {
    authorization_issuance_request_id: String,
    credential_mutation_request_id: String,
    install_id: String,
    expected_credential: ExpectedCredentialRequest,
    password: String,
    confirm_recovery: bool,
}

impl RecoverCredentialRequest {
    pub(super) fn into_parts(
        self,
        agent_id: String,
    ) -> Result<(NodeEndpointOwnerCredentialMutationRequest, String)> {
        let request = NodeEndpointOwnerCredentialMutationRequest::recover(
            self.authorization_issuance_request_id,
            self.credential_mutation_request_id,
            agent_id,
            self.install_id,
            self.expected_credential.into_domain()?,
            self.confirm_recovery,
        )?;
        Ok((request, self.password))
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::node_endpoint_transport) struct RevokeCredentialRequest {
    authorization_issuance_request_id: String,
    credential_mutation_request_id: String,
    install_id: String,
    expected_credential: ExpectedCredentialRequest,
    password: String,
    reason_code: String,
    confirm_revocation: bool,
}

impl RevokeCredentialRequest {
    pub(super) fn into_parts(
        self,
        agent_id: String,
    ) -> Result<(NodeEndpointOwnerCredentialMutationRequest, String)> {
        let request = NodeEndpointOwnerCredentialMutationRequest::revoke(
            self.authorization_issuance_request_id,
            self.credential_mutation_request_id,
            agent_id,
            self.install_id,
            self.expected_credential.into_domain()?,
            &self.reason_code,
            self.confirm_revocation,
        )?;
        Ok((request, self.password))
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ExpectedCredentialRequest {
    credential_id: String,
    credential_revision: u64,
    credential_digest: String,
}

impl ExpectedCredentialRequest {
    fn into_domain(self) -> Result<ExpectedNodeEndpointCredential> {
        ExpectedNodeEndpointCredential::new(
            self.credential_id,
            self.credential_revision,
            self.credential_digest,
        )
    }
}
