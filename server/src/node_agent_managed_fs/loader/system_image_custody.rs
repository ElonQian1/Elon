//! Linear prelease/lease custody for ordinary filesystem-resolved Windows system images.

use std::{convert::Infallible, fs::File};

use super::PinnedWindowsLoaderSystemImageFile;

/// Exact parent-relative candidate retained after terminal name resolution but before any
/// immutable-content lease or image-section authority exists.
#[must_use = "resolved system image candidates must advance into lease or failure custody"]
pub(crate) struct PinnedWindowsLoaderResolvedSystemImageCandidate {
    pub(in crate::node_agent_managed_fs) _file: File,
    pub(in crate::node_agent_managed_fs) parent_directory_identity_digest: String,
    pub(in crate::node_agent_managed_fs) normalized_name: String,
    pub(in crate::node_agent_managed_fs) resolved_component_identity_digest: String,
    pub(in crate::node_agent_managed_fs) image_file_identity_digest: String,
    pub(in crate::node_agent_managed_fs) parent_relative_open_receipt_digest: String,
    pub(in crate::node_agent_managed_fs) code_integrity_evidence_digest: String,
    pub(in crate::node_agent_managed_fs) concrete_servicing_generation_digest: String,
    pub(in crate::node_agent_managed_fs) servicing_resolution_receipt_digest: String,
    pub(in crate::node_agent_managed_fs) namespace_alias_currentness_receipt_digest: String,
    pub(in crate::node_agent_managed_fs) candidate_binding_digest: String,
    pub(in crate::node_agent_managed_fs) _resolved_system_image_candidate_backend_unavailable:
        Infallible,
}

/// Immutable scalar evidence retained when the candidate file moves into a positive lease image.
/// It preserves every field that formed the candidate binding, including servicing and namespace
/// currentness, without duplicating the consumed file handle.
pub(crate) struct ManagedLoaderSystemImageCandidateResolutionEvidence {
    pub(in crate::node_agent_managed_fs) parent_directory_identity_digest: String,
    pub(in crate::node_agent_managed_fs) normalized_name: String,
    pub(in crate::node_agent_managed_fs) resolved_component_identity_digest: String,
    pub(in crate::node_agent_managed_fs) image_file_identity_digest: String,
    pub(in crate::node_agent_managed_fs) parent_relative_open_receipt_digest: String,
    pub(in crate::node_agent_managed_fs) code_integrity_evidence_digest: String,
    pub(in crate::node_agent_managed_fs) concrete_servicing_generation_digest: String,
    pub(in crate::node_agent_managed_fs) servicing_resolution_receipt_digest: String,
    pub(in crate::node_agent_managed_fs) namespace_alias_currentness_receipt_digest: String,
    pub(in crate::node_agent_managed_fs) candidate_binding_digest: String,
}

/// Dispatched attempt owns both the exact candidate file and authenticated backend session. A
/// negative leaves both here; a positive transition must consume them into the dedicated positive
/// outcome and final system-image owner.
#[must_use = "system image lease attempt must remain with its candidate and backend session"]
pub(crate) struct ManagedLoaderSystemImageContentLeaseAcquisitionAttemptCustody {
    pub(in crate::node_agent_managed_fs) _authenticated_lease_session: File,
    pub(in crate::node_agent_managed_fs) resolution_request_ordinal: usize,
    pub(in crate::node_agent_managed_fs) candidate: PinnedWindowsLoaderResolvedSystemImageCandidate,
    pub(in crate::node_agent_managed_fs) lease_session_identity_digest: String,
    pub(in crate::node_agent_managed_fs) request_digest: String,
    pub(in crate::node_agent_managed_fs) query_nonce_digest: String,
    pub(in crate::node_agent_managed_fs) response_buffer: Vec<u8>,
    pub(in crate::node_agent_managed_fs) _system_content_lease_backend_unavailable: Infallible,
}

/// Authenticated negative response for the exact attempt. The attempt remains the linear owner of
/// candidate/session custody, while this receipt must bind the identical response bytes.
#[must_use = "authenticated system image lease rejection must remain with its exact attempt"]
pub(crate) struct ManagedLoaderSystemImageContentLeaseAuthenticatedNegativeReceipt {
    pub(in crate::node_agent_managed_fs) resolution_request_ordinal: usize,
    pub(in crate::node_agent_managed_fs) candidate_binding_digest: String,
    pub(in crate::node_agent_managed_fs) lease_session_identity_digest: String,
    pub(in crate::node_agent_managed_fs) request_digest: String,
    pub(in crate::node_agent_managed_fs) query_nonce_digest: String,
    pub(in crate::node_agent_managed_fs) negative_reason_digest: String,
    pub(in crate::node_agent_managed_fs) receipt_digest: String,
    pub(in crate::node_agent_managed_fs) authenticated_response_digest: String,
    pub(in crate::node_agent_managed_fs) authenticated_response: Vec<u8>,
    pub(in crate::node_agent_managed_fs) _authenticated_system_content_negative_backend_unavailable:
        Infallible,
}

/// A positive backend response has already consumed the candidate file into `image`; retaining
/// this outcome instead of the original attempt avoids duplicating that unique file owner during
/// outcome-uncertain recovery.
#[must_use = "positive system image lease outcome must enter final resolution or recovery custody"]
pub(crate) struct ManagedLoaderSystemImageContentLeasePositiveOutcomeCustody {
    pub(in crate::node_agent_managed_fs) _authenticated_lease_session: File,
    pub(in crate::node_agent_managed_fs) resolution_request_ordinal: usize,
    pub(in crate::node_agent_managed_fs) candidate_binding_digest: String,
    pub(in crate::node_agent_managed_fs) lease_session_identity_digest: String,
    pub(in crate::node_agent_managed_fs) request_digest: String,
    pub(in crate::node_agent_managed_fs) query_nonce_digest: String,
    pub(in crate::node_agent_managed_fs) authenticated_response_digest: String,
    pub(in crate::node_agent_managed_fs) authenticated_response: Vec<u8>,
    pub(in crate::node_agent_managed_fs) positive_receipt_digest: String,
    pub(in crate::node_agent_managed_fs) candidate_resolution_evidence:
        ManagedLoaderSystemImageCandidateResolutionEvidence,
    pub(in crate::node_agent_managed_fs) image: PinnedWindowsLoaderSystemImageFile,
    pub(in crate::node_agent_managed_fs) _authenticated_system_content_positive_backend_unavailable:
        Infallible,
}
