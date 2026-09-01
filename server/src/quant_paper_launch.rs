use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use std::{net::IpAddr, sync::Arc};

use super::quant_paper_access::{
    PaperAccessGrantResponse, PaperAccessScope, PaperGrantSigner, SignerConfigError,
};
use crate::{project_auth::auth_from_headers, types::AppState};

const PROTOCOL: &str = "yilong.quant.paper_launch.v1";
const READINESS_SCHEMA: &str = "yilong.quant.paper_launch_readiness.v1";
const TICKET_SCHEMA: &str = "yilong.quant.paper_launch_ticket.v1";
const WEB_URL_ENV: &str = "YILONG_QUANT_PAPER_WEB_URL";

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
    expires_in: i64,
    simulated: bool,
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

pub(crate) async fn issue(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
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
    let grant = match signer.issue(
        &user_id,
        vec![
            PaperAccessScope::PositionRead,
            PaperAccessScope::RedemptionRequest,
        ],
        chrono::Utc::now().timestamp(),
    ) {
        Ok(grant) => grant,
        Err(()) => {
            return unavailable(
                "quant_paper_launch_unavailable",
                "暂时无法创建量化 Paper 一键进入票据",
            )
        }
    };
    Json(build_ticket(target, grant)).into_response()
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

fn build_ticket(target: PaperLaunchTarget, grant: PaperAccessGrantResponse) -> PaperLaunchTicket {
    PaperLaunchTicket {
        schema: TICKET_SCHEMA,
        protocol: PROTOCOL,
        launch_url: target.url,
        access_token: grant.access_token,
        expires_in: grant.expires_in,
        simulated: true,
    }
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
    }
}
