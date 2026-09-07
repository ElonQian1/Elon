//! Fixed HTTPS origin and exact ACK binding; no redirect/discovery authority.
use super::Projection;
use serde::Deserialize;

pub(crate) fn endpoint(origin: &str) -> Result<reqwest::Url, &'static str> {
    let mut url = reqwest::Url::parse(origin).map_err(|_| "projection_https_invalid")?;
    let canonical = url.origin().ascii_serialization();
    if url.scheme() != "https"
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
        || url.path() != "/"
        || (origin != canonical && origin != format!("{canonical}/"))
    {
        return Err("projection_https_invalid");
    }
    url.set_path("/api/node/private-read-projections");
    Ok(url)
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Ack {
    schema: String,
    revision: String,
    connection_id: String,
    status: String,
}
pub(crate) fn validate_ack(bytes: &[u8], value: &Projection) -> Result<(), &'static str> {
    if bytes.len() > 4096 {
        return Err("projection_ack_invalid");
    }
    let ack: Ack = serde_json::from_slice(bytes).map_err(|_| "projection_ack_invalid")?;
    if ack.schema != "yilong.private_read_projection.ack.v1"
        || ack.revision != value.revision
        || ack.connection_id != value.connection_id
        || !matches!(ack.status.as_str(), "accepted" | "unchanged")
    {
        return Err("projection_ack_invalid");
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn private_transport_never_derives_or_downgrades_an_origin() {
        assert_eq!(
            endpoint("https://example.test:8443").unwrap().as_str(),
            "https://example.test:8443/api/node/private-read-projections"
        );
        for origin in [
            "http://example.test",
            "ws://example.test",
            "https://a:b@example.test",
            "https://example.test/path",
            "https://example.test/?next=a",
            "https://example.test/#a",
            "https://example.test/%2e/",
            " https://example.test",
        ] {
            assert!(endpoint(origin).is_err(), "{origin}");
        }
    }
}
