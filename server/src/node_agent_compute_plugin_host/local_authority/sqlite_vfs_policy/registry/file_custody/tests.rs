use std::{
    collections::VecDeque,
    fs,
    path::{Path, PathBuf},
    sync::Mutex,
};

use uuid::Uuid;

use super::*;
use crate::{
    node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::{
        registry::process_owner::ManagedSqliteRegistryProcessOwner, ManagedSqliteLogicalFileRole,
    },
    node_agent_managed_fs::{
        ManagedSqliteAccess, ManagedSqliteOpenMode, ManagedSqliteShmBudget, PinnedManagedRoot,
        PinnedManagedSqliteNamespace,
    },
};

const NONCE: [u8; 16] = [0x71; 16];

struct TestCustody;

impl ManagedSqliteRegistryCustody for TestCustody {
    fn ensure_registry_current(&self) -> anyhow::Result<()> {
        Ok(())
    }
}

struct TestNonceSource(Mutex<VecDeque<[u8; 16]>>);

impl TestNonceSource {
    fn one() -> Self {
        Self(Mutex::new(VecDeque::from([NONCE])))
    }
}

impl ManagedSqliteRegistryNonceSource for TestNonceSource {
    fn fill_nonce(&self, output: &mut [u8; 16]) -> Result<(), ()> {
        *output = self.0.lock().expect("nonce queue").pop_front().ok_or(())?;
        Ok(())
    }
}

fn test_namespace(label: &str) -> (PathBuf, PinnedManagedSqliteNamespace) {
    let path = std::env::temp_dir().join(format!(
        "elon-sqlite-file-custody-{label}-{}",
        Uuid::new_v4().simple()
    ));
    fs::create_dir_all(path.join("db")).expect("create test namespace");
    let root = PinnedManagedRoot::pin(&path, &"a".repeat(64)).expect("pin test root");
    let directory = root
        .pin_existing_directory(Path::new("db"))
        .expect("pin existing db directory");
    let namespace = directory
        .into_sqlite_namespace()
        .expect("bind SQLite namespace");
    drop(root);
    (path, namespace)
}

fn process_and_route() -> (
    &'static ManagedSqliteRegistryProcessOwner<TestCustody, TestNonceSource>,
    ManagedSqliteRegistryRouteHandle,
) {
    let process = ManagedSqliteRegistryProcessOwner::leak(TestNonceSource::one());
    let route = process.register(TestCustody).expect("register route");
    process.begin_open_attempt(route).expect("begin open");
    (process, route)
}

#[test]
fn real_main_and_sidecar_receipts_retire_their_exact_leases() {
    let (path, namespace) = test_namespace("rollback");
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
    process.begin_connection_close(route).expect("begin close");
    journal.close().expect("close journal with real receipt");
    main.close().expect("close main with real receipt");
    process
        .observe_connection_closed(route)
        .expect("observe closed connection");
    process.retire_closed(route).expect("retire exact route");

    drop(namespace);
    fs::remove_dir_all(path).expect("remove closed test namespace");
}

#[test]
fn real_wal_main_receipt_retires_main_and_shm_leases_together() {
    let (path, namespace) = test_namespace("wal");
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

    process.begin_connection_close(route).expect("begin close");
    wal_main.close().expect("close WAL main with real receipt");
    process
        .observe_connection_closed(route)
        .expect("observe closed connection");
    process.retire_closed(route).expect("retire exact route");

    drop(runtime);
    fs::remove_dir_all(path).expect("remove closed WAL namespace");
}

mod operations;
