//! Windows-only real SQLite fixture backed by the managed namespace and exact registry route.
//!
//! It is registered under one unique non-default name. A sealed test-only route collection maps
//! each connection's exact opaque main/journal/WAL names to its own registry route while every
//! route shares the registration's one pinned WAL runtime. Production registration remains
//! impossible.

use std::{
    ffi::CString,
    os::raw::c_void,
    path::Path,
    ptr,
    sync::{
        atomic::{AtomicU64, AtomicUsize, Ordering},
        Arc,
    },
};

use anyhow::{anyhow, Context};
use rusqlite::ffi;

use super::*;
use crate::{
    node_agent_compute_plugin_host::local_authority::{
        sqlite_vfs_abi::{observe_test_vfs_file_raw_slots, test_vfs_file_size},
        sqlite_vfs_policy::registry::{
            ManagedSqliteRegistryProcessOwner, ManagedSqliteTestVfsCallback,
            ManagedSqliteTestVfsFile, ManagedSqliteTestVfsRoute,
            ManagedSqliteTestVfsRouteCustodySnapshot, ManagedSqliteTestVfsRoutePhase,
        },
    },
    node_agent_managed_fs::PinnedManagedSqliteWalRuntime,
};

#[cfg(all(test, windows))]
mod a2_dynamic_evidence;
#[cfg(test)]
mod a2b1_cases;
#[cfg(test)]
mod a2b2_cases;
#[cfg(all(test, windows))]
mod a2c_barrier_runner;
#[cfg(all(test, windows))]
mod a2c_dms_shared_release_runner;
#[cfg(all(test, windows))]
mod a2c_mapping_close_runner;
#[cfg(all(test, windows))]
mod a2c_shm_file_close_runner;
#[cfg(all(test, windows))]
mod a2c_vfs_unregister_runner;
#[cfg(all(test, windows))]
mod a2c_view_unmap_runner;
#[cfg(all(test, windows))]
mod a2c_windows_runner;
#[cfg(all(test, windows))]
mod barrier_harness;
mod callbacks;
mod connection;
#[cfg(test)]
mod fault_matrix;
mod fault_script;
mod lifecycle_faults;
#[cfg(all(test, windows))]
mod live_registration;
mod multi_connection;
#[cfg(all(test, windows))]
mod registration_shutdown_custody;
#[cfg(all(test, windows))]
mod registration_shutdown_harness;
mod route_file;
mod shared_namespace;
mod shm_fault_script;
#[cfg(test)]
mod tests;

use connection::ManagedSqliteRoutedConnectionFixture;
use fault_script::{
    ManagedTestCallbackFaultController, ManagedTestCallbackFaultObservation,
    ManagedTestCallbackFaultOperation, ManagedTestCallbackFaultStep,
    ManagedTestCallbackFaultTiming, ManagedTestFaultingFile, ManagedTestRouteOrdinal,
};
use lifecycle_faults::{
    ManagedTestLifecycleFaultBinding, ManagedTestLifecycleFaultController,
    ManagedTestLifecycleFaultObservation, ManagedTestLifecycleFaultPhase,
    ManagedTestLifecycleFaultStep, ManagedTestLifecycleFaultTiming,
};
#[cfg(all(test, windows))]
use live_registration::ManagedTestVfsLiveRegistrationSnapshot;
use multi_connection::ManagedSqliteMultiConnectionFixture;
#[cfg(all(test, windows))]
use registration_shutdown_custody::{
    ManagedTestRegistrationShutdownTargetWitness, ManagedTestVfsRegistrationCustody,
    ManagedTestVfsRegistrationDisposition, ManagedTestVfsRetainedPartsSnapshot,
    ManagedTestVfsRetainedPartsWitness,
};
use route_file::ManagedTestRouteFile;
#[cfg(all(test, windows))]
use shared_namespace::ManagedTestBarrierLogicalRouteSnapshot;
use shared_namespace::{
    ManagedTestNonceSource, ManagedTestVfsRouteCollection, ManagedTestVfsRouteEntry,
};
use shm_fault_script::{
    ManagedTestRegistrationId, ManagedTestShmFaultPlanBinding, ManagedTestShmFaultPlanSlot,
    ManagedTestShmTargetWitness,
};

type TestProcessOwner = ManagedSqliteRegistryProcessOwner<TestCustody, ManagedTestNonceSource>;
type TestRoute = ManagedSqliteTestVfsRoute<TestCustody, ManagedTestNonceSource>;
type TestCallback = ManagedSqliteTestVfsCallback<TestCustody, ManagedTestNonceSource>;

static NEXT_VFS_ID: AtomicU64 = AtomicU64::new(1);

struct ManagedTestVfsContext {
    routes: Arc<ManagedTestVfsRouteCollection>,
    runtime: Arc<PinnedManagedSqliteWalRuntime>,
    faults: Arc<ManagedTestCallbackFaultController>,
    lifecycle: Arc<ManagedTestLifecycleFaultController>,
    backing: *mut ffi::sqlite3_vfs,
    counters: Arc<ManagedTestVfsCounters>,
}

struct ManagedTestVfsCounters {
    main_opens: AtomicUsize,
    journal_opens: AtomicUsize,
    wal_open_attempts: AtomicUsize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ManagedTestVfsCounts {
    main_opens: usize,
    journal_opens: usize,
    wal_open_attempts: usize,
}

struct ManagedTestVfsRegistration {
    id: ManagedTestRegistrationId,
    registration_shutdown_attempts: u32,
    table: Option<Box<ffi::sqlite3_vfs>>,
    name: Option<CString>,
    context: Option<Box<ManagedTestVfsContext>>,
    registered: bool,
    retained_parts_witness: Arc<ManagedTestVfsRetainedPartsWitness>,
}

impl ManagedTestVfsRegistration {
    fn register(root: &Path, _nonce_seed: [u8; 16]) -> anyhow::Result<Self> {
        // The legacy seed stays in the single-connection API only as a test label. Cross-
        // registration fencing comes exclusively from this checked process-unique identity.
        let id = NEXT_VFS_ID
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |current| {
                current.checked_add(1)
            })
            .map_err(|_| anyhow!("managed test VFS registration identity exhausted"))?;
        let id = ManagedTestRegistrationId::from_counter(id).map_err(anyhow::Error::msg)?;
        let lifecycle = ManagedTestLifecycleFaultController::new();
        let shared =
            shared_namespace::ManagedTestSharedNamespace::pin(root, id, Arc::clone(&lifecycle))?;
        // SAFETY: SQLite owns the default VFS for process lifetime. It is used only for entropy,
        // sleep and wall-clock callbacks; all database files stay in the managed namespace.
        let backing = unsafe { ffi::sqlite3_vfs_find(ptr::null()) };
        if backing.is_null() {
            return Err(anyhow!("SQLite default VFS unavailable"));
        }
        let name = CString::new(format!(
            "elon-test-managed-vfs-{}-{}",
            std::process::id(),
            id.counter_value()
        ))
        .context("construct managed test VFS name")?;
        let counters = Arc::new(ManagedTestVfsCounters {
            main_opens: AtomicUsize::new(0),
            journal_opens: AtomicUsize::new(0),
            wal_open_attempts: AtomicUsize::new(0),
        });
        let faults = Arc::new(ManagedTestCallbackFaultController::new());
        let mut context = Box::new(ManagedTestVfsContext {
            routes: shared.routes,
            runtime: shared.runtime,
            faults,
            lifecycle,
            backing,
            counters,
        });
        let mut table = Box::new(ffi::sqlite3_vfs {
            iVersion: 1,
            szOsFile: test_vfs_file_size(),
            mxPathname: 64,
            pNext: ptr::null_mut(),
            zName: name.as_ptr(),
            pAppData: (&mut *context as *mut ManagedTestVfsContext).cast::<c_void>(),
            xOpen: Some(callbacks::open),
            xDelete: Some(callbacks::delete),
            xAccess: Some(callbacks::access),
            xFullPathname: Some(callbacks::full_pathname),
            xDlOpen: Some(callbacks::dl_open),
            xDlError: Some(callbacks::dl_error),
            xDlSym: Some(callbacks::dl_sym),
            xDlClose: Some(callbacks::dl_close),
            xRandomness: Some(callbacks::randomness),
            xSleep: Some(callbacks::sleep),
            xCurrentTime: Some(callbacks::current_time),
            xGetLastError: Some(callbacks::get_last_error),
            xCurrentTimeInt64: None,
            xSetSystemCall: None,
            xGetSystemCall: None,
            xNextSystemCall: None,
        });
        // SAFETY: table, name and context have stable boxed/CString storage until unregister.
        let code = unsafe { ffi::sqlite3_vfs_register(&mut *table, 0) };
        if code != ffi::SQLITE_OK {
            return Err(anyhow!(
                "register managed test VFS failed with SQLite code {code}"
            ));
        }
        Ok(Self {
            id,
            registration_shutdown_attempts: 0,
            table: Some(table),
            name: Some(name),
            context: Some(context),
            registered: true,
            retained_parts_witness: Arc::new(ManagedTestVfsRetainedPartsWitness::default()),
        })
    }

    fn name(&self) -> anyhow::Result<&str> {
        self.name
            .as_ref()
            .expect("registered VFS name")
            .to_str()
            .context("managed test VFS name is UTF-8")
    }

    fn registration_id(&self) -> ManagedTestRegistrationId {
        self.id
    }

    fn counts(&self) -> ManagedTestVfsCounts {
        let context = self.context.as_ref().expect("registered VFS context");
        context.counters.snapshot()
    }

    fn counters(&self) -> Arc<ManagedTestVfsCounters> {
        Arc::clone(
            &self
                .context
                .as_ref()
                .expect("registered VFS context")
                .counters,
        )
    }

    fn routes(&self) -> Arc<ManagedTestVfsRouteCollection> {
        Arc::clone(
            &self
                .context
                .as_ref()
                .expect("registered VFS context")
                .routes,
        )
    }

    fn faults(&self) -> Arc<ManagedTestCallbackFaultController> {
        Arc::clone(
            &self
                .context
                .as_ref()
                .expect("registered VFS context")
                .faults,
        )
    }

    fn lifecycle(&self) -> Arc<ManagedTestLifecycleFaultController> {
        Arc::clone(
            &self
                .context
                .as_ref()
                .expect("registered VFS context")
                .lifecycle,
        )
    }

    fn live_route_count(&self) -> anyhow::Result<usize> {
        self.context
            .as_ref()
            .expect("registered VFS context")
            .routes
            .live_route_count()
    }

    fn unregister(mut self) -> anyhow::Result<()> {
        self.unregister_in_place()
    }

    fn unregister_in_place(&mut self) -> anyhow::Result<()> {
        self.unregister_in_place_with(ManagedTestVfsRouteCollection::live_route_count, |table| {
            // SAFETY: the empty-route proof is checked before this closure is invoked.
            unsafe { ffi::sqlite3_vfs_unregister(table) }
        })
    }

    fn unregister_in_place_with<ObserveRouteIndex, UnregisterNative>(
        &mut self,
        observe_route_index: ObserveRouteIndex,
        unregister_native: UnregisterNative,
    ) -> anyhow::Result<()>
    where
        ObserveRouteIndex: FnOnce(&ManagedTestVfsRouteCollection) -> anyhow::Result<usize>,
        UnregisterNative: FnOnce(*mut ffi::sqlite3_vfs) -> std::os::raw::c_int,
    {
        if !self.registered {
            return Ok(());
        }
        self.registration_shutdown_attempts = self
            .registration_shutdown_attempts
            .checked_add(1)
            .context("managed test VFS registration shutdown occurrence overflow")?;
        let lifecycle = Arc::clone(
            &self
                .context
                .as_ref()
                .expect("registered VFS context")
                .lifecycle,
        );
        #[cfg(all(test, windows))]
        match lifecycle.claim_registration_shutdown_quarantine() {
            Ok(Some(claim)) => {
                let retained = self.take_registered_parts();
                lifecycle
                    .retain_registration_shutdown_quarantine(claim, retained)
                    .map_err(anyhow::Error::msg)?;
                return Err(anyhow!(
                    "registration shutdown quarantined table/name/context custody"
                ));
            }
            Ok(None) => {}
            Err(error) => {
                self.retain_registered_parts();
                return Err(anyhow!(error));
            }
        }
        if lifecycle.is_terminal() {
            self.retain_registered_parts();
            return Err(anyhow!("managed test VFS lifecycle is terminal"));
        }
        let routes = &self
            .context
            .as_ref()
            .expect("registered VFS context")
            .routes;
        let live_routes = match observe_route_index(routes) {
            Ok(live_routes) => live_routes,
            Err(error) => {
                self.retain_registered_parts();
                return Err(error).context("inspect managed test VFS routes before unregister");
            }
        };
        if live_routes != 0 {
            self.retain_registered_parts();
            return Err(anyhow!(
                "refuse to unregister managed test VFS with {live_routes} live routes"
            ));
        }
        if lifecycle
            .before_registration(ManagedTestLifecycleFaultPhase::VfsUnregister)
            .unwrap_or(true)
        {
            lifecycle.retain_terminal(());
            self.retain_registered_parts();
            return Err(anyhow!("injected before managed test VFS unregister"));
        }
        let table = self.table.as_mut().expect("registered VFS table");
        // The default closure calls SQLite directly. The registration-shutdown harness may only
        // replace this call with its sealed deterministic native-result observer.
        let code = unregister_native(&mut **table);
        if code == ffi::SQLITE_OK {
            self.registered = false;
            if lifecycle
                .after_registration_success(ManagedTestLifecycleFaultPhase::VfsUnregister)
                .unwrap_or(true)
            {
                lifecycle.retain_terminal(());
                self.retain_registered_parts();
                Err(anyhow!("injected after managed test VFS unregister"))
            } else {
                Ok(())
            }
        } else {
            lifecycle.native_failure(None, ManagedTestLifecycleFaultPhase::VfsUnregister);
            lifecycle.retain_terminal(());
            self.retain_registered_parts();
            Err(anyhow!(
                "unregister managed test VFS failed with SQLite code {code}"
            ))
        }
    }
}

impl ManagedTestVfsCounters {
    fn snapshot(&self) -> ManagedTestVfsCounts {
        ManagedTestVfsCounts {
            main_opens: self.main_opens.load(Ordering::SeqCst),
            journal_opens: self.journal_opens.load(Ordering::SeqCst),
            wal_open_attempts: self.wal_open_attempts.load(Ordering::SeqCst),
        }
    }
}

impl Drop for ManagedTestVfsRegistration {
    fn drop(&mut self) {
        let _ = self.unregister_in_place();
    }
}
