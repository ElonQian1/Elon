use std::{
    collections::HashSet,
    net::SocketAddr,
    path::{Path, PathBuf},
    time::Duration,
};

use anyhow::{bail, Context, Result};
use serde::Deserialize;
use sha2::{Digest, Sha256};

use super::certificate_config::{
    normalize_dns_name, resolve_certificate_provider, CertificateProviderConfig,
};

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct EdgeConfig {
    schema: String,
    listen_addr: SocketAddr,
    #[serde(default)]
    certificate_chain_path: Option<PathBuf>,
    #[serde(default)]
    private_key_path: Option<PathBuf>,
    #[serde(default)]
    certificate_provider: Option<CertificateProviderConfig>,
    #[serde(skip)]
    effective_certificate_provider: Option<CertificateProviderConfig>,
    public_hosts: Vec<String>,
    #[serde(default = "default_connect_timeout_ms")]
    connect_timeout_ms: u64,
    #[serde(default = "default_request_timeout_ms")]
    request_timeout_ms: u64,
    #[serde(default = "default_tls_handshake_timeout_ms")]
    tls_handshake_timeout_ms: u64,
    #[serde(default = "default_connection_timeout_ms")]
    connection_timeout_ms: u64,
    #[serde(default = "default_reload_interval_seconds")]
    reload_interval_seconds: u64,
    #[serde(default = "default_max_request_body_bytes")]
    max_request_body_bytes: usize,
    #[serde(default = "default_max_response_body_bytes")]
    max_response_body_bytes: usize,
    routes: Vec<MerchantRouteConfig>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct MerchantRouteConfig {
    instance_id: String,
    public_base_path: String,
    upstream_addr: SocketAddr,
    #[serde(default = "default_enabled")]
    enabled: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ImmutableConfigIdentity {
    listen_addr: SocketAddr,
    certificate_provider: CertificateProviderConfig,
    connect_timeout_ms: u64,
    request_timeout_ms: u64,
    tls_handshake_timeout_ms: u64,
    connection_timeout_ms: u64,
    reload_interval_seconds: u64,
    max_request_body_bytes: usize,
    max_response_body_bytes: usize,
}

pub(crate) fn read_config(path: &Path) -> Result<(EdgeConfig, String)> {
    let bytes = std::fs::read(path).context("COMMERCE_EDGE_CONFIG_READ_FAILED")?;
    let digest = hex::encode(Sha256::digest(&bytes));
    let config = EdgeConfig::parse(&bytes)?;
    Ok((config, digest))
}

pub(crate) fn read_config_bytes(path: &Path) -> Result<Vec<u8>> {
    std::fs::read(path).context("COMMERCE_EDGE_CONFIG_READ_FAILED")
}

pub(crate) fn config_digest(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

impl EdgeConfig {
    pub(crate) fn parse(bytes: &[u8]) -> Result<Self> {
        let mut config: Self =
            serde_json::from_slice(bytes).context("COMMERCE_EDGE_CONFIG_PARSE_FAILED")?;
        config.validate_and_normalize()?;
        Ok(config)
    }

    fn validate_and_normalize(&mut self) -> Result<()> {
        if self.listen_addr.port() == 0 {
            bail!("COMMERCE_EDGE_LISTEN_PORT_INVALID");
        }
        validate_range(self.connect_timeout_ms, 100, 10_000, "CONNECT_TIMEOUT")?;
        validate_range(self.request_timeout_ms, 500, 60_000, "REQUEST_TIMEOUT")?;
        validate_range(
            self.tls_handshake_timeout_ms,
            500,
            30_000,
            "TLS_HANDSHAKE_TIMEOUT",
        )?;
        validate_range(
            self.connection_timeout_ms,
            1_000,
            120_000,
            "CONNECTION_TIMEOUT",
        )?;
        validate_range(self.reload_interval_seconds, 2, 300, "RELOAD_INTERVAL")?;
        validate_range(
            self.max_request_body_bytes as u64,
            1_024,
            4 * 1_024 * 1_024,
            "MAX_REQUEST_BODY",
        )?;
        validate_range(
            self.max_response_body_bytes as u64,
            1_024,
            8 * 1_024 * 1_024,
            "MAX_RESPONSE_BODY",
        )?;

        if self.public_hosts.is_empty() || self.public_hosts.len() > 32 {
            bail!("COMMERCE_EDGE_PUBLIC_HOST_COUNT_INVALID");
        }
        let mut hosts = HashSet::new();
        for host in &mut self.public_hosts {
            *host = normalize_dns_name(host)?;
            if !hosts.insert(host.clone()) {
                bail!("COMMERCE_EDGE_PUBLIC_HOST_DUPLICATE");
            }
        }
        self.effective_certificate_provider = Some(resolve_certificate_provider(
            &self.schema,
            self.certificate_chain_path.as_deref(),
            self.private_key_path.as_deref(),
            self.certificate_provider.as_ref(),
            &self.public_hosts,
            self.listen_addr,
        )?);

        if self.routes.is_empty() || self.routes.len() > 1_024 {
            bail!("COMMERCE_EDGE_ROUTE_COUNT_INVALID");
        }
        let mut instance_ids = HashSet::new();
        let mut public_paths = HashSet::new();
        let mut enabled_count = 0usize;
        for route in &mut self.routes {
            validate_instance_id(&route.instance_id)?;
            let expected_path = format!("/merchants/{}", route.instance_id);
            if route.public_base_path != expected_path {
                bail!("COMMERCE_EDGE_PUBLIC_BASE_PATH_MISMATCH");
            }
            if !route.upstream_addr.ip().is_loopback() || route.upstream_addr.port() < 1_024 {
                bail!("COMMERCE_EDGE_UPSTREAM_NOT_LOOPBACK");
            }
            if !instance_ids.insert(route.instance_id.clone()) {
                bail!("COMMERCE_EDGE_INSTANCE_DUPLICATE");
            }
            if !public_paths.insert(route.public_base_path.clone()) {
                bail!("COMMERCE_EDGE_PUBLIC_BASE_PATH_DUPLICATE");
            }
            enabled_count += usize::from(route.enabled);
        }
        if enabled_count == 0 {
            bail!("COMMERCE_EDGE_ENABLED_ROUTE_MISSING");
        }
        Ok(())
    }

    pub(crate) fn listen_addr(&self) -> SocketAddr {
        self.listen_addr
    }

    pub(crate) fn schema(&self) -> &str {
        &self.schema
    }

    pub(crate) fn certificate_provider(&self) -> &CertificateProviderConfig {
        self.effective_certificate_provider
            .as_ref()
            .expect("validated edge config has a certificate provider")
    }

    pub(crate) fn public_hosts(&self) -> &[String] {
        &self.public_hosts
    }

    pub(crate) fn routes(&self) -> &[MerchantRouteConfig] {
        &self.routes
    }

    pub(crate) fn connect_timeout(&self) -> Duration {
        Duration::from_millis(self.connect_timeout_ms)
    }

    pub(crate) fn request_timeout(&self) -> Duration {
        Duration::from_millis(self.request_timeout_ms)
    }

    pub(crate) fn tls_handshake_timeout(&self) -> Duration {
        Duration::from_millis(self.tls_handshake_timeout_ms)
    }

    pub(crate) fn connection_timeout(&self) -> Duration {
        Duration::from_millis(self.connection_timeout_ms)
    }

    pub(crate) fn reload_interval(&self) -> Duration {
        Duration::from_secs(self.reload_interval_seconds)
    }

    pub(crate) fn max_request_body_bytes(&self) -> usize {
        self.max_request_body_bytes
    }

    pub(crate) fn max_response_body_bytes(&self) -> usize {
        self.max_response_body_bytes
    }

    pub(crate) fn immutable_identity(&self) -> ImmutableConfigIdentity {
        ImmutableConfigIdentity {
            listen_addr: self.listen_addr,
            certificate_provider: self.certificate_provider().clone(),
            connect_timeout_ms: self.connect_timeout_ms,
            request_timeout_ms: self.request_timeout_ms,
            tls_handshake_timeout_ms: self.tls_handshake_timeout_ms,
            connection_timeout_ms: self.connection_timeout_ms,
            reload_interval_seconds: self.reload_interval_seconds,
            max_request_body_bytes: self.max_request_body_bytes,
            max_response_body_bytes: self.max_response_body_bytes,
        }
    }
}

impl MerchantRouteConfig {
    pub(crate) fn instance_id(&self) -> &str {
        &self.instance_id
    }

    pub(crate) fn public_base_path(&self) -> &str {
        &self.public_base_path
    }

    pub(crate) fn upstream_addr(&self) -> SocketAddr {
        self.upstream_addr
    }

    pub(crate) fn enabled(&self) -> bool {
        self.enabled
    }
}

fn validate_range(value: u64, minimum: u64, maximum: u64, label: &str) -> Result<()> {
    if !(minimum..=maximum).contains(&value) {
        bail!("COMMERCE_EDGE_{label}_OUT_OF_RANGE");
    }
    Ok(())
}

fn validate_instance_id(value: &str) -> Result<()> {
    let bytes = value.as_bytes();
    if !(3..=48).contains(&bytes.len())
        || !bytes[0].is_ascii_lowercase()
        || bytes
            .iter()
            .any(|value| !value.is_ascii_lowercase() && !value.is_ascii_digit() && *value != b'-')
    {
        bail!("COMMERCE_EDGE_INSTANCE_ID_INVALID");
    }
    Ok(())
}

const fn default_connect_timeout_ms() -> u64 {
    2_000
}
const fn default_request_timeout_ms() -> u64 {
    15_000
}
const fn default_tls_handshake_timeout_ms() -> u64 {
    10_000
}
const fn default_connection_timeout_ms() -> u64 {
    30_000
}
const fn default_reload_interval_seconds() -> u64 {
    5
}
const fn default_max_request_body_bytes() -> usize {
    1_048_576
}
const fn default_max_response_body_bytes() -> usize {
    4_194_304
}
const fn default_enabled() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_config(upstream: &str) -> Vec<u8> {
        format!(
            r#"{{
              "schema":"yilong.commerce-edge.v1",
              "listen_addr":"127.0.0.1:18443",
              "certificate_chain_path":"{}",
              "private_key_path":"{}",
              "public_hosts":["Commerce.Example.com"],
              "routes":[{{
                "instance_id":"coffee-a",
                "public_base_path":"/merchants/coffee-a",
                "upstream_addr":"{upstream}"
              }}]
            }}"#,
            absolute_fixture_path("fullchain.pem"),
            absolute_fixture_path("privkey.pem")
        )
        .into_bytes()
    }

    fn sample_acme_config() -> Vec<u8> {
        format!(
            r#"{{
              "schema":"yilong.commerce-edge.v2",
              "listen_addr":"0.0.0.0:443",
              "certificate_provider":{{
                "mode":"acme_tls_alpn_01",
                "domains":["Commerce.Example.com"],
                "contact":"mailto:ops@example.com",
                "cache_dir":"{}",
                "environment":"staging"
              }},
              "public_hosts":["commerce.example.com"],
              "routes":[{{
                "instance_id":"coffee-a",
                "public_base_path":"/merchants/coffee-a",
                "upstream_addr":"127.0.0.1:18081"
              }}]
            }}"#,
            absolute_fixture_path("edge-acme-cache")
        )
        .into_bytes()
    }

    fn absolute_fixture_path(name: &str) -> String {
        std::env::temp_dir()
            .join(name)
            .to_string_lossy()
            .replace('\\', "\\\\")
    }

    #[test]
    fn valid_config_normalizes_hosts_and_defaults() {
        let config = EdgeConfig::parse(&sample_config("127.0.0.1:18081")).unwrap();
        assert_eq!(config.public_hosts(), &["commerce.example.com"]);
        assert_eq!(config.request_timeout(), Duration::from_secs(15));
        assert_eq!(config.routes()[0].upstream_addr().port(), 18_081);
    }

    #[test]
    fn config_rejects_non_loopback_and_path_mismatch() {
        assert!(EdgeConfig::parse(&sample_config("10.0.0.5:18081")).is_err());
        assert!(EdgeConfig::parse(&sample_config("127.0.0.1:80")).is_err());
        let invalid = String::from_utf8(sample_config("127.0.0.1:18081"))
            .unwrap()
            .replace("/merchants/coffee-a", "/merchants/other");
        assert!(EdgeConfig::parse(invalid.as_bytes()).is_err());
    }

    #[test]
    fn config_rejects_unknown_fields_and_duplicate_hosts() {
        let unknown = String::from_utf8(sample_config("127.0.0.1:18081"))
            .unwrap()
            .replace("\"routes\":", "\"secret\":\"no\",\"routes\":");
        assert!(EdgeConfig::parse(unknown.as_bytes()).is_err());
        let duplicate = String::from_utf8(sample_config("127.0.0.1:18081"))
            .unwrap()
            .replace(
                "[\"Commerce.Example.com\"]",
                "[\"commerce.example.com\",\"COMMERCE.EXAMPLE.COM\"]",
            );
        assert!(EdgeConfig::parse(duplicate.as_bytes()).is_err());
        let mut duplicate_route: serde_json::Value =
            serde_json::from_slice(&sample_config("127.0.0.1:18081")).unwrap();
        let route = duplicate_route["routes"][0].clone();
        duplicate_route["routes"]
            .as_array_mut()
            .unwrap()
            .push(route);
        assert!(EdgeConfig::parse(&serde_json::to_vec(&duplicate_route).unwrap()).is_err());
    }

    #[test]
    fn acme_config_normalizes_domains_and_requires_exact_public_hosts() {
        let config = EdgeConfig::parse(&sample_acme_config()).unwrap();
        let acme = config.certificate_provider().acme().unwrap();
        assert_eq!(acme.domains, &["commerce.example.com"]);
        assert!(!acme.environment.is_production());

        let wrong_host = String::from_utf8(sample_acme_config()).unwrap().replace(
            "\"public_hosts\":[\"commerce.example.com\"]",
            "\"public_hosts\":[\"other.example.com\"]",
        );
        assert!(EdgeConfig::parse(wrong_host.as_bytes()).is_err());
    }

    #[test]
    fn acme_config_rejects_non_standard_port_relative_cache_and_bad_contact() {
        let non_standard_port = String::from_utf8(sample_acme_config())
            .unwrap()
            .replace("0.0.0.0:443", "0.0.0.0:8443");
        assert!(EdgeConfig::parse(non_standard_port.as_bytes()).is_err());

        let relative_cache = String::from_utf8(sample_acme_config())
            .unwrap()
            .replace(&absolute_fixture_path("edge-acme-cache"), "relative-cache");
        assert!(EdgeConfig::parse(relative_cache.as_bytes()).is_err());

        let bad_contact = String::from_utf8(sample_acme_config())
            .unwrap()
            .replace("mailto:ops@example.com", "ops@example.com");
        assert!(EdgeConfig::parse(bad_contact.as_bytes()).is_err());
    }

    #[test]
    fn acme_config_rejects_duplicate_domains_and_unknown_provider_fields() {
        let mut duplicate_domains: serde_json::Value =
            serde_json::from_slice(&sample_acme_config()).unwrap();
        duplicate_domains["certificate_provider"]["domains"] =
            serde_json::json!(["commerce.example.com", "COMMERCE.EXAMPLE.COM"]);
        assert!(EdgeConfig::parse(&serde_json::to_vec(&duplicate_domains).unwrap()).is_err());

        let mut unknown_field: serde_json::Value =
            serde_json::from_slice(&sample_acme_config()).unwrap();
        unknown_field["certificate_provider"]["account_key"] = serde_json::json!("forbidden");
        assert!(EdgeConfig::parse(&serde_json::to_vec(&unknown_field).unwrap()).is_err());
    }

    #[test]
    fn v2_pem_provider_is_supported_without_legacy_fields() {
        let mut config: serde_json::Value = serde_json::from_slice(&sample_acme_config()).unwrap();
        config["listen_addr"] = serde_json::json!("127.0.0.1:18443");
        config["certificate_provider"] = serde_json::json!({
            "mode": "pem",
            "certificate_chain_path": absolute_fixture_path("fullchain.pem"),
            "private_key_path": absolute_fixture_path("privkey.pem")
        });
        let parsed = EdgeConfig::parse(&serde_json::to_vec(&config).unwrap()).unwrap();
        assert!(parsed.certificate_provider().pem_paths().is_some());
    }
}
