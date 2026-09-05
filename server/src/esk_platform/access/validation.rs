use anyhow::Result;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use sha2::{Digest, Sha256};

use super::model::*;

pub(crate) fn valid_client(client: &str) -> bool {
    matches!(client, "quant.android" | "quant.web" | "quant.ai")
}

pub(crate) fn valid_scopes(scopes: &[AccessScope]) -> bool {
    !scopes.is_empty()
        && scopes.len() <= 3
        && scopes.contains(&AccessScope::EskSummaryRead)
        && scopes
            .iter()
            .enumerate()
            .all(|(i, scope)| !scopes[..i].contains(scope))
}

fn unreserved(value: &str, min: usize, max: usize) -> bool {
    (min..=max).contains(&value.len())
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-._~".contains(&b))
}

pub(crate) fn valid_secret(value: &str, prefix: &str) -> bool {
    value.strip_prefix(prefix).is_some_and(|secret| {
        secret.len() == 64
            && secret
                .bytes()
                .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
    })
}

pub(crate) fn valid_grant_id(value: &str) -> bool {
    value.strip_prefix("aag_").is_some_and(|id| {
        id.len() == 32
            && id
                .bytes()
                .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
    })
}

pub(crate) fn challenge(verifier: &str) -> Result<String> {
    if !unreserved(verifier, 43, 128) {
        return Err(AccessError::InvalidInput.into());
    }
    Ok(URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes())))
}

pub(crate) fn validate_redirect(client: &str, redirect: &str, public_url: &str) -> Result<()> {
    if !valid_client(client) || redirect.len() > 2048 {
        return Err(AccessError::InvalidInput.into());
    }
    let valid = match client {
        "quant.android" => redirect == "com.elon.quant:/asset-access/callback",
        "quant.web" => {
            let public = reqwest::Url::parse(public_url).map_err(|_| AccessError::Unavailable)?;
            if public.scheme() != "https"
                || public.host_str().is_none()
                || !public.username().is_empty()
                || public.password().is_some()
                || public.query().is_some()
                || public.fragment().is_some()
            {
                return Err(AccessError::Unavailable.into());
            }
            let expected = public
                .join("/quant/asset-access/callback")
                .map_err(|_| AccessError::Unavailable)?;
            redirect == expected.as_str()
        }
        "quant.ai" => reqwest::Url::parse(redirect).is_ok_and(|url| {
            url.scheme() == "http"
                && url.host_str() == Some("127.0.0.1")
                && url.port().is_some_and(|port| port >= 1024)
                && url.path() == "/asset-access/callback"
                && url.query().is_none()
                && url.fragment().is_none()
                && url.username().is_empty()
                && url.password().is_none()
                && url.as_str() == redirect
        }),
        _ => false,
    };
    if !valid {
        return Err(AccessError::InvalidInput.into());
    }
    Ok(())
}

pub(crate) fn validate_authorize(body: &AuthorizeBody, public_url: &str) -> Result<()> {
    if body.schema != AUTHORIZE_SCHEMA
        || !body.explicit_consent
        || body.confirmation != AUTHORIZE_CONFIRMATION
        || !valid_scopes(&body.scopes)
        || !(1..=MAX_GRANT_SECONDS).contains(&body.expires_in)
        || !unreserved(&body.state, 32, 128)
        || body.code_challenge_method != "S256"
        || body.code_challenge.len() != 43
        || URL_SAFE_NO_PAD
            .decode(&body.code_challenge)
            .map_or(true, |value| {
                value.len() != 32 || URL_SAFE_NO_PAD.encode(value) != body.code_challenge
            })
    {
        return Err(AccessError::InvalidInput.into());
    }
    validate_redirect(&body.client_id, &body.redirect_uri, public_url)
}

pub(crate) fn validate_exchange(body: &TokenBody, public_url: &str) -> Result<()> {
    if body.schema != TOKEN_SCHEMA
        || body.grant_type != "authorization_code"
        || !valid_secret(&body.code, "aac_")
        || !unreserved(&body.state, 32, 128)
    {
        return Err(AccessError::InvalidGrant.into());
    }
    challenge(&body.code_verifier).map_err(|_| AccessError::InvalidGrant)?;
    validate_redirect(&body.client_id, &body.redirect_uri, public_url)
        .map_err(|_| AccessError::InvalidGrant.into())
}

pub(crate) fn validate_revoke(body: &RevokeBody) -> Result<()> {
    if body.schema != REVOKE_SCHEMA || body.confirmation != REVOKE_CONFIRMATION {
        return Err(AccessError::InvalidInput.into());
    }
    Ok(())
}
