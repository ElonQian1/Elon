#![allow(dead_code)]

use std::path::Path;

use super::loader::{
    ManagedLoaderSystemImageContentLease, PinnedWindowsLoaderSearchDirectory,
    PinnedWindowsLoaderSystemImageFile,
};

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
