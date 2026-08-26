use std::fmt;

use super::loader::*;

impl fmt::Debug for ManagedLoaderFileIdentityAnchor {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ManagedLoaderFileIdentityAnchor")
            .field("relative_path", &"<redacted>")
            .field("identity_digest", &"<redacted>")
            .field("access_profile", &"<sealed>")
            .field("delete_pending", &self.delete_pending)
            .field("expected_digest", &"<redacted>")
            .field("expected_size_bytes", &self.expected_size_bytes)
            .finish()
    }
}

impl fmt::Debug for ManagedLoaderFileContentLease {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ManagedLoaderFileContentLease")
            .field("guard", &"<retained-kernel-content-lease>")
            .field("lease_generation_digest", &"<redacted>")
            .field("writable_open_denied", &self.writable_open_denied)
            .field(
                "existing_handle_write_denied",
                &self.existing_handle_write_denied,
            )
            .field("writable_mapping_denied", &self.writable_mapping_denied)
            .finish()
    }
}

impl fmt::Debug for ManagedLoaderFileContentLeaseAcquisitionAttemptCustody {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ManagedLoaderFileContentLeaseAcquisitionAttemptCustody")
            .field("session", &"<retained-kernel-content-guard-session>")
            .field("file_identity_digest", &"<redacted>")
            .field("request_digest", &"<redacted>")
            .field("response_bytes", &self.response_buffer.len())
            .finish()
    }
}

impl fmt::Debug for ManagedLoaderFileReopenReceipt {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ManagedLoaderFileReopenReceipt")
            .field("source_identity_digest", &"<redacted>")
            .field("source_relative_path", &"<redacted>")
            .field("comparison_receipt_digest", &"<redacted>")
            .finish()
    }
}

impl fmt::Debug for ManagedLoaderHandlePathReceipt {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ManagedLoaderHandlePathReceipt")
            .field("component_set_digest", &"<redacted>")
            .field("parent_chain_share_contract", &"<sealed-uninhabited>")
            .finish()
    }
}

impl fmt::Debug for PinnedManagedLoaderFile {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PinnedManagedLoaderFile")
            .field("file", &"<retained-loader-compatible>")
            .field("identity_digest", &"<redacted>")
            .field("digest", &"<redacted>")
            .field("canonical_path", &"<redacted-handle-derived>")
            .field("delete_pending", &self.delete_pending)
            .finish()
    }
}

impl fmt::Debug for PinnedManagedLoaderDirectory {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PinnedManagedLoaderDirectory")
            .field("directory", &"<retained>")
            .field("identity_digest", &"<redacted>")
            .field("canonical_path", &"<redacted-handle-derived>")
            .finish()
    }
}

impl fmt::Debug for PinnedWindowsLoaderSearchDirectory {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PinnedWindowsLoaderSearchDirectory")
            .field("directory", &"<retained-external-search-directory>")
            .field("identity_digest", &"<redacted>")
            .field("canonical_path", &"<redacted-handle-derived>")
            .finish()
    }
}

impl fmt::Debug for PinnedWindowsLoaderSystemImageFile {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PinnedWindowsLoaderSystemImageFile")
            .field("file", &"<retained-parent-relative-system-image>")
            .field("parent_directory_identity_digest", &"<redacted>")
            .field("normalized_name", &"<redacted>")
            .field("immutable_section_identity_digest", &"<redacted>")
            .finish()
    }
}

impl fmt::Debug for ManagedLoaderSystemImageContentLease {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ManagedLoaderSystemImageContentLease")
            .field("guard", &"<retained-kernel-system-content-lease>")
            .field("lease_generation_digest", &"<redacted>")
            .field("writable_open_denied", &self.writable_open_denied)
            .field("writable_mapping_denied", &self.writable_mapping_denied)
            .finish()
    }
}

impl fmt::Debug for ManagedLoaderNamespaceSession {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ManagedLoaderNamespaceSession")
            .field("driver_session", &"<retained-authenticated-session>")
            .field("grant_generation", &self.grant_generation)
            .finish()
    }
}

impl fmt::Debug for ManagedLoaderSearchedNameGrant {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ManagedLoaderSearchedNameGrant")
            .field("session", &"<shared-authenticated-session>")
            .field("grant_generation", &self.grant_generation)
            .field("normalized_name", &"<redacted>")
            .field("fence_generation_digest", &"<redacted>")
            .finish()
    }
}

impl fmt::Debug for ManagedLoaderSearchedNameGrantAcquisitionAttemptCustody {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ManagedLoaderSearchedNameGrantAcquisitionAttemptCustody")
            .field("session", &"<shared-authenticated-session>")
            .field("request_digest", &"<redacted>")
            .field("response_bytes", &self.response_buffer.len())
            .finish()
    }
}

impl fmt::Debug for ManagedLoaderAuthenticatedNegativeReceipt {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ManagedLoaderAuthenticatedNegativeReceipt")
            .field("receipt_digest", &"<redacted>")
            .field("negative_reason_digest", &"<redacted>")
            .finish()
    }
}

impl fmt::Debug for ManagedLoaderNamespaceQueryAttemptCustody {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ManagedLoaderNamespaceQueryAttemptCustody")
            .field("driver_session", &"<retained-query-attempt-session>")
            .field("request_digest", &"<redacted>")
            .field("grant_generation", &self.grant_generation)
            .field("query_nonce_digest", &"<redacted>")
            .field("response_bytes", &self.response_buffer.len())
            .finish()
    }
}

impl fmt::Debug for ManagedLoaderNamespaceQueryReceipt {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ManagedLoaderNamespaceQueryReceipt")
            .field("receipt_digest", &"<redacted>")
            .field("grant_generation", &self.grant_generation)
            .field("query_generation", &self.query_generation)
            .field("response_bytes", &self.authenticated_response.len())
            .finish()
    }
}

impl fmt::Debug for QuarantinedManagedLoaderFile {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("QuarantinedManagedLoaderFile")
            .field("file", &"<retained-rejected>")
            .field("anchor", &self.anchor)
            .finish()
    }
}

impl fmt::Debug for QuarantinedManagedLoaderSourceClose {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("QuarantinedManagedLoaderSourceClose")
            .field("source", &"<close-outcome-uncertain-no-drop>")
            .field("anchor", &self.anchor)
            .finish()
    }
}

impl fmt::Debug for ManagedLoaderParentRelativeReopenAttemptCustody {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ManagedLoaderParentRelativeReopenAttemptCustody")
            .field("anchor", &self.anchor)
            .field(
                "possibly_returned_handle",
                &self._possibly_returned_handle.is_some(),
            )
            .field("request_digest", &"<redacted>")
            .field("response_bytes", &self.response_buffer.len())
            .finish()
    }
}
