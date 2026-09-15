use super::{challenge::ChallengeServer, config::Config, storage};
use anyhow::{bail, Context, Result};
use instant_acme::{
    Account, AuthorizationStatus, ChallengeType, Identifier, LetsEncrypt, NewAccount, NewOrder,
    OrderStatus, RetryPolicy,
};
use rcgen::{CertificateParams, DistinguishedName, KeyPair};
use std::{fs, time::Duration};

pub(super) async fn issue(config: &Config) -> Result<i64> {
    let mut phase = "ACCOUNT";
    let result: Result<i64> = async {
        let path = config.dir.join("account.json");
        let account = match fs::read(&path) {
            Ok(bytes) => {
                Account::builder()?
                    .from_credentials(serde_json::from_slice(&bytes)?)
                    .await?
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                let directory = if config.staging {
                    LetsEncrypt::Staging
                } else {
                    LetsEncrypt::Production
                };
                let (account, credentials) = Account::builder()?
                    .create(
                        &NewAccount {
                            contact: &[],
                            terms_of_service_agreed: true,
                            only_return_existing: false,
                        },
                        directory.url().to_owned(),
                        None,
                    )
                    .await?;
                storage::atomic_private(&path, &serde_json::to_vec(&credentials)?)?;
                account
            }
            Err(e) => return Err(e.into()),
        };
        phase = "ORDER";
        let ids = [Identifier::Ip(config.ip)];
        let mut order = account
            .new_order(&NewOrder::new(&ids).profile("shortlived"))
            .await?;
        let mut challenge_server = None;
        phase = "AUTHORIZATION";
        let mut auths = order.authorizations();
        while let Some(auth) = auths.next().await {
            let mut auth = auth?;
            match auth.status {
                AuthorizationStatus::Valid => continue,
                AuthorizationStatus::Pending => {}
                _ => bail!("ACME authorization rejected"),
            }
            phase = "CHALLENGE_SELECT";
            let mut challenge = auth
                .challenge(ChallengeType::TlsAlpn01)
                .context("TLS-ALPN-01 unavailable")?;
            phase = "CHALLENGE_BIND";
            challenge_server = Some(
                ChallengeServer::bind(
                    config.listen,
                    config.ip,
                    challenge.key_authorization().digest().as_ref(),
                )
                .await?,
            );
            phase = "CHALLENGE_START";
            challenge.set_ready().await?;
        }
        let policy = RetryPolicy::new()
            .initial_delay(Duration::from_secs(2))
            .timeout(Duration::from_secs(180));
        phase = "CHALLENGE_VALIDATE";
        let ready = order.poll_ready(&policy).await;
        if !matches!(ready, Ok(OrderStatus::Ready)) {
            // Order status alone can omit the useful validation failure. Refresh
            // our own authorizations and retain only the typed CA error code.
            let mut auths = order.authorizations();
            while let Some(auth) = auths.next().await {
                let mut auth = auth?;
                let state = auth.refresh().await?;
                if let Some(error) = state.challenges.iter().find_map(|c| c.error.clone()) {
                    return Err(instant_acme::Error::Api(error).into());
                }
            }
            ready?;
            bail!("ACME order not ready");
        }
        drop(challenge_server);
        phase = "CERTIFICATE";
        let mut params = CertificateParams::new(vec![config.ip.to_string()])?;
        params.distinguished_name = DistinguishedName::new();
        let key = KeyPair::generate()?;
        order
            .finalize_csr(params.serialize_request(&key)?.der())
            .await?;
        let certificate = order.poll_certificate(&policy).await?;
        phase = "ACTIVATE";
        storage::activate(config, &certificate, &key.serialize_pem())
    }
    .await;
    result.map_err(|error| {
        let code = super::diagnostics::classify(&error);
        error.context(format!("ACCOUNT_ACME_{phase}_{code}"))
    })
}
