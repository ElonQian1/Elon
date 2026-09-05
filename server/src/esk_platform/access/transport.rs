use axum::{
    extract::Request,
    http::{header, HeaderValue, Method, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};

use crate::node_endpoint_transport::asset_access::VerifiedAssetTransport;

/// Run before handlers and JSON extraction. Proxy headers are deliberately not authority.
pub(crate) async fn require_secure_transport(request: Request, next: Next) -> Response {
    let mut response = match check(&request) {
        Err(status) => (
            status,
            Json(serde_json::json!({
                "error": if status == StatusCode::UPGRADE_REQUIRED {
                    "asset_access_secure_transport_required"
                } else { "asset_access_origin_rejected" }
            })),
        )
            .into_response(),
        Ok(()) => next.run(request).await,
    };
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
        .headers_mut()
        .insert(header::PRAGMA, HeaderValue::from_static("no-cache"));
    response.headers_mut().insert(
        header::REFERRER_POLICY,
        HeaderValue::from_static("no-referrer"),
    );
    // The existing legacy server has global permissive CORS. These private endpoints do not.
    for key in [
        header::ACCESS_CONTROL_ALLOW_ORIGIN,
        header::ACCESS_CONTROL_ALLOW_CREDENTIALS,
    ] {
        response.headers_mut().remove(key);
    }
    response
}

fn check(request: &Request) -> Result<(), StatusCode> {
    let transport = request
        .extensions()
        .get::<VerifiedAssetTransport>()
        .ok_or(StatusCode::UPGRADE_REQUIRED)?;
    let mut origins = request.headers().get_all(header::ORIGIN).iter();
    if let Some(origin) = origins.next() {
        if origins.next().is_some()
            || !origin
                .to_str()
                .is_ok_and(|value| transport.accepts_origin(value))
        {
            return Err(StatusCode::FORBIDDEN);
        }
    }
    if request.method() == Method::TRACE {
        return Err(StatusCode::METHOD_NOT_ALLOWED);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn forwarded_headers_never_authorize_plain_http() {
        for (name, value) in [
            ("forwarded", "proto=https"),
            ("x-forwarded-proto", "https"),
            ("x-forwarded-host", "secure.example"),
        ] {
            let request = Request::builder()
                .header(name, value)
                .body(axum::body::Body::empty())
                .unwrap();
            assert_eq!(check(&request), Err(StatusCode::UPGRADE_REQUIRED));
        }
    }
}
