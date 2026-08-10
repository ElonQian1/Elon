use chrono::{DateTime, Duration, Utc};
use serde::Serialize;

use super::*;
use crate::{
    node_compute_sharing::endpoint_authority::NodeEndpointOwnerCredentialMutationRequest,
    store::CurrentOwnerAccountSource,
};

const PASSWORD_EVIDENCE_SCHEMA: &str =
    "elon.node_endpoint.owner_password_reauthentication_evidence.v1";
const PASSWORD_EVIDENCE_ID_DOMAIN: &[u8] =
    b"ELON_NODE_ENDPOINT_OWNER_PASSWORD_REAUTHENTICATION_EVIDENCE_ID_V1";
const PASSWORD_EVIDENCE_DIGEST_DOMAIN: &[u8] =
    b"ELON_NODE_ENDPOINT_OWNER_PASSWORD_REAUTHENTICATION_EVIDENCE_V1";

pub(crate) fn authorize_password_owner_reauthentication(
    source: CurrentOwnerAccountSource,
    transport: VerifiedSecureOwnerApiTransport,
    request: &NodeEndpointOwnerCredentialMutationRequest,
    expected_current: Option<NodeEndpointCredentialBinding>,
    verified_at: DateTime<Utc>,
) -> Result<AuthorizedNodeEndpointOwnerReauthentication> {
    if source.session_revoked_at().is_some()
        || source.user_status() != "active"
        || !source.password_login_enabled()
    {
        bail!("NODE_ENDPOINT_OWNER_ACCOUNT_SOURCE_NOT_CURRENT");
    }
    let (_, mutation_digest) = request.canonical_json_and_digest()?;
    require_expected_request_current(request, expected_current.as_ref())?;
    let session_binding_digest =
        super::super::digests::derive_owner_account_session_binding_digest(
            source.account_session_id(),
            source.owner_user_id(),
            source.token_hash(),
            source.session_created_at(),
            source.session_expires_at(),
        )?;
    let account_auth_state_digest = super::super::digests::derive_owner_account_auth_state_digest(
        source.owner_user_id(),
        source.role(),
        source.user_status(),
        source.password_login_enabled(),
        source.password_changed_at(),
        source.user_updated_at(),
    )?;
    let factor_binding_digest = super::super::digests::derive_owner_password_factor_binding_digest(
        source.owner_user_id(),
        source.password_hash(),
        source.password_changed_at(),
    )?;
    let target_digest = authorization_target_digest_from_parts(
        request.authorization_action(),
        source.owner_user_id(),
        request.agent_id(),
        request.install_id(),
        expected_current
            .as_ref()
            .map(NodeEndpointCredentialBinding::credential_id),
        expected_current
            .as_ref()
            .map(NodeEndpointCredentialBinding::credential_revision),
        expected_current
            .as_ref()
            .map(NodeEndpointCredentialBinding::credential_digest),
        request.credential_mutation_request_id(),
        &mutation_digest,
    )?;
    let (evidence_id, evidence_digest) = password_evidence(
        source.owner_user_id(),
        source.account_session_id(),
        &factor_binding_digest,
        request,
        &mutation_digest,
        &target_digest,
        verified_at,
    )?;
    let session_expires_at =
        DateTime::parse_from_rfc3339(source.session_expires_at())?.with_timezone(&Utc);
    if verified_at >= session_expires_at {
        bail!("NODE_ENDPOINT_OWNER_ACCOUNT_SESSION_NOT_CURRENT");
    }
    let reauthentication_expires_at = verified_at
        .checked_add_signed(Duration::minutes(REAUTHENTICATION_LIFETIME_MINUTES))
        .ok_or_else(|| anyhow::anyhow!("NODE_ENDPOINT_OWNER_REAUTHENTICATION_EXPIRY_OVERFLOW"))?;

    Ok(AuthorizedNodeEndpointOwnerReauthentication {
        session: VerifiedCurrentAccountSession {
            owner_user_id: source.owner_user_id().to_string(),
            account_session_id: source.account_session_id().to_string(),
            session_binding_digest: session_binding_digest.clone(),
            account_auth_state_digest: account_auth_state_digest.clone(),
            verified_at,
            session_expires_at,
        },
        reauthentication: VerifiedRecentOwnerReauthentication {
            owner_user_id: source.owner_user_id().to_string(),
            account_session_id: source.account_session_id().to_string(),
            session_binding_digest,
            account_auth_state_digest,
            authentication_method: "password".to_string(),
            authentication_factor_id: "password".to_string(),
            authentication_factor_binding_digest: factor_binding_digest,
            authentication_evidence_id: evidence_id,
            authentication_evidence_digest: evidence_digest,
            authorization_action: request.authorization_action().to_string(),
            credential_mutation_request_id: request.credential_mutation_request_id().to_string(),
            credential_mutation_request_digest: mutation_digest,
            authorization_target_digest: target_digest,
            reauthenticated_at: verified_at,
            expires_at: reauthentication_expires_at,
        },
        transport,
        agent_id: request.agent_id().to_string(),
        install_id: request.install_id().to_string(),
        expected_credential: expected_current,
        authorization_issuance_request_id: request.authorization_issuance_request_id().to_string(),
    })
}

fn require_expected_request_current(
    request: &NodeEndpointOwnerCredentialMutationRequest,
    current: Option<&NodeEndpointCredentialBinding>,
) -> Result<()> {
    match (request.expected(), current) {
        (None, None) if request.authorization_action() == "initial_registration" => Ok(()),
        (Some(expected), Some(current))
            if expected.credential_id() == current.credential_id()
                && expected.credential_revision() == current.credential_revision()
                && expected.credential_digest() == current.credential_digest()
                && current.agent_id() == request.agent_id()
                && current.install_id() == request.install_id() =>
        {
            current.validate()
        }
        _ => bail!("NODE_ENDPOINT_OWNER_REAUTHENTICATION_EXPECTED_CURRENT_MISMATCH"),
    }
}

#[allow(clippy::too_many_arguments)]
fn password_evidence(
    owner_user_id: &str,
    account_session_id: &str,
    factor_binding_digest: &str,
    request: &NodeEndpointOwnerCredentialMutationRequest,
    mutation_digest: &str,
    target_digest: &str,
    verified_at: DateTime<Utc>,
) -> Result<(String, String)> {
    #[derive(Serialize)]
    struct Evidence<'a> {
        schema: &'static str,
        owner_user_id: &'a str,
        account_session_id: &'a str,
        authentication_method: &'static str,
        factor_binding_digest: &'a str,
        authorization_action: &'a str,
        authorization_issuance_request_id: &'a str,
        credential_mutation_request_id: &'a str,
        credential_mutation_request_digest: &'a str,
        authorization_target_digest: &'a str,
        verified_at: String,
    }
    let evidence = Evidence {
        schema: PASSWORD_EVIDENCE_SCHEMA,
        owner_user_id,
        account_session_id,
        authentication_method: "password",
        factor_binding_digest,
        authorization_action: request.authorization_action(),
        authorization_issuance_request_id: request.authorization_issuance_request_id(),
        credential_mutation_request_id: request.credential_mutation_request_id(),
        credential_mutation_request_digest: mutation_digest,
        authorization_target_digest: target_digest,
        verified_at: utc_nanos(verified_at),
    };
    let evidence_id = deterministic_identifier("nepwa_", PASSWORD_EVIDENCE_ID_DOMAIN, &evidence)?;
    let (_, evidence_digest) =
        canonical_domain_json_and_digest(PASSWORD_EVIDENCE_DIGEST_DOMAIN, &evidence)?;
    Ok((evidence_id, evidence_digest))
}
