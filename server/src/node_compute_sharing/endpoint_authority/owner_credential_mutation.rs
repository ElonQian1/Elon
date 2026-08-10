use anyhow::{bail, Result};
use serde::Serialize;

use super::{
    canonical::canonical_domain_json_and_digest,
    types::{bounded_identifier, is_sha256, safe_positive},
};

const MUTATION_REQUEST_SCHEMA: &str = "elon.node_endpoint.owner_credential_mutation_request.v1";
const MUTATION_REQUEST_DIGEST_DOMAIN: &[u8] =
    b"ELON_NODE_ENDPOINT_OWNER_CREDENTIAL_MUTATION_REQUEST_V1";

pub(crate) struct NodeEndpointOwnerCredentialMutationRequest {
    authorization_action: &'static str,
    authorization_issuance_request_id: String,
    credential_mutation_request_id: String,
    agent_id: String,
    install_id: String,
    expected: Option<ExpectedNodeEndpointCredential>,
    reason_code: Option<&'static str>,
}

pub(crate) struct ExpectedNodeEndpointCredential {
    credential_id: String,
    credential_revision: u64,
    credential_digest: String,
}

impl ExpectedNodeEndpointCredential {
    pub(crate) fn new(
        credential_id: String,
        credential_revision: u64,
        credential_digest: String,
    ) -> Result<Self> {
        let value = Self {
            credential_id,
            credential_revision,
            credential_digest,
        };
        value.validate()?;
        Ok(value)
    }

    pub(crate) fn credential_id(&self) -> &str {
        &self.credential_id
    }

    pub(crate) fn credential_revision(&self) -> u64 {
        self.credential_revision
    }

    pub(crate) fn credential_digest(&self) -> &str {
        &self.credential_digest
    }

    fn validate(&self) -> Result<()> {
        if !bounded_identifier(&self.credential_id, 160)
            || !safe_positive(self.credential_revision)
            || !is_sha256(&self.credential_digest)
        {
            bail!("NODE_ENDPOINT_OWNER_EXPECTED_CREDENTIAL_INVALID");
        }
        Ok(())
    }
}

impl NodeEndpointOwnerCredentialMutationRequest {
    pub(crate) fn issue(
        authorization_issuance_request_id: String,
        credential_mutation_request_id: String,
        agent_id: String,
        install_id: String,
        confirmed: bool,
    ) -> Result<Self> {
        Self::new(
            "initial_registration",
            authorization_issuance_request_id,
            credential_mutation_request_id,
            agent_id,
            install_id,
            None,
            None,
            confirmed,
        )
    }

    pub(crate) fn rotate(
        authorization_issuance_request_id: String,
        credential_mutation_request_id: String,
        agent_id: String,
        install_id: String,
        expected: ExpectedNodeEndpointCredential,
        confirmed: bool,
    ) -> Result<Self> {
        Self::new(
            "credential_rotation",
            authorization_issuance_request_id,
            credential_mutation_request_id,
            agent_id,
            install_id,
            Some(expected),
            None,
            confirmed,
        )
    }

    pub(crate) fn recover(
        authorization_issuance_request_id: String,
        credential_mutation_request_id: String,
        agent_id: String,
        install_id: String,
        expected: ExpectedNodeEndpointCredential,
        confirmed: bool,
    ) -> Result<Self> {
        Self::new(
            "account_recovery",
            authorization_issuance_request_id,
            credential_mutation_request_id,
            agent_id,
            install_id,
            Some(expected),
            None,
            confirmed,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn revoke(
        authorization_issuance_request_id: String,
        credential_mutation_request_id: String,
        agent_id: String,
        install_id: String,
        expected: ExpectedNodeEndpointCredential,
        reason_code: &str,
        confirmed: bool,
    ) -> Result<Self> {
        let reason_code = match reason_code {
            "owner_requested" => "owner_requested",
            "device_retired" => "device_retired",
            "suspected_compromise" => "suspected_compromise",
            _ => bail!("NODE_ENDPOINT_OWNER_REVOCATION_REASON_INVALID"),
        };
        Self::new(
            "owner_revocation",
            authorization_issuance_request_id,
            credential_mutation_request_id,
            agent_id,
            install_id,
            Some(expected),
            Some(reason_code),
            confirmed,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn new(
        authorization_action: &'static str,
        authorization_issuance_request_id: String,
        credential_mutation_request_id: String,
        agent_id: String,
        install_id: String,
        expected: Option<ExpectedNodeEndpointCredential>,
        reason_code: Option<&'static str>,
        confirmed: bool,
    ) -> Result<Self> {
        let value = Self {
            authorization_action,
            authorization_issuance_request_id,
            credential_mutation_request_id,
            agent_id,
            install_id,
            expected,
            reason_code,
        };
        if !confirmed {
            bail!("NODE_ENDPOINT_OWNER_CREDENTIAL_MUTATION_CONFIRMATION_REQUIRED");
        }
        value.validate()?;
        Ok(value)
    }

    pub(crate) fn authorization_action(&self) -> &str {
        self.authorization_action
    }

    pub(crate) fn authorization_issuance_request_id(&self) -> &str {
        &self.authorization_issuance_request_id
    }

    pub(crate) fn credential_mutation_request_id(&self) -> &str {
        &self.credential_mutation_request_id
    }

    pub(crate) fn agent_id(&self) -> &str {
        &self.agent_id
    }

    pub(crate) fn install_id(&self) -> &str {
        &self.install_id
    }

    pub(crate) fn expected(&self) -> Option<&ExpectedNodeEndpointCredential> {
        self.expected.as_ref()
    }

    pub(crate) fn reason_code(&self) -> Option<&str> {
        self.reason_code
    }

    pub(crate) fn returns_secret(&self) -> bool {
        self.authorization_action != "owner_revocation"
    }

    pub(crate) fn canonical_json_and_digest(&self) -> Result<(String, String)> {
        self.validate()?;
        #[derive(Serialize)]
        struct Envelope<'a> {
            schema: &'static str,
            authorization_action: &'a str,
            authorization_issuance_request_id: &'a str,
            credential_mutation_request_id: &'a str,
            agent_id: &'a str,
            install_id: &'a str,
            expected_credential_id: Option<&'a str>,
            expected_credential_revision: Option<u64>,
            expected_credential_digest: Option<&'a str>,
            reason_code: Option<&'a str>,
        }
        canonical_domain_json_and_digest(
            MUTATION_REQUEST_DIGEST_DOMAIN,
            &Envelope {
                schema: MUTATION_REQUEST_SCHEMA,
                authorization_action: self.authorization_action,
                authorization_issuance_request_id: &self.authorization_issuance_request_id,
                credential_mutation_request_id: &self.credential_mutation_request_id,
                agent_id: &self.agent_id,
                install_id: &self.install_id,
                expected_credential_id: self
                    .expected
                    .as_ref()
                    .map(ExpectedNodeEndpointCredential::credential_id),
                expected_credential_revision: self
                    .expected
                    .as_ref()
                    .map(ExpectedNodeEndpointCredential::credential_revision),
                expected_credential_digest: self
                    .expected
                    .as_ref()
                    .map(ExpectedNodeEndpointCredential::credential_digest),
                reason_code: self.reason_code,
            },
        )
    }

    fn validate(&self) -> Result<()> {
        if !bounded_request_id(&self.authorization_issuance_request_id)
            || !bounded_request_id(&self.credential_mutation_request_id)
            || !bounded_identifier(&self.agent_id, 160)
            || !bounded_identifier(&self.install_id, 512)
        {
            bail!("NODE_ENDPOINT_OWNER_CREDENTIAL_MUTATION_REQUEST_INVALID");
        }
        if let Some(expected) = &self.expected {
            expected.validate()?;
        }
        match (
            self.authorization_action,
            self.expected.is_some(),
            self.reason_code,
        ) {
            ("initial_registration", false, None)
            | ("credential_rotation" | "account_recovery", true, None)
            | ("owner_revocation", true, Some(_)) => Ok(()),
            _ => bail!("NODE_ENDPOINT_OWNER_CREDENTIAL_MUTATION_SHAPE_INVALID"),
        }
    }
}

fn bounded_request_id(value: &str) -> bool {
    let value = value.trim();
    (8..=160).contains(&value.len())
        && value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || "-_.:".contains(character))
}
