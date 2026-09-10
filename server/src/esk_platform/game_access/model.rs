use serde::{Deserialize, Serialize};

pub const CLIENT: &str = "esk-game.web";
pub const CONSENT: &str = "授权此游戏使用我的主账号及所选权限";

// Credentials deliberately have no Debug implementation.
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AuthorizeRequest {
    pub schema: String,
    pub client_id: String,
    pub redirect_uri: String,
    pub state: String,
    pub code_challenge: String,
    pub code_challenge_method: String,
    pub scopes: Vec<String>,
    pub expires_in_seconds: u16,
    pub explicit_consent: bool,
    pub confirmation: String,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExchangeRequest {
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
pub struct RevokeRequest {
    pub schema: String,
    pub expected_revision: String,
}

#[derive(Serialize)]
pub struct AuthorizationCode {
    pub schema: &'static str,
    pub code: String,
    pub state: String,
    pub redirect_uri: String,
    pub grant_id: String,
    pub code_expires_at_ms: String,
    pub expires_at_ms: String,
    pub scopes: Vec<String>,
}

#[derive(Serialize)]
pub struct GameToken {
    pub schema: &'static str,
    pub token_type: &'static str,
    pub audience: &'static str,
    pub access_token: String,
    pub grant_id: String,
    pub expires_at_ms: String,
    pub scopes: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    Disabled,
    InvalidInput,
    Unauthorized,
    InvalidGrant,
    ScopeMissing,
    RevisionConflict,
    Replay,
    Capacity,
    Unavailable,
    Corrupt,
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "game_access_{:?}", self)
    }
}
impl std::error::Error for Error {}
pub type Result<T, E = anyhow::Error> = std::result::Result<T, E>;
