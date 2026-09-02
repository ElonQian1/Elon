use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::HashSet, net::IpAddr, sync::Arc};

use super::quant_esk_asset_projection::{
    issue_esk_projection, issue_esk_projection_v2, ESK_PROJECTION_SCHEMA_V1,
    ESK_PROJECTION_SCHEMA_V2,
};
use super::quant_paper_access::{
    PaperAccessGrantResponse, PaperAccessScope, PaperGrantSigner, SignerConfigError,
};
use crate::{esk_asset::EskAssetMode, project_auth::auth_from_headers, types::AppState};

const PROTOCOL: &str = "yilong.quant.paper_launch.v1";
const READINESS_SCHEMA: &str = "yilong.quant.paper_launch_readiness.v1";
const TICKET_SCHEMA: &str = "yilong.quant.paper_launch_ticket.v1";
const WEB_URL_ENV: &str = "YILONG_QUANT_PAPER_WEB_URL";
const ESK_ALLOCATION_AUTHORIZATION_SCHEMA: &str = "yilong.esk.quant_allocation_authorization.v1";

#[derive(Debug, Serialize)]
struct PaperLaunchReadiness {
    schema: &'static str,
    protocol: &'static str,
    enabled: bool,
    simulated: bool,
    reason: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    launch_origin: Option<String>,
}

#[derive(Debug, Serialize)]
struct PaperLaunchTicket {
    schema: &'static str,
    protocol: &'static str,
    launch_url: String,
    access_token: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    esk_asset_projection: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    esk_quant_allocation_authorization: Option<String>,
    expires_in: i64,
    simulated: bool,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct IssuePaperLaunchRequest {
    #[serde(default)]
    capabilities: Vec<String>,
    #[serde(default)]
    esk_quant_allocation_request_id: Option<String>,
}

#[derive(Serialize)]
struct EskAllocationAuthorizationClaims<'a> {
    schema: &'static str,
    authorization_id: String,
    issuer: &'static str,
    audience: &'static str,
    project_id: &'static str,
    key_id: &'a str,
    grant_id: &'a str,
    participant_ref: &'a str,
    request_id: &'a str,
    amount: String,
    amount_base_units: String,
    request_revision: i64,
    risk_revision: &'a str,
    issued_at_unix: i64,
    expires_at_unix: i64,
    simulated: bool,
    funds_moved: bool,
    quant_units_issued: bool,
}

#[derive(Serialize)]
struct ErrorResponse {
    code: &'static str,
    message: &'static str,
}

#[derive(Debug)]
struct PaperLaunchTarget {
    url: String,
    origin: String,
}

#[derive(Debug)]
enum TargetConfigError {
    Disabled,
    Invalid,
}

pub(crate) async fn readiness(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    if let Err(response) = authorize_active_user(&state, &headers) {
        return response;
    }
    let signer = PaperGrantSigner::from_env();
    let target = PaperLaunchTarget::from_env();
    let (enabled, reason, launch_origin) = match (signer, target) {
        (Ok(_), Ok(target)) => (true, "ready", Some(target.origin)),
        (Err(SignerConfigError::Invalid), _) | (_, Err(TargetConfigError::Invalid)) => {
            (false, "configuration_invalid", None)
        }
        _ => (false, "configuration_required", None),
    };
    Json(PaperLaunchReadiness {
        schema: READINESS_SCHEMA,
        protocol: PROTOCOL,
        enabled,
        simulated: true,
        reason,
        launch_origin,
    })
    .into_response()
}

pub(crate) async fn issue(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(request): Json<IssuePaperLaunchRequest>,
) -> Response {
    let user_id = match authorize_active_user(&state, &headers) {
        Ok(user_id) => user_id,
        Err(response) => return response,
    };
    let signer = match PaperGrantSigner::from_env() {
        Ok(signer) => signer,
        Err(SignerConfigError::Disabled) => {
            return unavailable("quant_paper_launch_disabled", "量化 Paper 一键进入尚未配置")
        }
        Err(SignerConfigError::Invalid) => {
            return unavailable(
                "quant_paper_launch_misconfigured",
                "量化 Paper 一键进入配置无效",
            )
        }
    };
    let target = match PaperLaunchTarget::from_env() {
        Ok(target) => target,
        Err(TargetConfigError::Disabled) => {
            return unavailable("quant_paper_launch_disabled", "量化 Paper Web 地址尚未配置")
        }
        Err(TargetConfigError::Invalid) => {
            return unavailable(
                "quant_paper_launch_misconfigured",
                "量化 Paper Web 地址配置无效",
            )
        }
    };
    let capabilities = match validate_capabilities(&request.capabilities) {
        Ok(value) => value,
        Err(()) => {
            return error_response(
                StatusCode::BAD_REQUEST,
                "invalid_quant_paper_capabilities",
                "量化 Paper 启动 capability 无效",
            )
        }
    };
    let selected_request = if let Some(request_id) = request
        .esk_quant_allocation_request_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        if !capabilities.esk_allocation_authorization {
            return error_response(
                StatusCode::BAD_REQUEST,
                "quant_allocation_capability_required",
                "量化页面未声明 ESK 分配授权能力",
            );
        }
        match state
            .store
            .esk_quant_allocation_request(&user_id, request_id)
        {
            Ok(Some(record)) if matches!(record.status.as_str(), "submitted" | "accepted") => {
                Some(record)
            }
            Ok(Some(_)) => {
                return error_response(
                    StatusCode::CONFLICT,
                    "quant_allocation_request_not_launchable",
                    "当前 ESK 量化申请状态不能进入绑定流程",
                )
            }
            Ok(None) => {
                return error_response(
                    StatusCode::NOT_FOUND,
                    "quant_allocation_request_not_found",
                    "ESK 量化申请不存在",
                )
            }
            Err(error) => {
                tracing::warn!(error = %error, "failed to read selected ESK quant allocation request");
                return unavailable(
                    "quant_allocation_request_unavailable",
                    "暂时无法读取 ESK 量化申请",
                );
            }
        }
    } else {
        None
    };
    let now_unix = chrono::Utc::now().timestamp();
    let grant = match signer.issue(
        &user_id,
        vec![
            PaperAccessScope::PositionRead,
            PaperAccessScope::RedemptionRequest,
        ],
        now_unix,
    ) {
        Ok(grant) => grant,
        Err(()) => {
            return unavailable(
                "quant_paper_launch_unavailable",
                "暂时无法创建量化 Paper 一键进入票据",
            )
        }
    };
    let esk_asset_projection = if let Some(version) = capabilities.esk_projection_version {
        let mode = EskAssetMode::from_env();
        if matches!(mode, EskAssetMode::Invalid) {
            return unavailable("quant_esk_projection_misconfigured", "ESK 资产投影配置无效");
        }
        let ledger = match state.store.esk_account_ledger(&user_id) {
            Ok(ledger) => ledger,
            Err(error) => {
                tracing::warn!(error = %error, "failed to read ESK account for quant projection");
                return unavailable(
                    "quant_esk_projection_unavailable",
                    "暂时无法读取 ESK 资产投影",
                );
            }
        };
        let projection = match version {
            EskProjectionVersion::V1 => issue_esk_projection(
                &signer,
                &grant.grant_id,
                &grant.participant_ref,
                mode,
                ledger,
                now_unix,
                now_unix + grant.expires_in,
            ),
            EskProjectionVersion::V2 => issue_esk_projection_v2(
                &signer,
                &grant.grant_id,
                &grant.participant_ref,
                mode,
                ledger,
                now_unix,
                now_unix + grant.expires_in,
            ),
        };
        match projection {
            Ok(token) => Some(token),
            Err(()) => {
                return unavailable(
                    "quant_esk_projection_unavailable",
                    "暂时无法创建 ESK 资产投影",
                )
            }
        }
    } else {
        None
    };
    let esk_quant_allocation_authorization = match selected_request
        .as_ref()
        .filter(|record| record.status == "submitted")
    {
        Some(record) => match issue_esk_allocation_authorization(&signer, &grant, record, now_unix)
        {
            Ok(token) => Some(token),
            Err(()) => {
                return unavailable(
                    "quant_allocation_authorization_unavailable",
                    "暂时无法创建 ESK 量化申请授权",
                )
            }
        },
        None => None,
    };
    Json(build_ticket(
        target,
        grant,
        esk_asset_projection,
        esk_quant_allocation_authorization,
    ))
    .into_response()
}

fn authorize_active_user(state: &Arc<AppState>, headers: &HeaderMap) -> Result<String, Response> {
    let user = auth_from_headers(state, headers).map_err(|_| {
        error_response(
            StatusCode::UNAUTHORIZED,
            "authentication_required",
            "登录后才能进入量化 Paper",
        )
    })?;
    if user.status != "active" {
        return Err(error_response(
            StatusCode::FORBIDDEN,
            "account_not_active",
            "当前账号不能进入量化 Paper",
        ));
    }
    Ok(user.id)
}

impl PaperLaunchTarget {
    fn from_env() -> Result<Self, TargetConfigError> {
        match std::env::var(WEB_URL_ENV) {
            Ok(value) => Self::from_value(&value),
            Err(std::env::VarError::NotPresent) => Err(TargetConfigError::Disabled),
            Err(std::env::VarError::NotUnicode(_)) => Err(TargetConfigError::Invalid),
        }
    }

    fn from_value(value: &str) -> Result<Self, TargetConfigError> {
        let parsed = reqwest::Url::parse(value.trim()).map_err(|_| TargetConfigError::Invalid)?;
        if parsed.username() != ""
            || parsed.password().is_some()
            || parsed.query().is_some()
            || parsed.fragment().is_some()
        {
            return Err(TargetConfigError::Invalid);
        }
        let host = parsed.host_str().ok_or(TargetConfigError::Invalid)?;
        let secure = parsed.scheme() == "https";
        let loopback_http = parsed.scheme() == "http" && is_loopback(host);
        if !secure && !loopback_http {
            return Err(TargetConfigError::Invalid);
        }
        Ok(Self {
            origin: parsed.origin().ascii_serialization(),
            url: parsed.to_string(),
        })
    }
}

fn is_loopback(host: &str) -> bool {
    host.eq_ignore_ascii_case("localhost")
        || host
            .trim_matches(['[', ']'])
            .parse::<IpAddr>()
            .is_ok_and(|address| address.is_loopback())
}

fn build_ticket(
    target: PaperLaunchTarget,
    grant: PaperAccessGrantResponse,
    esk_asset_projection: Option<String>,
    esk_quant_allocation_authorization: Option<String>,
) -> PaperLaunchTicket {
    PaperLaunchTicket {
        schema: TICKET_SCHEMA,
        protocol: PROTOCOL,
        launch_url: target.url,
        access_token: grant.access_token,
        esk_asset_projection,
        esk_quant_allocation_authorization,
        expires_in: grant.expires_in,
        simulated: true,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EskProjectionVersion {
    V1,
    V2,
}

#[derive(Debug, PartialEq, Eq)]
struct LaunchCapabilities {
    esk_projection_version: Option<EskProjectionVersion>,
    esk_allocation_authorization: bool,
}

fn validate_capabilities(capabilities: &[String]) -> Result<LaunchCapabilities, ()> {
    if capabilities.len() > 8 {
        return Err(());
    }
    let mut seen = HashSet::with_capacity(capabilities.len());
    for capability in capabilities {
        if capability.is_empty()
            || capability.len() > 96
            || capability.chars().any(char::is_control)
            || !seen.insert(capability.as_str())
        {
            return Err(());
        }
    }
    let esk_projection_version = if seen.contains(ESK_PROJECTION_SCHEMA_V2) {
        Some(EskProjectionVersion::V2)
    } else if seen.contains(ESK_PROJECTION_SCHEMA_V1) {
        Some(EskProjectionVersion::V1)
    } else {
        None
    };
    Ok(LaunchCapabilities {
        esk_projection_version,
        esk_allocation_authorization: seen.contains(ESK_ALLOCATION_AUTHORIZATION_SCHEMA),
    })
}

fn issue_esk_allocation_authorization(
    signer: &PaperGrantSigner,
    grant: &PaperAccessGrantResponse,
    request: &crate::esk_asset::EskQuantAllocationRecord,
    issued_at_unix: i64,
) -> Result<String, ()> {
    let claims = EskAllocationAuthorizationClaims {
        schema: ESK_ALLOCATION_AUTHORIZATION_SCHEMA,
        authorization_id: esk_allocation_authorization_id(&request.request_id),
        issuer: "yilong-main",
        audience: "yilong-quant",
        project_id: "esk",
        key_id: signer.key_id(),
        grant_id: &grant.grant_id,
        participant_ref: &grant.participant_ref,
        request_id: &request.request_id,
        amount: crate::esk_asset::format_esk_amount(request.amount_base_units),
        amount_base_units: request.amount_base_units.to_string(),
        request_revision: request.revision,
        risk_revision: &request.risk_disclosure_revision,
        issued_at_unix,
        expires_at_unix: issued_at_unix + grant.expires_in,
        simulated: true,
        funds_moved: false,
        quant_units_issued: false,
    };
    signer.sign_token("yeqa1", &claims)
}

pub(crate) fn esk_allocation_authorization_id(request_id: &str) -> String {
    let mut digest = Sha256::new();
    digest.update(b"yilong-main-esk-quant-authorization-v1\0");
    digest.update(request_id.as_bytes());
    let encoded = format!("{:x}", digest.finalize());
    format!("eskauth_{}", &encoded[..32])
}

fn unavailable(code: &'static str, message: &'static str) -> Response {
    error_response(StatusCode::SERVICE_UNAVAILABLE, code, message)
}

fn error_response(status: StatusCode, code: &'static str, message: &'static str) -> Response {
    (status, Json(ErrorResponse { code, message })).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine as _;

    #[test]
    fn accepts_https_and_loopback_targets_without_query_or_fragment() {
        let target = PaperLaunchTarget::from_value("https://quant.example/paper").unwrap();
        assert_eq!(target.origin, "https://quant.example");
        assert_eq!(target.url, "https://quant.example/paper");
        assert!(PaperLaunchTarget::from_value("http://127.0.0.1:5173/").is_ok());
        for invalid in [
            "http://quant.example/paper",
            "https://user@quant.example/paper",
            "https://quant.example/paper?grant=secret",
            "https://quant.example/paper#secret",
            "javascript:alert(1)",
        ] {
            assert!(
                PaperLaunchTarget::from_value(invalid).is_err(),
                "accepted {invalid}"
            );
        }
    }

    #[test]
    fn schema_matches_the_cross_repository_contract() {
        let schema: serde_json::Value = serde_json::from_str(include_str!(
            "../../contracts/quant/paper-launch-v1.schema.json"
        ))
        .unwrap();
        assert_eq!(schema["$defs"]["protocol"]["const"], PROTOCOL);
        assert_eq!(
            schema["$defs"]["launchTicket"]["properties"]["schema"]["const"],
            TICKET_SCHEMA
        );
        assert_eq!(
            schema["$defs"]["launchTicket"]["properties"]["esk_asset_projection"]["pattern"],
            "^yep[12]\\.[A-Za-z0-9_-]+\\.[A-Za-z0-9_-]+$"
        );
    }

    #[test]
    fn capability_negotiation_is_explicit_and_bounded() {
        assert_eq!(
            validate_capabilities(&[]),
            Ok(LaunchCapabilities {
                esk_projection_version: None,
                esk_allocation_authorization: false,
            })
        );
        assert_eq!(
            validate_capabilities(&[ESK_PROJECTION_SCHEMA_V1.to_owned()]),
            Ok(LaunchCapabilities {
                esk_projection_version: Some(EskProjectionVersion::V1),
                esk_allocation_authorization: false,
            })
        );
        assert_eq!(
            validate_capabilities(&[
                ESK_PROJECTION_SCHEMA_V1.to_owned(),
                ESK_PROJECTION_SCHEMA_V2.to_owned(),
            ]),
            Ok(LaunchCapabilities {
                esk_projection_version: Some(EskProjectionVersion::V2),
                esk_allocation_authorization: false,
            })
        );
        assert!(validate_capabilities(&[
            ESK_PROJECTION_SCHEMA_V1.to_owned(),
            ESK_PROJECTION_SCHEMA_V1.to_owned(),
        ])
        .is_err());
    }

    #[test]
    fn allocation_authorization_is_deterministic_and_grant_bound() {
        let signer =
            PaperGrantSigner::from_material("main-paper-key-1".to_owned(), &[81; 32], &[82; 32])
                .unwrap();
        let grant = signer
            .issue(
                "private-user",
                vec![PaperAccessScope::PositionRead],
                1_788_192_000,
            )
            .unwrap();
        let request = crate::esk_asset::EskQuantAllocationRecord {
            request_id: "eskq_0123456789abcdef0123456789abcdef".to_owned(),
            user_id: "private-user".to_owned(),
            amount_base_units: 12_345_678,
            idempotency_key: "test".to_owned(),
            risk_disclosure_revision: crate::esk_asset::ESK_QUANT_RISK_DISCLOSURE_REVISION
                .to_owned(),
            status: "submitted".to_owned(),
            revision: 1,
            submitted_at: "2026-09-02T00:00:00Z".to_owned(),
            updated_at: "2026-09-02T00:00:00Z".to_owned(),
            replayed: false,
            binding_id: None,
            receipt_id: None,
            receipt_digest: None,
            receipt_key_id: None,
            quant_binding_revision: None,
            occurred_at_unix: None,
        };
        let token =
            issue_esk_allocation_authorization(&signer, &grant, &request, 1_788_192_000).unwrap();
        let segments = token.split('.').collect::<Vec<_>>();
        assert_eq!(segments[0], "yeqa1");
        let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(segments[1])
            .unwrap();
        let claims: serde_json::Value = serde_json::from_slice(&payload).unwrap();
        assert_eq!(claims["request_id"], request.request_id);
        assert_eq!(claims["amount"], "12.345678");
        assert_eq!(claims["grant_id"], grant.grant_id);
        assert_eq!(claims["participant_ref"], grant.participant_ref);
        assert_eq!(claims["expires_at_unix"], 1_788_192_300_i64);
        assert_eq!(
            claims["authorization_id"],
            esk_allocation_authorization_id(&request.request_id)
        );
        assert_eq!(claims["funds_moved"], false);
        assert_eq!(claims["quant_units_issued"], false);
    }
}
