use std::{
    error::Error as StdError,
    fmt, io,
    num::{NonZeroU32, NonZeroU64, NonZeroU8},
    ptr::NonNull,
};

use super::super::PinnedManagedSqliteMainFile;
use super::coordinator::{PinnedManagedSqliteShmConnection, PinnedManagedSqliteWalMainFile};

pub(super) const SHM_LOCK_BASE: u64 = 120;
pub(super) const SHM_LOCK_COUNT: u8 = 8;
pub(super) const SHM_DMS_OFFSET: u64 = SHM_LOCK_BASE + SHM_LOCK_COUNT as u64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ManagedSqliteShmBudget {
    max_region_size: u32,
    max_regions: u32,
    max_logical_bytes: u64,
    max_mapped_bytes: u64,
}

impl ManagedSqliteShmBudget {
    pub(crate) const fn authority_default() -> Self {
        Self {
            max_region_size: 64 * 1024,
            max_regions: 256,
            max_logical_bytes: 8 * 1024 * 1024,
            max_mapped_bytes: 24 * 1024 * 1024,
        }
    }

    pub(super) fn validate_region_size(self, size: NonZeroU32) -> io::Result<()> {
        if size.get() > self.max_region_size {
            return Err(invalid("NODE_MANAGED_SQLITE_SHM_REGION_SIZE_BUDGET"));
        }
        Ok(())
    }

    pub(super) fn validate_logical_end(self, region: u32, size: NonZeroU32) -> io::Result<u64> {
        if region >= self.max_regions {
            return Err(invalid("NODE_MANAGED_SQLITE_SHM_REGION_COUNT_BUDGET"));
        }
        let end = u64::from(region)
            .checked_add(1)
            .and_then(|count| count.checked_mul(u64::from(size.get())))
            .ok_or_else(|| invalid("NODE_MANAGED_SQLITE_SHM_LOGICAL_END_OVERFLOW"))?;
        if end > self.max_logical_bytes || end > i64::MAX as u64 {
            return Err(invalid("NODE_MANAGED_SQLITE_SHM_LOGICAL_SIZE_BUDGET"));
        }
        Ok(end)
    }

    pub(super) fn validate_mapped_total(self, mapped: u64) -> io::Result<()> {
        if mapped > self.max_mapped_bytes {
            return Err(invalid("NODE_MANAGED_SQLITE_SHM_MAPPED_SIZE_BUDGET"));
        }
        Ok(())
    }

    pub(super) fn validate_existing_size(self, size: u64) -> io::Result<()> {
        if size > self.max_logical_bytes || size > i64::MAX as u64 {
            return Err(invalid("NODE_MANAGED_SQLITE_SHM_EXISTING_SIZE_BUDGET"));
        }
        Ok(())
    }
}

impl Default for ManagedSqliteShmBudget {
    fn default() -> Self {
        Self::authority_default()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ManagedSqliteShmMapMode {
    Observe,
    Extend,
}

pub(crate) struct ManagedSqliteShmRegionPointer {
    pointer: NonNull<u8>,
    length: usize,
    region: u32,
    runtime_generation: NonZeroU64,
}

impl ManagedSqliteShmRegionPointer {
    pub(super) fn new(
        pointer: NonNull<u8>,
        length: usize,
        region: u32,
        runtime_generation: NonZeroU64,
    ) -> Self {
        Self {
            pointer,
            length,
            region,
            runtime_generation,
        }
    }

    /// The VFS adapter may expose this address only while its SHM connection remains attached.
    pub(crate) unsafe fn as_mut_ptr(&self) -> *mut u8 {
        self.pointer.as_ptr()
    }

    pub(crate) fn length(&self) -> usize {
        self.length
    }

    pub(crate) fn region(&self) -> u32 {
        self.region
    }

    pub(super) fn belongs_to(&self, generation: NonZeroU64) -> bool {
        self.runtime_generation == generation
    }
}

impl fmt::Debug for ManagedSqliteShmRegionPointer {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ManagedSqliteShmRegionPointer")
            .field("pointer", &"<mapped>")
            .field("length", &self.length)
            .field("region", &self.region)
            .finish()
    }
}

#[derive(Debug)]
pub(crate) enum ManagedSqliteShmMapOutcome {
    NotPresent,
    Mapped(ManagedSqliteShmRegionPointer),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ManagedSqliteShmLockAction {
    LockShared,
    LockExclusive,
    UnlockShared,
    UnlockExclusive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ManagedSqliteShmLockRequest {
    first: u8,
    count: NonZeroU8,
    action: ManagedSqliteShmLockAction,
}

impl ManagedSqliteShmLockRequest {
    pub(crate) fn new(
        first: u8,
        count: NonZeroU8,
        action: ManagedSqliteShmLockAction,
    ) -> io::Result<Self> {
        let end = first
            .checked_add(count.get())
            .ok_or_else(|| invalid("NODE_MANAGED_SQLITE_SHM_LOCK_RANGE_OVERFLOW"))?;
        if end > SHM_LOCK_COUNT {
            return Err(invalid("NODE_MANAGED_SQLITE_SHM_LOCK_RANGE_INVALID"));
        }
        if matches!(
            action,
            ManagedSqliteShmLockAction::LockShared | ManagedSqliteShmLockAction::UnlockShared
        ) && count.get() != 1
        {
            return Err(invalid(
                "NODE_MANAGED_SQLITE_SHM_SHARED_LOCK_NOT_SINGLE_SLOT",
            ));
        }
        Ok(Self {
            first,
            count,
            action,
        })
    }

    pub(super) fn first(self) -> u8 {
        self.first
    }

    pub(super) fn count(self) -> u8 {
        self.count.get()
    }

    pub(super) fn action(self) -> ManagedSqliteShmLockAction {
        self.action
    }

    pub(super) fn mask(self) -> u8 {
        let low = 1u16 << self.first;
        let high = 1u16 << (self.first + self.count.get());
        (high - low) as u8
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ManagedSqliteShmLockAttempt {
    Acquired,
    Contended,
}

#[derive(Debug)]
pub(super) enum ManagedSqliteShmDeleteDisposition<'main> {
    Keep,
    Delete {
        main: &'main PinnedManagedSqliteMainFile,
        runtime_generation: NonZeroU64,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ManagedSqliteShmUnmapMode {
    Keep,
    Delete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ManagedSqliteShmFailurePhase {
    Gate,
    RequestValidation,
    ExactSiblingOpen,
    DmsExclusiveAcquire,
    DmsTruncate,
    DmsExclusiveRelease,
    DmsSharedAcquire,
    FileSize,
    FileGrow,
    MappingCreate,
    ViewMap,
    LockAcquire,
    LockRelease,
    ConnectionDetach,
    ViewUnmap,
    MappingClose,
    DmsSharedRelease,
    FileClose,
    DeleteAuthorization,
    ExactSiblingDelete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ManagedSqliteShmFailureClass {
    ProtocolViolation,
    BusyNoMutation,
    BusyAfterKnownMutation,
    NotPresent,
    IoBeforeMutation,
    MutatedButKnown,
    OutcomeUncertainPoisoned,
    PlatformUnsupported,
}

pub(crate) struct ManagedSqliteShmFailure {
    phase: ManagedSqliteShmFailurePhase,
    class: ManagedSqliteShmFailureClass,
    error: io::Error,
    mutation_may_have_occurred: bool,
    lock_outcome_uncertain: bool,
}

impl ManagedSqliteShmFailure {
    pub(super) fn new(
        phase: ManagedSqliteShmFailurePhase,
        class: ManagedSqliteShmFailureClass,
        error: io::Error,
    ) -> Self {
        Self {
            phase,
            class,
            error,
            mutation_may_have_occurred: matches!(
                class,
                ManagedSqliteShmFailureClass::MutatedButKnown
                    | ManagedSqliteShmFailureClass::BusyAfterKnownMutation
            ),
            lock_outcome_uncertain: false,
        }
    }

    pub(super) fn poisoned(
        phase: ManagedSqliteShmFailurePhase,
        error: io::Error,
        mutation_may_have_occurred: bool,
        lock_outcome_uncertain: bool,
    ) -> Self {
        Self {
            phase,
            class: ManagedSqliteShmFailureClass::OutcomeUncertainPoisoned,
            error,
            mutation_may_have_occurred,
            lock_outcome_uncertain,
        }
    }

    pub(super) fn poisoned_code(
        phase: ManagedSqliteShmFailurePhase,
        code: &'static str,
        mutation_may_have_occurred: bool,
        lock_outcome_uncertain: bool,
    ) -> Self {
        Self::poisoned(
            phase,
            io::Error::other(code),
            mutation_may_have_occurred,
            lock_outcome_uncertain,
        )
    }

    pub(super) fn code(
        phase: ManagedSqliteShmFailurePhase,
        class: ManagedSqliteShmFailureClass,
        code: &'static str,
    ) -> Self {
        Self::new(phase, class, io::Error::other(code))
    }

    pub(crate) fn phase(&self) -> ManagedSqliteShmFailurePhase {
        self.phase
    }

    pub(crate) fn class(&self) -> ManagedSqliteShmFailureClass {
        self.class
    }

    pub(crate) fn mutation_may_have_occurred(&self) -> bool {
        self.mutation_may_have_occurred
    }

    pub(crate) fn lock_outcome_uncertain(&self) -> bool {
        self.lock_outcome_uncertain
    }
}

impl fmt::Debug for ManagedSqliteShmFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ManagedSqliteShmFailure")
            .field("phase", &self.phase)
            .field("class", &self.class)
            .field("error_kind", &self.error.kind())
            .field(
                "mutation_may_have_occurred",
                &self.mutation_may_have_occurred,
            )
            .field("lock_outcome_uncertain", &self.lock_outcome_uncertain)
            .finish()
    }
}

impl fmt::Display for ManagedSqliteShmFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("NODE_MANAGED_SQLITE_SHM_OPERATION_FAILED")
    }
}

impl StdError for ManagedSqliteShmFailure {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        Some(&self.error)
    }
}

#[must_use = "failed SHM unmap retains the connection and coordinator custody"]
pub(super) struct ManagedSqliteShmUnmapFailure {
    pub(super) failure: ManagedSqliteShmFailure,
    pub(super) connection: PinnedManagedSqliteShmConnection,
}

impl ManagedSqliteShmUnmapFailure {
    pub(crate) fn failure(&self) -> &ManagedSqliteShmFailure {
        &self.failure
    }
}

impl fmt::Debug for ManagedSqliteShmUnmapFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ManagedSqliteShmUnmapFailure")
            .field("failure", &self.failure)
            .field("connection", &"<retained>")
            .finish()
    }
}

impl fmt::Display for ManagedSqliteShmUnmapFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.failure.fmt(formatter)
    }
}

impl StdError for ManagedSqliteShmUnmapFailure {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        Some(&self.failure)
    }
}

#[must_use = "failed WAL-main unmap retains the main file and exact SHM connection custody"]
pub(crate) struct ManagedSqliteWalMainUnmapFailure {
    pub(super) failure: ManagedSqliteShmFailure,
    pub(super) wal_main: PinnedManagedSqliteWalMainFile,
}

impl ManagedSqliteWalMainUnmapFailure {
    pub(crate) fn failure(&self) -> &ManagedSqliteShmFailure {
        &self.failure
    }

    pub(crate) fn into_parts(self) -> (ManagedSqliteShmFailure, PinnedManagedSqliteWalMainFile) {
        (self.failure, self.wal_main)
    }
}

impl fmt::Debug for ManagedSqliteWalMainUnmapFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ManagedSqliteWalMainUnmapFailure")
            .field("failure", &self.failure)
            .field("wal_main", &"<retained>")
            .finish()
    }
}

impl fmt::Display for ManagedSqliteWalMainUnmapFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.failure.fmt(formatter)
    }
}

impl StdError for ManagedSqliteWalMainUnmapFailure {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        Some(&self.failure)
    }
}

fn invalid(code: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, code)
}
