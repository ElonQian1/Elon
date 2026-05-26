//! 统一 HTTP 错误类型
//!
//! 现有 handler 仍可使用 `project_auth::json_error`（保持向后兼容）；
//! 新 handler 推荐直接返回 `AppError`，编译器自动将其转为带状态码的 JSON 响应。
//!
//! # 示例
//! ```rust
//! async fn my_handler(...) -> Result<Json<MyResp>, AppError> {
//!     let user = state.store.get_user(&id)
//!         .map_err(|_| AppError::not_found("用户不存在"))?;
//!     Ok(Json(user))
//! }
//! ```

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use thiserror::Error;

/// 应用级 HTTP 错误枚举，每个变体映射到标准 HTTP 状态码。
#[derive(Debug, Error)]
pub enum AppError {
    #[error("未找到：{0}")]
    NotFound(String),

    #[error("未授权：{0}")]
    Unauthorized(String),

    #[error("请求无效：{0}")]
    BadRequest(String),

    #[error("权限不足：{0}")]
    Forbidden(String),

    #[error("服务器内部错误：{0}")]
    Internal(String),

    #[error("资源已存在：{0}")]
    Conflict(String),

    #[error("需要升级客户端：{0}")]
    UpgradeRequired(String),
}

impl AppError {
    pub fn not_found(msg: impl Into<String>) -> Self {
        Self::NotFound(msg.into())
    }
    pub fn unauthorized(msg: impl Into<String>) -> Self {
        Self::Unauthorized(msg.into())
    }
    pub fn bad_request(msg: impl Into<String>) -> Self {
        Self::BadRequest(msg.into())
    }
    pub fn forbidden(msg: impl Into<String>) -> Self {
        Self::Forbidden(msg.into())
    }
    pub fn internal(msg: impl Into<String>) -> Self {
        Self::Internal(msg.into())
    }

    fn status(&self) -> StatusCode {
        match self {
            Self::NotFound(_) => StatusCode::NOT_FOUND,
            Self::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            Self::BadRequest(_) => StatusCode::BAD_REQUEST,
            Self::Forbidden(_) => StatusCode::FORBIDDEN,
            Self::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::Conflict(_) => StatusCode::CONFLICT,
            Self::UpgradeRequired(_) => StatusCode::UPGRADE_REQUIRED,
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = self.status();
        let body = Json(serde_json::json!({
            "error": self.to_string(),
            "code": status.as_u16(),
        }));
        (status, body).into_response()
    }
}

/// `anyhow::Error` → `AppError::Internal`（无需细分错误类型时使用）
impl From<anyhow::Error> for AppError {
    fn from(err: anyhow::Error) -> Self {
        Self::Internal(err.to_string())
    }
}
