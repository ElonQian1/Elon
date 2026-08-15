use anyhow::Result;
use sha2::{Digest, Sha256};

use super::decode_digest;

const ROOT_TRANSCRIPT_DOMAIN: &[u8] =
    b"elon.external_pool_adapter.task_protocol_conformance.session.roots.v1\0";
const KDF_SALT_DOMAIN: &[u8] =
    b"elon.external_pool_adapter.task_protocol_conformance.session.kdf_salt.v1\0";

#[derive(Clone)]
pub(super) struct ExternalPoolAdapterTaskProtocolConformanceRoots {
    supervisor_session_policy_digest: [u8; 32],
    task_protocol_profile_digest: [u8; 32],
    run_nonce_digest: [u8; 32],
    fixture_catalog_digest: [u8; 32],
    registry_release_digest: [u8; 32],
    installation_content_digest: [u8; 32],
    capability_set_digest: [u8; 32],
    sandbox_reattestation_receipt_digest: [u8; 32],
    runtime_compatibility_verification_receipt_digest: [u8; 32],
    source_capsule_sha256: [u8; 32],
    launch_image_sha256: [u8; 32],
    public_fixture_delivery_root: [u8; 32],
    synthetic_fixture_lane_digest: [u8; 32],
    synthetic_fixture_executor_digest: [u8; 32],
}

impl ExternalPoolAdapterTaskProtocolConformanceRoots {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn new(
        supervisor_session_policy_digest: &str,
        task_protocol_profile_digest: &str,
        run_nonce_digest: &str,
        fixture_catalog_digest: &str,
        registry_release_digest: &str,
        installation_content_digest: &str,
        capability_set_digest: &str,
        sandbox_reattestation_receipt_digest: &str,
        runtime_compatibility_verification_receipt_digest: &str,
        source_capsule_sha256: &str,
        launch_image_sha256: &str,
        public_fixture_delivery_root: &str,
        synthetic_fixture_lane_digest: &str,
        synthetic_fixture_executor_digest: &str,
    ) -> Result<Self> {
        Ok(Self {
            supervisor_session_policy_digest: decode_digest(
                "task conformance supervisor/session policy",
                supervisor_session_policy_digest,
            )?,
            task_protocol_profile_digest: decode_digest(
                "task conformance protocol profile",
                task_protocol_profile_digest,
            )?,
            run_nonce_digest: decode_digest("task conformance run nonce", run_nonce_digest)?,
            fixture_catalog_digest: decode_digest(
                "task conformance fixture catalog",
                fixture_catalog_digest,
            )?,
            registry_release_digest: decode_digest(
                "task conformance registry release",
                registry_release_digest,
            )?,
            installation_content_digest: decode_digest(
                "task conformance installation content",
                installation_content_digest,
            )?,
            capability_set_digest: decode_digest(
                "task conformance capability set",
                capability_set_digest,
            )?,
            sandbox_reattestation_receipt_digest: decode_digest(
                "task conformance sandbox reattestation receipt",
                sandbox_reattestation_receipt_digest,
            )?,
            runtime_compatibility_verification_receipt_digest: decode_digest(
                "task conformance runtime compatibility receipt",
                runtime_compatibility_verification_receipt_digest,
            )?,
            source_capsule_sha256: decode_digest(
                "task conformance source capsule",
                source_capsule_sha256,
            )?,
            launch_image_sha256: decode_digest(
                "task conformance launch image",
                launch_image_sha256,
            )?,
            public_fixture_delivery_root: decode_digest(
                "task conformance public fixture delivery",
                public_fixture_delivery_root,
            )?,
            synthetic_fixture_lane_digest: decode_digest(
                "task conformance synthetic fixture lane",
                synthetic_fixture_lane_digest,
            )?,
            synthetic_fixture_executor_digest: decode_digest(
                "task conformance synthetic fixture executor",
                synthetic_fixture_executor_digest,
            )?,
        })
    }

    pub(super) fn launch_values(&self) -> [String; 14] {
        self.ordered().map(|digest| hex::encode(digest))
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

    fn ordered(&self) -> [&[u8; 32]; 14] {
        [
            &self.supervisor_session_policy_digest,
            &self.task_protocol_profile_digest,
            &self.run_nonce_digest,
            &self.fixture_catalog_digest,
            &self.registry_release_digest,
            &self.installation_content_digest,
            &self.capability_set_digest,
            &self.sandbox_reattestation_receipt_digest,
            &self.runtime_compatibility_verification_receipt_digest,
            &self.source_capsule_sha256,
            &self.launch_image_sha256,
            &self.public_fixture_delivery_root,
            &self.synthetic_fixture_lane_digest,
            &self.synthetic_fixture_executor_digest,
        ]
    }
}
