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
        sqlite_vfs_abi::test_vfs_file_size,
        sqlite_vfs_policy::registry::{
            ManagedSqliteRegistryProcessOwner, ManagedSqliteTestVfsRoute,
        },
    },
    node_agent_managed_fs::PinnedManagedSqliteWalRuntime,
};

mod callbacks;
mod connection;
#[cfg(test)]
mod fault_matrix;
mod fault_script;
mod multi_connection;
mod shared_namespace;
#[cfg(test)]
mod tests;

use connection::ManagedSqliteRoutedConnectionFixture;
use fault_script::{
    ManagedTestCallbackFaultController, ManagedTestCallbackFaultObservation,
    ManagedTestCallbackFaultOperation, ManagedTestCallbackFaultStep,
    ManagedTestCallbackFaultTiming, ManagedTestFaultingFile, ManagedTestRouteOrdinal,
};
use multi_connection::ManagedSqliteMultiConnectionFixture;
use shared_namespace::{
    ManagedTestNonceSource, ManagedTestVfsRouteCollection, ManagedTestVfsRouteEntry,
};

type TestProcessOwner = ManagedSqliteRegistryProcessOwner<TestCustody, ManagedTestNonceSource>;
type TestRoute = ManagedSqliteTestVfsRoute<TestCustody, ManagedTestNonceSource>;

static NEXT_VFS_ID: AtomicU64 = AtomicU64::new(1);

struct ManagedTestVfsContext {
    routes: Arc<ManagedTestVfsRouteCollection>,
    runtime: Arc<PinnedManagedSqliteWalRuntime>,
    faults: Arc<ManagedTestCallbackFaultController>,
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
    table: Option<Box<ffi::sqlite3_vfs>>,
    name: Option<CString>,
    context: Option<Box<ManagedTestVfsContext>>,
    registered: bool,
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
        let shared = shared_namespace::ManagedTestSharedNamespace::pin(root, id)?;
        // SAFETY: SQLite owns the default VFS for process lifetime. It is used only for entropy,
        // sleep and wall-clock callbacks; all database files stay in the managed namespace.
        let backing = unsafe { ffi::sqlite3_vfs_find(ptr::null()) };
        if backing.is_null() {
            return Err(anyhow!("SQLite default VFS unavailable"));
        }
        let name = CString::new(format!("elon-test-managed-vfs-{}-{id}", std::process::id()))
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
            table: Some(table),
            name: Some(name),
            context: Some(context),
            registered: true,
        })
    }

    fn name(&self) -> anyhow::Result<&str> {
        self.name
            .as_ref()
            .expect("registered VFS name")
            .to_str()
            .context("managed test VFS name is UTF-8")
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
        if !self.registered {
            return Ok(());
        }
        let live_routes = match self.live_route_count() {
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
        let table = self.table.as_mut().expect("registered VFS table");
        // SAFETY: an empty route collection proves that every SQLite connection closed and its
        // exact route custody retired before the shared callback context is released.
        let code = unsafe { ffi::sqlite3_vfs_unregister(&mut **table) };
        if code == ffi::SQLITE_OK {
            self.registered = false;
            Ok(())
        } else {
            self.retain_registered_parts();
            Err(anyhow!(
                "unregister managed test VFS failed with SQLite code {code}"
            ))
        }
    }

    fn retain_registered_parts(&mut self) {
        self.registered = false;
        if let Some(table) = self.table.take() {
            Box::leak(table);
        }
        if let Some(name) = self.name.take() {
            let _ = Box::leak(Box::new(name));
        }
        if let Some(context) = self.context.take() {
            Box::leak(context);
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
