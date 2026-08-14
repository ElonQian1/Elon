use std::{
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr},
    sync::Arc,
    time::Duration,
};

use anyhow::Result;
use base64::{engine::general_purpose::STANDARD, Engine as _};
use rustls::{RootCertStore, ServerConfig, SupportedProtocolVersion};
use rustls_pki_types::{CertificateDer, PrivatePkcs8KeyDer};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    task::JoinHandle,
};
use tokio_rustls::TlsAcceptor;

use super::{
    address_policy::{is_public_unicast, validate_and_order_dns_answers},
    no_work::exchange_external_pool_adapter_broker_no_work,
    target::ExternalPoolAdapterBrokerTlsTarget,
    transport::{connect_external_pool_adapter_broker_tls_for_test, leaf_spki_sha256},
};

const TEST_CA_DER_BASE64: &str = "MIIDKzCCAhOgAwIBAgIUR7g52Lu1o+c9HRLOGQ0veoTGKV4wDQYJKoZIhvcNAQELBQAwHTEbMBkGA1UEAwwSVjI2NCBMb2NhbCBUZXN0IENBMB4XDTI2MDgxNDE5NDczMFoXDTQ2MDgwOTE5NDczMFowHTEbMBkGA1UEAwwSVjI2NCBMb2NhbCBUZXN0IENBMIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAoHKIMddYph0JgzvaDsnkGvS203U7CnGet9aXwtTx9SFvCC9eb3OTDpKhsqwoi8MEf0YOZgsSWqdq7oYSs6hwNwbL9xXbRcu/Yxw2zSpbq3/SgElzelnAp9/mwyVuSDO7nRtbXLDIazoOmBJoVVuCBWT4sN0DLEWMibITv6FaAv8pji0a8eiCLby6nK6rSjhcq19Kq1/E0Dx6oOBrs4Rp06HM5lQ/A0+H1sjTn21awwBU4xXnRgjwBXD2xtXUaC5S4/whI3OIV5EnJalDxcfNxxyNSB5NQ6l90LPt27ZI5N0K/RReJUxrP4Kfd9h9Thx96iBbCq6mzl/d8K0Jw4ZyeQIDAQABo2MwYTAdBgNVHQ4EFgQUH0lX/2ClJ+DvRVyg2Ll4tPKQXx8wHwYDVR0jBBgwFoAUH0lX/2ClJ+DvRVyg2Ll4tPKQXx8wDwYDVR0TAQH/BAUwAwEB/zAOBgNVHQ8BAf8EBAMCAQYwDQYJKoZIhvcNAQELBQADggEBAGqjH7pFH4FFiqrRz8/DxBTF/FWUv5dr0BLEUpjqVXL7dmYlQ8BfJBRPSxP4Qxkh6slrdZ8eShHrrlV14Xn0AzzCI1yQbEhr6KqMFRr0NxEBYzkA1BEtOG4esaVEQa5Jx09Bl/qfv7oMwyif6meqGmlhL1CiVtS1a2FJfWy/SL5vBWvAJ3zTp3oKA1z23o+9Ci+R9aAFzgNaMIfnApMdlwDZvLNRlmpq+RG79BWQJtyrrbVLNVTnB7apCWYAxMIJJkdTJOgqallG82sngPGsXLXyU0i/ShecHpoUSZHhdxHqpCyur2/XC9fiy5ztan4YpV74ZTEh0Ibgi+Tg0s0IsZA=";
const TEST_LEAF_DER_BASE64: &str = "MIIDTDCCAjSgAwIBAgIUCRT/jjNGsV6JcwS1+dG0UYsSKbgwDQYJKoZIhvcNAQELBQAwHTEbMBkGA1UEAwwSVjI2NCBMb2NhbCBUZXN0IENBMB4XDTI2MDgxNDE5NDczMFoXDTQ2MDgwOTE5NDczMFowFDESMBAGA1UEAwwJbG9jYWxob3N0MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAqoiLKbmpMBKx2wT3z5rT+g1WmwQmyyZq/RWX82krnQXK2p8V42m0AyCOd8oNYzAptX+qzIjCqRIfeEkYxPFwXdpakzNOS3RDC8ZH24/wI1b5Oes1mMejeh61RKk7F0XuW67knf8azQk2lHmEKqTEyj2YkNhDLMwGlHl2aXInn41hJu07xaUN+QfqtlPX2gs4Yi0z+15h8thBxI0xKv65c2ny0PEuXhgX4wBKOmJ3kL8CWGXZhkMzpn1PO5ud1+UPFGkedegYctMO7Z3IulEyVcQ12frs/q/nyktAhRwVeH7UWlj+yYg5cYTRZ0JbMuLPArCi7gR12FrvdRu6gbTdgQIDAQABo4GMMIGJMBQGA1UdEQQNMAuCCWxvY2FsaG9zdDAMBgNVHRMBAf8EAjAAMA4GA1UdDwEB/wQEAwIFoDATBgNVHSUEDDAKBggrBgEFBQcDATAdBgNVHQ4EFgQUmM/zDRoEQL5yc/I/tDHz3rQv7HQwHwYDVR0jBBgwFoAUH0lX/2ClJ+DvRVyg2Ll4tPKQXx8wDQYJKoZIhvcNAQELBQADggEBAIfFXiXoF0pjBZNpE85xsoDRTmYFaAaK2nOXEdAear53WY6sKVVz2EeHfPb1R9/hBTGiwAnbqA44OrFvJnx+O/rNqtWscvNLXsJZTb+v7Zt+eP11oEqass9FSQ5qEbltJbguxbNLNpYGOs6GrfBwD/s4hBaFt8+QgVHVhvZqVSLdIGQyBoIQp0VC8lalfgsSsdUSmWtSqGTvWiRIkf11TFvXofBagGR2c/yBT9n+hT+ozSDQp/JQzr5UeRSgkgE8myMhIuoO5aiRqKOGrOmQkItsI1urVw0q8RafF+1d23ErqxS6C+XRA3wKVsRQUd5BQm7pz8B8ooH6yhq48aBxmBI=";
const TEST_LEAF_KEY_DER_BASE64: &str = "MIIEvQIBADANBgkqhkiG9w0BAQEFAASCBKcwggSjAgEAAoIBAQCqiIspuakwErHbBPfPmtP6DVabBCbLJmr9FZfzaSudBcranxXjabQDII53yg1jMCm1f6rMiMKpEh94SRjE8XBd2lqTM05LdEMLxkfbj/AjVvk56zWYx6N6HrVEqTsXRe5bruSd/xrNCTaUeYQqpMTKPZiQ2EMszAaUeXZpciefjWEm7TvFpQ35B+q2U9faCzhiLTP7XmHy2EHEjTEq/rlzafLQ8S5eGBfjAEo6YneQvwJYZdmGQzOmfU87m53X5Q8UaR516Bhy0w7tnci6UTJVxDXZ+uz+r+fKS0CFHBV4ftRaWP7JiDlxhNFnQlsy4s8CsKLuBHXYWu91G7qBtN2BAgMBAAECggEAArs0Z/VRNgYcV44smZtB23RTqdeAG9hpddSoEEu/RwDmGnq1p63Y75cHZa9UjI9+/Kkd3h3BxGIqbG/7RYFfAZCp77V3lQ3vbK3IF0aCG+I378su0YBj6EAE8aOmPEawPR6teIxXWjCdg6v4kLhH3AUTP4qjve6TdvlywbQEkJY+LCvhkcWtZ/gGK0oBgMVYzEKazlLlHHxVUq46o/5sSQokUGk41yElVSLPPoTCyZTGDZodD9KzFFCKs6eN5aXjkQI/G7j55emBZHrS2P6btteaVgxd8GklxlBKgHpaJ+N7d8NwpfL5xuilxT7FQxk1ZVItcK8KpRhz3FwwGrPM4wKBgQDEVMERp/Yn72LWgC2UaYyGd5TfF62OemDBPwiA0mSSYVQYZ4YMfT5udm3ssYms3mO9ER8ucNUz7SGD8atde6FkYWhcWWeOrqEtfX3X+DaFktGirs3JlXnJb/7EmAkwQuU7G1k+d9wRWqJOp6kqKcijQ7MAnDGwM3aVzAZHtSs45wKBgQDeXKNaDkH98DW+RH8XTA1a9cSm0ePMSZVSLbjEx1lC6xkGTK7ATib8KCj7FCkUaH6VuTza79vYEhalhlOXFaPUf92yP5oeEBYbaxb1Yl6nCsid4WbN9qXc+0lgOx7C3chN4Cos3F1k1FIQs9035YBGsF/KErY+WtjLHb1XnUNhVwKBgE5PtDcztOcXAGio9gVV2JymRDZ8fljvjXpnhx/DTCRrOB0H5htDNczf5lbcNhtDFauLkdF3ZkNxGcZEdmMydhzREcyMSNdL5rR7ct/bfPvopT/r09/NhKeJyahnMHsUo9Tgwsc9DgXKDiWrkLlls0cUMOlUZClxTaLQn8yoghYPAoGABW29jzVJ5yk2Jq8Fa0wwB0h4xJnbNeGWA6uaFzPGuhuDQOQeYBOIYB+a4IZdemIStRUQp0ez1lKauu/MmqOsnEOC5hcnbBR4dbLnnJYKOYnJ3BDksaKT6hE4eWD4H0nK2hve67l1jkCgwEej3vl7aD5mGEjcqikNoefX94ufWYECgYEAjafSz32KSIZdfG2QyhHOnQDIfmo9nqQSweiGZtoTDJDoN+kLWQqO+JzjUI0fllnwVtjdlVznz32a9ZQUB8fAFXTVOfdfc+hyfOqpgiD2Vdi2XNszPjnc3M0beEIPfdpIJlT7RjL3ZOxgtmMktovCKEkCMrk+dyj6dHjQXoz+BhE=";

struct LocalTlsServer {
    address: SocketAddr,
    certificate: CertificateDer<'static>,
    spki_pin: String,
    task: JoinHandle<Result<usize>>,
}

#[test]
fn public_address_policy_rejects_non_public_and_special_ranges() {
    let accepted = [
        IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1)),
        IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8)),
        IpAddr::V6("2606:4700:4700::1111".parse::<Ipv6Addr>().unwrap()),
    ];
    for address in accepted {
        assert!(is_public_unicast(address), "expected public: {address}");
    }
    let rejected = [
        "0.0.0.0",
        "10.0.0.1",
        "100.64.0.1",
        "127.0.0.1",
        "169.254.1.1",
        "172.16.0.1",
        "192.0.2.1",
        "192.168.1.1",
        "198.18.0.1",
        "198.51.100.1",
        "203.0.113.1",
        "224.0.0.1",
        "255.255.255.255",
        "::",
        "::1",
        "::ffff:127.0.0.1",
        "fc00::1",
        "fe80::1",
        "2001:db8::1",
        "2002::1",
        "64:ff9b::1",
        "ff02::1",
    ];
    for address in rejected {
        let address = address.parse::<IpAddr>().unwrap();
        assert!(!is_public_unicast(address), "expected rejection: {address}");
    }
}

#[test]
fn dns_answers_fail_closed_and_are_deterministically_deduplicated() {
    let public_v4 = SocketAddr::from(([8, 8, 8, 8], 443));
    let public_v6 = SocketAddr::new("2606:4700:4700::1111".parse().unwrap(), 443);
    let ordered =
        validate_and_order_dns_answers([public_v6, public_v4, public_v4], 443, 4).unwrap();
    assert_eq!(ordered, vec![public_v4, public_v6]);
    assert!(validate_and_order_dns_answers(
        [public_v4, SocketAddr::from(([127, 0, 0, 1], 443))],
        443,
        4,
    )
    .is_err());
    assert!(validate_and_order_dns_answers([public_v4], 8443, 4).is_err());
    assert!(validate_and_order_dns_answers([public_v4, public_v4], 443, 1).is_err());
}

#[tokio::test]
async fn local_tls13_fixture_accepts_webpki_hostname_and_exact_spki_without_app_bytes() -> Result<()>
{
    let server = start_server(&[&rustls::version::TLS13]).await?;
    let target = target(&server, "localhost", &server.spki_pin)?;
    let channel = connect_external_pool_adapter_broker_tls_for_test(
        target,
        vec![server.address],
        roots_with(&server.certificate)?,
    )
    .await?;
    assert!(channel.is_current());
    assert_eq!(channel.selected_address(), server.address);
    drop(channel);
    assert_eq!(server.task.await??, 0);
    Ok(())
}

#[tokio::test]
async fn local_fixture_rejects_hostname_mismatch() -> Result<()> {
    let server = start_server(&[&rustls::version::TLS13]).await?;
    let target = target(&server, "wrong.example", &server.spki_pin)?;
    assert!(connect_external_pool_adapter_broker_tls_for_test(
        target,
        vec![server.address],
        roots_with(&server.certificate)?,
    )
    .await
    .is_err());
    assert_eq!(server.task.await??, 0);
    Ok(())
}

#[tokio::test]
async fn local_fixture_rejects_exact_spki_mismatch() -> Result<()> {
    let server = start_server(&[&rustls::version::TLS13]).await?;
    let target = target(&server, "localhost", &"44".repeat(32))?;
    assert!(connect_external_pool_adapter_broker_tls_for_test(
        target,
        vec![server.address],
        roots_with(&server.certificate)?,
    )
    .await
    .is_err());
    assert_eq!(server.task.await??, 0);
    Ok(())
}

#[tokio::test]
async fn local_fixture_rejects_untrusted_certificate() -> Result<()> {
    let server = start_server(&[&rustls::version::TLS13]).await?;
    let target = target(&server, "localhost", &server.spki_pin)?;
    assert!(connect_external_pool_adapter_broker_tls_for_test(
        target,
        vec![server.address],
        RootCertStore::empty(),
    )
    .await
    .is_err());
    assert_eq!(server.task.await??, 0);
    Ok(())
}

#[tokio::test]
async fn tls12_only_server_is_rejected() -> Result<()> {
    let server = start_server(&[&rustls::version::TLS12]).await?;
    let target = target(&server, "localhost", &server.spki_pin)?;
    assert!(connect_external_pool_adapter_broker_tls_for_test(
        target,
        vec![server.address],
        roots_with(&server.certificate)?,
    )
    .await
    .is_err());
    assert_eq!(server.task.await??, 0);
    Ok(())
}

#[tokio::test]
async fn refused_tcp_connection_is_rejected() -> Result<()> {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await?;
    let address = listener.local_addr()?;
    drop(listener);
    let certificate = fixture_certificate(TEST_LEAF_DER_BASE64)?;
    let target = ExternalPoolAdapterBrokerTlsTarget::for_test(
        "localhost",
        address.port(),
        &spki_pin(&certificate)?,
    )?;
    assert!(connect_external_pool_adapter_broker_tls_for_test(
        target,
        vec![address],
        roots_with(&certificate)?,
    )
    .await
    .is_err());
    Ok(())
}

#[tokio::test]
async fn local_tls_fixture_relays_one_exact_bounded_no_work_exchange() -> Result<()> {
    const REQUEST: &[u8] = b"ELON-TEST-NO-WORK\n";
    const RESPONSE: &[u8] = b"ELON-TEST-NO-TASK\n";
    let server = start_exchange_server(REQUEST, RESPONSE).await?;
    let target = target(&server, "localhost", &server.spki_pin)?;
    let mut channel = connect_external_pool_adapter_broker_tls_for_test(
        target,
        vec![server.address],
        roots_with(&server.certificate)?,
    )
    .await?;
    let response = exchange_external_pool_adapter_broker_no_work(
        &mut channel,
        REQUEST,
        RESPONSE.len(),
        Duration::from_secs(1),
    )
    .await?;
    assert_eq!(&response[..], RESPONSE);
    assert!(exchange_external_pool_adapter_broker_no_work(
        &mut channel,
        REQUEST,
        RESPONSE.len(),
        Duration::from_secs(1),
    )
    .await
    .is_err());
    assert_eq!(server.task.await??, REQUEST.len());
    Ok(())
}

#[tokio::test]
async fn local_tls_fixture_rejects_truncated_no_work_response() -> Result<()> {
    const REQUEST: &[u8] = b"ELON-TEST-NO-WORK\n";
    const TRUNCATED_RESPONSE: &[u8] = b"ELON-TEST-NO-TASK";
    let server = start_exchange_server(REQUEST, TRUNCATED_RESPONSE).await?;
    let target = target(&server, "localhost", &server.spki_pin)?;
    let mut channel = connect_external_pool_adapter_broker_tls_for_test(
        target,
        vec![server.address],
        roots_with(&server.certificate)?,
    )
    .await?;
    assert!(exchange_external_pool_adapter_broker_no_work(
        &mut channel,
        REQUEST,
        TRUNCATED_RESPONSE.len() + 1,
        Duration::from_secs(1),
    )
    .await
    .is_err());
    assert_eq!(server.task.await??, REQUEST.len());
    Ok(())
}

fn target(
    server: &LocalTlsServer,
    server_name: &str,
    pin: &str,
) -> Result<ExternalPoolAdapterBrokerTlsTarget> {
    ExternalPoolAdapterBrokerTlsTarget::for_test(server_name, server.address.port(), pin)
}

async fn start_server(protocols: &[&'static SupportedProtocolVersion]) -> Result<LocalTlsServer> {
    let certificate = fixture_certificate(TEST_LEAF_DER_BASE64)?;
    let trust_anchor = fixture_certificate(TEST_CA_DER_BASE64)?;
    let private_key = PrivatePkcs8KeyDer::from(STANDARD.decode(TEST_LEAF_KEY_DER_BASE64)?);
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let config = ServerConfig::builder_with_provider(provider)
        .with_protocol_versions(protocols)?
        .with_no_client_auth()
        .with_single_cert(vec![certificate.clone()], private_key.into())?;
    let acceptor = TlsAcceptor::from(Arc::new(config));
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await?;
    let address = listener.local_addr()?;
    let task = tokio::spawn(async move {
        let (tcp, _) = listener.accept().await?;
        let mut tls = match acceptor.accept(tcp).await {
            Ok(stream) => stream,
            Err(_) => return Ok(0),
        };
        let mut one = [0_u8; 1];
        match tokio::time::timeout(Duration::from_millis(200), tls.read(&mut one)).await {
            Err(_) | Ok(Ok(0)) => Ok(0),
            Ok(Ok(read)) => Ok(read),
            Ok(Err(_)) => Ok(0),
        }
    });
    Ok(LocalTlsServer {
        address,
        spki_pin: spki_pin(&certificate)?,
        certificate: trust_anchor,
        task,
    })
}

async fn start_exchange_server(
    request: &'static [u8],
    response: &'static [u8],
) -> Result<LocalTlsServer> {
    let certificate = fixture_certificate(TEST_LEAF_DER_BASE64)?;
    let trust_anchor = fixture_certificate(TEST_CA_DER_BASE64)?;
    let private_key = PrivatePkcs8KeyDer::from(STANDARD.decode(TEST_LEAF_KEY_DER_BASE64)?);
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let config = ServerConfig::builder_with_provider(provider)
        .with_protocol_versions(&[&rustls::version::TLS13])?
        .with_no_client_auth()
        .with_single_cert(vec![certificate.clone()], private_key.into())?;
    let acceptor = TlsAcceptor::from(Arc::new(config));
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await?;
    let address = listener.local_addr()?;
    let task = tokio::spawn(async move {
        let (tcp, _) = listener.accept().await?;
        let mut tls = acceptor.accept(tcp).await?;
        let mut observed = vec![0_u8; request.len()];
        tls.read_exact(&mut observed).await?;
        if observed != request {
            anyhow::bail!("local no-work request mismatch");
        }
        tls.write_all(response).await?;
        tls.flush().await?;
        Ok(observed.len())
    });
    Ok(LocalTlsServer {
        address,
        spki_pin: spki_pin(&certificate)?,
        certificate: trust_anchor,
        task,
    })
}

fn roots_with(certificate: &CertificateDer<'static>) -> Result<RootCertStore> {
    let mut roots = RootCertStore::empty();
    roots.add(certificate.clone())?;
    Ok(roots)
}

fn spki_pin(certificate: &CertificateDer<'static>) -> Result<String> {
    Ok(hex::encode(leaf_spki_sha256(certificate.as_ref())?))
}

fn fixture_certificate(value: &str) -> Result<CertificateDer<'static>> {
    Ok(CertificateDer::from(STANDARD.decode(value)?))
}
