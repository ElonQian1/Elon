use std::sync::Arc;

use axum::{extract::State, http::StatusCode, Json};
use serde::Deserialize;

use crate::node_agent_registration::{provision_node, ProvisionNodeOutcome};
use crate::NodeRuntime;

#[derive(Deserialize)]
pub(crate) struct AdminLoginRequest {
    account: Option<String>,
    password: Option<String>,
    token: Option<String>,
}

pub(crate) async fn login(
    State(runtime): State<Arc<NodeRuntime>>,
    Json(request): Json<AdminLoginRequest>,
) -> (StatusCode, Json<serde_json::Value>) {
    let persisted_endpoint_required = runtime.endpoint_credentials.endpoint_required().await;
    let configured_secure_origin = runtime.cfg.endpoint_https_origin.as_deref();
    if persisted_endpoint_required && configured_secure_origin.is_none() {
        return error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "NODE_ENDPOINT_HTTPS_ORIGIN_REQUIRED".to_string(),
            true,
        );
    }
    let secure_mode = configured_secure_origin.is_some();
    let account = request
        .account
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let password = request
        .password
        .as_deref()
        .filter(|value| !value.is_empty());

    let token = if secure_mode {
        let (Some(account), Some(password)) = (account, password) else {
            return endpoint_account_password_required(persisted_endpoint_required);
        };
        match super::cloud_login(&runtime.cfg, account, password).await {
            Ok(token) => token,
            Err(error) => {
                return error_response(
                    StatusCode::UNAUTHORIZED,
                    format!("endpoint step-up 登录失败: {error}"),
                    persisted_endpoint_required,
                )
            }
        }
    } else if let Some(token) = request
        .token
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        token.to_string()
    } else {
        let (Some(account), Some(password)) = (account, password) else {
            return error_response(
                StatusCode::BAD_REQUEST,
                "请填写账号和密码，或直接粘贴 token".to_string(),
                false,
            );
        };
        match super::cloud_login(&runtime.cfg, account, password).await {
            Ok(token) => token,
            Err(error) => {
                return error_response(
                    StatusCode::UNAUTHORIZED,
                    format!("登录失败: {error}"),
                    false,
                )
            }
        }
    };

    let _bootstrap_transition = if secure_mode {
        Some(
            runtime
                .endpoint_credentials
                .lock_bootstrap_transition()
                .await,
        )
    } else {
        None
    };
    if secure_mode {
        let Some(origin) = configured_secure_origin else {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "NODE_ENDPOINT_HTTPS_ORIGIN_REQUIRED".to_string(),
                persisted_endpoint_required,
            );
        };
        if let Err(error) = runtime
            .endpoint_credentials
            .arm_endpoint_required(origin)
            .await
        {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("固定 endpoint no-downgrade 门禁失败: {error:#}"),
                persisted_endpoint_required,
            );
        }
        if let Err(error) = runtime.clear_legacy_creds_after_endpoint_arm().await {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("清除 legacy 节点凭据失败，已拒绝 secure register: {error:#}"),
                true,
            );
        }
    }

    let existing = if secure_mode {
        None
    } else {
        runtime.creds.read().await.clone()
    };
    let outcome =
        match provision_node(&runtime.cfg, &token, existing.as_ref(), &runtime.install_id).await {
            Ok(outcome) => outcome,
            Err(error) => {
                return error_response(
                    StatusCode::BAD_GATEWAY,
                    format!("注册节点失败: {error}"),
                    secure_mode,
                )
            }
        };

    if secure_mode {
        secure_bootstrap(runtime.clone(), token, password, outcome).await
    } else {
        match outcome {
            ProvisionNodeOutcome::Legacy(credentials) => {
                let agent_id = credentials.agent_id.clone();
                if let Err(error) = runtime.set_creds(Some(credentials)).await {
                    return error_response(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("节点凭证已签发，但本机持久化失败: {error:#}"),
                        false,
                    );
                }
                (
                    StatusCode::OK,
                    Json(serde_json::json!({"ok": true, "agent_id": agent_id})),
                )
            }
            ProvisionNodeOutcome::SecureBootstrapAnchor(_)
            | ProvisionNodeOutcome::EndpointAuthorityRequired(_) => error_response(
                StatusCode::BAD_GATEWAY,
                "legacy 注册收到不允许的 endpoint authority 响应".to_string(),
                false,
            ),
        }
    }
}

async fn secure_bootstrap(
    runtime: Arc<NodeRuntime>,
    token: String,
    password: Option<&str>,
    outcome: ProvisionNodeOutcome,
) -> (StatusCode, Json<serde_json::Value>) {
    let (Some(origin), Some(password)) = (runtime.cfg.endpoint_https_origin.as_deref(), password)
    else {
        return endpoint_account_password_required(true);
    };
    let result = match outcome {
        ProvisionNodeOutcome::SecureBootstrapAnchor(anchor) => {
            runtime
                .endpoint_credentials
                .bootstrap_after_legacy_registration(
                    origin,
                    &token,
                    password,
                    &anchor.agent_id,
                    &anchor.owner_user_id,
                    &runtime.install_id,
                )
                .await
        }
        ProvisionNodeOutcome::EndpointAuthorityRequired(current) => {
            runtime
                .endpoint_credentials
                .recover_existing_authority(origin, &token, password, current)
                .await
        }
        ProvisionNodeOutcome::Legacy(_) => {
            return error_response(
                StatusCode::BAD_GATEWAY,
                "secure register 响应被错误解析为 legacy credential".to_string(),
                true,
            )
        }
    };
    runtime.wake.notify_waiters();
    match result {
        Ok(binding) => {
            let agent_id = binding.agent_id.clone();
            (
                StatusCode::OK,
                Json(serde_json::json!({
                    "ok": true,
                    "agent_id": agent_id,
                    "endpoint_required": true,
                    "endpoint_credential": binding,
                })),
            )
        }
        Err(error) => error_response(
            StatusCode::BAD_GATEWAY,
            format!("endpoint credential bootstrap 未完成，已禁止 legacy 回退: {error:#}"),
            true,
        ),
    }
}

pub(crate) async fn logout(
    State(runtime): State<Arc<NodeRuntime>>,
) -> (StatusCode, Json<serde_json::Value>) {
    let _bootstrap_transition = runtime
        .endpoint_credentials
        .lock_bootstrap_transition()
        .await;
    let endpoint_required = runtime.endpoint_credentials.endpoint_required().await;
    if let Err(error) = runtime.endpoint_credentials.clear_secret_for_logout().await {
        return error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("清除本机 endpoint secret 失败: {error:#}"),
            endpoint_required,
        );
    }
    let legacy_clear_result = if endpoint_required {
        runtime.clear_legacy_creds_after_endpoint_arm().await
    } else {
        runtime.set_creds(None).await
    };
    if let Err(error) = legacy_clear_result {
        return error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("清除本机节点凭证失败: {error:#}"),
            endpoint_required,
        );
    }
    (StatusCode::OK, Json(serde_json::json!({"ok": true})))
}

fn endpoint_account_password_required(
    endpoint_required: bool,
) -> (StatusCode, Json<serde_json::Value>) {
    error_response(
        StatusCode::BAD_REQUEST,
        "NODE_ENDPOINT_ACCOUNT_PASSWORD_REQUIRED: secure endpoint 模式禁止 token-only 与无账号 token+password bootstrap".to_string(),
        endpoint_required,
    )
}

fn error_response(
    status: StatusCode,
    error: String,
    endpoint_required: bool,
) -> (StatusCode, Json<serde_json::Value>) {
    (
        status,
        Json(serde_json::json!({
            "ok": false,
            "endpoint_required": endpoint_required,
            "error": error,
        })),
    )
}
