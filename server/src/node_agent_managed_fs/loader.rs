#![allow(dead_code)]

mod name_grant_positive;
mod system_image_custody;

pub(crate) use system_image_custody::{
    ManagedLoaderSystemImageContentLeaseAcquisitionAttemptCustody,
    ManagedLoaderSystemImageContentLeaseAuthenticatedNegativeReceipt,
    ManagedLoaderSystemImageContentLeasePositiveOutcomeCustody,
    PinnedWindowsLoaderResolvedSystemImageCandidate,
};

use std::{
    convert::Infallible,
    fs::File,
    mem::ManuallyDrop,
    path::{Path, PathBuf},
    sync::Arc,
};

use sha2::{Digest, Sha256};

use super::{
    ManagedObjectBinding, PinnedManagedDirectory, PinnedManagedFile, PlatformFileIdentity,
};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum ManagedLoaderFileAccessProfile {
    ExecutableImage,
    ReadOnlyPackageAsset,
}

/// Kernel-enforced content/open/mapping lease for one exact FileId. Name fencing and a post-reopen
/// hash do not prevent a writable mapped view from mutating bytes later, so every transitioned
/// package file must retain this lease through image/import mapping. No producer exists until a
/// backend can deny writable opens, write/delete disposition, and writable section mappings.
#[must_use = "file content lease must outlive every loader image/import mapping"]
pub(crate) struct ManagedLoaderFileContentLease {
    pub(super) _kernel_content_guard: File,
    pub(super) file_identity_digest: String,
    pub(super) sealed_digest: String,
    pub(super) lease_generation_digest: String,
    pub(super) writable_open_denied: bool,
    pub(super) existing_handle_write_denied: bool,
    pub(super) writable_mapping_denied: bool,
    pub(super) eof_allocation_metadata_mutation_denied: bool,
    pub(super) rename_link_mutation_denied: bool,
    pub(super) delete_disposition_denied: bool,
    pub(super) immutable_content_policy_digest: String,
    pub(super) _immutable_content_backend_unavailable: Infallible,
}

/// Exact platform dispatch retained if acquiring a FileId content lease is rejected or uncertain.
#[must_use = "content-lease acquisition attempt must remain in failure custody"]
pub(crate) struct ManagedLoaderFileContentLeaseAcquisitionAttemptCustody {
    pub(super) _kernel_content_guard_session: File,
    pub(super) file_identity_digest: String,
    pub(super) sealed_digest: String,
    pub(super) request_digest: String,
    pub(super) query_nonce_digest: String,
    pub(super) response_buffer: Vec<u8>,
}

/// Authenticated negative response for one exact FileId/content-digest lease request. The receipt
/// has no constructor in this source slice; a future kernel backend must return it together with
/// the still-owned dispatch attempt before rejection can be called definitive.
#[must_use = "content-lease negative receipt must remain with rejected acquisition custody"]
pub(crate) struct ManagedLoaderFileContentLeaseAuthenticatedNegativeReceipt {
    pub(super) file_identity_digest: String,
    pub(super) sealed_digest: String,
    pub(super) request_digest: String,
    pub(super) query_nonce_digest: String,
    pub(super) negative_reason_digest: String,
    pub(super) receipt_digest: String,
    pub(super) authenticated_response_digest: String,
    pub(super) authenticated_response: Vec<u8>,
    pub(super) _authenticated_negative_backend_unavailable: Infallible,
}

/// Handle-derived identity retained across the future share-none close/reopen barrier.
///
/// This is recovery custody, not a reopen permit. No constructor or scalar extractor exists in the
/// current source slice because a safe producer must explicitly observe `CloseHandle`, keep the
/// exact parent chain, and classify every post-close failure as outcome-uncertain.
#[must_use = "loader transition anchors must remain in successor or recovery custody"]
pub(crate) struct ManagedLoaderFileIdentityAnchor {
    pub(super) _directory_handles: Vec<Arc<File>>,
    pub(super) root_volume_serial: u64,
    pub(super) root_identity_digest: String,
    pub(super) identity: PlatformFileIdentity,
    pub(super) identity_digest: String,
    pub(super) binding: ManagedObjectBinding,
    pub(super) relative_path: String,
    pub(super) expected_digest: String,
    pub(super) expected_size_bytes: u64,
    pub(super) access_profile: ManagedLoaderFileAccessProfile,
    pub(super) delete_pending: bool,
    pub(super) content_lease: ManagedLoaderFileContentLease,
}

/// Receipt produced only by consuming one identity anchor and the exact replacement handle. It
/// retains the source observation needed to prove volume/FileId/type/reparse/link/size,
/// parent-relative name/binding, close outcome, access profile, and content-lease continuity.
pub(crate) struct ManagedLoaderFileReopenReceipt {
    pub(super) source_root_volume_serial: u64,
    pub(super) source_root_identity_digest: String,
    pub(super) source_identity: PlatformFileIdentity,
    pub(super) source_identity_digest: String,
    pub(super) source_binding: ManagedObjectBinding,
    pub(super) source_relative_path: String,
    pub(super) source_expected_digest: String,
    pub(super) source_expected_size_bytes: u64,
    pub(super) source_access_profile: ManagedLoaderFileAccessProfile,
    pub(super) source_delete_pending: bool,
    pub(super) source_content_lease_generation_digest: String,
    pub(super) confirmed_close_receipt_digest: String,
    pub(super) parent_relative_open_receipt_digest: String,
    pub(super) replacement_canonical_path_digest: String,
    pub(super) comparison_receipt_digest: String,
    pub(super) _anchor_consuming_reopen_backend_unavailable: Infallible,
}

/// Handle-derived component chain for one exact canonical launch path. A path string/digest alone
/// cannot inhabit this receipt: the future backend must retain every parent, bind the first parent
/// to the managed root, chain each child FileId, and attest the share/grant contract.
pub(crate) struct ManagedLoaderHandlePathReceipt {
    pub(super) root_identity_digest: String,
    pub(super) final_identity_digest: String,
    pub(super) canonical_path_digest: String,
    pub(super) component_set_digest: String,
    pub(super) retained_parent_chain_share_contract_digest: String,
    pub(super) observation_receipt_digest: String,
    pub(super) _handle_path_backend_unavailable: Infallible,
}

/// One package file reopened parent-relative with the proposed Windows loader-compatible access
/// shape and then re-identified and re-hashed on that exact handle.
///
/// The type deliberately has no constructor in this architecture slice. A plain `File`, path,
/// digest, or caller assertion cannot inhabit it.
#[must_use = "loader-compatible package custody must remain bound to its admitted successor"]
pub(crate) struct PinnedManagedLoaderFile {
    pub(super) _file: File,
    pub(super) _directory_handles: Vec<Arc<File>>,
    pub(super) root_volume_serial: u64,
    pub(super) root_identity_digest: String,
    pub(super) identity: PlatformFileIdentity,
    pub(super) identity_digest: String,
    pub(super) binding: ManagedObjectBinding,
    pub(super) access_profile: ManagedLoaderFileAccessProfile,
    pub(super) relative_path: String,
    pub(super) canonical_path: PathBuf,
    pub(super) canonical_path_digest: String,
    pub(super) digest: String,
    pub(super) delete_pending: bool,
    pub(super) content_lease: ManagedLoaderFileContentLease,
    pub(super) reopen_receipt: ManagedLoaderFileReopenReceipt,
    pub(super) path_receipt: ManagedLoaderHandlePathReceipt,
}

/// A working-directory or package-directory owner that retains the original extraction handle;
/// it is not closed/reopened into a narrower access profile. Identity and canonical path are
/// derived from that same retained handle. Its share mode alone does not fence child names.
#[must_use = "loader directory custody must retain its namespace authority"]
pub(crate) struct PinnedManagedLoaderDirectory {
    pub(super) _directory: PinnedManagedDirectory,
    pub(super) root_identity_digest: String,
    pub(super) managed_relative_path: String,
    pub(super) canonical_path: PathBuf,
    pub(super) canonical_path_digest: String,
    pub(super) identity_digest: String,
    pub(super) path_receipt: ManagedLoaderHandlePathReceipt,
}

/// Retained handle-derived identity for an external Windows filesystem search directory such as
/// System32. It is distinct from package-root custody and from Object Manager/API-set/SxS policy.
#[must_use = "external loader search-directory custody must remain with resolution authority"]
pub(crate) struct PinnedWindowsLoaderSearchDirectory {
    pub(super) _directory: File,
    pub(super) root_identity_digest: String,
    pub(super) canonical_path: PathBuf,
    pub(super) canonical_path_digest: String,
    pub(super) identity_digest: String,
    pub(super) path_receipt: ManagedLoaderHandlePathReceipt,
    pub(super) namespace_alias_currentness_receipt_digest: String,
    pub(super) _external_search_path_currentness_backend_unavailable: Infallible,
}

/// Exact parent-relative system image retained for one filesystem-search resolution. A future
/// backend must open the named child under the already-pinned search directory, retain that file
/// handle, and bind its FileId to the immutable image section used by the loader graph.
#[must_use = "filesystem-resolved system images must remain in loader authority"]
pub(crate) struct PinnedWindowsLoaderSystemImageFile {
    pub(super) _file: File,
    pub(super) parent_directory_identity_digest: String,
    pub(super) normalized_name: String,
    pub(super) image_file_identity_digest: String,
    pub(super) immutable_section_identity_digest: String,
    pub(super) parent_relative_open_receipt_digest: String,
    pub(super) section_mapping_receipt_digest: String,
    pub(super) content_lease: ManagedLoaderSystemImageContentLease,
    pub(super) _parent_relative_system_image_backend_unavailable: Infallible,
}

/// Kernel-enforced immutability for an ordinary filesystem system image that Windows may reopen.
/// It closes the mutation gap left by name fencing and a one-time section-mapping receipt.
#[must_use = "system image content lease must outlive process image mapping"]
pub(crate) struct ManagedLoaderSystemImageContentLease {
    pub(super) _kernel_content_guard: File,
    pub(super) image_file_identity_digest: String,
    pub(super) immutable_section_identity_digest: String,
    pub(super) servicing_generation_digest: String,
    pub(super) lease_generation_digest: String,
    pub(super) writable_open_denied: bool,
    pub(super) existing_handle_write_denied: bool,
    pub(super) writable_mapping_denied: bool,
    pub(super) eof_allocation_metadata_mutation_denied: bool,
    pub(super) rename_link_mutation_denied: bool,
    pub(super) delete_disposition_denied: bool,
    pub(super) immutable_content_policy_digest: String,
    pub(super) _immutable_system_content_backend_unavailable: Infallible,
}

/// One private shared owner for the exact authenticated driver connection. Grants, queries, and
/// receipts clone this owner; none opens or substitutes an unrelated session handle.
struct ManagedLoaderNamespaceSessionOwner {
    _driver_session: File,
}

/// Opaque authenticated namespace-fence driver session retained for the lifetime of every grant.
#[must_use = "namespace session must outlive every loader fence grant"]
pub(crate) struct ManagedLoaderNamespaceSession {
    owner: Arc<ManagedLoaderNamespaceSessionOwner>,
    pub(super) session_identity_digest: String,
    pub(super) grant_generation: u64,
    pub(super) generation_domain_digest: String,
}

/// One exact parent/name grant, including an absent-or-shadow disposition that cannot be modeled
/// by an existing-object cleanup fence. The private shared owner keeps the authenticated session
/// alive until every grant and query receipt has left custody.
#[must_use = "searched-name grant must remain in namespace authority or recovery custody"]
pub(crate) struct ManagedLoaderSearchedNameGrant {
    owner: Arc<ManagedLoaderNamespaceSessionOwner>,
    pub(super) session_identity_digest: String,
    pub(super) grant_generation: u64,
    pub(super) generation_domain_digest: String,
    pub(super) parent_directory_identity_digest: String,
    pub(super) normalized_name: String,
    pub(super) disposition_digest: String,
    pub(super) fence_generation_digest: String,
    pub(super) request_digest: String,
    pub(super) query_nonce_digest: String,
    pub(super) authenticated_response: Vec<u8>,
    pub(super) authenticated_response_digest: String,
    pub(super) positive_receipt_digest: String,
    pub(super) _authenticated_positive_backend_unavailable: Infallible,
}

#[must_use = "name-grant acquisition attempt must remain in failure custody"]
pub(crate) struct ManagedLoaderSearchedNameGrantAcquisitionAttemptCustody {
    owner: Arc<ManagedLoaderNamespaceSessionOwner>,
    pub(super) session_identity_digest: String,
    pub(super) request_digest: String,
    pub(super) query_nonce_digest: String,
    pub(super) response_buffer: Vec<u8>,
}

/// Authenticated negative response required before a dispatched query can be called definitive.
/// No scalar constructor exists; a future backend must retain its authenticated response bytes.
#[must_use = "authenticated negative receipt must remain with rejected query custody"]
pub(crate) struct ManagedLoaderAuthenticatedNegativeReceipt {
    owner: Arc<ManagedLoaderNamespaceSessionOwner>,
    pub(super) session_identity_digest: String,
    pub(super) request_digest: String,
    pub(super) query_nonce_digest: String,
    pub(super) negative_reason_digest: String,
    pub(super) receipt_digest: String,
    pub(super) authenticated_response_digest: String,
    pub(super) authenticated_response: Vec<u8>,
    pub(super) _authenticated_negative_backend_unavailable: Infallible,
}

/// Dispatch custody retained whenever a fence/currentness query has a definitive or uncertain
/// failure. Request/response material cannot be reduced to a retryable scalar error.
#[must_use = "namespace query attempt must remain in failure or recovery custody"]
pub(crate) struct ManagedLoaderNamespaceQueryAttemptCustody {
    owner: Arc<ManagedLoaderNamespaceSessionOwner>,
    pub(super) session_identity_digest: String,
    pub(super) request_digest: String,
    pub(super) grant_generation: u64,
    pub(super) generation_domain_digest: String,
    pub(super) query_nonce_digest: String,
    pub(super) fence_generation_set_digest: String,
    pub(super) content_lease_generation_set_digest: String,
    pub(super) response_buffer: Vec<u8>,
}

/// Authenticated response bound to the retained namespace session and exact grant generation.
#[must_use = "namespace query receipt must remain in successor or process custody"]
pub(crate) struct ManagedLoaderNamespaceQueryReceipt {
    owner: Arc<ManagedLoaderNamespaceSessionOwner>,
    pub(super) authenticated_response: Vec<u8>,
    pub(super) authenticated_response_digest: String,
    pub(super) receipt_digest: String,
    pub(super) session_identity_digest: String,
    pub(super) grant_generation: u64,
    pub(super) query_generation: u64,
    pub(super) generation_domain_digest: String,
    pub(super) request_digest: String,
    pub(super) query_nonce_digest: String,
    pub(super) fence_generation_set_digest: String,
    pub(super) content_lease_generation_set_digest: String,
    pub(super) _authenticated_query_receipt_backend_unavailable: Infallible,
}

/// A replacement handle opened after the irreversible barrier but rejected before promotion to a
/// loader-compatible file. Keeping it here prevents a mismatch from collapsing into a retryable
/// path or digest error.
#[must_use = "rejected post-barrier handles must remain quarantined"]
pub(crate) struct QuarantinedManagedLoaderFile {
    pub(super) _file: File,
    pub(super) _directory_handles: Vec<Arc<File>>,
    pub(super) anchor: ManagedLoaderFileIdentityAnchor,
}

/// Opaque custody after a platform close attempt cannot prove whether the source handle closed.
///
/// The ordinary `File` destructor is deliberately suppressed: dropping a possibly-closed raw
/// handle could close a later handle that reused the same value. A future platform recovery API
/// must consume this value and resolve or process-isolate the quarantine; it cannot be retried as
/// a live `PinnedManagedFile`.
#[must_use = "uncertain source-close custody must remain quarantined"]
pub(crate) struct QuarantinedManagedLoaderSourceClose {
    pub(super) _source: ManuallyDrop<PinnedManagedFile>,
    pub(super) anchor: ManagedLoaderFileIdentityAnchor,
}

/// Linear custody after the source handle is confirmed closed but the parent-relative reopen has
/// not produced an admissible replacement. The anchor retains the exact parent chain and FileId
/// content lease; dispatch evidence and any possibly-returned handle stay attached to that owner.
#[must_use = "post-close reopen attempts must remain in recovery custody"]
pub(crate) struct ManagedLoaderParentRelativeReopenAttemptCustody {
    pub(super) anchor: ManagedLoaderFileIdentityAnchor,
    pub(super) _possibly_returned_handle: Option<File>,
    pub(super) confirmed_close_receipt_digest: String,
    pub(super) request_digest: String,
    pub(super) query_nonce_digest: String,
    pub(super) response_buffer: Vec<u8>,
}

/// Authenticated rejection for one exact post-close parent-relative open dispatch. Without this
/// receipt the operation remains outcome-uncertain even when the platform returned an error.
#[must_use = "post-close reopen negative receipts must remain with their exact attempt"]
pub(crate) struct ManagedLoaderParentRelativeReopenAuthenticatedNegativeReceipt {
    pub(super) source_identity_digest: String,
    pub(super) confirmed_close_receipt_digest: String,
    pub(super) request_digest: String,
    pub(super) query_nonce_digest: String,
    pub(super) negative_reason_digest: String,
    pub(super) receipt_digest: String,
    pub(super) authenticated_response_digest: String,
    pub(super) authenticated_response: Vec<u8>,
    pub(super) _authenticated_reopen_negative_backend_unavailable: Infallible,
}

impl PinnedManagedLoaderFile {
    pub(crate) fn matches_sealed_observation(
        &self,
        expected_digest: &str,
        expected_size_bytes: u64,
        expected_identity_digest: &str,
    ) -> bool {
        self.digest == expected_digest
            && self.identity.file_size == expected_size_bytes
            && self.identity_digest == expected_identity_digest
            && !self.delete_pending
            && self
                .content_lease
                .matches(expected_identity_digest, expected_digest)
    }

    pub(crate) fn is_executable_image(&self) -> bool {
        matches!(
            self.access_profile,
            ManagedLoaderFileAccessProfile::ExecutableImage
        )
    }

    /// Read-only launch material derived from the exact retained handle after reopen.
    pub(crate) fn handle_derived_canonical_path(&self) -> &Path {
        &self.canonical_path
    }

    pub(crate) fn digest_for_binding(&self) -> &str {
        &self.digest
    }

    pub(crate) fn content_lease_generation_digest(&self) -> &str {
        &self.content_lease.lease_generation_digest
    }

    pub(crate) fn content_lease_binding(&self) -> (&str, &str, &str, &str) {
        self.content_lease.binding()
    }

    pub(crate) fn matches_root_identity(&self, expected_root_identity_digest: &str) -> bool {
        self.root_identity_digest == expected_root_identity_digest
    }

    pub(crate) fn matches_plan_file(
        &self,
        expected_digest: &str,
        expected_size_bytes: u64,
        expected_executable: bool,
        expected_relative_path: &str,
        expected_root_identity_digest: &str,
        expected_file_identity_digest: &str,
    ) -> bool {
        self.digest == expected_digest
            && self.identity.file_size == expected_size_bytes
            && self.relative_path == expected_relative_path
            && self.matches_root_identity(expected_root_identity_digest)
            && self.identity_digest == expected_file_identity_digest
            && self.is_executable_image() == expected_executable
            && !self.delete_pending
            && self
                .content_lease
                .matches(expected_file_identity_digest, expected_digest)
            && self.reopen_receipt_matches(
                expected_relative_path,
                expected_digest,
                expected_size_bytes,
                expected_root_identity_digest,
                expected_file_identity_digest,
                expected_executable,
            )
    }

    fn reopen_receipt_matches(
        &self,
        expected_relative_path: &str,
        expected_digest: &str,
        expected_size_bytes: u64,
        expected_root_identity_digest: &str,
        expected_file_identity_digest: &str,
        expected_executable: bool,
    ) -> bool {
        let receipt = &self.reopen_receipt;
        let expected_profile = if expected_executable {
            ManagedLoaderFileAccessProfile::ExecutableImage
        } else {
            ManagedLoaderFileAccessProfile::ReadOnlyPackageAsset
        };
        let final_name_matches = Path::new(expected_relative_path)
            .file_name()
            .is_some_and(|name| name == self.binding.relative_name());
        receipt.source_root_volume_serial == self.root_volume_serial
            && receipt.source_root_identity_digest == expected_root_identity_digest
            && receipt.source_identity.volume_serial == self.identity.volume_serial
            && receipt.source_identity.file_id == self.identity.file_id
            && receipt.source_identity.file_size == expected_size_bytes
            && !receipt.source_identity.is_directory
            && !receipt.source_identity.is_reparse_point
            && receipt.source_identity.number_of_links == 1
            && receipt.source_identity_digest == expected_file_identity_digest
            && receipt.source_binding.identity_digest() == expected_file_identity_digest
            && receipt.source_binding.parent_identity_digest()
                == self.binding.parent_identity_digest()
            && receipt.source_binding.relative_name() == self.binding.relative_name()
            && receipt.source_relative_path == expected_relative_path
            && receipt.source_expected_digest == expected_digest
            && receipt.source_expected_size_bytes == expected_size_bytes
            && receipt.source_access_profile == expected_profile
            && !receipt.source_delete_pending
            && receipt.source_content_lease_generation_digest
                == self.content_lease.lease_generation_digest
            && receipt.replacement_canonical_path_digest == self.canonical_path_digest
            && self.identity.volume_serial == self.root_volume_serial
            && !self.identity.is_directory
            && !self.identity.is_reparse_point
            && self.identity.number_of_links == 1
            && self.binding.identity_digest() == self.identity_digest
            && final_name_matches
            && self.path_receipt.matches(
                expected_root_identity_digest,
                expected_file_identity_digest,
                &self.canonical_path_digest,
            )
            && [
                &receipt.confirmed_close_receipt_digest,
                &receipt.parent_relative_open_receipt_digest,
                &receipt.replacement_canonical_path_digest,
                &receipt.comparison_receipt_digest,
            ]
            .iter()
            .all(|digest| is_lower_sha256(digest))
    }
}

fn is_lower_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|value| value.is_ascii_digit() || (b'a'..=b'f').contains(&value))
}

impl ManagedLoaderFileContentLease {
    fn matches(&self, expected_identity_digest: &str, expected_digest: &str) -> bool {
        self.file_identity_digest == expected_identity_digest
            && self.sealed_digest == expected_digest
            && self.writable_open_denied
            && self.existing_handle_write_denied
            && self.writable_mapping_denied
            && self.eof_allocation_metadata_mutation_denied
            && self.rename_link_mutation_denied
            && self.delete_disposition_denied
    }

    pub(crate) fn binding(&self) -> (&str, &str, &str, &str) {
        (
            &self.file_identity_digest,
            &self.sealed_digest,
            &self.lease_generation_digest,
            &self.immutable_content_policy_digest,
        )
    }
}

impl ManagedLoaderFileContentLeaseAuthenticatedNegativeReceipt {
    pub(crate) fn matches_attempt(
        &self,
        attempt: &ManagedLoaderFileContentLeaseAcquisitionAttemptCustody,
    ) -> bool {
        self.file_identity_digest == attempt.file_identity_digest
            && self.sealed_digest == attempt.sealed_digest
            && self.request_digest == attempt.request_digest
            && self.query_nonce_digest == attempt.query_nonce_digest
            && is_lower_sha256(&self.file_identity_digest)
            && is_lower_sha256(&self.sealed_digest)
            && is_lower_sha256(&self.request_digest)
            && is_lower_sha256(&self.query_nonce_digest)
            && is_lower_sha256(&self.negative_reason_digest)
            && is_lower_sha256(&self.receipt_digest)
            && is_lower_sha256(&self.authenticated_response_digest)
            && self.authenticated_response_digest
                == hex::encode(Sha256::digest(&self.authenticated_response))
            && !self.authenticated_response.is_empty()
    }
}

impl ManagedLoaderParentRelativeReopenAttemptCustody {
    pub(crate) fn binding(&self) -> (&str, &str, &str, &str) {
        (
            &self.anchor.identity_digest,
            &self.confirmed_close_receipt_digest,
            &self.request_digest,
            &self.query_nonce_digest,
        )
    }

    pub(crate) fn returned_positive_is_none(&self) -> bool {
        self._possibly_returned_handle.is_none()
    }
}

impl ManagedLoaderParentRelativeReopenAuthenticatedNegativeReceipt {
    pub(crate) fn matches_attempt(
        &self,
        attempt: &ManagedLoaderParentRelativeReopenAttemptCustody,
    ) -> bool {
        let (identity, close, request, nonce) = attempt.binding();
        self.source_identity_digest == identity
            && self.confirmed_close_receipt_digest == close
            && self.request_digest == request
            && self.query_nonce_digest == nonce
            && [
                &self.source_identity_digest,
                &self.confirmed_close_receipt_digest,
                &self.request_digest,
                &self.query_nonce_digest,
                &self.negative_reason_digest,
                &self.receipt_digest,
                &self.authenticated_response_digest,
            ]
            .iter()
            .all(|digest| is_lower_sha256(digest))
            && self.authenticated_response_digest
                == hex::encode(Sha256::digest(&self.authenticated_response))
            && !self.authenticated_response.is_empty()
    }
}

impl ManagedLoaderHandlePathReceipt {
    pub(super) fn matches(
        &self,
        expected_root_identity_digest: &str,
        expected_final_identity_digest: &str,
        expected_canonical_path_digest: &str,
    ) -> bool {
        self.root_identity_digest == expected_root_identity_digest
            && self.final_identity_digest == expected_final_identity_digest
            && self.canonical_path_digest == expected_canonical_path_digest
            && [
                &self.root_identity_digest,
                &self.final_identity_digest,
                &self.canonical_path_digest,
                &self.component_set_digest,
                &self.retained_parent_chain_share_contract_digest,
                &self.observation_receipt_digest,
            ]
            .iter()
            .all(|digest| is_lower_sha256(digest))
    }

    pub(super) fn binding(&self) -> (&str, &str, &str, &str, &str, &str) {
        (
            &self.root_identity_digest,
            &self.final_identity_digest,
            &self.canonical_path_digest,
            &self.component_set_digest,
            &self.retained_parent_chain_share_contract_digest,
            &self.observation_receipt_digest,
        )
    }
}

impl PinnedManagedLoaderDirectory {
    pub(crate) fn matches_sealed_identity(&self, expected_identity_digest: &str) -> bool {
        self.identity_digest == expected_identity_digest
    }

    /// Read-only launch material derived from the exact retained directory handle.
    pub(crate) fn handle_derived_canonical_path(&self) -> &Path {
        &self.canonical_path
    }

    pub(crate) fn matches_root_identity(&self, expected_root_identity_digest: &str) -> bool {
        self.root_identity_digest == expected_root_identity_digest
            && self._directory.root_identity_digest == expected_root_identity_digest
            && self.path_receipt.matches(
                expected_root_identity_digest,
                &self.identity_digest,
                &self.canonical_path_digest,
            )
    }

    pub(crate) fn matches_managed_relative_path(&self, expected_relative_path: &str) -> bool {
        self.managed_relative_path == expected_relative_path
    }

    pub(crate) fn canonical_path_digest(&self) -> &str {
        &self.canonical_path_digest
    }

    pub(crate) fn handle_path_binding(&self) -> (&str, &str, &str, &str, &str, &str) {
        self.path_receipt.binding()
    }
}

impl PinnedManagedLoaderFile {
    pub(crate) fn handle_path_binding(&self) -> (&str, &str, &str, &str, &str, &str) {
        self.path_receipt.binding()
    }

    pub(crate) fn loader_search_binding(&self) -> Option<(&str, &str)> {
        self.binding
            .relative_name()
            .to_str()
            .map(|name| (self.binding.parent_identity_digest(), name))
    }
}

impl ManagedLoaderNamespaceSession {
    pub(crate) fn binding(&self) -> (&str, u64, &str) {
        (
            &self.session_identity_digest,
            self.grant_generation,
            &self.generation_domain_digest,
        )
    }
}

impl ManagedLoaderSearchedNameGrant {
    pub(crate) fn matches_session(&self, session: &ManagedLoaderNamespaceSession) -> bool {
        Arc::ptr_eq(&self.owner, &session.owner)
            && self.session_identity_digest == session.session_identity_digest
            && self.grant_generation == session.grant_generation
            && self.generation_domain_digest == session.generation_domain_digest
    }

    pub(crate) fn binding(&self) -> (u64, &str, &str, &str, &str) {
        (
            self.grant_generation,
            &self.parent_directory_identity_digest,
            &self.normalized_name,
            &self.disposition_digest,
            &self.fence_generation_digest,
        )
    }
}

impl ManagedLoaderSearchedNameGrantAcquisitionAttemptCustody {
    pub(crate) fn matches_session(&self, session: &ManagedLoaderNamespaceSession) -> bool {
        Arc::ptr_eq(&self.owner, &session.owner)
            && self.session_identity_digest == session.session_identity_digest
    }

    pub(crate) fn request_binding(&self) -> (&str, &str) {
        (&self.request_digest, &self.query_nonce_digest)
    }
}

impl ManagedLoaderAuthenticatedNegativeReceipt {
    pub(crate) fn matches_query(
        &self,
        session: &ManagedLoaderNamespaceSession,
        request_digest: &str,
        query_nonce_digest: &str,
    ) -> bool {
        Arc::ptr_eq(&self.owner, &session.owner)
            && self.session_identity_digest == session.session_identity_digest
            && self.request_digest == request_digest
            && self.query_nonce_digest == query_nonce_digest
            && is_lower_sha256(&self.session_identity_digest)
            && is_lower_sha256(&self.request_digest)
            && is_lower_sha256(&self.query_nonce_digest)
            && is_lower_sha256(&self.negative_reason_digest)
            && is_lower_sha256(&self.receipt_digest)
            && is_lower_sha256(&self.authenticated_response_digest)
            && !self.authenticated_response.is_empty()
            && self.authenticated_response_digest
                == hex::encode(Sha256::digest(&self.authenticated_response))
    }
}

impl ManagedLoaderNamespaceQueryAttemptCustody {
    pub(crate) fn binding(&self) -> (&str, u64, &str, &str, &str, &str, &str) {
        (
            &self.session_identity_digest,
            self.grant_generation,
            &self.generation_domain_digest,
            &self.request_digest,
            &self.query_nonce_digest,
            &self.fence_generation_set_digest,
            &self.content_lease_generation_set_digest,
        )
    }

    pub(crate) fn matches_session(&self, session: &ManagedLoaderNamespaceSession) -> bool {
        Arc::ptr_eq(&self.owner, &session.owner)
            && self.session_identity_digest == session.session_identity_digest
            && self.grant_generation == session.grant_generation
            && self.generation_domain_digest == session.generation_domain_digest
    }
}

impl ManagedLoaderNamespaceQueryReceipt {
    pub(crate) fn authenticated_response_is_bound(&self) -> bool {
        !self.authenticated_response.is_empty()
            && is_lower_sha256(&self.authenticated_response_digest)
            && self.authenticated_response_digest
                == hex::encode(Sha256::digest(&self.authenticated_response))
    }

    pub(crate) fn binding(&self) -> (&str, u64, u64, &str, &str, &str, &str, &str, &str) {
        (
            &self.session_identity_digest,
            self.grant_generation,
            self.query_generation,
            &self.generation_domain_digest,
            &self.receipt_digest,
            &self.request_digest,
            &self.query_nonce_digest,
            &self.fence_generation_set_digest,
            &self.content_lease_generation_set_digest,
        )
    }

    pub(crate) fn matches_session(&self, session: &ManagedLoaderNamespaceSession) -> bool {
        Arc::ptr_eq(&self.owner, &session.owner)
            && self.session_identity_digest == session.session_identity_digest
            && self.grant_generation == session.grant_generation
            && self.generation_domain_digest == session.generation_domain_digest
    }
}
