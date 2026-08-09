use std::fs::File;

use super::main::{invalid_lock_failure as invalid, platform_lock_failure as platform_failure};
use super::{
    lock_domain::ManagedSqliteLockDomainGuard, platform, ManagedSqliteLockAttempt,
    ManagedSqliteLockFailure, ManagedSqliteLockFailureKind, ManagedSqliteLockFailurePhase,
    ManagedSqliteObservedLock, ManagedSqliteRequestedLock, ManagedSqliteUnlockTarget,
    PinnedManagedSqliteMainFile, PlatformManagedSqliteLockAttempt,
};

const PENDING_BYTE: u64 = 0x4000_0000;
const RESERVED_BYTE: u64 = PENDING_BYTE + 1;
const SHARED_FIRST: u64 = PENDING_BYTE + 2;
const SHARED_SIZE: u64 = 510;

impl PinnedManagedSqliteMainFile {
    pub(crate) fn lock_level(
        &self,
    ) -> std::result::Result<ManagedSqliteObservedLock, ManagedSqliteLockFailure> {
        let domain = self.live_lock_domain()?;
        Ok(domain.held().level())
    }

    pub(crate) fn lock_to(
        &mut self,
        requested: ManagedSqliteRequestedLock,
    ) -> std::result::Result<ManagedSqliteLockAttempt, ManagedSqliteLockFailure> {
        let read_write = self.is_read_write();
        let file = &self.file.file;
        let mut domain = self.live_lock_domain()?;
        if requested != ManagedSqliteRequestedLock::Shared && !read_write {
            return Err(ManagedSqliteLockFailure::message(
                ManagedSqliteLockFailurePhase::RequestValidation,
                ManagedSqliteLockFailureKind::ReadOnly,
                "NODE_MANAGED_SQLITE_WRITE_LOCK_ON_READ_ONLY_FILE",
                false,
            ));
        }
        if domain.held().level().rank() >= requested.rank() {
            return Ok(ManagedSqliteLockAttempt::Acquired);
        }
        match requested {
            ManagedSqliteRequestedLock::Shared => acquire_shared(file, &mut domain),
            ManagedSqliteRequestedLock::Reserved => acquire_reserved(file, &mut domain),
            ManagedSqliteRequestedLock::Exclusive => acquire_exclusive(file, &mut domain),
        }
    }

    pub(crate) fn unlock_to(
        &mut self,
        target: ManagedSqliteUnlockTarget,
    ) -> std::result::Result<(), ManagedSqliteLockFailure> {
        let file = &self.file.file;
        let mut domain = self.live_lock_domain()?;
        match target {
            ManagedSqliteUnlockTarget::None => unlock_all(file, &mut domain),
            ManagedSqliteUnlockTarget::Shared => unlock_to_shared(file, &mut domain),
        }
    }

    pub(crate) fn check_reserved_lock(
        &mut self,
    ) -> std::result::Result<bool, ManagedSqliteLockFailure> {
        let file = &self.file.file;
        let mut domain = self.live_lock_domain()?;
        if domain.held().reserved
            || domain.held().pending
            || domain.held().exclusive
            || domain.another_owner_is_writer()
        {
            return Ok(true);
        }
        // A shared probe conflicts with a real RESERVED exclusive lock, but not with another
        // simultaneous probe in a different process. This avoids probe-vs-probe false positives.
        match try_lock(file, RESERVED_BYTE, 1, false) {
            Ok(PlatformManagedSqliteLockAttempt::Contended) => Ok(true),
            Ok(PlatformManagedSqliteLockAttempt::Acquired) => {
                if let Err(error) = unlock(file, RESERVED_BYTE, 1) {
                    return Err(poison(
                        &mut domain,
                        ManagedSqliteLockFailurePhase::ReservedProbeRelease,
                        error,
                    ));
                }
                Ok(false)
            }
            Err(error) => Err(platform_failure(
                ManagedSqliteLockFailurePhase::ReservedProbe,
                error,
            )),
        }
    }

    pub(super) fn release_locks_for_drop(&mut self) {
        if let Ok(mut domain) = self.lock_owner.lock() {
            if domain.is_terminal() {
                return;
            }
            let _ = unlock_all(&self.file.file, &mut domain);
        }
    }
}

fn acquire_shared(
    file: &File,
    domain: &mut ManagedSqliteLockDomainGuard<'_>,
) -> std::result::Result<ManagedSqliteLockAttempt, ManagedSqliteLockFailure> {
    if domain.held().level() != ManagedSqliteObservedLock::None {
        return Err(invalid("NODE_MANAGED_SQLITE_SHARED_LOCK_SEQUENCE_INVALID"));
    }
    if domain.another_owner_has_pending_or_exclusive() {
        return Ok(ManagedSqliteLockAttempt::Contended);
    }
    match try_lock(file, PENDING_BYTE, 1, true) {
        Ok(PlatformManagedSqliteLockAttempt::Contended) => {
            return Ok(ManagedSqliteLockAttempt::Contended);
        }
        Ok(PlatformManagedSqliteLockAttempt::Acquired) => domain.held_mut().pending = true,
        Err(error) => {
            return Err(platform_failure(
                ManagedSqliteLockFailurePhase::AcquirePending,
                error,
            ));
        }
    }
    let shared = try_lock(file, SHARED_FIRST, SHARED_SIZE, false);
    if matches!(&shared, Ok(PlatformManagedSqliteLockAttempt::Acquired)) {
        domain.held_mut().shared = true;
    }
    if let Err(error) = unlock(file, PENDING_BYTE, 1) {
        return Err(poison(
            domain,
            ManagedSqliteLockFailurePhase::ReleaseTemporaryPending,
            error,
        ));
    }
    domain.held_mut().pending = false;
    match shared {
        Ok(PlatformManagedSqliteLockAttempt::Acquired) => Ok(ManagedSqliteLockAttempt::Acquired),
        Ok(PlatformManagedSqliteLockAttempt::Contended) => Ok(ManagedSqliteLockAttempt::Contended),
        Err(error) => Err(platform_failure(
            ManagedSqliteLockFailurePhase::AcquireShared,
            error,
        )),
    }
}

fn acquire_reserved(
    file: &File,
    domain: &mut ManagedSqliteLockDomainGuard<'_>,
) -> std::result::Result<ManagedSqliteLockAttempt, ManagedSqliteLockFailure> {
    if domain.held().level() != ManagedSqliteObservedLock::Shared {
        return Err(invalid(
            "NODE_MANAGED_SQLITE_RESERVED_LOCK_SEQUENCE_INVALID",
        ));
    }
    if domain.another_owner_is_writer() {
        return Ok(ManagedSqliteLockAttempt::Contended);
    }
    match try_lock(file, RESERVED_BYTE, 1, true) {
        Ok(PlatformManagedSqliteLockAttempt::Acquired) => {
            domain.held_mut().reserved = true;
            Ok(ManagedSqliteLockAttempt::Acquired)
        }
        Ok(PlatformManagedSqliteLockAttempt::Contended) => Ok(ManagedSqliteLockAttempt::Contended),
        Err(error) => Err(platform_failure(
            ManagedSqliteLockFailurePhase::AcquireReserved,
            error,
        )),
    }
}

fn acquire_exclusive(
    file: &File,
    domain: &mut ManagedSqliteLockDomainGuard<'_>,
) -> std::result::Result<ManagedSqliteLockAttempt, ManagedSqliteLockFailure> {
    if !matches!(
        domain.held().level(),
        ManagedSqliteObservedLock::Shared
            | ManagedSqliteObservedLock::Reserved
            | ManagedSqliteObservedLock::Pending
    ) || !domain.held().shared
    {
        return Err(invalid(
            "NODE_MANAGED_SQLITE_EXCLUSIVE_LOCK_SEQUENCE_INVALID",
        ));
    }
    let reserved_acquired_here = if domain.held().reserved {
        false
    } else {
        if domain.another_owner_is_writer() {
            return Ok(ManagedSqliteLockAttempt::Contended);
        }
        match try_lock(file, RESERVED_BYTE, 1, true) {
            Ok(PlatformManagedSqliteLockAttempt::Acquired) => {
                domain.held_mut().reserved = true;
                true
            }
            Ok(PlatformManagedSqliteLockAttempt::Contended) => {
                return Ok(ManagedSqliteLockAttempt::Contended);
            }
            Err(error) => {
                return Err(platform_failure(
                    ManagedSqliteLockFailurePhase::AcquireReserved,
                    error,
                ));
            }
        }
    };
    if !domain.held().pending && domain.another_owner_has_pending_or_exclusive() {
        release_direct_reservation(file, domain, reserved_acquired_here)?;
        return Ok(ManagedSqliteLockAttempt::Contended);
    }
    if !domain.held().pending {
        match try_lock(file, PENDING_BYTE, 1, true) {
            Ok(PlatformManagedSqliteLockAttempt::Contended) => {
                release_direct_reservation(file, domain, reserved_acquired_here)?;
                return Ok(ManagedSqliteLockAttempt::Contended);
            }
            Ok(PlatformManagedSqliteLockAttempt::Acquired) => domain.held_mut().pending = true,
            Err(error) => {
                release_direct_reservation(file, domain, reserved_acquired_here)?;
                return Err(platform_failure(
                    ManagedSqliteLockFailurePhase::AcquirePending,
                    error,
                ));
            }
        }
    }
    if domain.another_owner_holds_lock() {
        return Ok(ManagedSqliteLockAttempt::Contended);
    }
    if let Err(error) = unlock(file, SHARED_FIRST, SHARED_SIZE) {
        return Err(poison(
            domain,
            ManagedSqliteLockFailurePhase::ReleaseSharedForExclusive,
            error,
        ));
    }
    domain.held_mut().shared = false;
    match try_lock(file, SHARED_FIRST, SHARED_SIZE, true) {
        Ok(PlatformManagedSqliteLockAttempt::Acquired) => {
            domain.held_mut().exclusive = true;
            Ok(ManagedSqliteLockAttempt::Acquired)
        }
        Ok(PlatformManagedSqliteLockAttempt::Contended) => {
            restore_shared(file, domain)?;
            Ok(ManagedSqliteLockAttempt::Contended)
        }
        Err(error) => {
            restore_shared(file, domain)?;
            Err(platform_failure(
                ManagedSqliteLockFailurePhase::AcquireExclusive,
                error,
            ))
        }
    }
}

fn release_direct_reservation(
    file: &File,
    domain: &mut ManagedSqliteLockDomainGuard<'_>,
    acquired_here: bool,
) -> std::result::Result<(), ManagedSqliteLockFailure> {
    if !acquired_here {
        return Ok(());
    }
    release(
        file,
        domain,
        RESERVED_BYTE,
        1,
        ManagedSqliteLockFailurePhase::ReleaseReserved,
    )?;
    domain.held_mut().reserved = false;
    Ok(())
}

fn unlock_to_shared(
    file: &File,
    domain: &mut ManagedSqliteLockDomainGuard<'_>,
) -> std::result::Result<(), ManagedSqliteLockFailure> {
    if matches!(
        domain.held().level(),
        ManagedSqliteObservedLock::None | ManagedSqliteObservedLock::Shared
    ) {
        return Ok(());
    }
    if domain.held().exclusive {
        release(
            file,
            domain,
            SHARED_FIRST,
            SHARED_SIZE,
            ManagedSqliteLockFailurePhase::ReleaseExclusive,
        )?;
        domain.held_mut().exclusive = false;
        restore_shared(file, domain)?;
    }
    if domain.held().reserved {
        release(
            file,
            domain,
            RESERVED_BYTE,
            1,
            ManagedSqliteLockFailurePhase::ReleaseReserved,
        )?;
        domain.held_mut().reserved = false;
    }
    if domain.held().pending {
        release(
            file,
            domain,
            PENDING_BYTE,
            1,
            ManagedSqliteLockFailurePhase::ReleasePending,
        )?;
        domain.held_mut().pending = false;
    }
    if !domain.held().shared {
        return Err(poison_message(
            domain,
            ManagedSqliteLockFailurePhase::RestoreShared,
            "NODE_MANAGED_SQLITE_SHARED_LOCK_NOT_PROVEN",
        ));
    }
    Ok(())
}

fn unlock_all(
    file: &File,
    domain: &mut ManagedSqliteLockDomainGuard<'_>,
) -> std::result::Result<(), ManagedSqliteLockFailure> {
    if domain.held().exclusive {
        release(
            file,
            domain,
            SHARED_FIRST,
            SHARED_SIZE,
            ManagedSqliteLockFailurePhase::ReleaseExclusive,
        )?;
        domain.held_mut().exclusive = false;
    }
    if domain.held().reserved {
        release(
            file,
            domain,
            RESERVED_BYTE,
            1,
            ManagedSqliteLockFailurePhase::ReleaseReserved,
        )?;
        domain.held_mut().reserved = false;
    }
    if domain.held().shared {
        release(
            file,
            domain,
            SHARED_FIRST,
            SHARED_SIZE,
            ManagedSqliteLockFailurePhase::ReleaseShared,
        )?;
        domain.held_mut().shared = false;
    }
    if domain.held().pending {
        release(
            file,
            domain,
            PENDING_BYTE,
            1,
            ManagedSqliteLockFailurePhase::ReleasePending,
        )?;
        domain.held_mut().pending = false;
    }
    Ok(())
}

fn restore_shared(
    file: &File,
    domain: &mut ManagedSqliteLockDomainGuard<'_>,
) -> std::result::Result<(), ManagedSqliteLockFailure> {
    match try_lock(file, SHARED_FIRST, SHARED_SIZE, false) {
        Ok(PlatformManagedSqliteLockAttempt::Acquired) => {
            domain.held_mut().shared = true;
            Ok(())
        }
        Ok(PlatformManagedSqliteLockAttempt::Contended) => Err(poison_message(
            domain,
            ManagedSqliteLockFailurePhase::RestoreShared,
            "NODE_MANAGED_SQLITE_SHARED_LOCK_RESTORE_CONTENDED",
        )),
        Err(error) => Err(poison(
            domain,
            ManagedSqliteLockFailurePhase::RestoreShared,
            error,
        )),
    }
}

fn release(
    file: &File,
    domain: &mut ManagedSqliteLockDomainGuard<'_>,
    offset: u64,
    length: u64,
    phase: ManagedSqliteLockFailurePhase,
) -> std::result::Result<(), ManagedSqliteLockFailure> {
    unlock(file, offset, length).map_err(|error| poison(domain, phase, error))
}

fn try_lock(
    file: &File,
    offset: u64,
    length: u64,
    exclusive: bool,
) -> std::io::Result<PlatformManagedSqliteLockAttempt> {
    platform::try_lock_sqlite_byte_range(file, offset, length, exclusive)
}

fn unlock(file: &File, offset: u64, length: u64) -> std::io::Result<()> {
    platform::unlock_sqlite_byte_range(file, offset, length)
}

fn poison(
    domain: &mut ManagedSqliteLockDomainGuard<'_>,
    phase: ManagedSqliteLockFailurePhase,
    error: std::io::Error,
) -> ManagedSqliteLockFailure {
    domain.poison();
    ManagedSqliteLockFailure::new(
        phase,
        ManagedSqliteLockFailureKind::StateUncertain,
        error,
        true,
    )
}

fn poison_message(
    domain: &mut ManagedSqliteLockDomainGuard<'_>,
    phase: ManagedSqliteLockFailurePhase,
    code: &'static str,
) -> ManagedSqliteLockFailure {
    poison(domain, phase, std::io::Error::other(code))
}
