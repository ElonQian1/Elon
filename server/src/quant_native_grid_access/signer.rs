use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use hmac::{Hmac, Mac};
use ring::signature::Ed25519KeyPair;
use serde::Serialize;
use sha2::Sha256;

use super::model::{Error, GrantResponse, Scope, ENVIRONMENT, MAX_LIFETIME};

const KEY: &str = "YILONG_QUANT_NATIVE_GRID_SIGNING_KEY_ID";
const SEED: &str = "YILONG_QUANT_NATIVE_GRID_SIGNING_SEED_BASE64URL";
const SUBJECT: &str = "YILONG_QUANT_NATIVE_GRID_SUBJECT_SECRET_BASE64URL";

pub(super) struct Signer {
    key_id: String,
    signing_key: Ed25519KeyPair,
    subject_secret: [u8; 32],
}

#[derive(Serialize)]
struct Claims<'a> {
    schema: &'static str,
    issuer: &'static str,
    audience: &'static str,
    environment: &'static str,
    grant_id: &'a str,
    subject_ref: &'a str,
    key_id: &'a str,
    scopes: &'a [Scope],
    issued_at_unix: i64,
    expires_at_unix: i64,
}

impl Signer {
    pub fn from_env() -> Result<Self, Error> {
        #[cfg(test)]
        if let Some(value) = test_config::value() {
            return value;
        }
        Self::from_lookup(|key| match std::env::var(key) {
            Ok(value) => Ok(Some(value)),
            Err(std::env::VarError::NotPresent) => Ok(None),
            Err(_) => Err(Error::Misconfigured),
        })
    }

    pub fn from_lookup(
        read: impl Fn(&str) -> Result<Option<String>, Error>,
    ) -> Result<Self, Error> {
        match (read(KEY)?, read(SEED)?, read(SUBJECT)?) {
            (None, None, None) => Err(Error::Disabled),
            (Some(key_id), Some(seed), Some(subject)) => {
                if !(3..=64).contains(&key_id.len())
                    || !key_id
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
                {
                    return Err(Error::Misconfigured);
                }
                let seed = decode_secret(&seed)?;
                let subject_secret = decode_secret(&subject)?;
                let signing_key =
                    Ed25519KeyPair::from_seed_unchecked(&seed).map_err(|_| Error::Misconfigured)?;
                Ok(Self {
                    key_id,
                    signing_key,
                    subject_secret,
                })
            }
            _ => Err(Error::Misconfigured),
        }
    }

    pub fn issue(
        &self,
        user: &str,
        scopes: Vec<Scope>,
        now: i64,
        expires: i64,
    ) -> Result<GrantResponse, Error> {
        self.issue_with_id(
            user,
            scopes,
            now,
            expires,
            format!("ngg_{}", uuid::Uuid::new_v4().simple()),
        )
    }

    fn issue_with_id(
        &self,
        user: &str,
        scopes: Vec<Scope>,
        now: i64,
        expires: i64,
        grant_id: String,
    ) -> Result<GrantResponse, Error> {
        if user.trim().is_empty()
            || user == "local-owner"
            || now <= 0
            || expires
                .checked_sub(now)
                .filter(|v| (1..=MAX_LIFETIME).contains(v))
                .is_none()
            || scopes.is_empty()
            || scopes.len() > 3
            || scopes
                .iter()
                .enumerate()
                .any(|(i, v)| scopes[..i].contains(v))
        {
            return Err(Error::InvalidInput);
        }
        let mut mac =
            Hmac::<Sha256>::new_from_slice(&self.subject_secret).map_err(|_| Error::Unavailable)?;
        mac.update(b"yilong.quant.native_grid.subject.v1\0");
        mac.update(user.as_bytes());
        let subject_ref = format!("ngu_{}", hex::encode(&mac.finalize().into_bytes()[..20]));
        let claims = Claims {
            schema: "yilong.quant.native_grid_access_grant.v1",
            issuer: "yilong-main",
            audience: "yilong-quant-native-grid",
            environment: ENVIRONMENT,
            grant_id: &grant_id,
            subject_ref: &subject_ref,
            key_id: &self.key_id,
            scopes: &scopes,
            issued_at_unix: now,
            expires_at_unix: expires,
        };
        let payload = serde_json::to_vec(&claims).map_err(|_| Error::Unavailable)?;
        if payload.len() > 2048 {
            return Err(Error::Unavailable);
        }
        let input = format!("yng1.{}", URL_SAFE_NO_PAD.encode(payload));
        let signature = self.signing_key.sign(input.as_bytes());
        let access_token = format!("{input}.{}", URL_SAFE_NO_PAD.encode(signature.as_ref()));
        if access_token.len() > 4096 {
            return Err(Error::Unavailable);
        }
        Ok(GrantResponse {
            schema: "yilong.quant.native_grid_issue_result.v1",
            token_type: "Bearer",
            access_token,
            grant_id,
            subject_ref,
            scopes,
            expires_in: expires - now,
            expires_at_unix: expires,
            environment: ENVIRONMENT,
        })
    }
}

fn decode_secret(raw: &str) -> Result<[u8; 32], Error> {
    if raw.len() != 43 {
        return Err(Error::Misconfigured);
    }
    let bytes = URL_SAFE_NO_PAD
        .decode(raw)
        .map_err(|_| Error::Misconfigured)?;
    if URL_SAFE_NO_PAD.encode(&bytes) != raw {
        return Err(Error::Misconfigured);
    }
    bytes.try_into().map_err(|_| Error::Misconfigured)
}

#[cfg(test)]
#[path = "signer_tests.rs"]
mod tests;

#[cfg(test)]
pub(crate) mod test_config {
    use super::*;
    use std::cell::Cell;
    thread_local! {static ENABLED:Cell<Option<bool>>=const {Cell::new(None)};}
    pub(crate) struct Guard(Option<bool>);
    impl Drop for Guard {
        fn drop(&mut self) {
            ENABLED.with(|v| v.set(self.0));
        }
    }
    pub(crate) fn set(enabled: bool) -> Guard {
        Guard(ENABLED.with(|v| v.replace(Some(enabled))))
    }
    pub(super) fn value() -> Option<Result<Signer, Error>> {
        ENABLED.with(|v| v.get()).map(|enabled| {
            if !enabled {
                Err(Error::Disabled)
            } else {
                Signer::from_lookup(|name| {
                    Ok(Some(match name {
                        KEY => "native-grid-test-key".into(),
                        SEED => URL_SAFE_NO_PAD.encode([7; 32]),
                        SUBJECT => URL_SAFE_NO_PAD.encode([11; 32]),
                        _ => unreachable!(),
                    }))
                })
            }
        })
    }
}
