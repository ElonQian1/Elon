use std::{os::raw::c_void, ptr};

use rusqlite::ffi;

use super::*;
use crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_abi::HandleBoundSqliteAbiTestFile;

unsafe fn methods(file: *mut ffi::sqlite3_file) -> &'static ffi::sqlite3_io_methods {
    // SAFETY: callers retain the installed test allocation and invoke callbacks serially.
    let methods = unsafe { (*file).pMethods };
    assert!(!methods.is_null());
    // SAFETY: installation always points at the process-lifetime immutable callback table.
    unsafe { &*methods }
}

fn install(
    file: ManagedSqliteRegistryPinnedFile<TestCustody, TestNonceSource>,
) -> HandleBoundSqliteAbiTestFile {
    HandleBoundSqliteAbiTestFile::install(HandleBoundSqliteAbiFile::from_pinned(file))
}

#[test]
fn rollback_files_execute_real_io_lock_and_close_callbacks() {
    let (path, namespace) = test_namespace("abi-rollback");
    let main_file = namespace
        .open(
            ManagedSqliteFileKind::Main,
            ManagedSqliteAccess::ReadWrite,
            ManagedSqliteOpenMode::OpenOrCreate,
        )
        .expect("open main")
        .into_main_file()
        .expect("bind main lock domain");
    let journal_file = namespace
        .open(
            ManagedSqliteFileKind::Journal,
            ManagedSqliteAccess::ReadWrite,
            ManagedSqliteOpenMode::OpenOrCreate,
        )
        .expect("open journal");
    let (process, route) = process_and_route();
    let main = ManagedSqliteRegistryPinnedFile::bind_main(
        process,
        route,
        main_file,
        process.claim_main(route).expect("claim main"),
    )
    .expect("pair main");
    let journal = ManagedSqliteRegistryPinnedFile::bind_sidecar(
        process,
        route,
        journal_file,
        process
            .claim_sidecar(route, ManagedSqliteLogicalFileRole::Journal)
            .expect("claim journal"),
    )
    .expect("pair journal");
    process.activate_connection(route).expect("activate");
    let main = install(main);
    let journal = install(journal);
    let main_file = main.file();
    let journal_file = journal.file();

    // SAFETY: both installed allocations and all buffers remain live and callbacks are serialized.
    unsafe {
        let main_methods = methods(main_file);
        assert_eq!(
            main_methods.xWrite.expect("xWrite")(
                main_file,
                b"main".as_ptr().cast::<c_void>(),
                4,
                2,
            ),
            ffi::SQLITE_OK
        );
        let mut size = -1;
        assert_eq!(
            main_methods.xFileSize.expect("xFileSize")(main_file, &mut size),
            ffi::SQLITE_OK
        );
        assert_eq!(size, 6);
        let mut output = [0xff_u8; 7];
        assert_eq!(
            main_methods.xRead.expect("xRead")(
                main_file,
                output.as_mut_ptr().cast::<c_void>(),
                7,
                0,
            ),
            ffi::SQLITE_IOERR_SHORT_READ
        );
        assert_eq!(&output, b"\0\0main\0");
        assert_eq!(
            main_methods.xLock.expect("xLock")(main_file, ffi::SQLITE_LOCK_SHARED),
            ffi::SQLITE_OK
        );
        assert_eq!(
            main_methods.xLock.expect("xLock")(main_file, ffi::SQLITE_LOCK_RESERVED),
            ffi::SQLITE_OK
        );
        let mut reserved = 0;
        assert_eq!(
            main_methods.xCheckReservedLock.expect("xCheckReservedLock")(main_file, &mut reserved,),
            ffi::SQLITE_OK
        );
        assert_eq!(reserved, 1);
        assert_eq!(
            main_methods.xUnlock.expect("xUnlock")(main_file, ffi::SQLITE_LOCK_NONE),
            ffi::SQLITE_OK
        );

        let journal_methods = methods(journal_file);
        assert_eq!(
            journal_methods.xWrite.expect("journal xWrite")(
                journal_file,
                b"journal".as_ptr().cast::<c_void>(),
                7,
                0,
            ),
            ffi::SQLITE_OK
        );
        assert_ne!(
            journal_methods.xSync.expect("journal xSync")(journal_file, 0),
            ffi::SQLITE_OK
        );
        assert_eq!(
            journal_methods.xSync.expect("journal xSync")(journal_file, ffi::SQLITE_SYNC_FULL,),
            ffi::SQLITE_OK
        );
    }

    process.begin_connection_close(route).expect("begin close");
    // SAFETY: xClose exclusively consumes each still-installed state.
    unsafe {
        assert_eq!(
            methods(journal_file).xClose.expect("journal xClose")(journal_file),
            ffi::SQLITE_OK
        );
        assert_eq!(
            methods(main_file).xClose.expect("main xClose")(main_file),
            ffi::SQLITE_OK
        );
    }
    assert!(journal.is_cleared());
    assert!(main.is_cleared());
    process
        .observe_connection_closed(route)
        .expect("observe closed connection");
    let _retirement = process.retire_closed(route).expect("retire exact route");

    drop(namespace);
    fs::remove_dir_all(path).expect("remove closed rollback namespace");
}

#[test]
fn wal_main_executes_real_shm_callbacks_and_joint_close() {
    let (path, namespace) = test_namespace("abi-wal");
    let main_file = namespace
        .open(
            ManagedSqliteFileKind::Main,
            ManagedSqliteAccess::ReadWrite,
            ManagedSqliteOpenMode::OpenOrCreate,
        )
        .expect("open main")
        .into_main_file()
        .expect("bind main lock domain");
    let runtime = namespace
        .into_wal_runtime(ManagedSqliteShmBudget::authority_default())
        .expect("create WAL runtime");
    let wal_main_file = runtime
        .bind_main_file(main_file)
        .expect("bind WAL main and SHM connection");
    let (process, route) = process_and_route();
    let main_lease = process.claim_main(route).expect("claim main");
    process.activate_connection(route).expect("activate");
    let shm_lease = process.claim_shm(route).expect("claim SHM");
    let wal_main = ManagedSqliteRegistryPinnedFile::bind_wal_main(
        process,
        route,
        wal_main_file,
        main_lease,
        shm_lease,
    )
    .expect("pair WAL main");
    let wal_main = install(wal_main);
    let file = wal_main.file();

    // SAFETY: the installed allocation and output pointer remain live across serialized callbacks.
    unsafe {
        let methods = methods(file);
        assert_eq!(
            methods.xWrite.expect("xWrite")(file, b"wal-main".as_ptr().cast::<c_void>(), 8, 0,),
            ffi::SQLITE_OK
        );
        let mut mapped = ptr::null_mut();
        assert_eq!(
            methods.xShmMap.expect("xShmMap")(file, 0, 32 * 1024, 1, &mut mapped),
            ffi::SQLITE_OK
        );
        assert!(!mapped.is_null());
        assert_eq!(
            methods.xShmLock.expect("xShmLock")(
                file,
                0,
                1,
                ffi::SQLITE_SHM_LOCK | ffi::SQLITE_SHM_SHARED,
            ),
            ffi::SQLITE_OK
        );
        assert_ne!(
            methods.xShmLock.expect("xShmLock")(file, 0, 1, 0),
            ffi::SQLITE_OK
        );
        methods.xShmBarrier.expect("xShmBarrier")(file);
        assert_eq!(
            methods.xShmLock.expect("xShmLock")(
                file,
                0,
                1,
                ffi::SQLITE_SHM_UNLOCK | ffi::SQLITE_SHM_SHARED,
            ),
            ffi::SQLITE_OK
        );
        assert_eq!(
            methods.xShmUnmap.expect("xShmUnmap")(file, 0),
            ffi::SQLITE_OK
        );
    }

    process.begin_connection_close(route).expect("begin close");
    // SAFETY: xClose exclusively consumes the installed WAL state.
    assert_eq!(
        unsafe { methods(file).xClose.expect("xClose")(file) },
        ffi::SQLITE_OK
    );
    assert!(wal_main.is_cleared());
    process
        .observe_connection_closed(route)
        .expect("observe closed connection");
    let _retirement = process.retire_closed(route).expect("retire exact route");

    drop(runtime);
    fs::remove_dir_all(path).expect("remove closed WAL namespace");
}

#[test]
fn real_xclose_rejection_clears_raw_state_and_quarantines_route() {
    let (path, namespace) = test_namespace("abi-close-rejection");
    let main_file = namespace
        .open(
            ManagedSqliteFileKind::Main,
            ManagedSqliteAccess::ReadWrite,
            ManagedSqliteOpenMode::OpenOrCreate,
        )
        .expect("open main")
        .into_main_file()
        .expect("bind main lock domain");
    let (process, route) = process_and_route();
    let main = ManagedSqliteRegistryPinnedFile::bind_main(
        process,
        route,
        main_file,
        process.claim_main(route).expect("claim main"),
    )
    .expect("pair main");
    process.activate_connection(route).expect("activate");
    let main = install(main);
    let file = main.file();

    // Closing the physical file while the registry is Active is deliberately rejected. The raw
    // state must still be consumed exactly once, and the exact route must become unreachable.
    // SAFETY: xClose exclusively consumes the installed state.
    assert_ne!(
        unsafe { methods(file).xClose.expect("xClose")(file) },
        ffi::SQLITE_OK
    );
    assert!(main.is_cleared());
    assert!(matches!(
        process.phase(route),
        Err(ManagedSqliteRegistryProcessRouteRejection::Route(
            super::super::super::owner::ManagedSqliteRegistryRouteRejection::UnknownOrRetired,
        ))
    ));

    drop(namespace);
    fs::remove_dir_all(path).expect("remove physically closed namespace");
}
