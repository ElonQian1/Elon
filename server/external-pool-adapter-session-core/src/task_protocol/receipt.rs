use zeroize::Zeroize;

use super::ExternalPoolAdapterTaskOperationKind;

/// Host proof of one exact child-accepted ELTP exchange.
///
/// It is deliberately non-Clone, non-Debug and non-serializable. Callers may persist only its
/// bounded digest/length projections after validating the semantic observation.
pub struct ExternalPoolAdapterTaskProtocolHostReceipt {
    pub(super) ordinal: u64,
    pub(super) operation: ExternalPoolAdapterTaskOperationKind,
    pub(super) exchange_nonce_digest: [u8; 32],
    pub(super) request_digest: [u8; 32],
    pub(super) command_digest: [u8; 32],
    pub(super) outbox_operation_digest: [u8; 32],
    pub(super) delivery_attempt_digest: [u8; 32],
    pub(super) route_authorization_digest: [u8; 32],
    pub(super) executor_binding_digest: [u8; 32],
    pub(super) fence_digest: [u8; 32],
    pub(super) upstream_request_bytes: u32,
    pub(super) upstream_request_sha256: [u8; 32],
    pub(super) upstream_response_bytes: u32,
    pub(super) upstream_response_sha256: [u8; 32],
    pub(super) semantic_observation_bytes: u32,
    pub(super) semantic_observation_sha256: [u8; 32],
    pub(super) session_transcript_digest: [u8; 32],
    pub(super) exchange_root: [u8; 32],
}

impl ExternalPoolAdapterTaskProtocolHostReceipt {
    pub fn ordinal(&self) -> u64 {
        self.ordinal
    }

    pub fn operation(&self) -> ExternalPoolAdapterTaskOperationKind {
        self.operation
    }

    pub fn exchange_nonce_digest_hex(&self) -> String {
        hex::encode(self.exchange_nonce_digest)
    }

    pub fn request_digest_hex(&self) -> String {
        hex::encode(self.request_digest)
    }

    pub fn command_digest_hex(&self) -> String {
        hex::encode(self.command_digest)
    }

    pub fn outbox_operation_digest_hex(&self) -> String {
        hex::encode(self.outbox_operation_digest)
    }

    pub fn delivery_attempt_digest_hex(&self) -> String {
        hex::encode(self.delivery_attempt_digest)
    }

    pub fn route_authorization_digest_hex(&self) -> String {
        hex::encode(self.route_authorization_digest)
    }

    pub fn executor_binding_digest_hex(&self) -> String {
        hex::encode(self.executor_binding_digest)
    }

    pub fn fence_digest_hex(&self) -> String {
        hex::encode(self.fence_digest)
    }

    pub fn upstream_request_bytes(&self) -> u32 {
        self.upstream_request_bytes
    }

    pub fn upstream_request_sha256_hex(&self) -> String {
        hex::encode(self.upstream_request_sha256)
    }

    pub fn upstream_response_bytes(&self) -> u32 {
        self.upstream_response_bytes
    }

    pub fn upstream_response_sha256_hex(&self) -> String {
        hex::encode(self.upstream_response_sha256)
    }

    pub fn semantic_observation_bytes(&self) -> u32 {
        self.semantic_observation_bytes
    }

    pub fn semantic_observation_sha256_hex(&self) -> String {
        hex::encode(self.semantic_observation_sha256)
    }

    pub fn session_transcript_digest_hex(&self) -> String {
        hex::encode(self.session_transcript_digest)
    }

    pub fn exchange_root_hex(&self) -> String {
        hex::encode(self.exchange_root)
    }
}

impl Drop for ExternalPoolAdapterTaskProtocolHostReceipt {
    fn drop(&mut self) {
        self.ordinal.zeroize();
        self.exchange_nonce_digest.zeroize();
        self.request_digest.zeroize();
        self.command_digest.zeroize();
        self.outbox_operation_digest.zeroize();
        self.delivery_attempt_digest.zeroize();
        self.route_authorization_digest.zeroize();
        self.executor_binding_digest.zeroize();
        self.fence_digest.zeroize();
        self.upstream_request_bytes.zeroize();
        self.upstream_request_sha256.zeroize();
        self.upstream_response_bytes.zeroize();
        self.upstream_response_sha256.zeroize();
        self.semantic_observation_bytes.zeroize();
        self.semantic_observation_sha256.zeroize();
        self.session_transcript_digest.zeroize();
        self.exchange_root.zeroize();
    }
}
