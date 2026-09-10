use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Action {
    Authenticate {},
    Inventory {},
    WalletBind {},
    Order {
        order_id: String,
    },
    Quote {
        asset_id: String,
        policy_id: String,
    },
    AcceptQuote {
        quote_id: String,
        idempotency_key: String,
    },
    PrincipalWithdraw {
        position_id: String,
        idempotency_key: String,
    },
}
impl Action {
    pub(super) fn values(&self) -> Vec<&str> {
        match self {
            Self::Authenticate {} => vec!["authenticate"],
            Self::Inventory {} => vec!["inventory"],
            Self::WalletBind {} => vec!["wallet_bind"],
            Self::Order { order_id } => vec!["order", order_id],
            Self::Quote {
                asset_id,
                policy_id,
            } => vec!["quote", asset_id, policy_id],
            Self::AcceptQuote {
                quote_id,
                idempotency_key,
            } => vec!["accept_quote", quote_id, idempotency_key],
            Self::PrincipalWithdraw {
                position_id,
                idempotency_key,
            } => vec!["principal_withdraw", position_id, idempotency_key],
        }
    }
    pub(super) fn scope(&self) -> &str {
        match self {
            Self::Authenticate {} => "play",
            Self::Inventory {} | Self::Order { .. } => "inventory_read",
            Self::WalletBind {} => "wallet_bind",
            Self::Quote { .. } | Self::AcceptQuote { .. } => "redeem",
            Self::PrincipalWithdraw { .. } => "principal_withdraw",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Challenge {
    pub domain: String,
    pub main_issuer: String,
    pub audience: String,
    pub stage: String,
    pub nonce: String,
    pub credential_digest: String,
    pub action: Action,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Grant {
    pub main_user_id: String,
    pub main_session_id: String,
    pub grant_id: String,
    pub revision: String,
    pub not_before_ms: String,
    pub expires_at_ms: String,
    pub scopes: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Observation {
    pub challenge: Challenge,
    pub grant: Grant,
    pub observed_at_ms: String,
    pub key_id: String,
    pub signature_hex: String,
}

/// Supplied by operator configuration, not by the untrusted response.
pub struct Authority {
    pub main_issuer: String,
    pub key_id: String,
    pub public_key: [u8; 32],
}

#[derive(Debug, Clone, Serialize)]
pub struct VerifiedObservation {
    pub main_user_id: String,
    pub main_session_id: String,
    pub grant_id: String,
    pub revision: String,
    pub valid_until_ms: String,
    pub authorization_digest: String,
    pub wallet_bound: bool,
    pub funds_moved: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    InvalidContract,
    ChallengeMismatch,
    AuthorityMismatch,
    ObservationExpired,
    ScopeMissing,
    SignatureInvalid,
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for Error {}
