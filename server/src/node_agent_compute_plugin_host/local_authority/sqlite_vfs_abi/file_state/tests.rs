use std::{
    mem::MaybeUninit,
    ptr::NonNull,
    sync::{Arc, Mutex},
};

use super::*;
use crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_abi::{
    io_core, io_shm, result_codes, types::InertHandleBoundSqliteFile,
};

#[derive(Default)]
struct FakeFacts {
    bytes: Vec<u8>,
    syncs: usize,
    barriers: usize,
    unmaps: Vec<bool>,
    closes: usize,
    drops: usize,
    shm: Vec<u8>,
}

struct FakeFile {
    facts: Arc<Mutex<FakeFacts>>,
    panic_on_read: bool,
    close_ok: bool,
}

impl FakeFile {
    fn new(facts: Arc<Mutex<FakeFacts>>) -> Self {
        Self {
            facts,
            panic_on_read: false,
            close_ok: true,
        }
    }
}

impl Drop for FakeFile {
    fn drop(&mut self) {
        self.facts.lock().expect("fake facts").drops += 1;
    }
}

impl HandleBoundSqliteFileOperations for FakeFile {
    fn read_at_zero_filled(&mut self, offset: u64, buffer: &mut [u8]) -> Result<usize, ()> {
        if self.panic_on_read {
            panic!("test callback panic");
        }
        buffer.fill(0);
        let facts = self.facts.lock().map_err(|_| ())?;
        let offset = usize::try_from(offset).map_err(drop)?;
        if offset >= facts.bytes.len() {
            return Ok(0);
        }
        let count = buffer.len().min(facts.bytes.len() - offset);
        buffer[..count].copy_from_slice(&facts.bytes[offset..offset + count]);
        Ok(count)
    }

    fn write_all_at(&mut self, offset: u64, bytes: &[u8]) -> Result<(), ()> {
        let mut facts = self.facts.lock().map_err(|_| ())?;
        let offset = usize::try_from(offset).map_err(drop)?;
        let end = offset.checked_add(bytes.len()).ok_or(())?;
        let current_len = facts.bytes.len();
        facts.bytes.resize(current_len.max(end), 0);
        facts.bytes[offset..end].copy_from_slice(bytes);
        Ok(())
    }

    fn truncate(&mut self, size: u64) -> Result<(), ()> {
        let mut facts = self.facts.lock().map_err(|_| ())?;
        facts.bytes.resize(usize::try_from(size).map_err(drop)?, 0);
        Ok(())
    }

    fn size(&mut self) -> Result<u64, ()> {
        u64::try_from(self.facts.lock().map_err(|_| ())?.bytes.len()).map_err(drop)
    }

    fn full_sync(&mut self) -> Result<(), ()> {
        self.facts.lock().map_err(|_| ())?.syncs += 1;
        Ok(())
    }

    fn lock_to(
        &mut self,
        level: HandleBoundSqliteAbiLockLevel,
    ) -> Result<HandleBoundSqliteAbiAttempt, ()> {
        Ok(if level == HandleBoundSqliteAbiLockLevel::Exclusive {
            HandleBoundSqliteAbiAttempt::Busy
        } else {
            HandleBoundSqliteAbiAttempt::Acquired
        })
    }

    fn unlock_to(&mut self, _level: HandleBoundSqliteAbiUnlockLevel) -> Result<(), ()> {
        Ok(())
    }

    fn check_reserved_lock(&mut self) -> Result<bool, ()> {
        Ok(true)
    }

    fn shm_map(
        &mut self,
        _region: u32,
        region_size: NonZeroU32,
        _extend: bool,
    ) -> Result<HandleBoundSqliteAbiShmMap, ()> {
        let mut facts = self.facts.lock().map_err(|_| ())?;
        facts.shm.resize(region_size.get() as usize, 0);
        let pointer = NonNull::new(facts.shm.as_mut_ptr().cast()).ok_or(())?;
        Ok(HandleBoundSqliteAbiShmMap::Mapped(pointer))
    }

    fn shm_lock(
        &mut self,
        _first: u8,
        _count: NonZeroU8,
        action: HandleBoundSqliteAbiShmLockAction,
    ) -> Result<HandleBoundSqliteAbiAttempt, ()> {
        Ok(
            if action == HandleBoundSqliteAbiShmLockAction::LockExclusive {
                HandleBoundSqliteAbiAttempt::Busy
            } else {
                HandleBoundSqliteAbiAttempt::Acquired
            },
        )
    }

    fn shm_barrier(&mut self) -> Result<(), ()> {
        self.facts.lock().map_err(|_| ())?.barriers += 1;
        Ok(())
    }

    fn shm_unmap(&mut self, delete: bool) -> Result<(), ()> {
        self.facts.lock().map_err(|_| ())?.unmaps.push(delete);
        Ok(())
    }

    fn close(self: Box<Self>) -> Result<(), ()> {
        self.facts.lock().map_err(|_| ())?.closes += 1;
        if self.close_ok {
            Ok(())
        } else {
            Err(())
        }
    }
}

fn install(
    fake: FakeFile,
) -> (
    Box<MaybeUninit<InertHandleBoundSqliteFile>>,
    *mut ffi::sqlite3_file,
) {
    let mut storage = Box::new(MaybeUninit::<InertHandleBoundSqliteFile>::uninit());
    let file = storage.as_mut_ptr().cast::<ffi::sqlite3_file>();
    // SAFETY: this is fresh, aligned storage with the exact published file layout.
    assert!(unsafe { raw_state::initialize_fresh_file(file) });
    let state = HandleBoundSqliteFileState::from_test(fake);
    // SAFETY: this test owns and serializes the initialized allocation.
    assert!(unsafe { raw_state::install_state(file, state) }.is_ok());
    (storage, file)
}

fn assert_cleared(storage: &MaybeUninit<InertHandleBoundSqliteFile>) {
    // SAFETY: install initialized the exact storage and it remains alive.
    let file = unsafe { storage.assume_init_ref() };
    assert!(file.base.pMethods.is_null());
    assert!(file.state.is_null());
}

#[cfg(windows)]
fn raw_close_control_installed(storage: &MaybeUninit<InertHandleBoundSqliteFile>) -> bool {
    // SAFETY: install initialized the exact test-only sidecar slot and storage remains alive.
    !unsafe { storage.assume_init_ref() }
        .raw_close_control
        .is_null()
}

#[test]
fn core_callbacks_route_short_read_write_truncate_size_and_sync() {
    let facts = Arc::new(Mutex::new(FakeFacts {
        bytes: b"abc".to_vec(),
        ..FakeFacts::default()
    }));
    let (storage, file) = install(FakeFile::new(Arc::clone(&facts)));

    let mut output = [0xff_u8; 5];
    // SAFETY: buffers and the installed file remain valid and serialized for each callback.
    assert_eq!(
        unsafe { io_core::read(file, output.as_mut_ptr().cast(), 5, 0) },
        ffi::SQLITE_IOERR_SHORT_READ
    );
    assert_eq!(&output, b"abc\0\0");
    assert_eq!(
        unsafe { io_core::write(file, b"XY".as_ptr().cast(), 2, 1) },
        ffi::SQLITE_OK
    );
    let mut size = -1;
    assert_eq!(
        unsafe { io_core::file_size(file, &mut size) },
        ffi::SQLITE_OK
    );
    assert_eq!(size, 3);
    assert_eq!(unsafe { io_core::truncate(file, 2) }, ffi::SQLITE_OK);
    assert_eq!(
        unsafe { io_core::sync(file, ffi::SQLITE_SYNC_FULL) },
        ffi::SQLITE_OK
    );
    assert_eq!(
        unsafe { io_core::sync(file, 0) },
        result_codes::SYNC_UNAVAILABLE
    );
    assert_eq!(unsafe { io_core::close(file) }, ffi::SQLITE_OK);

    let facts = facts.lock().expect("fake facts");
    assert_eq!(&facts.bytes, b"aX");
    assert_eq!(facts.syncs, 1);
    assert_eq!(facts.closes, 1);
    assert_eq!(facts.drops, 1);
    drop(facts);
    assert_cleared(storage.as_ref());
}

#[test]
fn lock_callbacks_preserve_busy_and_reject_unsupported_levels() {
    let facts = Arc::new(Mutex::new(FakeFacts::default()));
    let (storage, file) = install(FakeFile::new(Arc::clone(&facts)));

    // SAFETY: the installed file remains valid and serialized for each callback.
    assert_eq!(
        unsafe { io_core::lock(file, ffi::SQLITE_LOCK_SHARED) },
        ffi::SQLITE_OK
    );
    assert_eq!(
        unsafe { io_core::lock(file, ffi::SQLITE_LOCK_EXCLUSIVE) },
        ffi::SQLITE_BUSY
    );
    assert_eq!(
        unsafe { io_core::lock(file, ffi::SQLITE_LOCK_PENDING) },
        result_codes::LOCK_UNAVAILABLE
    );
    assert_eq!(
        unsafe { io_core::unlock(file, ffi::SQLITE_LOCK_NONE) },
        ffi::SQLITE_OK
    );
    let mut reserved = 0;
    assert_eq!(
        unsafe { io_core::check_reserved_lock(file, &mut reserved) },
        ffi::SQLITE_OK
    );
    assert_eq!(reserved, 1);
    assert_eq!(unsafe { io_core::close(file) }, ffi::SQLITE_OK);
    assert_eq!(facts.lock().expect("fake facts").drops, 1);
    assert_cleared(storage.as_ref());
}

#[test]
fn shm_callbacks_route_mapping_busy_barrier_and_unmap() {
    let facts = Arc::new(Mutex::new(FakeFacts::default()));
    let (storage, file) = install(FakeFile::new(Arc::clone(&facts)));
    let mut mapped = std::ptr::null_mut();

    // SAFETY: output and the installed file remain valid and serialized for each callback.
    assert_eq!(
        unsafe { io_shm::map(file, 0, 32, 1, &mut mapped) },
        ffi::SQLITE_OK
    );
    assert!(!mapped.is_null());
    assert_eq!(
        unsafe { io_shm::lock(file, 0, 1, ffi::SQLITE_SHM_LOCK | ffi::SQLITE_SHM_SHARED,) },
        ffi::SQLITE_OK
    );
    assert_eq!(
        unsafe { io_shm::lock(file, 0, 1, ffi::SQLITE_SHM_LOCK | ffi::SQLITE_SHM_EXCLUSIVE,) },
        ffi::SQLITE_BUSY
    );
    unsafe { io_shm::barrier(file) };
    assert_eq!(unsafe { io_shm::unmap(file, 0) }, ffi::SQLITE_OK);
    assert_eq!(unsafe { io_core::close(file) }, ffi::SQLITE_OK);

    let facts = facts.lock().expect("fake facts");
    assert_eq!(facts.barriers, 1);
    assert_eq!(facts.unmaps, [false]);
    assert_eq!(facts.drops, 1);
    drop(facts);
    assert_cleared(storage.as_ref());
}

#[test]
fn close_failure_consumes_state_and_drops_exactly_once() {
    let facts = Arc::new(Mutex::new(FakeFacts::default()));
    let mut fake = FakeFile::new(Arc::clone(&facts));
    fake.close_ok = false;
    let (storage, file) = install(fake);

    // SAFETY: xClose exclusively consumes the installed state.
    assert_eq!(
        unsafe { io_core::close(file) },
        result_codes::CLOSE_UNAVAILABLE
    );
    assert_eq!(
        unsafe { io_core::close(file) },
        result_codes::CLOSE_UNAVAILABLE
    );
    let facts = facts.lock().expect("fake facts");
    assert_eq!(facts.closes, 1);
    assert_eq!(facts.drops, 1);
    drop(facts);
    assert_cleared(storage.as_ref());
}

#[cfg(windows)]
#[test]
fn raw_close_witness_survives_allocation_release_and_records_the_exact_transition() {
    let facts = Arc::new(Mutex::new(FakeFacts::default()));
    let (storage, file) = install(FakeFile::new(Arc::clone(&facts)));
    // SAFETY: the installed allocation is live and this test serializes observation and close.
    let witness = unsafe { raw_state::observe_test_vfs_file_raw_close_witness(file) }
        .expect("installed raw close witness");

    // SAFETY: xClose exclusively consumes this exact installed state.
    assert_eq!(unsafe { io_core::close(file) }, ffi::SQLITE_OK);
    assert_cleared(storage.as_ref());
    assert!(!raw_close_control_installed(storage.as_ref()));
    drop(storage);

    assert_eq!(
        witness.snapshot(),
        raw_state::HandleBoundSqliteAbiRawCloseWitnessSnapshot {
            raw_close_entries: 1,
            raw_close_entry_order: 1,
            state_take_attempts: 1,
            state_take_attempt_order: 2,
            methods_clears: 1,
            methods_clear_order: 3,
            state_take_successes: 1,
            state_take_success_order: 4,
            state_close_custody_retentions: 0,
            state_close_custody_retention_order: 0,
            state_close_attempts: 1,
            state_close_attempt_order: 5,
            state_abandons: 0,
            state_abandon_order: 0,
        }
    );
}

#[cfg(windows)]
#[test]
fn raw_state_take_rejection_is_exact_allocation_bound_and_second_close_is_rejected() {
    let facts = Arc::new(Mutex::new(FakeFacts::default()));
    let (storage, file) = install(FakeFile::new(Arc::clone(&facts)));
    // Save the real callback while pMethods is installed; the first invocation deliberately clears
    // that slot, while the simulated Connection continues to own the allocation and callback fn.
    let close_callback = unsafe {
        (*(*file).pMethods)
            .xClose
            .expect("installed real xClose callback")
    };
    // SAFETY: this exact installed allocation remains live and serialized throughout the test.
    let witness = unsafe { raw_state::arm_test_vfs_file_raw_state_take_rejection(file) }
        .expect("arm exact-allocation raw-state take rejection");
    let storage = std::mem::ManuallyDrop::new(storage);

    // Arming one allocation must not alter another allocation's ordinary physical close path.
    let other_facts = Arc::new(Mutex::new(FakeFacts::default()));
    let (other_storage, other_file) = install(FakeFile::new(Arc::clone(&other_facts)));
    assert_eq!(unsafe { io_core::close(other_file) }, ffi::SQLITE_OK);
    assert_cleared(other_storage.as_ref());
    let other_facts = other_facts.lock().expect("other fake facts");
    assert_eq!(other_facts.closes, 1);
    assert_eq!(other_facts.drops, 1);
    drop(other_facts);
    assert_eq!(
        witness.snapshot(),
        raw_state::HandleBoundSqliteAbiRawCloseWitnessSnapshot {
            raw_close_entries: 0,
            raw_close_entry_order: 0,
            state_take_attempts: 0,
            state_take_attempt_order: 0,
            methods_clears: 0,
            methods_clear_order: 0,
            state_take_successes: 0,
            state_take_success_order: 0,
            state_close_custody_retentions: 0,
            state_close_custody_retention_order: 0,
            state_close_attempts: 0,
            state_close_attempt_order: 0,
            state_abandons: 0,
            state_abandon_order: 0,
        }
    );

    // First xClose clears both raw slots after a successful take, then transfers the typed state to
    // explicit process-lifetime custody before any physical close attempt.
    assert_eq!(
        unsafe { close_callback(file) },
        result_codes::CLOSE_UNAVAILABLE
    );
    assert_cleared(storage.as_ref());
    assert!(raw_close_control_installed(storage.as_ref()));
    let first = witness.snapshot();
    assert_eq!(
        first,
        raw_state::HandleBoundSqliteAbiRawCloseWitnessSnapshot {
            raw_close_entries: 1,
            raw_close_entry_order: 1,
            state_take_attempts: 1,
            state_take_attempt_order: 2,
            methods_clears: 1,
            methods_clear_order: 3,
            state_take_successes: 1,
            state_take_success_order: 4,
            state_close_custody_retentions: 1,
            state_close_custody_retention_order: 5,
            state_close_attempts: 0,
            state_close_attempt_order: 0,
            state_abandons: 0,
            state_abandon_order: 0,
        }
    );
    let primary_facts = facts.lock().expect("primary fake facts");
    assert_eq!(primary_facts.closes, 0);
    assert_eq!(primary_facts.drops, 0);
    drop(primary_facts);

    // The saved real callback can enter again even though pMethods is null. Its selected delta is
    // entry=1/take-attempt=1 with no second success, methods clear, or physical work.
    assert_eq!(
        unsafe { close_callback(file) },
        result_codes::CLOSE_UNAVAILABLE
    );
    let second = witness.snapshot();
    assert_cleared(storage.as_ref());
    assert!(raw_close_control_installed(storage.as_ref()));
    assert_eq!(second.raw_close_entries - first.raw_close_entries, 1);
    assert_eq!(second.state_take_attempts - first.state_take_attempts, 1);
    assert_eq!(second.methods_clears - first.methods_clears, 0);
    assert_eq!(second.state_take_successes - first.state_take_successes, 0);
    assert_eq!(
        second.state_close_custody_retentions - first.state_close_custody_retentions,
        0
    );
    assert_eq!(second.state_close_attempts - first.state_close_attempts, 0);
    assert_eq!(second.state_abandons - first.state_abandons, 0);
    assert_eq!(second.raw_close_entries, 2);
    assert_eq!(second.state_take_attempts, 2);
    assert_eq!(second.methods_clears, 1);
    assert_eq!(second.state_take_successes, 1);
    assert_eq!(facts.lock().expect("primary fake facts").closes, 0);
    assert_eq!(facts.lock().expect("primary fake facts").drops, 0);

    // `storage` intentionally models the Connection's process-lifetime allocation custody. Its
    // sidecar owns the ManuallyDrop-wrapped typed state; no ad-hoc Box::leak is involved.
}

#[test]
fn callback_panic_abandons_state_and_prevents_a_second_drop() {
    let facts = Arc::new(Mutex::new(FakeFacts::default()));
    let mut fake = FakeFile::new(Arc::clone(&facts));
    fake.panic_on_read = true;
    let (storage, file) = install(fake);
    let mut output = [0xff_u8; 1];

    // SAFETY: the test supplies a valid output and serializes the installed allocation.
    assert_eq!(
        unsafe { io_core::read(file, output.as_mut_ptr().cast(), 1, 0) },
        result_codes::READ_UNAVAILABLE
    );
    assert_eq!(output, [0]);
    assert_eq!(
        unsafe { io_core::close(file) },
        result_codes::CLOSE_UNAVAILABLE
    );
    assert_eq!(facts.lock().expect("fake facts").drops, 1);
    assert_cleared(storage.as_ref());
}
