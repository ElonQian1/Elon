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
