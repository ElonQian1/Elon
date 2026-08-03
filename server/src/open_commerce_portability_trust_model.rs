use serde::{Deserialize, Serialize};

pub(crate) const CONSUMER_PORTABILITY_TRUST_KEY_SCHEMA: &str =
    "open_commerce.consumer_portability_trust_key.v1";
pub(crate) const CONSUMER_PORTABILITY_TRUST_KEY_ALGORITHM: &str = "rsa-pkcs1v15-sha256";

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct CreateConsumerPortabilityTrustKeyRequest {
    pub source_operator: String,
    pub public_key_pem: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ConsumerPortabilityTrustKey {
    pub schema: String,
    pub id: String,
    pub source_operator: String,
    pub key_id: String,
    pub algorithm: String,
    pub public_key_pem: String,
    pub status: String,
    pub created_at: String,
    pub revoked_at: Option<String>,
}
