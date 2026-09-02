use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

pub(crate) use super::quant_paper_signer::{PaperGrantSigner, SignerConfigError};
use crate::{project_auth::auth_from_headers, types::AppState};

const SCHEMA: &str = "yilong.quant.paper_access_grant.v1";
const ISSUER: &str = "yilong-main";
const AUDIENCE: &str = "yilong-quant";
const RISK_REVISION: &str = "paper-participation-risk-v1";
const TOKEN_PREFIX: &str = "ypg1";
const GRANT_LIFETIME_SECONDS: i64 = 300;

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub(crate) enum PaperAccessScope {
    #[serde(rename = "paper.position.read")]
    PositionRead,
    #[serde(rename = "paper.redemption.request")]
    RedemptionRequest,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct IssuePaperAccessGrantRequest {
    scopes: Vec<PaperAccessScope>,
}

#[derive(Debug, Serialize)]
struct PaperAccessGrantClaims {
    schema: &'static str,
    grant_id: String,
    issuer: &'static str,
    audience: &'static str,
    key_id: String,
    participant_ref: String,
    scopes: Vec<PaperAccessScope>,
    risk_revision: &'static str,
    issued_at_unix: i64,
    expires_at_unix: i64,
    simulated: bool,
}

#[derive(Debug, Serialize)]
pub(crate) struct PaperAccessGrantResponse {
    token_type: &'static str,
    pub(crate) access_token: String,
    #[serde(skip_serializing)]
    pub(crate) grant_id: String,
    pub(crate) expires_in: i64,
    pub(crate) participant_ref: String,
    scopes: Vec<PaperAccessScope>,
    simulated: bool,
}

#[derive(Serialize)]
struct ErrorResponse {
    code: &'static str,
    message: &'static str,
}

pub(crate) async fn issue(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(request): Json<IssuePaperAccessGrantRequest>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(_) => {
            return error_response(
                StatusCode::UNAUTHORIZED,
                "authentication_required",
                "登录后才能签发量化 Paper 授权",
            )
        }
    };
    if user.status != "active" {
        return error_response(
            StatusCode::FORBIDDEN,
            "account_not_active",
            "当前账号不能签发量化 Paper 授权",
        );
    }
    let signer = match PaperGrantSigner::from_env() {
        Ok(signer) => signer,
        Err(SignerConfigError::Disabled) => {
            return error_response(
                StatusCode::SERVICE_UNAVAILABLE,
                "quant_paper_access_disabled",
                "量化 Paper 授权签发尚未配置",
            )
        }
        Err(SignerConfigError::Invalid) => {
            return error_response(
                StatusCode::SERVICE_UNAVAILABLE,
                "quant_paper_access_misconfigured",
                "量化 Paper 授权签发配置无效",
            )
        }
    };
    let scopes = match validate_scopes(request.scopes) {
        Ok(scopes) => scopes,
        Err(()) => {
            return error_response(
                StatusCode::BAD_REQUEST,
                "invalid_quant_paper_scope",
                "量化 Paper 授权 scope 无效",
            )
        }
    };
    match signer.issue(&user.id, scopes, chrono::Utc::now().timestamp()) {
        Ok(response) => Json(response).into_response(),
        Err(()) => error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "quant_paper_access_unavailable",
            "暂时无法签发量化 Paper 授权",
        ),
    }
}

impl PaperGrantSigner {
    pub(crate) fn issue(
        &self,
        user_id: &str,
        scopes: Vec<PaperAccessScope>,
        now_unix: i64,
    ) -> Result<PaperAccessGrantResponse, ()> {
        if user_id.trim().is_empty() || now_unix <= 0 {
            return Err(());
        }
        let participant_ref = self.participant_ref(user_id)?;
        let grant_id = format!("qpg_{}", Uuid::new_v4().simple());
        let claims = PaperAccessGrantClaims {
            schema: SCHEMA,
            grant_id: grant_id.clone(),
            issuer: ISSUER,
            audience: AUDIENCE,
            key_id: self.key_id().to_owned(),
            participant_ref: participant_ref.clone(),
            scopes: scopes.clone(),
            risk_revision: RISK_REVISION,
            issued_at_unix: now_unix,
            expires_at_unix: now_unix + GRANT_LIFETIME_SECONDS,
            simulated: true,
        };
        Ok(PaperAccessGrantResponse {
            token_type: "Bearer",
            access_token: self.sign_token(TOKEN_PREFIX, &claims)?,
            grant_id,
            expires_in: GRANT_LIFETIME_SECONDS,
            participant_ref,
            scopes,
            simulated: true,
        })
    }
}

fn validate_scopes(mut scopes: Vec<PaperAccessScope>) -> Result<Vec<PaperAccessScope>, ()> {
    if scopes.is_empty() || scopes.len() > 2 {
        return Err(());
    }
    scopes.sort_unstable();
    let original_len = scopes.len();
    scopes.dedup();
    if scopes.len() != original_len {
        return Err(());
    }
    Ok(scopes)
}

fn error_response(status: StatusCode, code: &'static str, message: &'static str) -> Response {
    (status, Json(ErrorResponse { code, message })).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
    use ring::signature::{KeyPair, UnparsedPublicKey, ED25519};

    const SEED: [u8; 32] = [7; 32];
    const SUBJECT_SECRET: [u8; 32] = [11; 32];

    #[test]
    fn cross_repository_fixture_uses_the_main_grant_serializer() {
        // Public deterministic test material only; never deploy this seed.
        const INTEROP_TEST_SEED: [u8; 32] = [61; 32];
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../contracts/quant/esk-paper-cross-repo-interoperability-v1.fixture.json"
        ))
        .unwrap();
        let key_id = fixture["main"]["key_id"].as_str().unwrap();
        let signer =
            PaperGrantSigner::from_material(key_id.to_owned(), &INTEROP_TEST_SEED, &[63; 32])
                .unwrap();
        let claims = PaperAccessGrantClaims {
            schema: SCHEMA,
            grant_id: fixture["expected"]["grant_id"].as_str().unwrap().to_owned(),
            issuer: ISSUER,
            audience: AUDIENCE,
            key_id: key_id.to_owned(),
            participant_ref: fixture["expected"]["participant_ref"]
                .as_str()
                .unwrap()
                .to_owned(),
            scopes: vec![
                PaperAccessScope::PositionRead,
                PaperAccessScope::RedemptionRequest,
            ],
            risk_revision: RISK_REVISION,
            issued_at_unix: fixture["expected"]["issued_at_unix"].as_i64().unwrap(),
            expires_at_unix: fixture["expected"]["expires_at_unix"].as_i64().unwrap(),
            simulated: true,
        };

        assert_eq!(
            signer.sign_token(TOKEN_PREFIX, &claims).unwrap(),
            fixture["main"]["grant_token"].as_str().unwrap()
        );
        assert_eq!(
            URL_SAFE_NO_PAD.encode(signer.public_key_bytes()),
            fixture["main"]["public_key_base64url"].as_str().unwrap()
        );
    }

    #[test]
    fn issues_a_verifiable_short_lived_grant_without_raw_account_identity() {
        let signer =
            PaperGrantSigner::from_material("paper-key-2026-09".to_owned(), &SEED, &SUBJECT_SECRET)
                .unwrap();
        let first = signer
            .issue(
                "private-user-id-123",
                vec![
                    PaperAccessScope::PositionRead,
                    PaperAccessScope::RedemptionRequest,
                ],
                1_788_192_000,
            )
            .unwrap();
        let second = signer
            .issue(
                "private-user-id-123",
                vec![PaperAccessScope::PositionRead],
                1_788_192_001,
            )
            .unwrap();

        assert_eq!(first.participant_ref, second.participant_ref);
        assert_ne!(
            first.participant_ref,
            signer.participant_ref("another-user-id").unwrap()
        );
        assert!(!first.access_token.contains("private-user-id-123"));
        let (payload, signature) = decode_token(&first.access_token);
        UnparsedPublicKey::new(&ED25519, signer.signing_key.public_key().as_ref())
            .verify(&payload, &signature)
            .unwrap();
        let claims: serde_json::Value = serde_json::from_slice(&payload).unwrap();
        assert_eq!(claims["participant_ref"], first.participant_ref);
        assert_eq!(claims["expires_at_unix"], 1_788_192_300_i64);
        assert!(claims.get("user_id").is_none());
        assert!(claims.get("account").is_none());
    }

    #[test]
    fn rejects_duplicate_or_empty_scopes_and_invalid_key_material() {
        assert!(validate_scopes(Vec::new()).is_err());
        assert!(validate_scopes(vec![
            PaperAccessScope::PositionRead,
            PaperAccessScope::PositionRead,
        ])
        .is_err());
        assert!(
            PaperGrantSigner::from_material("bad key".to_owned(), &SEED, &SUBJECT_SECRET,).is_err()
        );
    }

    #[test]
    fn schema_matches_the_signed_payload_version() {
        let schema: serde_json::Value = serde_json::from_str(include_str!(
            "../../contracts/quant/paper-access-grant-v1.schema.json"
        ))
        .unwrap();
        assert_eq!(schema["properties"]["schema"]["const"], SCHEMA);
        assert_eq!(schema["properties"]["audience"]["const"], AUDIENCE);
    }

    fn decode_token(token: &str) -> (Vec<u8>, Vec<u8>) {
        let segments = token.split('.').collect::<Vec<_>>();
        assert_eq!(segments[0], TOKEN_PREFIX);
        (
            URL_SAFE_NO_PAD.decode(segments[1]).unwrap(),
            URL_SAFE_NO_PAD.decode(segments[2]).unwrap(),
        )
    }
}
