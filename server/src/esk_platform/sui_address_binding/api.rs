use std::sync::Arc;

use axum::{
    body::Bytes,
    extract::{FromRequest, FromRequestParts, Path, Query, Request, State},
    http::{header, HeaderMap},
    response::Response,
    Json,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use chrono::{Duration, Utc};
use ring::rand::{SecureRandom, SystemRandom};
use serde::Deserialize;

use crate::{project_auth::bearer_token, types::AppState};

use super::{
    canonical_timestamp, valid_challenge_id, validate_platform_request, validate_wallet_response,
    verify_wallet_response, AddressBindingError, ChallengeMaterial, PlatformAddressBindingRequest,
    WalletResponseBody,
};

use super::wire;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct EmptyQuery {}

pub(super) async fn create_challenge(
    State(state): State<Arc<AppState>>,
    request: Request,
) -> Response {
    let (mut parts, body) = request.into_parts();
    let (user_id, token) = match authenticate(&state, &parts.headers) {
        Ok(value) => value,
        Err(response) => return response,
    };
    if Query::<EmptyQuery>::from_request_parts(&mut parts, &state)
        .await
        .is_err()
    {
        return invalid_input();
    }
    let body = match Json::<PlatformAddressBindingRequest>::from_request(
        Request::from_parts(parts, body),
        &state,
    )
    .await
    {
        Ok(Json(value)) => value,
        Err(_) => return invalid_input(),
    };
    if validate_platform_request(&body).is_err() {
        return invalid_input();
    }
    let material = match challenge_material(&body) {
        Ok(value) => value,
        Err(error) => return wire::domain_error(error.into()),
    };
    match state
        .store
        .create_esk_sui_address_binding_challenge(&user_id, &token, &material)
    {
        Ok(challenge) => wire::challenge_response(challenge),
        Err(error) => wire::domain_error(error),
    }
}

pub(super) async fn complete_challenge(
    State(state): State<Arc<AppState>>,
    request: Request,
) -> Response {
    let (mut parts, body) = request.into_parts();
    let (user_id, token) = match authenticate(&state, &parts.headers) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let challenge_id = match Path::<String>::from_request_parts(&mut parts, &state).await {
        Ok(Path(value)) if valid_challenge_id(&value) => value,
        _ => return invalid_input(),
    };
    if Query::<EmptyQuery>::from_request_parts(&mut parts, &state)
        .await
        .is_err()
    {
        return invalid_input();
    }
    let body =
        match Json::<WalletResponseBody>::from_request(Request::from_parts(parts, body), &state)
            .await
        {
            Ok(Json(value)) => value,
            Err(_) => return invalid_input(),
        };
    if body.challenge_id != challenge_id || validate_wallet_response(&body).is_err() {
        return invalid_input();
    }
    let challenge =
        match state
            .store
            .load_esk_sui_address_binding_challenge(&user_id, &token, &challenge_id)
        {
            Ok(value) => value,
            Err(error) => return wire::domain_error(error),
        };
    let verified = match verify_wallet_response(&challenge, &body, Utc::now()) {
        Ok(value) => value,
        Err(error) => return wire::domain_error(error.into()),
    };
    match state
        .store
        .complete_esk_sui_address_binding(&user_id, &token, &challenge_id, &verified)
    {
        Ok(binding) => wire::bound_response(binding),
        Err(error) => wire::domain_error(error),
    }
}

pub(super) async fn get_my_binding(
    State(state): State<Arc<AppState>>,
    request: Request,
) -> Response {
    let (mut parts, body) = request.into_parts();
    let (user_id, token) = match authenticate(&state, &parts.headers) {
        Ok(value) => value,
        Err(response) => return response,
    };
    if Query::<EmptyQuery>::from_request_parts(&mut parts, &state)
        .await
        .is_err()
    {
        return invalid_input();
    }
    let body = Bytes::from_request(Request::from_parts(parts, body), &state).await;
    if !matches!(body, Ok(ref value) if value.is_empty()) {
        return invalid_input();
    }
    match state.store.get_esk_sui_address_binding(&user_id, &token) {
        Ok(Some(binding)) => wire::bound_response(binding),
        Ok(None) => wire::unbound_response(),
        Err(error) => wire::domain_error(error),
    }
}

fn authenticate(state: &AppState, headers: &HeaderMap) -> Result<(String, String), Response> {
    if headers.get_all(header::AUTHORIZATION).iter().count() != 1 {
        return Err(authentication_error(AddressBindingError::Unauthorized));
    }
    let token = bearer_token(headers)
        .filter(|value| value.len() <= 8_192)
        .ok_or_else(|| authentication_error(AddressBindingError::Unauthorized))?
        .to_owned();
    if state.owner_token.as_deref() == Some(token.as_str()) || state.admin_token == token {
        return Err(authentication_error(AddressBindingError::Unauthorized));
    }

    let user_id = state
        .store
        .authenticate_esk_sui_address_binding_user_id(&token)
        .map_err(wire::domain_error)?;
    if user_id == "local-owner" {
        return Err(authentication_error(AddressBindingError::Unauthorized));
    }
    Ok((user_id, token))
}

fn authentication_error(error: AddressBindingError) -> Response {
    wire::domain_error(error.into())
}

fn challenge_material(
    request: &PlatformAddressBindingRequest,
) -> Result<ChallengeMaterial, AddressBindingError> {
    let mut nonce = [0_u8; 32];
    SystemRandom::new()
        .fill(&mut nonce)
        .map_err(|_| AddressBindingError::RandomUnavailable)?;
    let issued_at = Utc::now();
    let expires_at = issued_at
        .checked_add_signed(Duration::seconds(i64::from(request.ttl_seconds)))
        .ok_or(AddressBindingError::InvalidInput)?;
    Ok(ChallengeMaterial {
        address: request.address.clone(),
        ttl_seconds: request.ttl_seconds,
        nonce_base64: BASE64.encode(nonce),
        issued_at: canonical_timestamp(issued_at),
        expires_at: canonical_timestamp(expires_at),
    })
}

fn invalid_input() -> Response {
    wire::domain_error(AddressBindingError::InvalidInput.into())
}
