//! Purpose-specific quarantine custody for partially dispatched recursive acquisition.

use std::convert::Infallible;

use crate::node_agent_managed_fs::{
    ManagedLoaderAuthenticatedNegativeReceipt, ManagedLoaderSearchedNameGrant,
    ManagedLoaderSearchedNameGrantAcquisitionAttemptCustody,
    ManagedLoaderSystemImageContentLeaseAcquisitionAttemptCustody,
    ManagedLoaderSystemImageContentLeaseAuthenticatedNegativeReceipt,
    ManagedLoaderSystemImageContentLeasePositiveOutcomeCustody,
    PinnedWindowsLoaderResolvedSystemImageCandidate,
};

use super::super::WindowsPostLeaseSystemImageParseReceipt;
use super::custody::{
    WindowsRecursiveWaveCandidateAcquisitionCustody, WindowsRecursiveWaveCompletedCustody,
    WindowsRecursiveWaveGrantAcquisitionCustody, WindowsRecursiveWaveLeaseAcquisitionCustody,
    WindowsRecursiveWaveSameOwnerParseCustody,
};

/// Only an exactly bound authenticated negative with no positive outcome is definitive. Missing,
/// malformed, conflicting, timed-out or positive-but-invalid responses are outcome-uncertain.
pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) enum WindowsRecursiveWaveAdvanceFailureClass
{
    DefinitiveRejected,
    OutcomeUncertain,
}

/// Source-only parent-relative candidate attempt. The whole post-grant stage retains the relevant
/// directory/session owners; the response bytes remain here until a future authenticated backend
/// classifies and consumes the attempt.
#[must_use = "candidate attempt must remain with the whole wave after dispatch"]
struct WindowsRecursiveFilesystemCandidateAcquisitionAttemptCustody {
    resolution_request_ordinal: usize,
    parent_directory_identity_digest: String,
    normalized_name: String,
    expected_candidate_binding_digest: String,
    request_digest: String,
    query_nonce_digest: String,
    response_buffer: Vec<u8>,
    _filesystem_candidate_backend_unavailable: Infallible,
}

#[must_use = "authenticated candidate rejection must remain with its exact attempt"]
struct WindowsRecursiveFilesystemCandidateAuthenticatedNegativeReceipt {
    resolution_request_ordinal: usize,
    expected_candidate_binding_digest: String,
    request_digest: String,
    query_nonce_digest: String,
    negative_reason_digest: String,
    authenticated_response_digest: String,
    authenticated_response: Vec<u8>,
    receipt_digest: String,
    _authenticated_candidate_negative_backend_unavailable: Infallible,
}

/// Source-only same-owner parser attempt. The exact immutable owner stays inside `custody`; this
/// value binds the active ordinal/material and retains every returned byte without exposing a
/// scalar retry permit.
#[must_use = "parse attempt must remain beside its exact immutable source owner"]
struct WindowsRecursiveSameOwnerParseAttemptCustody {
    parse_receipt_ordinal: usize,
    producer_module_request_ordinal: usize,
    source_owner_binding_digest: String,
    image_material_identity_digest: String,
    parser_policy_digest: String,
    request_digest: String,
    query_nonce_digest: String,
    response_buffer: Vec<u8>,
    _same_owner_parser_backend_unavailable: Infallible,
}

#[must_use = "authenticated parse rejection must remain with its exact attempt and source"]
struct WindowsRecursiveSameOwnerParseAuthenticatedNegativeReceipt {
    parse_receipt_ordinal: usize,
    source_owner_binding_digest: String,
    image_material_identity_digest: String,
    parser_policy_digest: String,
    request_digest: String,
    query_nonce_digest: String,
    negative_reason_digest: String,
    authenticated_response_digest: String,
    authenticated_response: Vec<u8>,
    receipt_digest: String,
    _authenticated_parse_negative_backend_unavailable: Infallible,
}

/// Failure after a searched-name dispatch. Both a returned positive and negative may be retained;
/// their coexistence is represented rather than silently choosing one.
struct WindowsRecursiveGrantAdvanceFailureCustody<'root> {
    custody: WindowsRecursiveWaveGrantAcquisitionCustody<'root>,
    active_attempt: ManagedLoaderSearchedNameGrantAcquisitionAttemptCustody,
    returned_positive: Option<ManagedLoaderSearchedNameGrant>,
    returned_negative: Option<ManagedLoaderAuthenticatedNegativeReceipt>,
    returned_transport_bytes: Vec<u8>,
}

/// Failure after a filesystem candidate dispatch. A positive candidate remains linear here when
/// it cannot be safely advanced to a lease.
struct WindowsRecursiveCandidateAdvanceFailureCustody<'root> {
    custody: WindowsRecursiveWaveCandidateAcquisitionCustody<'root>,
    active_attempt: WindowsRecursiveFilesystemCandidateAcquisitionAttemptCustody,
    returned_positive: Option<PinnedWindowsLoaderResolvedSystemImageCandidate>,
    returned_negative: Option<WindowsRecursiveFilesystemCandidateAuthenticatedNegativeReceipt>,
    returned_transport_bytes: Vec<u8>,
}

/// The candidate has exactly one live location after lease dispatch. Before a positive transition
/// it remains inside the managed attempt. After a positive response consumes it into `outcome`,
/// the old attempt cannot coexist; a conflicting authenticated negative is retained beside the
/// positive owner instead.
enum WindowsRecursiveLeaseDispatchOutcomeCustody {
    Attempt {
        active_attempt: ManagedLoaderSystemImageContentLeaseAcquisitionAttemptCustody,
        returned_negative: Option<ManagedLoaderSystemImageContentLeaseAuthenticatedNegativeReceipt>,
        returned_transport_bytes: Vec<u8>,
    },
    PositiveOutcome {
        outcome: ManagedLoaderSystemImageContentLeasePositiveOutcomeCustody,
        conflicting_negative:
            Option<ManagedLoaderSystemImageContentLeaseAuthenticatedNegativeReceipt>,
        returned_transport_bytes: Vec<u8>,
    },
}

/// Failure after immutable-content lease dispatch, with a mutually exclusive candidate owner
/// state rather than an impossible simultaneous attempt and positive image.
struct WindowsRecursiveLeaseAdvanceFailureCustody<'root> {
    custody: WindowsRecursiveWaveLeaseAcquisitionCustody<'root>,
    dispatch_outcome: WindowsRecursiveLeaseDispatchOutcomeCustody,
}

/// Parser failure retains the whole wave (including the exact pending owner), active attempt,
/// authenticated negative and any positive receipt that could not be validated.
struct WindowsRecursiveParseAdvanceFailureCustody<'root> {
    custody: WindowsRecursiveWaveSameOwnerParseCustody<'root>,
    active_attempt: WindowsRecursiveSameOwnerParseAttemptCustody,
    returned_positive: Option<WindowsPostLeaseSystemImageParseReceipt>,
    returned_negative: Option<WindowsRecursiveSameOwnerParseAuthenticatedNegativeReceipt>,
    returned_transport_bytes: Vec<u8>,
}

/// A final validation failure has no detached retry path; the already-completed whole wave remains
/// parked for a future explicit recovery/release backend.
struct WindowsRecursiveWaveSealFailureCustody<'root> {
    custody: WindowsRecursiveWaveCompletedCustody<'root>,
    validation_failure_digest: String,
}

enum WindowsRecursiveWaveAdvanceFailureStage<'root> {
    SearchedNameGrant(WindowsRecursiveGrantAdvanceFailureCustody<'root>),
    FilesystemCandidate(WindowsRecursiveCandidateAdvanceFailureCustody<'root>),
    ImmutableContentLease(WindowsRecursiveLeaseAdvanceFailureCustody<'root>),
    SameOwnerParse(WindowsRecursiveParseAdvanceFailureCustody<'root>),
    WaveSeal(WindowsRecursiveWaveSealFailureCustody<'root>),
}

/// Whole-graph post-dispatch quarantine. It intentionally offers no owner, handle, response or
/// retry extractor; a future recovery backend must consume the entire parked graph.
#[must_use = "recursive failure custody must be explicitly recovered or released as a whole"]
pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) struct WindowsRecursiveWaveAdvanceFailureCustody<
    'root,
> {
    class: WindowsRecursiveWaveAdvanceFailureClass,
    stage: WindowsRecursiveWaveAdvanceFailureStage<'root>,
    failure_custody_digest: String,
    _failure_classifier_and_recovery_producer_unavailable: Infallible,
}
