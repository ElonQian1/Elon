use std::{ffi::OsStr, ffi::OsString, fmt};

mod lifecycle;

pub(super) use lifecycle::PlatformParentRelativeObservation;
pub(crate) use lifecycle::{
    ManagedDeleteDisposition, ManagedExpectedIdentityMatchPresence,
    ManagedNamespaceDurabilityFailure, ManagedNamespaceDurable, ManagedNamespaceObservationFailure,
    ManagedParentRelativeAbsence, ManagedParentRelativeIdentityConflict,
    ManagedParentRelativeObservation, QuarantinedManagedNamespaceObject,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ManagedObjectKind {
    File,
    Directory,
}

/// Immutable, handle-derived identity for one direct child of a pinned managed directory.
///
/// The relative name is the exact single component used by the parent-handle-relative open. The
/// two digests are derived from the opened object and parent handles; no canonical path string is
/// used as authority. This value is evidence only and grants no filesystem mutation capability.
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct ManagedObjectBinding {
    object_kind: ManagedObjectKind,
    relative_name: OsString,
    identity_digest: String,
    parent_identity_digest: String,
}

impl ManagedObjectBinding {
    pub(super) fn file(
        relative_name: &OsStr,
        identity_digest: String,
        parent_identity_digest: String,
    ) -> Self {
        Self::new(
            ManagedObjectKind::File,
            relative_name,
            identity_digest,
            parent_identity_digest,
        )
    }

    pub(super) fn directory(
        relative_name: &OsStr,
        identity_digest: String,
        parent_identity_digest: String,
    ) -> Self {
        Self::new(
            ManagedObjectKind::Directory,
            relative_name,
            identity_digest,
            parent_identity_digest,
        )
    }

    fn new(
        object_kind: ManagedObjectKind,
        relative_name: &OsStr,
        identity_digest: String,
        parent_identity_digest: String,
    ) -> Self {
        Self {
            object_kind,
            relative_name: relative_name.to_os_string(),
            identity_digest,
            parent_identity_digest,
        }
    }

    pub(crate) fn relative_name(&self) -> &OsStr {
        &self.relative_name
    }

    pub(crate) fn identity_digest(&self) -> &str {
        &self.identity_digest
    }

    pub(crate) fn parent_identity_digest(&self) -> &str {
        &self.parent_identity_digest
    }

    pub(crate) fn is_directory(&self) -> bool {
        self.object_kind == ManagedObjectKind::Directory
    }
}

impl fmt::Debug for ManagedObjectBinding {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ManagedObjectBinding")
            .field("object_kind", &self.object_kind)
            .field("relative_name", &"<redacted>")
            .field("identity_digest", &"<redacted>")
            .field("parent_identity_digest", &"<redacted>")
            .finish()
    }
}
