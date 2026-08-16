use axum::{
    http::{header::AUTHORIZATION, HeaderMap, StatusCode},
    response::Response,
};
use sha2::{Digest, Sha256};

use crate::types::AppState;

use super::json_error;

pub(super) fn authenticate_node_bearer(
    state: &AppState,
    headers: &HeaderMap,
    node_id: &str,
) -> Result<String, Response> {
    let node_id = node_id.trim();
    let presented = bearer_token(headers).ok_or_else(unauthorized)?;
    let expected_hash = state
        .store
        .get_node_credential_hash(node_id)
        .map_err(|error| {
            tracing::warn!(node_id, %error, "failed to load Rust cache fleet node credential");
            json_error(StatusCode::INTERNAL_SERVER_ERROR, "节点凭证校验失败")
        })?
        .ok_or_else(unauthorized)?;
    let presented_hash = hex::encode(Sha256::digest(presented.as_bytes()));
    if !constant_time_eq(expected_hash.as_bytes(), presented_hash.as_bytes()) {
        return Err(unauthorized());
    }
    state
        .store
        .get_node_credential_owner(node_id)
        .map_err(|error| {
            tracing::warn!(node_id, %error, "failed to load Rust cache fleet node owner");
            json_error(StatusCode::INTERNAL_SERVER_ERROR, "节点归属校验失败")
        })?
        .ok_or_else(unauthorized)
}

fn bearer_token(headers: &HeaderMap) -> Option<&str> {
    let value = headers.get(AUTHORIZATION)?.to_str().ok()?.trim();
    let (scheme, token) = value.split_once(' ')?;
    if !scheme.eq_ignore_ascii_case("bearer") || token.trim().is_empty() {
        return None;
    }
    Some(token.trim())
}

fn unauthorized() -> Response {
    json_error(StatusCode::UNAUTHORIZED, "节点凭证无效")
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    left.iter()
        .zip(right)
        .fold(0_u8, |difference, (left, right)| {
            difference | (left ^ right)
        })
        == 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    #[test]
    fn bearer_parser_is_strict_and_case_insensitive() {
        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_static("bEaReR node-secret"),
        );
        assert_eq!(bearer_token(&headers), Some("node-secret"));
        headers.insert(AUTHORIZATION, HeaderValue::from_static("Basic node-secret"));
        assert_eq!(bearer_token(&headers), None);
    }

    #[test]
    fn digest_comparison_rejects_length_and_content_drift() {
        assert!(constant_time_eq(b"same", b"same"));
        assert!(!constant_time_eq(b"same", b"diff"));
        assert!(!constant_time_eq(b"same", b"longer"));
    }
}
