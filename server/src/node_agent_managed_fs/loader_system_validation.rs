#![allow(dead_code)]

use std::path::Path;

use sha2::{Digest, Sha256};

use super::loader::{
    ManagedLoaderSystemImageCandidateResolutionEvidence, ManagedLoaderSystemImageContentLease,
    ManagedLoaderSystemImageContentLeaseAcquisitionAttemptCustody,
    ManagedLoaderSystemImageContentLeaseAuthenticatedNegativeReceipt,
    ManagedLoaderSystemImageContentLeasePositiveOutcomeCustody,
    PinnedWindowsLoaderResolvedSystemImageCandidate, PinnedWindowsLoaderSearchDirectory,
    PinnedWindowsLoaderSystemImageFile,
};

impl ManagedLoaderSystemImageCandidateResolutionEvidence {
    #[allow(clippy::too_many_arguments)]
    fn matches_resolution_request(
        &self,
        parent_directory_identity_digest: &str,
        normalized_name: &str,
        resolved_component_identity_digest: &str,
        image_file_identity_digest: &str,
        concrete_servicing_generation_digest: &str,
        code_integrity_evidence_digest: &str,
        servicing_resolution_receipt_digest: &str,
        namespace_alias_currentness_receipt_digest: &str,
        candidate_binding_digest: &str,
    ) -> bool {
        self.parent_directory_identity_digest == parent_directory_identity_digest
            && self.normalized_name == normalized_name
            && self.resolved_component_identity_digest == resolved_component_identity_digest
            && self.image_file_identity_digest == image_file_identity_digest
            && self.concrete_servicing_generation_digest == concrete_servicing_generation_digest
            && self.code_integrity_evidence_digest == code_integrity_evidence_digest
            && self.servicing_resolution_receipt_digest == servicing_resolution_receipt_digest
            && self.namespace_alias_currentness_receipt_digest
                == namespace_alias_currentness_receipt_digest
            && self.candidate_binding_digest == candidate_binding_digest
            && self.binding_is_self_consistent()
    }

    fn binding_is_self_consistent(&self) -> bool {
        [
            &self.parent_directory_identity_digest,
            &self.resolved_component_identity_digest,
            &self.image_file_identity_digest,
            &self.parent_relative_open_receipt_digest,
            &self.code_integrity_evidence_digest,
            &self.concrete_servicing_generation_digest,
            &self.servicing_resolution_receipt_digest,
            &self.namespace_alias_currentness_receipt_digest,
            &self.candidate_binding_digest,
        ]
        .iter()
        .all(|digest| is_lower_sha256(digest))
            && self.candidate_binding_digest
                == system_image_candidate_binding_digest(
                    &self.parent_directory_identity_digest,
                    &self.normalized_name,
                    &self.resolved_component_identity_digest,
                    &self.image_file_identity_digest,
                    &self.parent_relative_open_receipt_digest,
                    &self.code_integrity_evidence_digest,
                    &self.concrete_servicing_generation_digest,
                    &self.servicing_resolution_receipt_digest,
                    &self.namespace_alias_currentness_receipt_digest,
                )
    }

    pub(crate) fn binding(&self) -> (&str, &str, &str, &str, &str, &str, &str, &str, &str, &str) {
        (
            &self.parent_directory_identity_digest,
            &self.normalized_name,
            &self.resolved_component_identity_digest,
            &self.image_file_identity_digest,
            &self.parent_relative_open_receipt_digest,
            &self.code_integrity_evidence_digest,
            &self.concrete_servicing_generation_digest,
            &self.servicing_resolution_receipt_digest,
            &self.namespace_alias_currentness_receipt_digest,
            &self.candidate_binding_digest,
        )
    }
}

impl PinnedWindowsLoaderSearchDirectory {
    pub(crate) fn matches_handle_binding(
        &self,
        expected_path: &Path,
        expected_path_digest: &str,
        expected_identity_digest: &str,
    ) -> bool {
        self.canonical_path == expected_path
            && self.canonical_path_digest == expected_path_digest
            && self.identity_digest == expected_identity_digest
            && self.path_receipt.matches(
                &self.root_identity_digest,
                expected_identity_digest,
                expected_path_digest,
            )
            && is_lower_sha256(&self.root_identity_digest)
            && is_lower_sha256(&self.namespace_alias_currentness_receipt_digest)
    }

    pub(crate) fn path_currentness_binding(&self) -> (&str, &str, &str, &str, &str, &str, &str) {
        let (root, final_identity, path, components, parent_chain, observation) =
            self.path_receipt.binding();
        (
            root,
            final_identity,
            path,
            components,
            parent_chain,
            observation,
            &self.namespace_alias_currentness_receipt_digest,
        )
    }
}

impl PinnedWindowsLoaderResolvedSystemImageCandidate {
    pub(crate) fn binding(&self) -> (&str, &str, &str, &str, &str, &str, &str, &str, &str, &str) {
        (
            &self.parent_directory_identity_digest,
            &self.normalized_name,
            &self.resolved_component_identity_digest,
            &self.image_file_identity_digest,
            &self.parent_relative_open_receipt_digest,
            &self.code_integrity_evidence_digest,
            &self.concrete_servicing_generation_digest,
            &self.servicing_resolution_receipt_digest,
            &self.namespace_alias_currentness_receipt_digest,
            &self.candidate_binding_digest,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn matches_resolution_request(
        &self,
        parent_directory_identity_digest: &str,
        normalized_name: &str,
        resolved_component_identity_digest: &str,
        image_file_identity_digest: &str,
        concrete_servicing_generation_digest: &str,
        code_integrity_evidence_digest: &str,
        servicing_resolution_receipt_digest: &str,
        namespace_alias_currentness_receipt_digest: &str,
    ) -> bool {
        self.parent_directory_identity_digest == parent_directory_identity_digest
            && self.normalized_name == normalized_name
            && self.resolved_component_identity_digest == resolved_component_identity_digest
            && self.image_file_identity_digest == image_file_identity_digest
            && self.concrete_servicing_generation_digest == concrete_servicing_generation_digest
            && self.code_integrity_evidence_digest == code_integrity_evidence_digest
            && self.servicing_resolution_receipt_digest == servicing_resolution_receipt_digest
            && self.namespace_alias_currentness_receipt_digest
                == namespace_alias_currentness_receipt_digest
            && [
                &self.parent_directory_identity_digest,
                &self.resolved_component_identity_digest,
                &self.image_file_identity_digest,
                &self.parent_relative_open_receipt_digest,
                &self.code_integrity_evidence_digest,
                &self.concrete_servicing_generation_digest,
                &self.servicing_resolution_receipt_digest,
                &self.namespace_alias_currentness_receipt_digest,
                &self.candidate_binding_digest,
            ]
            .iter()
            .all(|digest| is_lower_sha256(digest))
            && self.candidate_binding_digest
                == system_image_candidate_binding_digest(
                    &self.parent_directory_identity_digest,
                    &self.normalized_name,
                    &self.resolved_component_identity_digest,
                    &self.image_file_identity_digest,
                    &self.parent_relative_open_receipt_digest,
                    &self.code_integrity_evidence_digest,
                    &self.concrete_servicing_generation_digest,
                    &self.servicing_resolution_receipt_digest,
                    &self.namespace_alias_currentness_receipt_digest,
                )
    }

    fn binding_is_self_consistent(&self) -> bool {
        self.matches_resolution_request(
            &self.parent_directory_identity_digest,
            &self.normalized_name,
            &self.resolved_component_identity_digest,
            &self.image_file_identity_digest,
            &self.concrete_servicing_generation_digest,
            &self.code_integrity_evidence_digest,
            &self.servicing_resolution_receipt_digest,
            &self.namespace_alias_currentness_receipt_digest,
        )
    }
}

impl ManagedLoaderSystemImageContentLeaseAcquisitionAttemptCustody {
    pub(crate) fn binding(&self) -> (usize, &str, &str, &str, &str) {
        (
            self.resolution_request_ordinal,
            &self.candidate.candidate_binding_digest,
            &self.lease_session_identity_digest,
            &self.request_digest,
            &self.query_nonce_digest,
        )
    }
}

impl ManagedLoaderSystemImageContentLeaseAuthenticatedNegativeReceipt {
    pub(crate) fn matches_attempt(
        &self,
        attempt: &ManagedLoaderSystemImageContentLeaseAcquisitionAttemptCustody,
    ) -> bool {
        let (ordinal, candidate, session, request, nonce) = attempt.binding();
        self.resolution_request_ordinal == ordinal
            && attempt.candidate.binding_is_self_consistent()
            && self.candidate_binding_digest == candidate
            && self.lease_session_identity_digest == session
            && self.request_digest == request
            && self.query_nonce_digest == nonce
            && [
                &self.candidate_binding_digest,
                &self.lease_session_identity_digest,
                &self.request_digest,
                &self.query_nonce_digest,
                &self.negative_reason_digest,
                &self.receipt_digest,
                &self.authenticated_response_digest,
            ]
            .iter()
            .all(|digest| is_lower_sha256(digest))
            && self.authenticated_response == attempt.response_buffer
            && self.authenticated_response_digest
                == hex::encode(Sha256::digest(&self.authenticated_response))
            && self.receipt_digest
                == system_image_negative_receipt_digest(
                    self.resolution_request_ordinal,
                    &self.candidate_binding_digest,
                    &self.lease_session_identity_digest,
                    &self.request_digest,
                    &self.query_nonce_digest,
                    &self.negative_reason_digest,
                    &self.authenticated_response_digest,
                )
            && !self.authenticated_response.is_empty()
    }
}

impl ManagedLoaderSystemImageContentLeasePositiveOutcomeCustody {
    pub(crate) fn binding(&self) -> (usize, &str, &str, &str, &str, &str, &str) {
        (
            self.resolution_request_ordinal,
            &self.candidate_binding_digest,
            &self.lease_session_identity_digest,
            &self.request_digest,
            &self.query_nonce_digest,
            &self.authenticated_response_digest,
            &self.positive_receipt_digest,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn matches_resolution_request(
        &self,
        resolution_request_ordinal: usize,
        candidate_binding_digest: &str,
        lease_request_digest: &str,
        parent_directory_identity_digest: &str,
        normalized_name: &str,
        image_file_identity_digest: &str,
        immutable_section_identity_digest: &str,
        servicing_generation_digest: &str,
    ) -> bool {
        let image_binding = self.image.binding();
        let (_, _, _, _, image_parent_relative_open_receipt_digest, _) = image_binding;
        let lease_binding = self.image.content_lease_binding();
        self.resolution_request_ordinal == resolution_request_ordinal
            && self.candidate_binding_digest == candidate_binding_digest
            && self
                .candidate_resolution_evidence
                .binding_is_self_consistent()
            && self.candidate_resolution_evidence.candidate_binding_digest
                == self.candidate_binding_digest
            && self
                .candidate_resolution_evidence
                .parent_relative_open_receipt_digest
                == image_parent_relative_open_receipt_digest
            && self.request_digest == lease_request_digest
            && [
                &self.candidate_binding_digest,
                &self.lease_session_identity_digest,
                &self.request_digest,
                &self.query_nonce_digest,
                &self.authenticated_response_digest,
                &self.positive_receipt_digest,
            ]
            .iter()
            .all(|digest| is_lower_sha256(digest))
            && !self.authenticated_response.is_empty()
            && self.authenticated_response_digest
                == hex::encode(Sha256::digest(&self.authenticated_response))
            && self.image.matches_resolution(
                parent_directory_identity_digest,
                normalized_name,
                image_file_identity_digest,
                immutable_section_identity_digest,
                servicing_generation_digest,
            )
            && self.positive_receipt_digest
                == system_image_positive_receipt_digest(
                    self.resolution_request_ordinal,
                    &self.candidate_binding_digest,
                    &self.lease_session_identity_digest,
                    &self.request_digest,
                    &self.query_nonce_digest,
                    &self.authenticated_response_digest,
                    image_binding,
                    lease_binding,
                )
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn matches_candidate_resolution_request(
        &self,
        parent_directory_identity_digest: &str,
        normalized_name: &str,
        resolved_component_identity_digest: &str,
        image_file_identity_digest: &str,
        concrete_servicing_generation_digest: &str,
        code_integrity_evidence_digest: &str,
        servicing_resolution_receipt_digest: &str,
        namespace_alias_currentness_receipt_digest: &str,
    ) -> bool {
        self.candidate_resolution_evidence
            .matches_resolution_request(
                parent_directory_identity_digest,
                normalized_name,
                resolved_component_identity_digest,
                image_file_identity_digest,
                concrete_servicing_generation_digest,
                code_integrity_evidence_digest,
                servicing_resolution_receipt_digest,
                namespace_alias_currentness_receipt_digest,
                &self.candidate_binding_digest,
            )
    }

    pub(crate) fn candidate_resolution_binding(
        &self,
    ) -> (&str, &str, &str, &str, &str, &str, &str, &str, &str, &str) {
        self.candidate_resolution_evidence.binding()
    }

    pub(crate) fn image(&self) -> &PinnedWindowsLoaderSystemImageFile {
        &self.image
    }
}

impl PinnedWindowsLoaderSystemImageFile {
    pub(crate) fn binding(&self) -> (&str, &str, &str, &str, &str, &str) {
        (
            &self.parent_directory_identity_digest,
            &self.normalized_name,
            &self.image_file_identity_digest,
            &self.immutable_section_identity_digest,
            &self.parent_relative_open_receipt_digest,
            &self.section_mapping_receipt_digest,
        )
    }

    pub(crate) fn matches_resolution(
        &self,
        parent_directory_identity_digest: &str,
        normalized_name: &str,
        image_file_identity_digest: &str,
        immutable_section_identity_digest: &str,
        servicing_generation_digest: &str,
    ) -> bool {
        self.parent_directory_identity_digest == parent_directory_identity_digest
            && self.normalized_name == normalized_name
            && self.image_file_identity_digest == image_file_identity_digest
            && self.immutable_section_identity_digest == immutable_section_identity_digest
            && self.content_lease.matches_resolution(
                image_file_identity_digest,
                immutable_section_identity_digest,
                servicing_generation_digest,
            )
            && [
                &self.parent_directory_identity_digest,
                &self.image_file_identity_digest,
                &self.immutable_section_identity_digest,
                &self.parent_relative_open_receipt_digest,
                &self.section_mapping_receipt_digest,
            ]
            .iter()
            .all(|digest| is_lower_sha256(digest))
    }

    pub(crate) fn content_lease_binding(&self) -> (&str, &str, &str, &str, &str) {
        self.content_lease.binding()
    }
}

impl ManagedLoaderSystemImageContentLease {
    fn matches_resolution(
        &self,
        image_file_identity_digest: &str,
        immutable_section_identity_digest: &str,
        servicing_generation_digest: &str,
    ) -> bool {
        self.image_file_identity_digest == image_file_identity_digest
            && self.immutable_section_identity_digest == immutable_section_identity_digest
            && self.servicing_generation_digest == servicing_generation_digest
            && self.writable_open_denied
            && self.existing_handle_write_denied
            && self.writable_mapping_denied
            && self.eof_allocation_metadata_mutation_denied
            && self.rename_link_mutation_denied
            && self.delete_disposition_denied
            && [
                &self.image_file_identity_digest,
                &self.immutable_section_identity_digest,
                &self.servicing_generation_digest,
                &self.lease_generation_digest,
                &self.immutable_content_policy_digest,
            ]
            .iter()
            .all(|digest| is_lower_sha256(digest))
    }

    fn binding(&self) -> (&str, &str, &str, &str, &str) {
        (
            &self.image_file_identity_digest,
            &self.immutable_section_identity_digest,
            &self.servicing_generation_digest,
            &self.lease_generation_digest,
            &self.immutable_content_policy_digest,
        )
    }
}

fn is_lower_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|value| value.is_ascii_digit() || (b'a'..=b'f').contains(&value))
}

#[allow(clippy::too_many_arguments)]
fn system_image_negative_receipt_digest(
    resolution_request_ordinal: usize,
    candidate_binding_digest: &str,
    lease_session_identity_digest: &str,
    request_digest: &str,
    query_nonce_digest: &str,
    negative_reason_digest: &str,
    authenticated_response_digest: &str,
) -> String {
    let mut digest = Sha256::new();
    digest.update(b"ELON_MANAGED_LOADER_SYSTEM_IMAGE_CONTENT_LEASE_NEGATIVE_V1");
    digest.update((resolution_request_ordinal as u64).to_le_bytes());
    for value in [
        candidate_binding_digest,
        lease_session_identity_digest,
        request_digest,
        query_nonce_digest,
        negative_reason_digest,
        authenticated_response_digest,
    ] {
        digest.update((value.len() as u64).to_le_bytes());
        digest.update(value.as_bytes());
    }
    hex::encode(digest.finalize())
}

#[allow(clippy::too_many_arguments)]
fn system_image_positive_receipt_digest(
    resolution_request_ordinal: usize,
    candidate_binding_digest: &str,
    lease_session_identity_digest: &str,
    request_digest: &str,
    query_nonce_digest: &str,
    authenticated_response_digest: &str,
    image_binding: (&str, &str, &str, &str, &str, &str),
    lease_binding: (&str, &str, &str, &str, &str),
) -> String {
    let mut digest = Sha256::new();
    digest.update(b"ELON_MANAGED_LOADER_SYSTEM_IMAGE_CONTENT_LEASE_POSITIVE_V1");
    digest.update((resolution_request_ordinal as u64).to_le_bytes());
    for value in [
        candidate_binding_digest,
        lease_session_identity_digest,
        request_digest,
        query_nonce_digest,
        authenticated_response_digest,
        image_binding.0,
        image_binding.1,
        image_binding.2,
        image_binding.3,
        image_binding.4,
        image_binding.5,
        lease_binding.0,
        lease_binding.1,
        lease_binding.2,
        lease_binding.3,
        lease_binding.4,
    ] {
        digest.update((value.len() as u64).to_le_bytes());
        digest.update(value.as_bytes());
    }
    hex::encode(digest.finalize())
}

#[allow(clippy::too_many_arguments)]
fn system_image_candidate_binding_digest(
    parent_directory_identity_digest: &str,
    normalized_name: &str,
    resolved_component_identity_digest: &str,
    image_file_identity_digest: &str,
    parent_relative_open_receipt_digest: &str,
    code_integrity_evidence_digest: &str,
    concrete_servicing_generation_digest: &str,
    servicing_resolution_receipt_digest: &str,
    namespace_alias_currentness_receipt_digest: &str,
) -> String {
    let mut digest = Sha256::new();
    digest.update(b"ELON_MANAGED_LOADER_RESOLVED_SYSTEM_IMAGE_CANDIDATE_V1");
    for value in [
        parent_directory_identity_digest,
        normalized_name,
        resolved_component_identity_digest,
        image_file_identity_digest,
        parent_relative_open_receipt_digest,
        code_integrity_evidence_digest,
        concrete_servicing_generation_digest,
        servicing_resolution_receipt_digest,
        namespace_alias_currentness_receipt_digest,
    ] {
        digest.update((value.len() as u64).to_le_bytes());
        digest.update(value.as_bytes());
    }
    hex::encode(digest.finalize())
}
