use anyhow::{bail, Result};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

use super::super::{
    canonical::{
        canonical_domain_json_and_digest, deterministic_identifier, ensure_canonical_readback,
        ensure_time_order, parse_utc_nanos, utc_nanos,
    },
    types::{bounded_identifier, is_sha256, NodeEndpointCredentialBinding},
};

const OWNER_REAUTHENTICATION_SCHEMA: &str = "elon.node_endpoint.owner_reauthentication.v1";
const OWNER_REAUTHENTICATION_ID_DOMAIN: &[u8] = b"ELON_NODE_ENDPOINT_OWNER_REAUTHENTICATION_ID_V1";
const OWNER_REAUTHENTICATION_DIGEST_DOMAIN: &[u8] =
    b"ELON_NODE_ENDPOINT_OWNER_REAUTHENTICATION_RECEIPT_V1";
const AUTHORIZATION_TARGET_DOMAIN: &[u8] = b"ELON_NODE_ENDPOINT_OWNER_AUTHORIZATION_TARGET_V1";
const REAUTHENTICATION_LIFETIME_MINUTES: i64 = 5;
const MAX_TRANSPORT_TO_REAUTH_SECONDS: i64 = 30;

/// Durable account-session facts produced only by a future Store-private bearer verifier.
pub(crate) struct VerifiedCurrentAccountSession {
    owner_user_id: String,
    account_session_id: String,
    session_binding_digest: String,
    account_auth_state_digest: String,
    verified_at: DateTime<Utc>,
    session_expires_at: DateTime<Utc>,
}

/// Fresh, purpose-bound factor verification. Login, device trust, and token touch are insufficient.
pub(crate) struct VerifiedRecentOwnerReauthentication {
    owner_user_id: String,
    account_session_id: String,
    session_binding_digest: String,
    account_auth_state_digest: String,
    authentication_method: String,
    authentication_factor_id: String,
    authentication_factor_binding_digest: String,
    authentication_evidence_id: String,
    authentication_evidence_digest: String,
    authorization_action: String,
    credential_mutation_request_id: String,
    credential_mutation_request_digest: String,
    authorization_target_digest: String,
    reauthenticated_at: DateTime<Utc>,
    expires_at: DateTime<Utc>,
}

/// Request-scoped HTTPS evidence. The endpoint WSS proof has a different audience and cannot be
/// converted into this type.
pub(crate) struct VerifiedSecureOwnerApiTransport {
    source: String,
    evidence_schema: String,
    evidence_id: String,
    evidence_digest: String,
    verifier_revision: u64,
    verifier_digest: String,
    server_instance_id: String,
    request_binding_digest: String,
    verified_at: DateTime<Utc>,
}

/// Sealed authorization input. No constructor is exposed in this batch, so the Store kernel stays
/// dormant until account-session, recent-factor, and secure owner-API producers are all present.
pub(crate) struct AuthorizedNodeEndpointOwnerReauthentication {
    session: VerifiedCurrentAccountSession,
    reauthentication: VerifiedRecentOwnerReauthentication,
    transport: VerifiedSecureOwnerApiTransport,
    agent_id: String,
    install_id: String,
    expected_credential: Option<NodeEndpointCredentialBinding>,
    authorization_issuance_request_id: String,
}

impl AuthorizedNodeEndpointOwnerReauthentication {
    pub(crate) fn owner_user_id(&self) -> &str {
        &self.session.owner_user_id
    }

    pub(crate) fn account_session_id(&self) -> &str {
        &self.session.account_session_id
    }

    pub(crate) fn authorization_issuance_request_id(&self) -> &str {
        &self.authorization_issuance_request_id
    }

    pub(crate) fn prepare(
        &self,
        recorded_at: DateTime<Utc>,
    ) -> Result<PreparedNodeEndpointOwnerReauthentication> {
        self.validate_sources(recorded_at)?;
        let expected = self.expected_credential.as_ref();
        let target_digest = authorization_target_digest_from_parts(
            &self.reauthentication.authorization_action,
            &self.session.owner_user_id,
            &self.agent_id,
            &self.install_id,
            expected.map(NodeEndpointCredentialBinding::credential_id),
            expected.map(NodeEndpointCredentialBinding::credential_revision),
            expected.map(NodeEndpointCredentialBinding::credential_digest),
            &self.reauthentication.credential_mutation_request_id,
            &self.reauthentication.credential_mutation_request_digest,
        )?;
        if target_digest != self.reauthentication.authorization_target_digest {
            bail!("NODE_ENDPOINT_OWNER_REAUTHENTICATION_TARGET_MISMATCH");
        }
        #[derive(Serialize)]
        struct Identity<'a> {
            owner_user_id: &'a str,
            authorization_issuance_request_id: &'a str,
            authentication_evidence_digest: &'a str,
            authorization_target_digest: &'a str,
        }
        let receipt_id = deterministic_identifier(
            "nerauth_",
            OWNER_REAUTHENTICATION_ID_DOMAIN,
            &Identity {
                owner_user_id: &self.session.owner_user_id,
                authorization_issuance_request_id: &self.authorization_issuance_request_id,
                authentication_evidence_digest: &self
                    .reauthentication
                    .authentication_evidence_digest,
                authorization_target_digest: &target_digest,
            },
        )?;
        let envelope = NodeEndpointOwnerReauthenticationEnvelope {
            schema: OWNER_REAUTHENTICATION_SCHEMA.to_string(),
            reauthentication_receipt_id: receipt_id,
            owner_user_id: self.session.owner_user_id.clone(),
            account_session_id: self.session.account_session_id.clone(),
            session_binding_digest: self.session.session_binding_digest.clone(),
            account_auth_state_digest: self.session.account_auth_state_digest.clone(),
            authentication_method: self.reauthentication.authentication_method.clone(),
            authentication_factor_id: self.reauthentication.authentication_factor_id.clone(),
            authentication_factor_binding_digest: self
                .reauthentication
                .authentication_factor_binding_digest
                .clone(),
            authentication_evidence_id: self.reauthentication.authentication_evidence_id.clone(),
            authentication_evidence_digest: self
                .reauthentication
                .authentication_evidence_digest
                .clone(),
            authorization_issuance_request_id: self.authorization_issuance_request_id.clone(),
            authorization_action: self.reauthentication.authorization_action.clone(),
            credential_mutation_request_id: self
                .reauthentication
                .credential_mutation_request_id
                .clone(),
            credential_mutation_request_digest: self
                .reauthentication
                .credential_mutation_request_digest
                .clone(),
            authorization_target_digest: target_digest,
            agent_id: self.agent_id.clone(),
            install_id: self.install_id.clone(),
            expected_credential_id: expected.map(|value| value.credential_id().to_string()),
            expected_credential_revision: expected.map(|value| value.credential_revision()),
            expected_credential_digest: expected.map(|value| value.credential_digest().to_string()),
            secure_transport_source: self.transport.source.clone(),
            secure_transport_evidence_schema: self.transport.evidence_schema.clone(),
            secure_transport_evidence_id: self.transport.evidence_id.clone(),
            secure_transport_evidence_digest: self.transport.evidence_digest.clone(),
            secure_transport_verifier_revision: self.transport.verifier_revision,
            secure_transport_verifier_digest: self.transport.verifier_digest.clone(),
            secure_transport_server_instance_id: self.transport.server_instance_id.clone(),
            secure_transport_request_binding_digest: self.transport.request_binding_digest.clone(),
            secure_transport_verified_at: utc_nanos(self.transport.verified_at),
            reauthenticated_at: utc_nanos(self.reauthentication.reauthenticated_at),
            expires_at: utc_nanos(self.reauthentication.expires_at),
            recorded_at: utc_nanos(recorded_at),
        };
        envelope.validate()?;
        let (receipt_json, receipt_digest) =
            canonical_domain_json_and_digest(OWNER_REAUTHENTICATION_DIGEST_DOMAIN, &envelope)?;
        Ok(PreparedNodeEndpointOwnerReauthentication {
            envelope,
            receipt_json,
            receipt_digest,
        })
    }

    fn validate_sources(&self, recorded_at: DateTime<Utc>) -> Result<()> {
        let reauth = &self.reauthentication;
        if self.session.owner_user_id != reauth.owner_user_id
            || self.session.account_session_id != reauth.account_session_id
            || self.session.session_binding_digest != reauth.session_binding_digest
            || self.session.account_auth_state_digest != reauth.account_auth_state_digest
            || !is_sha256(&self.session.session_binding_digest)
            || !is_sha256(&self.session.account_auth_state_digest)
            || !matches!(
                reauth.authentication_method.as_str(),
                "password" | "google_oidc"
            )
            || !matches!(
                self.transport.source.as_str(),
                "direct_tls" | "trusted_proxy_mtls"
            )
            || self.transport.request_binding_digest != reauth.credential_mutation_request_digest
        {
            bail!("NODE_ENDPOINT_OWNER_REAUTHENTICATION_SOURCE_MISMATCH");
        }
        ensure_time_order(
            self.session.verified_at,
            reauth.reauthenticated_at,
            "NODE_ENDPOINT_ACCOUNT_SESSION_VERIFIED_AFTER_REAUTHENTICATION",
        )?;
        ensure_time_order(
            self.transport.verified_at,
            reauth.reauthenticated_at,
            "NODE_ENDPOINT_OWNER_TRANSPORT_VERIFIED_AFTER_REAUTHENTICATION",
        )?;
        if self
            .transport
            .verified_at
            .checked_add_signed(Duration::seconds(MAX_TRANSPORT_TO_REAUTH_SECONDS))
            .is_none_or(|deadline| reauth.reauthenticated_at > deadline)
        {
            bail!("NODE_ENDPOINT_OWNER_TRANSPORT_REAUTHENTICATION_WINDOW_EXPIRED");
        }
        let expected_expiry = reauth
            .reauthenticated_at
            .checked_add_signed(Duration::minutes(REAUTHENTICATION_LIFETIME_MINUTES))
            .ok_or_else(|| {
                anyhow::anyhow!("NODE_ENDPOINT_OWNER_REAUTHENTICATION_EXPIRY_OVERFLOW")
            })?;
        if reauth.expires_at != expected_expiry
            || recorded_at < reauth.reauthenticated_at
            || recorded_at >= reauth.expires_at
            || recorded_at >= self.session.session_expires_at
        {
            bail!("NODE_ENDPOINT_OWNER_REAUTHENTICATION_TIME_INVALID");
        }
        validate_target_shape(
            &reauth.authorization_action,
            &self.session.owner_user_id,
            &self.agent_id,
            &self.install_id,
            self.expected_credential.as_ref(),
        )
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct NodeEndpointOwnerReauthenticationEnvelope {
    schema: String,
    reauthentication_receipt_id: String,
    owner_user_id: String,
    account_session_id: String,
    session_binding_digest: String,
    account_auth_state_digest: String,
    authentication_method: String,
    authentication_factor_id: String,
    authentication_factor_binding_digest: String,
    authentication_evidence_id: String,
    authentication_evidence_digest: String,
    authorization_issuance_request_id: String,
    authorization_action: String,
    credential_mutation_request_id: String,
    credential_mutation_request_digest: String,
    authorization_target_digest: String,
    agent_id: String,
    install_id: String,
    expected_credential_id: Option<String>,
    expected_credential_revision: Option<u64>,
    expected_credential_digest: Option<String>,
    secure_transport_source: String,
    secure_transport_evidence_schema: String,
    secure_transport_evidence_id: String,
    secure_transport_evidence_digest: String,
    secure_transport_verifier_revision: u64,
    secure_transport_verifier_digest: String,
    secure_transport_server_instance_id: String,
    secure_transport_request_binding_digest: String,
    secure_transport_verified_at: String,
    reauthenticated_at: String,
    expires_at: String,
    recorded_at: String,
}

mod accessors;

pub(crate) struct PreparedNodeEndpointOwnerReauthentication {
    envelope: NodeEndpointOwnerReauthenticationEnvelope,
    receipt_json: String,
    receipt_digest: String,
}

impl PreparedNodeEndpointOwnerReauthentication {
    pub(crate) fn envelope(&self) -> &NodeEndpointOwnerReauthenticationEnvelope {
        &self.envelope
    }
    pub(crate) fn receipt_json(&self) -> &str {
        &self.receipt_json
    }
    pub(crate) fn receipt_digest(&self) -> &str {
        &self.receipt_digest
    }
}

fn validate_target_shape(
    action: &str,
    owner_user_id: &str,
    agent_id: &str,
    install_id: &str,
    expected: Option<&NodeEndpointCredentialBinding>,
) -> Result<()> {
    if !bounded_identifier(owner_user_id, 160)
        || !bounded_identifier(agent_id, 160)
        || !bounded_identifier(install_id, 512)
    {
        bail!("NODE_ENDPOINT_OWNER_REAUTHENTICATION_TARGET_INVALID");
    }
    match (action, expected) {
        ("initial_registration", None) => Ok(()),
        ("credential_rotation" | "owner_revocation", Some(value))
            if value.owner_user_id() == owner_user_id
                && value.agent_id() == agent_id
                && value.install_id() == install_id
                && value.status() == "active" =>
        {
            value.validate()
        }
        ("account_recovery", Some(value))
            if value.owner_user_id() == owner_user_id
                && value.agent_id() == agent_id
                && value.install_id() == install_id =>
        {
            value.validate()
        }
        _ => bail!("NODE_ENDPOINT_OWNER_REAUTHENTICATION_TARGET_SHAPE_INVALID"),
    }
}

#[allow(clippy::too_many_arguments)]
fn authorization_target_digest_from_parts(
    action: &str,
    owner_user_id: &str,
    agent_id: &str,
    install_id: &str,
    expected_credential_id: Option<&str>,
    expected_credential_revision: Option<u64>,
    expected_credential_digest: Option<&str>,
    mutation_request_id: &str,
    mutation_request_digest: &str,
) -> Result<String> {
    #[derive(Serialize)]
    struct Target<'a> {
        action: &'a str,
        owner_user_id: &'a str,
        agent_id: &'a str,
        install_id: &'a str,
        expected_credential_id: Option<&'a str>,
        expected_credential_revision: Option<u64>,
        expected_credential_digest: Option<&'a str>,
        mutation_request_id: &'a str,
        mutation_request_digest: &'a str,
    }
    canonical_domain_json_and_digest(
        AUTHORIZATION_TARGET_DOMAIN,
        &Target {
            action,
            owner_user_id,
            agent_id,
            install_id,
            expected_credential_id,
            expected_credential_revision,
            expected_credential_digest,
            mutation_request_id,
            mutation_request_digest,
        },
    )
    .map(|(_, digest)| digest)
}
