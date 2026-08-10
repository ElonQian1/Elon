use std::{net::SocketAddr, path::PathBuf};

use anyhow::{bail, Context, Result};

const ENABLED_ENV: &str = "NODE_ENDPOINT_DIRECT_TLS_ENABLED";
const LISTEN_ADDR_ENV: &str = "NODE_ENDPOINT_DIRECT_TLS_LISTEN_ADDR";
const CERT_CHAIN_PATH_ENV: &str = "NODE_ENDPOINT_DIRECT_TLS_CERT_CHAIN_PATH";
const PRIVATE_KEY_PATH_ENV: &str = "NODE_ENDPOINT_DIRECT_TLS_PRIVATE_KEY_PATH";
const VERIFIER_REVISION_ENV: &str = "NODE_ENDPOINT_DIRECT_TLS_VERIFIER_REVISION";
const OWNER_CREDENTIAL_API_ENABLED_ENV: &str = "NODE_ENDPOINT_OWNER_CREDENTIAL_API_ENABLED";
const MAX_IJSON_SAFE_INTEGER: u64 = 9_007_199_254_740_991;

pub(super) struct DirectTlsTransportConfig {
    pub(super) listen_addr: SocketAddr,
    pub(super) certificate_chain_path: PathBuf,
    pub(super) private_key_path: PathBuf,
    pub(super) verifier_revision: u64,
    pub(super) owner_credential_api_enabled: bool,
}

impl DirectTlsTransportConfig {
    pub(super) fn from_env(legacy_addr: SocketAddr) -> Result<Option<Self>> {
        let enabled = optional_env(ENABLED_ENV);
        let listen_addr = optional_env(LISTEN_ADDR_ENV);
        let certificate_chain_path = optional_env(CERT_CHAIN_PATH_ENV);
        let private_key_path = optional_env(PRIVATE_KEY_PATH_ENV);
        let verifier_revision = optional_env(VERIFIER_REVISION_ENV);
        let owner_credential_api_enabled =
            match optional_env(OWNER_CREDENTIAL_API_ENABLED_ENV).as_deref() {
                None | Some("false") => false,
                Some("true") => true,
                Some(_) => bail!("NODE_ENDPOINT_OWNER_CREDENTIAL_API_ENABLED_INVALID"),
            };
        if owner_credential_api_enabled && enabled.as_deref() != Some("true") {
            bail!("NODE_ENDPOINT_OWNER_CREDENTIAL_API_DIRECT_TLS_REQUIRED");
        }
        let configured = [
            listen_addr.as_ref(),
            certificate_chain_path.as_ref(),
            private_key_path.as_ref(),
            verifier_revision.as_ref(),
        ]
        .iter()
        .any(|value| value.is_some());

        match enabled.as_deref() {
            None | Some("false") if !configured => return Ok(None),
            None | Some("false") => bail!("NODE_ENDPOINT_DIRECT_TLS_ENABLE_REQUIRED"),
            Some("true") => {}
            Some(_) => bail!("NODE_ENDPOINT_DIRECT_TLS_ENABLED_INVALID"),
        }

        let listen_addr: SocketAddr = required(listen_addr, LISTEN_ADDR_ENV)?
            .parse()
            .with_context(|| format!("{LISTEN_ADDR_ENV} is not a socket address"))?;
        if listen_addr == legacy_addr || listen_addr.port() == 0 {
            bail!("NODE_ENDPOINT_DIRECT_TLS_LISTEN_ADDR_INVALID");
        }
        let certificate_chain_path = absolute_path(
            required(certificate_chain_path, CERT_CHAIN_PATH_ENV)?,
            CERT_CHAIN_PATH_ENV,
        )?;
        let private_key_path = absolute_path(
            required(private_key_path, PRIVATE_KEY_PATH_ENV)?,
            PRIVATE_KEY_PATH_ENV,
        )?;
        let verifier_revision: u64 = required(verifier_revision, VERIFIER_REVISION_ENV)?
            .parse()
            .with_context(|| format!("{VERIFIER_REVISION_ENV} is not an integer"))?;
        if verifier_revision == 0 || verifier_revision > MAX_IJSON_SAFE_INTEGER {
            bail!("NODE_ENDPOINT_DIRECT_TLS_VERIFIER_REVISION_INVALID");
        }

        Ok(Some(Self {
            listen_addr,
            certificate_chain_path,
            private_key_path,
            verifier_revision,
            owner_credential_api_enabled,
        }))
    }
}

fn optional_env(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn required(value: Option<String>, name: &str) -> Result<String> {
    value.ok_or_else(|| anyhow::anyhow!("{name} is required"))
}

fn absolute_path(value: String, name: &str) -> Result<PathBuf> {
    let path = PathBuf::from(value);
    if !path.is_absolute() {
        bail!("{name} must be an absolute path");
    }
    Ok(path)
}
