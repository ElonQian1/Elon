use anyhow::{bail, Context, Result};
use std::{
    net::{IpAddr, SocketAddr},
    path::PathBuf,
};

#[derive(Clone)]
pub(super) struct Config {
    pub ip: IpAddr,
    pub listen: SocketAddr,
    pub dir: PathBuf,
    pub staging: bool,
}
impl Config {
    pub fn from_env() -> Result<Option<Self>> {
        Self::from_lookup(|k| std::env::var(k).ok())
    }
    pub(super) fn from_lookup(get: impl Fn(&str) -> Option<String>) -> Result<Option<Self>> {
        match get("ACCOUNT_ACME_ENABLED").as_deref() {
            None | Some("false") => return Ok(None),
            Some("true") => {}
            _ => bail!("ACCOUNT_ACME_ENABLED must be true or false"),
        }
        if get("ACCOUNT_ACME_ACCEPT_TOS").as_deref() != Some("true") {
            bail!("ACCOUNT_ACME_ACCEPT_TOS must be true");
        }
        let ip: IpAddr = get("ACCOUNT_ACME_IP")
            .context("ACCOUNT_ACME_IP required")?
            .parse()?;
        if ip.is_unspecified() || ip.is_loopback() || ip.is_multicast() {
            bail!("ACCOUNT_ACME_IP must be a public IP");
        }
        let listen: SocketAddr = get("ACCOUNT_ACME_LISTEN_ADDR")
            .unwrap_or_else(|| "0.0.0.0:443".into())
            .parse()?;
        if listen.port() != 443 {
            bail!("ACME TLS-ALPN-01 requires public port 443");
        }
        let dir =
            PathBuf::from(get("ACCOUNT_ACME_DATA_DIR").context("ACCOUNT_ACME_DATA_DIR required")?);
        if !dir.is_absolute() {
            bail!("ACCOUNT_ACME_DATA_DIR must be absolute");
        }
        let staging = match get("ACCOUNT_ACME_STAGING").as_deref() {
            None | Some("false") => false,
            Some("true") => true,
            _ => bail!("ACCOUNT_ACME_STAGING must be true or false"),
        };
        Ok(Some(Self {
            ip,
            listen,
            dir: dir
                .join(if staging { "staging" } else { "production" })
                .join(ip.to_string()),
            staging,
        }))
    }
    pub fn bundle(&self) -> PathBuf {
        self.dir.join("active.pem")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn requires_explicit_enablement_tos_ip_and_isolated_directory() {
        assert!(Config::from_lookup(|_| None).unwrap().is_none());
        assert!(Config::from_lookup(|_| Some("true".into())).is_err());
        let get = |key: &str| {
            Some(match key {
                "ACCOUNT_ACME_ENABLED" | "ACCOUNT_ACME_ACCEPT_TOS" => "true".into(),
                "ACCOUNT_ACME_IP" => "43.139.149.158".into(),
                "ACCOUNT_ACME_DATA_DIR" => std::env::temp_dir().to_string_lossy().into_owned(),
                "ACCOUNT_ACME_LISTEN_ADDR" => "0.0.0.0:443".into(),
                _ => "false".into(),
            })
        };
        let c = Config::from_lookup(get).unwrap().unwrap();
        assert!(c.bundle().ends_with("production/43.139.149.158/active.pem"));
        assert!(Config::from_lookup(|k| if k == "ACCOUNT_ACME_ACCEPT_TOS" {
            None
        } else {
            get(k)
        })
        .is_err());
        assert!(Config::from_lookup(|k| if k == "ACCOUNT_ACME_LISTEN_ADDR" {
            Some("0.0.0.0:8443".into())
        } else {
            get(k)
        })
        .is_err());
    }
}
