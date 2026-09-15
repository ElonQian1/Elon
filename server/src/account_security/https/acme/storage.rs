use super::config::Config;
use anyhow::{bail, Context, Result};
use rustls::{
    client::danger::ServerCertVerifier, client::WebPkiServerVerifier, sign::CertifiedKey,
    RootCertStore,
};
use rustls_pki_types::{pem::PemObject, CertificateDer, PrivateKeyDer, ServerName, UnixTime};
use std::{fs, io::Write, net::IpAddr, path::Path, sync::Arc};

pub(crate) fn pair(bytes: &[u8]) -> Result<(Vec<CertificateDer<'static>>, PrivateKeyDer<'static>)> {
    let chain =
        CertificateDer::pem_slice_iter(bytes).collect::<std::result::Result<Vec<_>, _>>()?;
    let key = PrivateKeyDer::from_pem_slice(bytes)?;
    if chain.is_empty() {
        bail!("empty certificate chain");
    }
    let provider = rustls::crypto::ring::default_provider();
    CertifiedKey::from_der(chain.clone(), key.clone_key(), &provider)?.keys_match()?;
    Ok((chain, key))
}

pub(super) fn validate(bytes: &[u8], ip: IpAddr, trusted: bool, min_remaining: i64) -> Result<i64> {
    let (chain, _) = pair(bytes)?;
    let (_, cert) = x509_parser::parse_x509_certificate(chain[0].as_ref())
        .map_err(|_| anyhow::anyhow!("invalid X509"))?;
    let now = chrono::Utc::now().timestamp();
    let end = cert.validity().not_after.timestamp();
    if cert.validity().not_before.timestamp() > now || end <= now + min_remaining {
        bail!("certificate outside required validity");
    }
    let san = cert.subject_alternative_name()?.context("IP SAN missing")?;
    let expected = match ip {
        IpAddr::V4(ip) => ip.octets().to_vec(),
        IpAddr::V6(ip) => ip.octets().to_vec(),
    };
    if !san.value.general_names.iter().any(|n| matches!(n, x509_parser::extensions::GeneralName::IPAddress(bytes) if *bytes == expected.as_slice())) { bail!("certificate IP SAN mismatch"); }
    if trusted {
        let roots = RootCertStore {
            roots: webpki_roots::TLS_SERVER_ROOTS.to_vec(),
        };
        let verifier = WebPkiServerVerifier::builder_with_provider(
            Arc::new(roots),
            Arc::new(rustls::crypto::ring::default_provider()),
        )
        .build()?;
        verifier.verify_server_cert(
            &chain[0],
            &chain[1..],
            &ServerName::IpAddress(ip.into()),
            &[],
            UnixTime::now(),
        )?;
    }
    Ok(end)
}

/// One PEM contains the entire keypair, replaced by a single atomic rename.
/// Readers take one byte snapshot, so certificate and key cannot straddle renewals.
pub(super) fn atomic_private(path: &Path, bytes: &[u8]) -> Result<()> {
    let parent = path.parent().context("missing storage parent")?;
    fs::create_dir_all(parent)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(parent, fs::Permissions::from_mode(0o700))?;
    }
    let temporary = parent.join(format!(".{}.tmp", uuid::Uuid::new_v4()));
    let result = (|| {
        let mut options = fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options.open(&temporary)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        drop(file);
        fs::rename(&temporary, path)?;
        #[cfg(unix)]
        fs::File::open(parent)?.sync_all()?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}
pub(super) fn activate(config: &Config, certificate: &str, key: &str) -> Result<i64> {
    let bytes = format!("{certificate}\n{key}\n");
    let end = validate(bytes.as_bytes(), config.ip, !config.staging, 48 * 3600)?;
    atomic_private(&config.bundle(), bytes.as_bytes())?;
    Ok(end)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_untrusted_wrong_ip_mismatched_key_and_keeps_last_pair() {
        let cert = rcgen::generate_simple_self_signed(vec!["43.139.149.158".into()]).unwrap();
        let pem = format!("{}{}", cert.cert.pem(), cert.key_pair.serialize_pem());
        let ip = "43.139.149.158".parse().unwrap();
        assert!(validate(pem.as_bytes(), ip, false, 0).is_ok());
        assert!(validate(pem.as_bytes(), ip, true, 0).is_err());
        assert!(validate(pem.as_bytes(), "1.1.1.1".parse().unwrap(), false, 0).is_err());
        let wrong = format!(
            "{}{}",
            cert.cert.pem(),
            rcgen::KeyPair::generate().unwrap().serialize_pem()
        );
        assert!(pair(wrong.as_bytes()).is_err());
        let root = std::env::temp_dir().join(format!("elon-acme-test-{}", uuid::Uuid::new_v4()));
        let c = Config {
            ip,
            listen: "127.0.0.1:443".parse().unwrap(),
            dir: root.clone(),
            staging: true,
        };
        activate(&c, &cert.cert.pem(), &cert.key_pair.serialize_pem()).unwrap();
        let before = fs::read(c.bundle()).unwrap();
        assert!(activate(&c, "invalid certificate", "invalid key").is_err());
        assert_eq!(before, fs::read(c.bundle()).unwrap());
        fs::remove_file(c.bundle()).unwrap();
        fs::remove_dir(root).unwrap();
    }
}
