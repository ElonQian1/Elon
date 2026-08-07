use std::{error::Error as StdError, fmt, fs::File, sync::Arc, time::Instant};

use super::super::{identity_digest, managed_parent_identity_digest, platform};
use super::ManagedObjectBinding;

pub(crate) enum PlatformParentRelativeObservation {
    Absent,
    Present(File),
}

struct ManagedParentNamespaceCustody {
    binding: ManagedObjectBinding,
    root_volume_serial: u64,
    root_identity_digest: String,
    parent_handles: Vec<Arc<File>>,
}

impl ManagedParentNamespaceCustody {
    fn parent_handle(&self) -> std::io::Result<&File> {
        self.parent_handles
            .last()
            .map(Arc::as_ref)
            .ok_or_else(|| std::io::Error::other("NODE_MANAGED_NAMESPACE_PARENT_HANDLE_MISSING"))
    }

    fn validate_parent_identity(&self) -> std::io::Result<()> {
        let actual = managed_parent_identity_digest(
            &self.root_identity_digest,
            self.parent_handle()?,
            self.root_volume_serial,
        )
        .map_err(|error| std::io::Error::other(error.to_string()))?;
        if actual != self.binding.parent_identity_digest() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "NODE_MANAGED_NAMESPACE_PARENT_IDENTITY_CHANGED",
            ));
        }
        Ok(())
    }
}

/// A delete disposition accepted for one exact object handle. The target handle has been closed,
/// but this value deliberately retains the exact parent handle and does not claim name absence.
#[must_use = "delete disposition must be observed relative to its retained exact parent handle"]
pub(crate) struct ManagedDeleteDisposition {
    custody: ManagedParentNamespaceCustody,
}

impl ManagedDeleteDisposition {
    pub(in crate::node_agent_managed_fs) fn new(
        binding: ManagedObjectBinding,
        root_volume_serial: u64,
        root_identity_digest: String,
        parent_handles: Vec<Arc<File>>,
    ) -> Self {
        Self {
            custody: ManagedParentNamespaceCustody {
                binding,
                root_volume_serial,
                root_identity_digest,
                parent_handles,
            },
        }
    }

    pub(crate) fn identity_digest(&self) -> Option<&str> {
        Some(self.custody.binding.identity_digest())
    }

    pub(crate) fn is_directory(&self) -> bool {
        self.custody.binding.is_directory()
    }

    pub(crate) fn object_binding(&self) -> &ManagedObjectBinding {
        &self.custody.binding
    }

    pub(crate) fn observe_parent_relative(
        self,
    ) -> Result<ManagedParentRelativeObservation, ManagedNamespaceObservationFailure> {
        if let Err(error) = self.custody.validate_parent_identity() {
            return Err(ManagedNamespaceObservationFailure::new(error, self, None));
        }
        let parent = match self.custody.parent_handle() {
            Ok(parent) => parent,
            Err(error) => {
                return Err(ManagedNamespaceObservationFailure::new(error, self, None));
            }
        };
        let observation =
            match platform::observe_child_relative(parent, self.custody.binding.relative_name()) {
                Ok(observation) => observation,
                Err(error) => {
                    return Err(ManagedNamespaceObservationFailure::new(error, self, None));
                }
            };
        match observation {
            PlatformParentRelativeObservation::Absent => {
                if let Err(error) = self.custody.validate_parent_identity() {
                    return Err(ManagedNamespaceObservationFailure::new(error, self, None));
                }
                Ok(ManagedParentRelativeObservation::Absent(
                    ManagedParentRelativeAbsence {
                        custody: self.custody,
                    },
                ))
            }
            PlatformParentRelativeObservation::Present(observed_object) => {
                let observed_identity = match platform::inspect(&observed_object) {
                    Ok(identity) => identity,
                    Err(error) => {
                        return Err(ManagedNamespaceObservationFailure::new(
                            error,
                            self,
                            Some(observed_object),
                        ));
                    }
                };
                if let Err(error) = self.custody.validate_parent_identity() {
                    return Err(ManagedNamespaceObservationFailure::new(
                        error,
                        self,
                        Some(observed_object),
                    ));
                }
                let observed_identity_digest =
                    identity_digest(&self.custody.root_identity_digest, None, observed_identity);
                let is_expected = observed_identity_digest
                    == self.custody.binding.identity_digest()
                    && observed_identity.is_directory == self.custody.binding.is_directory();
                if is_expected {
                    Ok(ManagedParentRelativeObservation::ExpectedIdentityMatch(
                        ManagedExpectedIdentityMatchPresence {
                            custody: self.custody,
                            _observed_object: observed_object,
                            observed_identity_digest,
                        },
                    ))
                } else {
                    Ok(ManagedParentRelativeObservation::IdentityConflict(
                        ManagedParentRelativeIdentityConflict {
                            custody: self.custody,
                            _observed_object: observed_object,
                            observed_identity_digest,
                        },
                    ))
                }
            }
        }
    }
}

impl fmt::Debug for ManagedDeleteDisposition {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ManagedDeleteDisposition")
            .field("object_kind", &self.custody.binding.object_kind)
            .field("parent_handle", &"<retained>")
            .finish()
    }
}

#[must_use = "namespace observation must be committed, retried, or retained for recovery"]
pub(crate) enum ManagedParentRelativeObservation {
    Absent(ManagedParentRelativeAbsence),
    ExpectedIdentityMatch(ManagedExpectedIdentityMatchPresence),
    IdentityConflict(ManagedParentRelativeIdentityConflict),
}

impl fmt::Debug for ManagedParentRelativeObservation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let outcome = match self {
            Self::Absent(_) => "absent",
            Self::ExpectedIdentityMatch(_) => "expected_identity_match",
            Self::IdentityConflict(_) => "identity_conflict",
        };
        formatter
            .debug_struct("ManagedParentRelativeObservation")
            .field("outcome", &outcome)
            .finish()
    }
}

/// Parent-handle-relative proof that the exact original name was absent after target close.
#[must_use = "absence must be committed before attempting a namespace durability barrier"]
pub(crate) struct ManagedParentRelativeAbsence {
    custody: ManagedParentNamespaceCustody,
}

impl ManagedParentRelativeAbsence {
    pub(crate) fn object_binding(&self) -> &ManagedObjectBinding {
        &self.custody.binding
    }

    pub(crate) fn into_disposition(self) -> ManagedDeleteDisposition {
        ManagedDeleteDisposition {
            custody: self.custody,
        }
    }

    /// The Windows durability primitive and its pre/post absence checks are intentionally not
    /// available yet. Returning the linear absence in the error prevents callers from minting a
    /// `namespace_durable` event from disposition or absence alone.
    pub(crate) fn make_namespace_durable(
        self,
    ) -> Result<ManagedNamespaceDurable, ManagedNamespaceDurabilityFailure> {
        Err(ManagedNamespaceDurabilityFailure {
            error: std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                "NODE_MANAGED_NAMESPACE_DURABILITY_UNAVAILABLE",
            ),
            absence: self,
        })
    }
}

impl fmt::Debug for ManagedParentRelativeAbsence {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ManagedParentRelativeAbsence")
            .field("object_kind", &self.custody.binding.object_kind)
            .field("parent_handle", &"<retained>")
            .finish()
    }
}

/// A live same-name object observed after delete disposition. It is retained so a replacement is
/// never silently converted into absence or reopened by path.
#[must_use = "same-name presence must be committed or retained for operator recovery"]
pub(crate) struct ManagedExpectedIdentityMatchPresence {
    custody: ManagedParentNamespaceCustody,
    _observed_object: File,
    observed_identity_digest: String,
}

impl ManagedExpectedIdentityMatchPresence {
    pub(crate) fn expected_binding(&self) -> &ManagedObjectBinding {
        &self.custody.binding
    }

    pub(crate) fn observed_identity_digest(&self) -> &str {
        &self.observed_identity_digest
    }
}

impl fmt::Debug for ManagedExpectedIdentityMatchPresence {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ManagedExpectedIdentityMatchPresence")
            .field("expected_identity_digest", &"<redacted>")
            .field("observed_identity_digest", &"<redacted>")
            .field("observed_object", &"<retained>")
            .finish()
    }
}

/// A same-name replacement with a different handle-derived identity. This is a permanent
/// fail-closed outcome and exposes no transition back to disposition or absence observation.
#[must_use = "identity conflict must be retained for recovery or explicit operator resolution"]
pub(crate) struct ManagedParentRelativeIdentityConflict {
    custody: ManagedParentNamespaceCustody,
    _observed_object: File,
    observed_identity_digest: String,
}

impl ManagedParentRelativeIdentityConflict {
    pub(crate) fn expected_binding(&self) -> &ManagedObjectBinding {
        &self.custody.binding
    }

    pub(crate) fn observed_identity_digest(&self) -> &str {
        &self.observed_identity_digest
    }
}

impl fmt::Debug for ManagedParentRelativeIdentityConflict {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ManagedParentRelativeIdentityConflict")
            .field("expected_identity_digest", &"<redacted>")
            .field("observed_identity_digest", &"<redacted>")
            .field("observed_object", &"<retained>")
            .finish()
    }
}

#[must_use = "failed observation retains disposition and any opened same-name object"]
pub(crate) struct ManagedNamespaceObservationFailure {
    error: std::io::Error,
    disposition: ManagedDeleteDisposition,
    observed_object: Option<QuarantinedManagedNamespaceObject>,
}

impl ManagedNamespaceObservationFailure {
    fn new(
        error: std::io::Error,
        disposition: ManagedDeleteDisposition,
        observed_object: Option<File>,
    ) -> Self {
        Self {
            error,
            disposition,
            observed_object: observed_object
                .map(|file| QuarantinedManagedNamespaceObject { _file: file }),
        }
    }

    pub(crate) fn into_parts(
        self,
    ) -> (
        std::io::Error,
        ManagedDeleteDisposition,
        Option<QuarantinedManagedNamespaceObject>,
    ) {
        (self.error, self.disposition, self.observed_object)
    }
}

/// An object handle opened during an inconclusive namespace observation. No operation is exposed;
/// retaining it prevents a failed inspection from becoming an implicit path reopen.
pub(crate) struct QuarantinedManagedNamespaceObject {
    _file: File,
}

impl fmt::Debug for QuarantinedManagedNamespaceObject {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("QuarantinedManagedNamespaceObject")
            .field("file", &"<retained>")
            .finish()
    }
}

impl fmt::Debug for ManagedNamespaceObservationFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ManagedNamespaceObservationFailure")
            .field("error_kind", &self.error.kind())
            .field("raw_os_error", &self.error.raw_os_error())
            .field("disposition", &"<retained>")
            .field(
                "observed_object",
                &self.observed_object.as_ref().map(|_| "<retained>"),
            )
            .finish()
    }
}

impl fmt::Display for ManagedNamespaceObservationFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "NODE_MANAGED_NAMESPACE_OBSERVATION_FAILED: {}",
            self.error
        )
    }
}

impl StdError for ManagedNamespaceObservationFailure {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        Some(&self.error)
    }
}

/// Reserved success capability for a future verified namespace barrier. The completion instant is
/// mandatory so Store binding can require a strictly later authenticated trusted-time witness.
#[must_use = "durable namespace evidence must be bound to a later trusted-time observation"]
pub(crate) struct ManagedNamespaceDurable {
    _custody: ManagedParentNamespaceCustody,
    completed_at: Instant,
}

impl ManagedNamespaceDurable {
    pub(crate) fn completed_at(&self) -> Instant {
        self.completed_at
    }
}

impl fmt::Debug for ManagedNamespaceDurable {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ManagedNamespaceDurable")
            .field("parent_handle", &"<retained>")
            .field("completed_at", &self.completed_at)
            .finish()
    }
}

#[must_use = "failed durability retains the exact parent-relative absence capability"]
pub(crate) struct ManagedNamespaceDurabilityFailure {
    error: std::io::Error,
    absence: ManagedParentRelativeAbsence,
}

impl ManagedNamespaceDurabilityFailure {
    pub(crate) fn into_parts(self) -> (std::io::Error, ManagedParentRelativeAbsence) {
        (self.error, self.absence)
    }
}

impl fmt::Debug for ManagedNamespaceDurabilityFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ManagedNamespaceDurabilityFailure")
            .field("error_kind", &self.error.kind())
            .field("absence", &"<retained>")
            .finish()
    }
}

impl fmt::Display for ManagedNamespaceDurabilityFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "NODE_MANAGED_NAMESPACE_DURABILITY_FAILED: {}",
            self.error
        )
    }
}

impl StdError for ManagedNamespaceDurabilityFailure {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        Some(&self.error)
    }
}
