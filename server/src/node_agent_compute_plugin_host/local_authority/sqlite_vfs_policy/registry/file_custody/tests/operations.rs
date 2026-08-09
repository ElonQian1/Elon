use std::num::{NonZeroU32, NonZeroU8};

use super::super::operations::ManagedSqliteRegistryPinnedFileOperationRejection;
use super::*;
use crate::node_agent_managed_fs::{
    ManagedSqliteLockAttempt, ManagedSqliteObservedLock, ManagedSqliteRequestedLock,
    ManagedSqliteShmLockAction, ManagedSqliteShmLockAttempt, ManagedSqliteShmLockRequest,
    ManagedSqliteShmMapMode, ManagedSqliteShmMapOutcome, ManagedSqliteShmUnmapMode,
    ManagedSqliteUnlockTarget,
};

#[test]
fn routed_io_and_main_locks_preserve_exact_rollback_custody() {
    let (path, namespace) = test_namespace("operations-rollback");
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
    let mut main = ManagedSqliteRegistryPinnedFile::bind_main(
        process,
        route,
        main_file,
        process.claim_main(route).expect("claim main"),
    )
    .expect("pair main");
    let mut journal = ManagedSqliteRegistryPinnedFile::bind_sidecar(
        process,
        route,
        journal_file,
        process
            .claim_sidecar(route, ManagedSqliteLogicalFileRole::Journal)
            .expect("claim journal"),
    )
    .expect("pair journal");
    process.activate_connection(route).expect("activate");

    main.write_all_at(2, b"main").expect("write routed main");
    assert_eq!(main.size().expect("main size"), 6);
    let mut main_bytes = [0xff; 7];
    assert_eq!(
        main.read_at_zero_filled(0, &mut main_bytes)
            .expect("read routed main"),
        6
    );
    assert_eq!(&main_bytes, b"\0\0main\0");
    main.truncate(4).expect("truncate routed main");
    main.full_sync().expect("sync routed main");
    main.revalidate().expect("revalidate routed main");

    journal
        .write_all_at(1, b"journal")
        .expect("write routed journal");
    assert_eq!(journal.size().expect("journal size"), 8);
    journal.truncate(3).expect("truncate routed journal");
    journal.full_sync().expect("sync routed journal");
    journal.revalidate().expect("revalidate routed journal");

    assert_eq!(
        main.lock_level().expect("initial lock level"),
        ManagedSqliteObservedLock::None
    );
    assert!(!main.check_reserved_lock().expect("initial reserved probe"));
    assert_eq!(
        main.lock_to(ManagedSqliteRequestedLock::Shared)
            .expect("acquire shared"),
        ManagedSqliteLockAttempt::Acquired
    );
    assert_eq!(
        main.lock_to(ManagedSqliteRequestedLock::Reserved)
            .expect("acquire reserved"),
        ManagedSqliteLockAttempt::Acquired
    );
    assert!(main.check_reserved_lock().expect("held reserved probe"));
    main.unlock_to(ManagedSqliteUnlockTarget::Shared)
        .expect("unlock to shared");
    main.unlock_to(ManagedSqliteUnlockTarget::None)
        .expect("unlock all");
    assert!(matches!(
        journal.lock_level(),
        Err(ManagedSqliteRegistryPinnedFileOperationRejection::UnsupportedFileRole)
    ));

    process.begin_connection_close(route).expect("begin close");
    journal.close().expect("close routed journal");
    main.close().expect("close routed main");
    process
        .observe_connection_closed(route)
        .expect("observe closed connection");
    process.retire_closed(route).expect("retire exact route");

    drop(namespace);
    fs::remove_dir_all(path).expect("remove closed rollback namespace");
}

#[test]
fn routed_wal_shm_operations_stop_after_explicit_unmap() {
    let (path, namespace) = test_namespace("operations-wal");
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
    let mut wal_main = ManagedSqliteRegistryPinnedFile::bind_wal_main(
        process,
        route,
        wal_main_file,
        main_lease,
        shm_lease,
    )
    .expect("pair WAL main");

    wal_main
        .write_all_at(0, b"wal-main")
        .expect("write WAL main through IO callback");
    assert_eq!(wal_main.size().expect("WAL main size"), 8);
    let mapped = wal_main
        .shm_map(
            0,
            NonZeroU32::new(32 * 1024).expect("non-zero region size"),
            ManagedSqliteShmMapMode::Extend,
        )
        .expect("map routed SHM region");
    let ManagedSqliteShmMapOutcome::Mapped(pointer) = mapped else {
        panic!("extended SHM region must be mapped");
    };
    assert_eq!(pointer.region(), 0);
    assert_eq!(pointer.length(), 32 * 1024);

    let lock = ManagedSqliteShmLockRequest::new(
        0,
        NonZeroU8::new(1).expect("non-zero lock range"),
        ManagedSqliteShmLockAction::LockShared,
    )
    .expect("valid shared SHM lock");
    assert_eq!(
        wal_main.shm_lock(lock).expect("lock routed SHM"),
        ManagedSqliteShmLockAttempt::Acquired
    );
    wal_main.shm_barrier().expect("routed SHM barrier");
    let unlock = ManagedSqliteShmLockRequest::new(
        0,
        NonZeroU8::new(1).expect("non-zero lock range"),
        ManagedSqliteShmLockAction::UnlockShared,
    )
    .expect("valid shared SHM unlock");
    assert_eq!(
        wal_main.shm_lock(unlock).expect("unlock routed SHM"),
        ManagedSqliteShmLockAttempt::Acquired
    );

    wal_main
        .shm_unmap(ManagedSqliteShmUnmapMode::Keep)
        .expect("unmap routed SHM");
    assert!(matches!(
        wal_main.shm_barrier(),
        Err(ManagedSqliteRegistryPinnedFileOperationRejection::ShmDetached)
    ));

    process.begin_connection_close(route).expect("begin close");
    wal_main.close().expect("close routed WAL main");
    process
        .observe_connection_closed(route)
        .expect("observe closed connection");
    process.retire_closed(route).expect("retire exact route");

    drop(runtime);
    fs::remove_dir_all(path).expect("remove closed WAL namespace");
}
