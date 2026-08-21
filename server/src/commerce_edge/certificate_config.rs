use std::{
    collections::HashSet,
    net::SocketAddr,
    path::{Path, PathBuf},
};

use anyhow::{bail, Result};
use serde::Deserialize;

pub(super) const CONFIG_SCHEMA_V1: &str = "yilong.commerce-edge.v1";
pub(super) const CONFIG_SCHEMA_V2: &str = "yilong.commerce-edge.v2";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(tag = "mode", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum CertificateProviderConfig {
    Pem {
        certificate_chain_path: PathBuf,
        private_key_path: PathBuf,
    },
    #[serde(rename = "acme_tls_alpn_01")]
    AcmeTlsAlpn01 {
        domains: Vec<String>,
        contact: String,
        cache_dir: PathBuf,
        environment: AcmeEnvironment,
    },
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum AcmeEnvironment {
    Staging,
    Production,
}

pub(super) fn resolve_certificate_provider(
    schema: &str,
    legacy_certificate_chain_path: Option<&Path>,
    legacy_private_key_path: Option<&Path>,
    configured_provider: Option<&CertificateProviderConfig>,
    public_hosts: &[String],
    listen_addr: SocketAddr,
) -> Result<CertificateProviderConfig> {
    let provider = match schema {
        CONFIG_SCHEMA_V1 => {
            if configured_provider.is_some() {
                bail!("COMMERCE_EDGE_V1_CERTIFICATE_PROVIDER_FORBIDDEN");
            }
            CertificateProviderConfig::Pem {
                certificate_chain_path: legacy_certificate_chain_path
                    .ok_or_else(|| anyhow::anyhow!("COMMERCE_EDGE_CERTIFICATE_CHAIN_PATH_MISSING"))?
                    .to_path_buf(),
                private_key_path: legacy_private_key_path
                    .ok_or_else(|| anyhow::anyhow!("COMMERCE_EDGE_PRIVATE_KEY_PATH_MISSING"))?
                    .to_path_buf(),
            }
        }
        CONFIG_SCHEMA_V2 => {
            if legacy_certificate_chain_path.is_some() || legacy_private_key_path.is_some() {
                bail!("COMMERCE_EDGE_V2_LEGACY_CERTIFICATE_FIELDS_FORBIDDEN");
            }
            configured_provider
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("COMMERCE_EDGE_CERTIFICATE_PROVIDER_MISSING"))?
        }
        _ => bail!("COMMERCE_EDGE_CONFIG_SCHEMA_UNSUPPORTED"),
    };

    validate_provider(provider, public_hosts, listen_addr)
}

fn validate_provider(
    mut provider: CertificateProviderConfig,
    public_hosts: &[String],
    listen_addr: SocketAddr,
) -> Result<CertificateProviderConfig> {
    match &mut provider {
        CertificateProviderConfig::Pem {
            certificate_chain_path,
            private_key_path,
        } => {
            validate_absolute_path(certificate_chain_path, "CERTIFICATE_CHAIN")?;
            validate_absolute_path(private_key_path, "PRIVATE_KEY")?;
            if certificate_chain_path == private_key_path {
                bail!("COMMERCE_EDGE_PEM_PATHS_MUST_DIFFER");
            }
        }
        CertificateProviderConfig::AcmeTlsAlpn01 {
            domains,
            contact,
            cache_dir,
            ..
        } => {
            if listen_addr.port() != 443 {
                bail!("COMMERCE_EDGE_ACME_REQUIRES_PORT_443");
            }
            validate_absolute_path(cache_dir, "ACME_CACHE")?;
            validate_contact(contact)?;
            normalize_and_validate_domains(domains, public_hosts)?;
        }
    }
    Ok(provider)
}

fn normalize_and_validate_domains(
    domains: &mut Vec<String>,
    public_hosts: &[String],
) -> Result<()> {
    if domains.is_empty() || domains.len() > 32 {
        bail!("COMMERCE_EDGE_ACME_DOMAIN_COUNT_INVALID");
    }
    let mut seen = HashSet::new();
    for domain in domains.iter_mut() {
        *domain = normalize_dns_name(domain)?;
        if !seen.insert(domain.clone()) {
            bail!("COMMERCE_EDGE_ACME_DOMAIN_DUPLICATE");
        }
    }
    let public_hosts = public_hosts.iter().cloned().collect::<HashSet<_>>();
    if seen != public_hosts {
        bail!("COMMERCE_EDGE_ACME_DOMAINS_MUST_MATCH_PUBLIC_HOSTS");
    }
    Ok(())
}

fn validate_contact(value: &str) -> Result<()> {
    if value.len() > 320
        || !value.starts_with("mailto:")
        || value[7..].is_empty()
        || !value[7..].contains('@')
        || value
            .bytes()
            .any(|byte| byte.is_ascii_whitespace() || byte.is_ascii_control())
    {
        bail!("COMMERCE_EDGE_ACME_CONTACT_INVALID");
    }
    Ok(())
}

fn validate_absolute_path(path: &Path, label: &str) -> Result<()> {
    if !path.is_absolute() || path.as_os_str().is_empty() {
        bail!("COMMERCE_EDGE_{label}_PATH_NOT_ABSOLUTE");
    }
    Ok(())
}

pub(super) fn normalize_dns_name(value: &str) -> Result<String> {
    let host = value.trim().to_ascii_lowercase();
    if host.is_empty()
        || host.len() > 253
        || host.contains(':')
        || host.contains('/')
        || host.starts_with('.')
        || host.ends_with('.')
    {
        bail!("COMMERCE_EDGE_PUBLIC_HOST_INVALID");
    }
    for label in host.split('.') {
        if label.is_empty()
            || label.len() > 63
            || label.starts_with('-')
            || label.ends_with('-')
            || label
                .bytes()
                .any(|value| !value.is_ascii_alphanumeric() && value != b'-')
        {
            bail!("COMMERCE_EDGE_PUBLIC_HOST_INVALID");
        }
    }
    Ok(host)
}

impl CertificateProviderConfig {
    pub(crate) fn pem_paths(&self) -> Option<(&Path, &Path)> {
        match self {
            Self::Pem {
                certificate_chain_path,
                private_key_path,
            } => Some((certificate_chain_path, private_key_path)),
            Self::AcmeTlsAlpn01 { .. } => None,
        }
    }

    pub(crate) fn acme(&self) -> Option<AcmeSettings<'_>> {
        match self {
            Self::AcmeTlsAlpn01 {
                domains,
                contact,
                cache_dir,
                environment,
            } => Some(AcmeSettings {
                domains,
                contact,
                cache_dir,
                environment: *environment,
            }),
            Self::Pem { .. } => None,
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) struct AcmeSettings<'a> {
    pub(crate) domains: &'a [String],
    pub(crate) contact: &'a str,
    pub(crate) cache_dir: &'a Path,
    pub(crate) environment: AcmeEnvironment,
}

impl AcmeEnvironment {
    pub(crate) fn is_production(self) -> bool {
        matches!(self, Self::Production)
    }
}
