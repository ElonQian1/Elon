//! This assembly is reachable only after the direct rustls listener verifies its handshake.
use std::sync::Arc;

use axum::{Extension, Router};

use crate::{esk_asset::platform::access, types::AppState};

/// Private construction prevents headers or legacy HTTP handlers from minting TLS authority.
#[derive(Clone)]
pub(crate) struct VerifiedAssetTransport {
    allowed_origin: Option<String>,
}

impl VerifiedAssetTransport {
    pub(crate) fn accepts_origin(&self, origin: &str) -> bool {
        self.allowed_origin.as_deref() == Some(origin)
    }
}

fn proof(public_url: &str) -> VerifiedAssetTransport {
    let allowed_origin = reqwest::Url::parse(public_url)
        .ok()
        .filter(|url| {
            url.scheme() == "https"
                && url.host_str().is_some()
                && url.username().is_empty()
                && url.password().is_none()
                && url.query().is_none()
                && url.fragment().is_none()
                && url.path() == "/"
        })
        .map(|url| url.origin().ascii_serialization());
    VerifiedAssetTransport { allowed_origin }
}

pub(crate) fn browser_routes(
    public_url: &str,
    data_dir: &std::path::Path,
) -> Router<Arc<AppState>> {
    crate::esk_asset::platform::game_access::browser::routes(data_dir)
        .layer(Extension(proof(public_url)))
}

pub(crate) fn routes(public_url: &str) -> Router<Arc<AppState>> {
    access::routes()
        .merge(crate::esk_asset::platform::game_access::routes())
        .merge(crate::esk_asset::platform::game_rewards::routes())
        .merge(crate::private_read_projection_api::routes())
        .layer(Extension(proof(public_url)))
}

#[cfg(test)]
pub(crate) fn test_routes(public_url: &str) -> Router<Arc<AppState>> {
    routes(public_url)
}
