use serde::{Deserialize, Serialize};

pub(crate) const MERCHANT_IDENTITY_KEY_SCHEMA: &str = "open_commerce.merchant_identity_key.v1";
pub(crate) const MERCHANT_IDENTITY_KEY_ALGORITHM: &str = "rsa-pkcs1v15-sha256";
pub(crate) const MERCHANT_IDENTITY_PROOF_PROTOCOL: &str =
    "open_commerce.merchant_identity_proof.v1";

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct CreateMerchantIdentityKeyRequest {
    pub public_key_pem: String,
    pub proof_signature_base64: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct OpenCommerceMerchantIdentityKey {
    pub schema: String,
    pub id: String,
    pub project_id: String,
    pub merchant_id: String,
    pub key_id: String,
    pub algorithm: String,
    pub public_key_pem: String,
    #[serde(skip)]
    pub(crate) proof_signature_base64: String,
    pub status: String,
    pub proof_verified_at: String,
    pub created_at: String,
    pub revoked_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct OpenCommercePublicMerchantIdentityKey {
    pub key_id: String,
    pub algorithm: String,
    pub proof_verified_at: String,
    pub created_at: String,
}

impl From<OpenCommerceMerchantIdentityKey> for OpenCommercePublicMerchantIdentityKey {
    fn from(value: OpenCommerceMerchantIdentityKey) -> Self {
        Self {
            key_id: value.key_id,
            algorithm: value.algorithm,
            proof_verified_at: value.proof_verified_at,
            created_at: value.created_at,
        }
    }
}
