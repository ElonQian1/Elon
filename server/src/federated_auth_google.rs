//! Google OpenID Connect verification for first-party login and account binding.

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use chrono::Utc;
use rsa::{
    pkcs1v15::{Signature, VerifyingKey},
    signature::Verifier,
    BigUint, RsaPublicKey,
};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{collections::HashMap, sync::OnceLock, time::Duration};
use thiserror::Error;
use tokio::{sync::RwLock, time::Instant};

use crate::store::VerifiedIdentity;

const GOOGLE_JWKS_URL: &str = "https://www.googleapis.com/oauth2/v3/certs";

#[derive(Debug, Clone)]
pub struct GoogleOidcConfig {
    pub client_ids: Vec<String>,
}

impl GoogleOidcConfig {
    pub fn from_env() -> Option<Self> {
        let values = std::env::var("ELON_GOOGLE_OIDC_CLIENT_IDS")
            .ok()
            .or_else(|| std::env::var("ELON_GOOGLE_OIDC_CLIENT_ID").ok())?;
        let client_ids = values
            .split(',')
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
            .collect::<Vec<_>>();
        (!client_ids.is_empty()).then_some(Self { client_ids })
    }

    pub fn primary_client_id(&self) -> &str {
        &self.client_ids[0]
    }
}

#[derive(Debug, Error)]
pub enum GoogleIdentityError {
    #[error("Google 登录尚未配置")]
    NotConfigured,
    #[error("Google ID token 格式无效")]
    MalformedToken,
    #[error("Google ID token 使用了不受支持的签名算法")]
    UnsupportedAlgorithm,
    #[error("无法验证 Google ID token 签名")]
    InvalidSignature,
    #[error("Google 登录挑战不匹配")]
    InvalidNonce,
    #[error("Google ID token 的签发方、受众或有效期无效")]
    InvalidClaims,
    #[error("Google 账号没有已验证邮箱")]
    UnverifiedEmail,
    #[error("暂时无法获取 Google 公钥")]
    KeyServiceUnavailable,
}

#[derive(Debug, Deserialize)]
struct JwtHeader {
    alg: String,
    kid: String,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum Audience {
    One(String),
    Many(Vec<String>),
}

#[derive(Debug, Deserialize)]
struct GoogleClaims {
    iss: String,
    sub: String,
    aud: Audience,
    azp: Option<String>,
    exp: i64,
    iat: Option<i64>,
    nonce: Option<String>,
    email: Option<String>,
    email_verified: Option<bool>,
    name: Option<String>,
    picture: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct Jwk {
    kid: String,
    kty: String,
    alg: Option<String>,
    n: String,
    e: String,
}

#[derive(Debug, Deserialize)]
struct JwkSet {
    keys: Vec<Jwk>,
}

struct JwkCache {
    keys: HashMap<String, Jwk>,
    valid_until: Instant,
}

static GOOGLE_JWKS: OnceLock<RwLock<Option<JwkCache>>> = OnceLock::new();

pub async fn verify_google_id_token(
    token: &str,
    expected_nonce_hash: &str,
) -> Result<VerifiedIdentity, GoogleIdentityError> {
    let config = GoogleOidcConfig::from_env().ok_or(GoogleIdentityError::NotConfigured)?;
    let segments = token.trim().split('.').collect::<Vec<_>>();
    if segments.len() != 3 || segments.iter().any(|segment| segment.is_empty()) {
        return Err(GoogleIdentityError::MalformedToken);
    }
    let header: JwtHeader = decode_segment(segments[0])?;
    if header.alg != "RS256" {
        return Err(GoogleIdentityError::UnsupportedAlgorithm);
    }
    let claims: GoogleClaims = decode_segment(segments[1])?;
    let signature_bytes = URL_SAFE_NO_PAD
        .decode(segments[2])
        .map_err(|_| GoogleIdentityError::MalformedToken)?;
    let signature = Signature::try_from(signature_bytes.as_slice())
        .map_err(|_| GoogleIdentityError::MalformedToken)?;
    let jwk = google_jwk(&header.kid).await?;
    if jwk.kty != "RSA" || jwk.alg.as_deref().is_some_and(|alg| alg != "RS256") {
        return Err(GoogleIdentityError::UnsupportedAlgorithm);
    }
    let modulus = BigUint::from_bytes_be(
        &URL_SAFE_NO_PAD
            .decode(jwk.n)
            .map_err(|_| GoogleIdentityError::KeyServiceUnavailable)?,
    );
    let exponent = BigUint::from_bytes_be(
        &URL_SAFE_NO_PAD
            .decode(jwk.e)
            .map_err(|_| GoogleIdentityError::KeyServiceUnavailable)?,
    );
    let public_key = RsaPublicKey::new(modulus, exponent)
        .map_err(|_| GoogleIdentityError::KeyServiceUnavailable)?;
    VerifyingKey::<Sha256>::new(public_key)
        .verify(
            format!("{}.{}", segments[0], segments[1]).as_bytes(),
            &signature,
        )
        .map_err(|_| GoogleIdentityError::InvalidSignature)?;

    validate_claims(&claims, &config, expected_nonce_hash)?;
    Ok(VerifiedIdentity {
        provider: "google".to_string(),
        issuer: "https://accounts.google.com".to_string(),
        subject: claims.sub,
        email: claims.email.ok_or(GoogleIdentityError::UnverifiedEmail)?,
        display_name: claims.name,
        avatar_url: claims.picture,
        nonce: claims.nonce.ok_or(GoogleIdentityError::InvalidNonce)?,
    })
}

fn validate_claims(
    claims: &GoogleClaims,
    config: &GoogleOidcConfig,
    expected_nonce_hash: &str,
) -> Result<(), GoogleIdentityError> {
    if !matches!(
        claims.iss.as_str(),
        "accounts.google.com" | "https://accounts.google.com"
    ) || claims.sub.is_empty()
    {
        return Err(GoogleIdentityError::InvalidClaims);
    }
    let audiences = match &claims.aud {
        Audience::One(value) => vec![value.as_str()],
        Audience::Many(values) => values.iter().map(String::as_str).collect(),
    };
    if !audiences.iter().any(|audience| {
        config
            .client_ids
            .iter()
            .any(|allowed| allowed.as_str() == *audience)
    }) || (audiences.len() > 1
        && !claims
            .azp
            .as_deref()
            .is_some_and(|azp| config.client_ids.iter().any(|allowed| allowed == azp)))
    {
        return Err(GoogleIdentityError::InvalidClaims);
    }
    let timestamp = Utc::now().timestamp();
    if claims.exp <= timestamp - 60
        || claims
            .iat
            .is_some_and(|issued_at| issued_at > timestamp + 60)
    {
        return Err(GoogleIdentityError::InvalidClaims);
    }
    if claims.email_verified != Some(true)
        || claims
            .email
            .as_deref()
            .is_none_or(|email| !email.contains('@'))
    {
        return Err(GoogleIdentityError::UnverifiedEmail);
    }
    let nonce = claims
        .nonce
        .as_deref()
        .ok_or(GoogleIdentityError::InvalidNonce)?;
    if hex::encode(Sha256::digest(nonce.as_bytes())) != expected_nonce_hash {
        return Err(GoogleIdentityError::InvalidNonce);
    }
    Ok(())
}

fn decode_segment<T: for<'de> Deserialize<'de>>(value: &str) -> Result<T, GoogleIdentityError> {
    let bytes = URL_SAFE_NO_PAD
        .decode(value)
        .map_err(|_| GoogleIdentityError::MalformedToken)?;
    serde_json::from_slice(&bytes).map_err(|_| GoogleIdentityError::MalformedToken)
}

async fn google_jwk(kid: &str) -> Result<Jwk, GoogleIdentityError> {
    let cache = GOOGLE_JWKS.get_or_init(|| RwLock::new(None));
    {
        let guard = cache.read().await;
        if let Some(entry) = guard
            .as_ref()
            .filter(|entry| entry.valid_until > Instant::now())
        {
            if let Some(key) = entry.keys.get(kid) {
                return Ok(key.clone());
            }
        }
    }

    let response = reqwest::Client::new()
        .get(GOOGLE_JWKS_URL)
        .timeout(Duration::from_secs(10))
        .send()
        .await
        .map_err(|_| GoogleIdentityError::KeyServiceUnavailable)?;
    if !response.status().is_success() {
        return Err(GoogleIdentityError::KeyServiceUnavailable);
    }
    let max_age = response
        .headers()
        .get(reqwest::header::CACHE_CONTROL)
        .and_then(|value| value.to_str().ok())
        .and_then(parse_max_age)
        .unwrap_or(300)
        .clamp(60, 3600);
    if response
        .content_length()
        .is_some_and(|length| length > 1024 * 1024)
    {
        return Err(GoogleIdentityError::KeyServiceUnavailable);
    }
    let bytes = response
        .bytes()
        .await
        .map_err(|_| GoogleIdentityError::KeyServiceUnavailable)?;
    if bytes.len() > 1024 * 1024 {
        return Err(GoogleIdentityError::KeyServiceUnavailable);
    }
    let key_set = serde_json::from_slice::<JwkSet>(&bytes)
        .map_err(|_| GoogleIdentityError::KeyServiceUnavailable)?;
    let keys = key_set
        .keys
        .into_iter()
        .map(|key| (key.kid.clone(), key))
        .collect::<HashMap<_, _>>();
    let key = keys
        .get(kid)
        .cloned()
        .ok_or(GoogleIdentityError::InvalidSignature)?;
    *cache.write().await = Some(JwkCache {
        keys,
        valid_until: Instant::now() + Duration::from_secs(max_age),
    });
    Ok(key)
}

fn parse_max_age(value: &str) -> Option<u64> {
    value.split(',').find_map(|part| {
        part.trim()
            .strip_prefix("max-age=")
            .and_then(|seconds| seconds.parse().ok())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_control_max_age_is_bounded_by_caller() {
        assert_eq!(parse_max_age("public, max-age=1234"), Some(1234));
        assert_eq!(parse_max_age("no-cache"), None);
    }

    #[test]
    fn claims_require_expected_audience_and_nonce() {
        let nonce = "nonce-for-test";
        let config = GoogleOidcConfig {
            client_ids: vec!["web-client.apps.googleusercontent.com".to_string()],
        };
        let claims = GoogleClaims {
            iss: "https://accounts.google.com".to_string(),
            sub: "stable-subject".to_string(),
            aud: Audience::One("web-client.apps.googleusercontent.com".to_string()),
            azp: None,
            exp: Utc::now().timestamp() + 300,
            iat: Some(Utc::now().timestamp()),
            nonce: Some(nonce.to_string()),
            email: Some("verified@example.com".to_string()),
            email_verified: Some(true),
            name: None,
            picture: None,
        };
        let expected_nonce = hex::encode(Sha256::digest(nonce.as_bytes()));
        assert!(validate_claims(&claims, &config, &expected_nonce).is_ok());
        assert!(matches!(
            validate_claims(&claims, &config, "wrong-nonce-hash"),
            Err(GoogleIdentityError::InvalidNonce)
        ));
        let wrong_config = GoogleOidcConfig {
            client_ids: vec!["other-client".to_string()],
        };
        assert!(matches!(
            validate_claims(&claims, &wrong_config, &expected_nonce),
            Err(GoogleIdentityError::InvalidClaims)
        ));
    }
}
