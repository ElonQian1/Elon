use anyhow::{bail, Context, Result};
use std::{net::SocketAddr, path::PathBuf};

#[derive(Clone)]
pub(super) struct Config {
    pub listen: SocketAddr,
    pub certificate: PathBuf,
    pub key: PathBuf,
}

impl Config {
    pub fn from_env() -> Result<Option<Self>> {
        Self::from_lookup(|name| std::env::var(name).ok())
    }

    fn from_lookup(get: impl Fn(&str) -> Option<String>) -> Result<Option<Self>> {
        match get("ACCOUNT_HTTPS_ENABLED").as_deref() {
            None | Some("false") => return Ok(None),
            Some("true") => {}
            _ => bail!("ACCOUNT_HTTPS_ENABLED must be true or false"),
        }
        let listen: SocketAddr = get("ACCOUNT_HTTPS_LISTEN_ADDR")
            .unwrap_or_else(|| "0.0.0.0:443".into())
            .parse()
            .context("ACCOUNT_HTTPS_LISTEN_ADDR invalid")?;
        if listen.port() == 0 {
            bail!("ACCOUNT_HTTPS_LISTEN_ADDR requires a port");
        }
        let path = |name| -> Result<PathBuf> {
            let path = PathBuf::from(get(name).context(format!("missing {name}"))?);
            if !path.is_absolute() {
                bail!("{name} must be absolute");
            }
            Ok(path)
        };
        Ok(Some(Self {
            listen,
            certificate: path("ACCOUNT_HTTPS_CERTIFICATE_PATH")?,
            key: path("ACCOUNT_HTTPS_PRIVATE_KEY_PATH")?,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn accepts_absolute_paths_and_explicit_bind_address() {
        let config = Config::from_lookup(|key| {
            Some(match key {
                "ACCOUNT_HTTPS_ENABLED" => "true".into(),
                "ACCOUNT_HTTPS_LISTEN_ADDR" => "127.0.0.1:8443".into(),
                _ => std::env::temp_dir()
                    .join("certificate.pem")
                    .to_string_lossy()
                    .into_owned(),
            })
        })
        .unwrap()
        .unwrap();
        assert_eq!(config.listen.port(), 8443);
        assert!(config.certificate.is_absolute());
    }
    #[test]
    fn defaults_off_and_rejects_partial_or_ambiguous_config() {
        assert!(Config::from_lookup(|_| None).unwrap().is_none());
        assert!(Config::from_lookup(|_| Some("true".into())).is_err());
        assert!(
            Config::from_lookup(|key| (key == "ACCOUNT_HTTPS_ENABLED").then(|| "yes".into()))
                .is_err()
        );
        assert!(Config::from_lookup(|key| Some(
            if key == "ACCOUNT_HTTPS_ENABLED" {
                "true"
            } else if key == "ACCOUNT_HTTPS_LISTEN_ADDR" {
                "127.0.0.1:443"
            } else {
                "relative.pem"
            }
            .into()
        ))
        .is_err());
    }
}
