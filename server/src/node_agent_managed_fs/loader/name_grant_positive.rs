//! Authenticated positive-response binding for namespace name grants.

use sha2::{Digest, Sha256};

use std::sync::Arc;

use super::{
    is_lower_sha256, ManagedLoaderSearchedNameGrant,
    ManagedLoaderSearchedNameGrantAcquisitionAttemptCustody,
};

impl ManagedLoaderSearchedNameGrant {
    pub(crate) fn authenticated_positive_binding(&self) -> (&str, &str, &str, &str) {
        (
            &self.request_digest,
            &self.query_nonce_digest,
            &self.authenticated_response_digest,
            &self.positive_receipt_digest,
        )
    }

    pub(crate) fn authenticated_positive_is_bound(&self) -> bool {
        !self.authenticated_response.is_empty()
            && [
                &self.request_digest,
                &self.query_nonce_digest,
                &self.authenticated_response_digest,
                &self.positive_receipt_digest,
            ]
            .into_iter()
            .all(|digest| is_lower_sha256(digest))
            && self.authenticated_response_digest
                == hex::encode(Sha256::digest(&self.authenticated_response))
            && self.positive_receipt_digest == self.recompute_positive_receipt_digest()
    }

    pub(crate) fn matches_attempt(
        &self,
        attempt: &ManagedLoaderSearchedNameGrantAcquisitionAttemptCustody,
    ) -> bool {
        Arc::ptr_eq(&self.owner, &attempt.owner)
            && self.session_identity_digest == attempt.session_identity_digest
            && self.request_digest == attempt.request_digest
            && self.query_nonce_digest == attempt.query_nonce_digest
            && self.authenticated_response == attempt.response_buffer
            && self.authenticated_positive_is_bound()
    }

    fn recompute_positive_receipt_digest(&self) -> String {
        let mut digest = Sha256::new();
        digest.update(b"ELON_MANAGED_LOADER_SEARCHED_NAME_GRANT_POSITIVE_V1");
        for value in [
            &self.session_identity_digest,
            &self.generation_domain_digest,
            &self.parent_directory_identity_digest,
            &self.normalized_name,
            &self.disposition_digest,
            &self.fence_generation_digest,
            &self.request_digest,
            &self.query_nonce_digest,
            &self.authenticated_response_digest,
        ] {
            digest.update((value.len() as u64).to_le_bytes());
            digest.update(value.as_bytes());
        }
        digest.update(self.grant_generation.to_le_bytes());
        hex::encode(digest.finalize())
    }
}
