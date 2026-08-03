use serde::{Deserialize, Serialize};

pub(crate) const CONSUMER_DATA_VAULT_ENVELOPE_SCHEMA: &str =
    "open_commerce.consumer_data_vault_envelope.v1";
pub(crate) const CONSUMER_DATA_VAULT_ITEM_SCHEMA: &str =
    "open_commerce.consumer_data_vault_item.v1";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ConsumerDataVaultKdf {
    pub name: String,
    pub hash: String,
    pub iterations: u32,
    pub salt_base64: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ConsumerDataVaultCipher {
    pub name: String,
    pub nonce_base64: String,
    pub auth_tag_length_bits: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ConsumerDataVaultEnvelope {
    pub schema: String,
    pub record_id: String,
    pub revision: i64,
    pub kdf: ConsumerDataVaultKdf,
    pub cipher: ConsumerDataVaultCipher,
    pub ciphertext_base64: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct CreateConsumerDataVaultItemRequest {
    pub id: String,
    pub label: String,
    pub item_kind: String,
    pub envelope: ConsumerDataVaultEnvelope,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct UpdateConsumerDataVaultItemRequest {
    pub expected_revision: i64,
    pub label: String,
    pub item_kind: String,
    pub envelope: ConsumerDataVaultEnvelope,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct DeleteConsumerDataVaultItemRequest {
    pub expected_revision: i64,
    pub confirmed_by_user: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ConsumerDataVaultItem {
    pub schema: String,
    pub id: String,
    pub label: String,
    pub item_kind: String,
    pub envelope: ConsumerDataVaultEnvelope,
    pub ciphertext_sha256: String,
    pub ciphertext_bytes: i64,
    pub revision: i64,
    pub server_can_decrypt: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ConsumerDataVaultItemSummary {
    pub schema: String,
    pub id: String,
    pub label: String,
    pub item_kind: String,
    pub ciphertext_sha256: String,
    pub ciphertext_bytes: i64,
    pub revision: i64,
    pub server_can_decrypt: bool,
    pub created_at: String,
    pub updated_at: String,
}

impl ConsumerDataVaultItem {
    pub(crate) fn summary(&self) -> ConsumerDataVaultItemSummary {
        ConsumerDataVaultItemSummary {
            schema: self.schema.clone(),
            id: self.id.clone(),
            label: self.label.clone(),
            item_kind: self.item_kind.clone(),
            ciphertext_sha256: self.ciphertext_sha256.clone(),
            ciphertext_bytes: self.ciphertext_bytes,
            revision: self.revision,
            server_can_decrypt: false,
            created_at: self.created_at.clone(),
            updated_at: self.updated_at.clone(),
        }
    }
}
