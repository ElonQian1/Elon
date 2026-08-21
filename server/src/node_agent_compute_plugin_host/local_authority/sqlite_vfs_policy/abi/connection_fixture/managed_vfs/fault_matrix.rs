//! Targeted source matrix for exact, fixture-owned callback faults.
//!
//! Five unit tests have historical pass evidence; they are not A2b2 Windows dynamic cases.

use std::{
    fs,
    num::{NonZeroU32, NonZeroU8},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::{SystemTime, UNIX_EPOCH},
};

use crate::node_agent_compute_plugin_host::local_authority::{
    sqlite_vfs_abi::HandleBoundSqliteFileOperations,
    sqlite_vfs_policy::{
        HandleBoundSqliteAbiAttempt, HandleBoundSqliteAbiLockLevel,
        HandleBoundSqliteAbiShmLockAction, HandleBoundSqliteAbiShmMap,
        HandleBoundSqliteAbiUnlockLevel, ManagedSqliteLogicalFileRole,
    },
};

use super::*;

#[test]
fn callback_fault_installation_is_bounded_unique_and_before_only() -> anyhow::Result<()> {
    let route = ManagedTestRouteOrdinal::test_value(1);
    let before = step(
        route,
        ManagedSqliteLogicalFileRole::Main,
        ManagedTestCallbackFaultOperation::ShmMap,
        1,
        ManagedTestCallbackFaultTiming::BeforeCall,
    );
    let after = step(
        route,
        ManagedSqliteLogicalFileRole::Main,
        ManagedTestCallbackFaultOperation::ShmMap,
        2,
        ManagedTestCallbackFaultTiming::AfterSuccess,
    );
    assert!(ManagedTestCallbackFaultStep::new(
        route,
        ManagedSqliteLogicalFileRole::Main,
        ManagedTestCallbackFaultOperation::ShmMap,
        0,
        ManagedTestCallbackFaultTiming::BeforeCall,
    )
    .is_err());
    assert!(ManagedTestCallbackFaultController::new()
        .install(&[])
        .is_err());
    assert!(ManagedTestCallbackFaultController::new()
        .install(&vec![before; 33])
        .is_err());
    assert!(ManagedTestCallbackFaultController::new()
        .install(&[before, before])
        .is_err());

    let controller = ManagedTestCallbackFaultController::new();
    assert!(controller.install(&[after]).is_err());
    controller.install(&[before]).map_err(anyhow::Error::msg)?;
    assert!(controller.install(&[before]).is_err());
    assert_eq!(controller.pending_count().map_err(anyhow::Error::msg)?, 1);
    Ok(())
}

#[test]
fn callback_fault_is_exact_to_route_role_and_occurrence() -> anyhow::Result<()> {
    let root = unique_fault_root("multi-route-fence");
    let fixture = ManagedSqliteMultiConnectionFixture::open(&root, [0x61; 16])?;
    let controller = fixture.callback_fault_controller();
    let selected_route = fixture.route_ordinal(0)?;
    let sibling_route = fixture.route_ordinal(1)?;
    assert_ne!(selected_route, sibling_route);
    let selected = step(
        selected_route,
        ManagedSqliteLogicalFileRole::Main,
        ManagedTestCallbackFaultOperation::ShmMap,
        2,
        ManagedTestCallbackFaultTiming::BeforeCall,
    );
    fixture
        .install_callback_fault_script(&[selected])
        .map_err(anyhow::Error::msg)?;

    let selected_calls = Arc::new(ProbeCalls::default());
    let sibling_calls = Arc::new(ProbeCalls::default());
    let wrong_role_calls = Arc::new(ProbeCalls::default());
    let mut selected_file = faulting_probe(
        Arc::clone(&controller),
        selected_route,
        ManagedSqliteLogicalFileRole::Main,
        Arc::clone(&selected_calls),
    );
    let mut sibling_file = faulting_probe(
        Arc::clone(&controller),
        sibling_route,
        ManagedSqliteLogicalFileRole::Main,
        Arc::clone(&sibling_calls),
    );
    let mut wrong_role_file = faulting_probe(
        Arc::clone(&controller),
        selected_route,
        ManagedSqliteLogicalFileRole::Wal,
        Arc::clone(&wrong_role_calls),
    );

    assert!(map(&mut selected_file).is_ok());
    assert!(map(&mut sibling_file).is_ok());
    assert!(map(&mut wrong_role_file).is_ok());
    assert!(map(&mut selected_file).is_err());
    assert!(map(&mut selected_file).is_ok());
    assert_eq!(selected_calls.shm_map.load(Ordering::SeqCst), 2);
    assert_eq!(sibling_calls.shm_map.load(Ordering::SeqCst), 1);
    assert_eq!(wrong_role_calls.shm_map.load(Ordering::SeqCst), 1);
    assert_eq!(
        fixture
            .pending_callback_fault_count()
            .map_err(anyhow::Error::msg)?,
        0
    );
    assert_eq!(
        fixture
            .callback_fault_observations()
            .map_err(anyhow::Error::msg)?
            .into_iter()
            .map(ManagedTestCallbackFaultObservation::step)
            .collect::<Vec<_>>(),
        vec![selected]
    );
    fixture.close()?;
    fs::remove_dir_all(&root)?;
    Ok(())
}

#[test]
fn callback_fault_returns_before_call_without_invoking_inner_operation() -> anyhow::Result<()> {
    let controller = Arc::new(ManagedTestCallbackFaultController::new());
    let route = ManagedTestRouteOrdinal::test_value(21);
    let lock = step(
        route,
        ManagedSqliteLogicalFileRole::Main,
        ManagedTestCallbackFaultOperation::ShmLock,
        1,
        ManagedTestCallbackFaultTiming::BeforeCall,
    );
    let barrier = step(
        route,
        ManagedSqliteLogicalFileRole::Main,
        ManagedTestCallbackFaultOperation::ShmBarrier,
        2,
        ManagedTestCallbackFaultTiming::BeforeCall,
    );
    let unmap = step(
        route,
        ManagedSqliteLogicalFileRole::Main,
        ManagedTestCallbackFaultOperation::ShmUnmap,
        1,
        ManagedTestCallbackFaultTiming::BeforeCall,
    );
    controller
        .install(&[lock, barrier, unmap])
        .map_err(anyhow::Error::msg)?;
    let calls = Arc::new(ProbeCalls::default());
    let mut file = faulting_probe(
        Arc::clone(&controller),
        route,
        ManagedSqliteLogicalFileRole::Main,
        Arc::clone(&calls),
    );

    assert!(file
        .shm_lock(
            0,
            NonZeroU8::new(1).expect("non-zero lock count"),
            HandleBoundSqliteAbiShmLockAction::LockShared,
        )
        .is_err());
    assert!(file.shm_barrier().is_ok());
    assert!(file.shm_barrier().is_err());
    assert!(file.shm_unmap(false).is_err());
    assert_eq!(calls.shm_lock.load(Ordering::SeqCst), 0);
    assert_eq!(calls.shm_barrier.load(Ordering::SeqCst), 1);
    assert_eq!(calls.shm_unmap.load(Ordering::SeqCst), 0);
    assert_eq!(controller.pending_count().map_err(anyhow::Error::msg)?, 0);
    Ok(())
}

#[test]
fn callback_close_faults_are_exact_and_never_physically_retry_inner() -> anyhow::Result<()> {
    let controller = Arc::new(ManagedTestCallbackFaultController::new());
    let selected_route = ManagedTestRouteOrdinal::test_value(31);
    let sibling_route = ManagedTestRouteOrdinal::test_value(32);
    let main = step(
        selected_route,
        ManagedSqliteLogicalFileRole::Main,
        ManagedTestCallbackFaultOperation::FileClose,
        1,
        ManagedTestCallbackFaultTiming::BeforeCall,
    );
    let wal = step(
        selected_route,
        ManagedSqliteLogicalFileRole::Wal,
        ManagedTestCallbackFaultOperation::FileClose,
        1,
        ManagedTestCallbackFaultTiming::BeforeCall,
    );
    controller
        .install(&[main, wal])
        .map_err(anyhow::Error::msg)?;

    let sibling_calls = Arc::new(ProbeCalls::default());
    let sibling = faulting_probe(
        Arc::clone(&controller),
        sibling_route,
        ManagedSqliteLogicalFileRole::Main,
        Arc::clone(&sibling_calls),
    );
    assert!(Box::new(sibling).close().is_ok());
    assert_eq!(sibling_calls.close.load(Ordering::SeqCst), 1);

    for (role, expected) in [
        (ManagedSqliteLogicalFileRole::Main, main),
        (ManagedSqliteLogicalFileRole::Wal, wal),
    ] {
        let calls = Arc::new(ProbeCalls::default());
        let selected = faulting_probe(
            Arc::clone(&controller),
            selected_route,
            role,
            Arc::clone(&calls),
        );
        assert!(Box::new(selected).close().is_err());
        assert_eq!(calls.close.load(Ordering::SeqCst), 0);
        assert_eq!(calls.drops.load(Ordering::SeqCst), 1);
        assert!(observed_steps(&controller)?.contains(&expected));
    }
    assert_eq!(controller.pending_count().map_err(anyhow::Error::msg)?, 0);
    Ok(())
}

#[test]
fn same_seed_registrations_have_distinct_cross_fenced_logical_names() -> anyhow::Result<()> {
    let root_a = unique_fault_root("registration-a");
    let root_b = unique_fault_root("registration-b");
    let registration_a = ManagedTestVfsRegistration::register(&root_a, [0x55; 16])?;
    let registration_b = ManagedTestVfsRegistration::register(&root_b, [0x55; 16])?;
    let routes_a = registration_a.routes();
    let routes_b = registration_b.routes();
    let entry_a = routes_a.register_route(Arc::new(AtomicUsize::new(0)))?;
    let entry_b = routes_b.register_route(Arc::new(AtomicUsize::new(0)))?;

    assert_ne!(
        entry_a.main_name().to_bytes(),
        entry_b.main_name().to_bytes()
    );
    assert!(routes_b
        .resolve(Some(entry_a.main_name().to_bytes()))
        .is_err());
    entry_a.route().abort_unopened_for_test();
    entry_b.route().abort_unopened_for_test();
    routes_a.retire_route(&entry_a)?;
    routes_b.retire_route(&entry_b)?;
    registration_a.unregister()?;
    registration_b.unregister()?;
    fs::remove_dir_all(&root_a)?;
    fs::remove_dir_all(&root_b)?;
    Ok(())
}

fn step(
    route: ManagedTestRouteOrdinal,
    role: ManagedSqliteLogicalFileRole,
    operation: ManagedTestCallbackFaultOperation,
    occurrence: u32,
    timing: ManagedTestCallbackFaultTiming,
) -> ManagedTestCallbackFaultStep {
    ManagedTestCallbackFaultStep::new(route, role, operation, occurrence, timing)
        .expect("valid fault-matrix step")
}

fn observed_steps(
    controller: &ManagedTestCallbackFaultController,
) -> anyhow::Result<Vec<ManagedTestCallbackFaultStep>> {
    Ok(controller
        .observations()
        .map_err(anyhow::Error::msg)?
        .into_iter()
        .map(ManagedTestCallbackFaultObservation::step)
        .collect())
}

fn map(file: &mut ManagedTestFaultingFile<ProbeFile>) -> Result<HandleBoundSqliteAbiShmMap, ()> {
    file.shm_map(
        0,
        NonZeroU32::new(32 * 1024).expect("non-zero SHM region"),
        true,
    )
}

fn faulting_probe(
    controller: Arc<ManagedTestCallbackFaultController>,
    route: ManagedTestRouteOrdinal,
    role: ManagedSqliteLogicalFileRole,
    calls: Arc<ProbeCalls>,
) -> ManagedTestFaultingFile<ProbeFile> {
    ManagedTestFaultingFile::new(ProbeFile { calls }, controller, route, role)
}

#[derive(Default)]
struct ProbeCalls {
    shm_map: AtomicUsize,
    shm_lock: AtomicUsize,
    shm_barrier: AtomicUsize,
    shm_unmap: AtomicUsize,
    close: AtomicUsize,
    drops: AtomicUsize,
}

struct ProbeFile {
    calls: Arc<ProbeCalls>,
}

impl Drop for ProbeFile {
    fn drop(&mut self) {
        self.calls.drops.fetch_add(1, Ordering::SeqCst);
    }
}

impl HandleBoundSqliteFileOperations for ProbeFile {
    fn read_at_zero_filled(&mut self, _offset: u64, buffer: &mut [u8]) -> Result<usize, ()> {
        buffer.fill(0);
        Ok(buffer.len())
    }

    fn write_all_at(&mut self, _offset: u64, _bytes: &[u8]) -> Result<(), ()> {
        Ok(())
    }

    fn truncate(&mut self, _size: u64) -> Result<(), ()> {
        Ok(())
    }

    fn size(&mut self) -> Result<u64, ()> {
        Ok(0)
    }

    fn full_sync(&mut self) -> Result<(), ()> {
        Ok(())
    }

    fn lock_to(
        &mut self,
        _level: HandleBoundSqliteAbiLockLevel,
    ) -> Result<HandleBoundSqliteAbiAttempt, ()> {
        Ok(HandleBoundSqliteAbiAttempt::Acquired)
    }

    fn unlock_to(&mut self, _level: HandleBoundSqliteAbiUnlockLevel) -> Result<(), ()> {
        Ok(())
    }

    fn check_reserved_lock(&mut self) -> Result<bool, ()> {
        Ok(false)
    }

    fn shm_map(
        &mut self,
        _region: u32,
        _region_size: NonZeroU32,
        _extend: bool,
    ) -> Result<HandleBoundSqliteAbiShmMap, ()> {
        self.calls.shm_map.fetch_add(1, Ordering::SeqCst);
        Ok(HandleBoundSqliteAbiShmMap::NotPresent)
    }

    fn shm_lock(
        &mut self,
        _first: u8,
        _count: NonZeroU8,
        _action: HandleBoundSqliteAbiShmLockAction,
    ) -> Result<HandleBoundSqliteAbiAttempt, ()> {
        self.calls.shm_lock.fetch_add(1, Ordering::SeqCst);
        Ok(HandleBoundSqliteAbiAttempt::Acquired)
    }

    fn shm_barrier(&mut self) -> Result<(), ()> {
        self.calls.shm_barrier.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    fn shm_unmap(&mut self, _delete: bool) -> Result<(), ()> {
        self.calls.shm_unmap.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    fn close(self: Box<Self>) -> Result<(), ()> {
        self.calls.close.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

fn unique_fault_root(label: &str) -> std::path::PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "elon-managed-vfs-fault-{label}-{}-{unique}",
        std::process::id()
    ))
}
