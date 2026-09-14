//! Personal native-paper grants; exchange, wallet and participation authority stays separate.
use crate::{esk_asset::platform::access::transport::require_secure_transport, types::AppState};
use axum::{
    extract::{rejection::JsonRejection, DefaultBodyLimit, Request, State},
    http::{header, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use model::{Error, IssueRequest};
use std::sync::Arc;

mod issue;
mod model;
mod signer;

#[cfg(test)]
pub(crate) use signer::test_config;

pub(crate) fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/api/me/quant/native-grid/access-grants",
            get(readiness).post(issue),
        )
        .layer(DefaultBodyLimit::max(4096))
        .layer(middleware::from_fn(no_query))
        .layer(middleware::from_fn(require_secure_transport))
}

async fn no_query(request: Request, next: Next) -> Response {
    if request.uri().query().is_some() {
        return error(Error::InvalidInput);
    }
    next.run(request).await
}

fn session_token<'a>(
    headers: &'a axum::http::HeaderMap,
    static_owner: Option<&str>,
    static_admin: &str,
) -> Result<&'a str, Error> {
    if headers.get_all(header::AUTHORIZATION).iter().count() != 1 {
        return Err(Error::Unauthorized);
    }
    let token = headers
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .filter(|v| !v.is_empty() && v.len() <= 8192 && v.trim() == *v)
        .ok_or(Error::Unauthorized)?;
    if static_owner == Some(token) || static_admin == token {
        return Err(Error::Unauthorized);
    }
    Ok(token)
}

async fn readiness(State(state): State<Arc<AppState>>, headers: axum::http::HeaderMap) -> Response {
    let result = (|| {
        let token = session_token(&headers, state.owner_token.as_deref(), &state.admin_token)?;
        let conn = state.store.conn().map_err(|_| Error::Unavailable)?;
        issue::session_on(&conn, token, chrono::Utc::now().timestamp())?;
        Ok::<_, Error>(signer::Signer::from_env())
    })();
    match result {
        Err(cause) => error(cause),
        Ok(config) => Json(serde_json::json!({
            "schema":"yilong.quant.native_grid_issuer_readiness.v1", "environment":model::ENVIRONMENT,
            "enabled":config.is_ok(), "reason":match config {Ok(_)=>"ready",Err(Error::Disabled)=>"configuration_required",Err(_)=>"configuration_invalid"},
            "maximum_expires_in":model::MAX_LIFETIME,
            "scopes":["native_grid.read","native_grid.create","native_grid.control"]
        })).into_response(),
    }
}

async fn issue(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    body: Result<Json<IssueRequest>, JsonRejection>,
) -> Response {
    let result = (|| {
        let token = session_token(&headers, state.owner_token.as_deref(), &state.admin_token)?;
        let Json(request) = body.map_err(|_| Error::InvalidInput)?;
        request.validate()?;
        let mut conn = state.store.conn().map_err(|_| Error::Unavailable)?;
        issue::session_on(&conn, token, chrono::Utc::now().timestamp())?;
        let signer = signer::Signer::from_env()?;
        issue::issue_on(&mut conn, token, &request, &signer, || {
            chrono::Utc::now().timestamp()
        })
    })();
    match result {
        Ok(value) => Json(value).into_response(),
        Err(cause) => error(cause),
    }
}

fn error(cause: Error) -> Response {
    let (status, code) = match cause {
        Error::InvalidInput => (StatusCode::BAD_REQUEST, "native_grid_issue_invalid"),
        Error::Unauthorized => (StatusCode::UNAUTHORIZED, "native_grid_session_required"),
        Error::Disabled => (
            StatusCode::SERVICE_UNAVAILABLE,
            "native_grid_issuer_disabled",
        ),
        Error::Misconfigured => (
            StatusCode::SERVICE_UNAVAILABLE,
            "native_grid_issuer_misconfigured",
        ),
        Error::Unavailable => (
            StatusCode::SERVICE_UNAVAILABLE,
            "native_grid_issuer_unavailable",
        ),
    };
    (status, Json(serde_json::json!({"error":code}))).into_response()
}

#[cfg(test)]
mod header_tests {
    use super::*;

    #[test]
    fn quant_native_grid_http_header_never_accepts_static_owner_or_admin() {
        let mut headers = axum::http::HeaderMap::new();
        for value in [
            "Bearer static-owner",
            "Bearer static-admin",
            "Basic test",
            "Bearer ",
            "Bearer unknown ",
        ] {
            headers.insert(header::AUTHORIZATION, value.parse().unwrap());
            assert_eq!(
                session_token(&headers, Some("static-owner"), "static-admin"),
                Err(Error::Unauthorized)
            );
        }
        headers.insert(
            header::AUTHORIZATION,
            "Bearer synthetic-user".parse().unwrap(),
        );
        assert_eq!(
            session_token(&headers, Some("static-owner"), "static-admin"),
            Ok("synthetic-user")
        );
        headers.append(
            header::AUTHORIZATION,
            "Bearer synthetic-user".parse().unwrap(),
        );
        assert_eq!(
            session_token(&headers, Some("static-owner"), "static-admin"),
            Err(Error::Unauthorized)
        );
    }
}
