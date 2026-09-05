use serde::{Deserialize, Serialize};

pub(crate) const PLATFORM_REQUEST_SCHEMA: &str =
    "yilong.esk.sui.platform_address_binding_request.v2";
pub(crate) const CHALLENGE_SCHEMA: &str = "yilong.esk.sui.address_binding_challenge.v1";
pub(crate) const WALLET_RESPONSE_SCHEMA: &str = "yilong.esk.sui.address_binding_wallet_response.v1";
pub(crate) const NETWORK: &str = "testnet";
pub(crate) const PURPOSE: &str = "user_asset_migration";
pub(crate) const MIN_TTL_SECONDS: u32 = 120;
pub(crate) const MAX_TTL_SECONDS: u32 = 900;
pub(crate) const MAX_MESSAGE_BYTES: usize = 2_048;
pub(crate) const MAX_SIGNATURE_BYTES: usize = 2_048;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct PlatformAddressBindingRequest {
    pub schema: String,
    pub address: String,
    pub ttl_seconds: u32,
}

/// User-independent material prepared by the service after authentication.
///
/// The subject commitment is deliberately not part of this value. The store
/// chooses an existing commitment or atomically records a fresh candidate,
/// then passes that selected value separately to challenge assembly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ChallengeMaterial {
    pub address: String,
    pub ttl_seconds: u32,
    pub nonce_base64: String,
    pub issued_at: String,
    pub expires_at: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct AddressBindingChallenge {
    pub schema: String,
    pub challenge_id: String,
    pub network: String,
    pub purpose: String,
    pub subject_commitment: String,
    pub address: String,
    pub ttl_seconds: u32,
    pub nonce_base64: String,
    pub issued_at: String,
    pub expires_at: String,
    pub message_base64: String,
    pub message_sha256: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct WalletResponseBody {
    pub schema: String,
    pub challenge_id: String,
    pub message_base64: String,
    pub signature: String,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub(crate) enum SignatureScheme {
    Ed25519,
    Secp256k1,
    Secp256r1,
}

impl SignatureScheme {
    pub(crate) const fn flag(self) -> u8 {
        match self {
            Self::Ed25519 => 0,
            Self::Secp256k1 => 1,
            Self::Secp256r1 => 2,
        }
    }

    pub(crate) const fn as_str(&self) -> &'static str {
        match self {
            Self::Ed25519 => "ed25519",
            Self::Secp256k1 => "secp256k1",
            Self::Secp256r1 => "secp256r1",
        }
    }

    pub(crate) fn parse(value: &str) -> Result<Self, AddressBindingError> {
        match value {
            "ed25519" => Ok(Self::Ed25519),
            "secp256k1" => Ok(Self::Secp256k1),
            "secp256r1" => Ok(Self::Secp256r1),
            _ => Err(AddressBindingError::CorruptLedger),
        }
    }
}

/// Private handoff from local signature verification into the append-only
/// store. It contains no signing key or wallet configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct VerifiedWalletResponse {
    pub challenge_id: String,
    pub address: String,
    pub subject_commitment: String,
    pub message_sha256: String,
    pub signature_scheme: SignatureScheme,
    pub signature_sha256: String,
    pub response_digest: String,
    pub verified_at: String,
    pub wallet_response_json: String,
}

/// Complete private ledger projection. This type is intentionally not
/// serializable so an HTTP handler cannot accidentally expose its private
/// user, subject, response, or signature material.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AddressBindingRecord {
    pub binding_id: String,
    pub user_id: String,
    pub challenge_id: String,
    pub address: String,
    pub network: String,
    pub subject_commitment: String,
    pub message_sha256: String,
    pub signature_scheme: SignatureScheme,
    pub signature_sha256: String,
    pub response_digest: String,
    pub binding_receipt_sha256: String,
    pub wallet_response_json: String,
    pub issued_at: String,
    pub expires_at: String,
    pub verified_at: String,
    pub bound_at: String,
    pub replayed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AddressBindingError {
    InvalidInput,
    InvalidChallenge,
    InvalidResponse,
    ChallengeIdMismatch,
    MessageMismatch,
    NotYetValid,
    Expired,
    UnsupportedSignatureScheme,
    SignatureInvalid,
    Unauthorized,
    NotFound,
    RateLimited,
    Conflict,
    CorruptLedger,
    Storage,
    RandomUnavailable,
}

impl AddressBindingError {
    pub(crate) const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "INVALID_INPUT",
            Self::InvalidChallenge => "INVALID_CHALLENGE",
            Self::InvalidResponse => "INVALID_RESPONSE",
            Self::ChallengeIdMismatch => "CHALLENGE_ID_MISMATCH",
            Self::MessageMismatch => "MESSAGE_MISMATCH",
            Self::NotYetValid => "CHALLENGE_NOT_YET_VALID",
            Self::Expired => "CHALLENGE_EXPIRED",
            Self::UnsupportedSignatureScheme => "UNSUPPORTED_SIGNATURE_SCHEME",
            Self::SignatureInvalid => "SIGNATURE_INVALID",
            Self::Unauthorized => "ESK_PLATFORM_SUI_BINDING_NOT_AUTHORIZED",
            Self::NotFound => "ESK_PLATFORM_SUI_BINDING_NOT_FOUND",
            Self::RateLimited => "ESK_PLATFORM_SUI_BINDING_RATE_LIMITED",
            Self::Conflict => "ESK_PLATFORM_SUI_BINDING_CONFLICT",
            Self::CorruptLedger => "ESK_PLATFORM_SUI_BINDING_LEDGER_INCONSISTENT",
            Self::Storage => "ESK_PLATFORM_SUI_BINDING_STORAGE_ERROR",
            Self::RandomUnavailable => "ESK_PLATFORM_SUI_BINDING_RANDOM_UNAVAILABLE",
        }
    }
}

impl std::fmt::Display for AddressBindingError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.code())
    }
}

impl std::error::Error for AddressBindingError {}
