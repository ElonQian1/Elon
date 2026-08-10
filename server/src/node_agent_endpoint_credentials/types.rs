use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

pub(super) const STORE_SCHEMA: &str = "elon.node-endpoint-credential.v1";
pub(super) const STORE_FILE: &str = "node-endpoint-credential.v1.json";
pub(super) const WINDOWS_PROTECTION: &str = "WINDOWS_DPAPI_CURRENT_USER";

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct EndpointAuthorityBinding {
    pub(crate) agent_id: String,
    pub(crate) owner_user_id: String,
    pub(crate) install_id: String,
    pub(crate) credential_id: String,
    pub(crate) credential_revision: u64,
    pub(crate) credential_digest: String,
    pub(crate) status: String,
}

impl EndpointAuthorityBinding {
    pub(crate) fn validate(&self) -> Result<()> {
        if !bounded_identifier(&self.agent_id, 160)
            || !bounded_identifier(&self.owner_user_id, 160)
            || !bounded_identifier(&self.install_id, 512)
            || !bounded_identifier(&self.credential_id, 160)
            || self.credential_revision == 0
            || !sha256_digest(&self.credential_digest)
            || !matches!(self.status.as_str(), "active" | "revoked")
        {
            bail!("NODE_ENDPOINT_CREDENTIAL_BINDING_INVALID");
        }
        Ok(())
    }

    pub(super) fn same_credential(&self, other: &Self) -> bool {
        self.agent_id == other.agent_id
            && self.owner_user_id == other.owner_user_id
            && self.install_id == other.install_id
            && self.credential_id == other.credential_id
            && self.credential_revision == other.credential_revision
            && self.credential_digest == other.credential_digest
            && self.status == other.status
    }

    pub(super) fn expected(&self) -> ExpectedEndpointCredential {
        ExpectedEndpointCredential {
            credential_id: self.credential_id.clone(),
            credential_revision: self.credential_revision,
            credential_digest: self.credential_digest.clone(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ExpectedEndpointCredential {
    pub(super) credential_id: String,
    pub(super) credential_revision: u64,
    pub(super) credential_digest: String,
}

impl ExpectedEndpointCredential {
    pub(super) fn validate(&self) -> Result<()> {
        if !bounded_identifier(&self.credential_id, 160)
            || self.credential_revision == 0
            || !sha256_digest(&self.credential_digest)
        {
            bail!("NODE_ENDPOINT_EXPECTED_CREDENTIAL_INVALID");
        }
        Ok(())
    }
}

/// The sole in-process plaintext owner. This type intentionally implements
/// neither Clone, Debug nor Serialize, and zeroes its allocation on Drop.
pub(super) struct EndpointSecret(Vec<u8>);

impl Drop for EndpointSecret {
    fn drop(&mut self) {
        self.0.fill(0);
    }
}

impl EndpointSecret {
    pub(super) fn from_bytes(bytes: Vec<u8>) -> Result<Self> {
        let secret = Self(bytes);
        let value = std::str::from_utf8(&secret.0)
            .map_err(|_| anyhow::anyhow!("NODE_ENDPOINT_SECRET_INVALID"))?;
        if value.len() != 43
            || !value
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
        {
            bail!("NODE_ENDPOINT_SECRET_INVALID");
        }
        Ok(secret)
    }

    pub(super) fn from_string(value: String) -> Result<Self> {
        Self::from_bytes(value.into_bytes())
    }

    pub(super) fn plaintext_bytes(&self) -> &[u8] {
        &self.0
    }
}

pub(super) struct CurrentEndpointCredential {
    pub(super) binding: EndpointAuthorityBinding,
    pub(super) secret: Option<EndpointSecret>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum PendingMutationAction {
    Issue,
    Recover,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct PendingEndpointMutation {
    pub(super) action: PendingMutationAction,
    pub(super) authorization_issuance_request_id: String,
    pub(super) credential_mutation_request_id: String,
    pub(super) agent_id: String,
    pub(super) owner_user_id: String,
    pub(super) install_id: String,
    pub(super) expected_credential: Option<ExpectedEndpointCredential>,
    pub(super) prepared_at: String,
}

impl PendingEndpointMutation {
    pub(super) fn validate(&self) -> Result<()> {
        if !bounded_request_id(&self.authorization_issuance_request_id)
            || !bounded_request_id(&self.credential_mutation_request_id)
            || !bounded_identifier(&self.agent_id, 160)
            || !bounded_identifier(&self.owner_user_id, 160)
            || !bounded_identifier(&self.install_id, 512)
            || !(20..=64).contains(&self.prepared_at.len())
            || chrono::DateTime::parse_from_rfc3339(&self.prepared_at).is_err()
        {
            bail!("NODE_ENDPOINT_PENDING_MUTATION_INVALID");
        }
        match (self.action, self.expected_credential.as_ref()) {
            (PendingMutationAction::Issue, None) => Ok(()),
            (PendingMutationAction::Recover, Some(expected)) => expected.validate(),
            _ => bail!("NODE_ENDPOINT_PENDING_MUTATION_INVALID"),
        }
    }
}

pub(super) struct EndpointCredentialState {
    pub(super) endpoint_required: bool,
    pub(super) endpoint_https_origin: Option<String>,
    pub(super) current: Option<CurrentEndpointCredential>,
    pub(super) pending_mutation: Option<PendingEndpointMutation>,
}

impl EndpointCredentialState {
    pub(super) fn absent() -> Self {
        Self {
            endpoint_required: false,
            endpoint_https_origin: None,
            current: None,
            pending_mutation: None,
        }
    }
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct PersistedEndpointCredentialState {
    pub(super) schema: String,
    pub(super) endpoint_required: bool,
    pub(super) endpoint_https_origin: String,
    pub(super) current: Option<PersistedCurrentEndpointCredential>,
    pub(super) pending_mutation: Option<PendingEndpointMutation>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct PersistedCurrentEndpointCredential {
    pub(super) binding: EndpointAuthorityBinding,
    pub(super) protection: String,
    pub(super) protected_secret_base64: Option<String>,
}

fn bounded_identifier(value: &str, max: usize) -> bool {
    !value.is_empty()
        && value == value.trim()
        && value.len() <= max
        && value
            .chars()
            .all(|character| !character.is_control() && character != '\0')
}

fn bounded_request_id(value: &str) -> bool {
    (8..=160).contains(&value.len())
        && value == value.trim()
        && value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || "-_.:".contains(character))
}

fn sha256_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}
