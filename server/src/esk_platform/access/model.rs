use serde::{Deserialize, Serialize};
use std::fmt;

pub(crate) const AUDIENCE: &str = "yilong-quant";
pub(crate) const AUTHORIZE_SCHEMA: &str = "yilong.asset_access.authorize.v1";
pub(crate) const TOKEN_SCHEMA: &str = "yilong.asset_access.token_request.v1";
pub(crate) const REVOKE_SCHEMA: &str = "yilong.asset_access.revoke.v1";
pub(crate) const AUTHORIZE_CONFIRMATION: &str = "授权量化只读我的资产";
pub(crate) const REVOKE_CONFIRMATION: &str = "撤销只读资产授权";
pub(crate) const CLIENT_HEADER: &str = "x-elon-asset-client";
pub(crate) const CODE_LIFETIME_SECONDS: i64 = 120;
pub(crate) const MAX_GRANT_SECONDS: i64 = 3600;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub(crate) enum AccessScope {
    #[serde(rename = "profile.read")]
    ProfileRead,
    #[serde(rename = "esk.summary.read")]
    EskSummaryRead,
    #[serde(rename = "esk.progress.read")]
    EskProgressRead,
}

impl AccessScope {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::ProfileRead => "profile.read",
            Self::EskSummaryRead => "esk.summary.read",
            Self::EskProgressRead => "esk.progress.read",
        }
    }
}

// These types intentionally omit Debug: requests and responses contain credentials.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct AuthorizeBody {
    pub schema: String,
    pub client_id: String,
    pub redirect_uri: String,
    pub state: String,
    pub code_challenge: String,
    pub code_challenge_method: String,
    pub scopes: Vec<AccessScope>,
    pub expires_in: i64,
    pub explicit_consent: bool,
    pub confirmation: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct TokenBody {
    pub schema: String,
    pub grant_type: String,
    pub client_id: String,
    pub redirect_uri: String,
    pub state: String,
    pub code: String,
    pub code_verifier: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RevokeBody {
    pub schema: String,
    pub confirmation: String,
}

#[derive(Serialize)]
pub(crate) struct AuthorizationCode {
    pub schema: &'static str,
    pub code: String,
    pub state: String,
    pub client_id: String,
    pub redirect_uri: String,
    pub code_expires_at: String,
    pub grant_id: String,
    pub expires_at: String,
    pub scopes: Vec<AccessScope>,
}

#[derive(Serialize)]
pub(crate) struct AccessToken {
    pub schema: &'static str,
    pub token_type: &'static str,
    pub access_token: String,
    pub audience: &'static str,
    pub subject: String,
    pub client_id: String,
    pub grant_id: String,
    pub expires_in: i64,
    pub expires_at: String,
    pub scopes: Vec<AccessScope>,
}

#[derive(Serialize)]
pub(crate) struct GrantOverview {
    pub grant_id: String,
    pub client_id: String,
    pub subject: String,
    pub scopes: Vec<AccessScope>,
    pub created_at: String,
    pub expires_at: String,
    pub status: &'static str,
}

#[derive(Serialize)]
pub(crate) struct AccessIdentity {
    pub schema: &'static str,
    pub audience: &'static str,
    pub subject: String,
    pub client_id: String,
    pub grant_id: String,
    pub expires_at: String,
    pub scopes: Vec<AccessScope>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nickname: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum AccessError {
    InvalidInput,
    InvalidGrant,
    Unauthorized,
    InsufficientScope,
    NotFound,
    Capacity,
    Unavailable,
    Corrupt,
}

impl AccessError {
    pub(crate) fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "asset_access_invalid_input",
            Self::InvalidGrant => "asset_access_invalid_grant",
            Self::Unauthorized => "asset_access_unauthorized",
            Self::InsufficientScope => "asset_access_insufficient_scope",
            Self::NotFound => "asset_access_not_found",
            Self::Capacity => "asset_access_capacity",
            Self::Unavailable => "asset_access_unavailable",
            Self::Corrupt => "asset_access_corrupt",
        }
    }
}

impl fmt::Display for AccessError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}
impl std::error::Error for AccessError {}
