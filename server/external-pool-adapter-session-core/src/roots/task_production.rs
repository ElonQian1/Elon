use anyhow::Result;
use sha2::{Digest, Sha256};

use super::decode_digest;

const ROOT_TRANSCRIPT_DOMAIN: &[u8] =
    b"elon.external_pool_adapter.task_protocol.production.session.roots.v1\0";
const KDF_SALT_DOMAIN: &[u8] =
    b"elon.external_pool_adapter.task_protocol.production.session.kdf_salt.v1\0";

#[derive(Clone)]
pub(super) struct ExternalPoolAdapterTaskProtocolProductionRoots {
    supervisor_session_policy_digest: [u8; 32],
    runtime_launch_profile_digest: [u8; 32],
    task_protocol_profile_digest: [u8; 32],
    upstream_transport_target_digest: [u8; 32],
    supervisor_session_policy_companion_digest: [u8; 32],
    launch_image_sha256: [u8; 32],
    ephemeral_task_secret_delivery_root: [u8; 32],
    task_protocol_conformance_run_receipt_digest: [u8; 32],
}

impl ExternalPoolAdapterTaskProtocolProductionRoots {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn new(
        supervisor_session_policy_digest: &str,
        runtime_launch_profile_digest: &str,
        task_protocol_profile_digest: &str,
        upstream_transport_target_digest: &str,
        supervisor_session_policy_companion_digest: &str,
        launch_image_sha256: &str,
        ephemeral_task_secret_delivery_root: &str,
        task_protocol_conformance_run_receipt_digest: &str,
    ) -> Result<Self> {
        Ok(Self {
            supervisor_session_policy_digest: decode_digest(
                "task production supervisor/session policy",
                supervisor_session_policy_digest,
            )?,
            runtime_launch_profile_digest: decode_digest(
                "task production runtime launch profile",
                runtime_launch_profile_digest,
            )?,
            task_protocol_profile_digest: decode_digest(
                "task production protocol profile",
                task_protocol_profile_digest,
            )?,
            upstream_transport_target_digest: decode_digest(
                "task production upstream transport target",
                upstream_transport_target_digest,
            )?,
            supervisor_session_policy_companion_digest: decode_digest(
                "task production supervisor/session policy companion",
                supervisor_session_policy_companion_digest,
            )?,
            launch_image_sha256: decode_digest(
                "task production launch image",
                launch_image_sha256,
            )?,
            ephemeral_task_secret_delivery_root: decode_digest(
                "task production ephemeral secret delivery",
                ephemeral_task_secret_delivery_root,
            )?,
            task_protocol_conformance_run_receipt_digest: decode_digest(
                "task production conformance run receipt",
                task_protocol_conformance_run_receipt_digest,
            )?,
        })
    }

    pub(super) fn launch_values(&self) -> [String; 8] {
        self.ordered().map(hex::encode)
    }

    pub(super) fn transcript_digest(&self) -> [u8; 32] {
        self.digest(ROOT_TRANSCRIPT_DOMAIN, None)
    }

    pub(super) fn kdf_salt(&self, host_nonce: &[u8; 32], child_nonce: &[u8; 32]) -> [u8; 32] {
        self.digest(KDF_SALT_DOMAIN, Some((host_nonce, child_nonce)))
    }

    fn digest(&self, domain: &[u8], nonces: Option<(&[u8; 32], &[u8; 32])>) -> [u8; 32] {
        let mut digest = Sha256::new();
        digest.update(domain);
        for root in self.ordered() {
            digest.update(root);
        }
        if let Some((host_nonce, child_nonce)) = nonces {
            digest.update(host_nonce);
            digest.update(child_nonce);
        }
        digest.finalize().into()
    }

    fn ordered(&self) -> [&[u8; 32]; 8] {
        [
            &self.supervisor_session_policy_digest,
            &self.runtime_launch_profile_digest,
            &self.task_protocol_profile_digest,
            &self.upstream_transport_target_digest,
            &self.supervisor_session_policy_companion_digest,
            &self.launch_image_sha256,
            &self.ephemeral_task_secret_delivery_root,
            &self.task_protocol_conformance_run_receipt_digest,
        ]
    }
}
