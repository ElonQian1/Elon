//! Windows-only real SQLite fixture backed by the managed namespace and exact registry route.
//!
//! This fixture intentionally supports rollback-journal mode only. It is registered under a
//! unique non-default name for one test connection and is unregistered after SQLite closes every
//! managed file. Production registration remains impossible.

use std::{
    ffi::CString,
    fs,
    os::raw::{c_int, c_void},
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
        sqlite_vfs_abi::test_vfs_file_size, sqlite_vfs_policy::registry::ManagedSqliteTestVfsRoute,
    },
    node_agent_managed_fs::{PinnedManagedRoot, PinnedManagedSqliteNamespace},
};

mod callbacks;
mod connection;
#[cfg(test)]
mod tests;

use connection::ManagedSqliteRoutedConnectionFixture;

type TestRoute = ManagedSqliteTestVfsRoute<TestCustody, FixedNonceSource>;

static NEXT_VFS_ID: AtomicU64 = AtomicU64::new(1);

struct ManagedTestVfsContext {
    route: Arc<TestRoute>,
    namespace: PinnedManagedSqliteNamespace,
    backing: *mut ffi::sqlite3_vfs,
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
    fn register(root: &Path, route: Arc<TestRoute>) -> anyhow::Result<Self> {
        fs::create_dir_all(root.join("db"))
            .with_context(|| format!("create managed VFS fixture root at {}", root.display()))?;
        let pinned_root = PinnedManagedRoot::pin(root, &"b".repeat(64))
            .with_context(|| format!("pin managed VFS fixture root at {}", root.display()))?;
        let directory = pinned_root
            .pin_existing_directory(Path::new("db"))
            .context("pin managed VFS fixture database directory")?;
        let namespace = directory
            .into_sqlite_namespace()
            .map_err(|failure| anyhow!("bind managed SQLite namespace: {failure:?}"))?;
        drop(pinned_root);

        // SAFETY: SQLite owns the default VFS for process lifetime. It is used only for entropy,
        // sleep and wall-clock callbacks; all database files stay in the managed namespace.
        let backing = unsafe { ffi::sqlite3_vfs_find(ptr::null()) };
        if backing.is_null() {
            return Err(anyhow!("SQLite default VFS unavailable"));
        }
        let id = NEXT_VFS_ID.fetch_add(1, Ordering::SeqCst);
        let name = CString::new(format!("elon-test-managed-vfs-{}-{id}", std::process::id()))
            .context("construct managed test VFS name")?;
        let mut context = Box::new(ManagedTestVfsContext {
            route,
            namespace,
            backing,
            main_opens: AtomicUsize::new(0),
            journal_opens: AtomicUsize::new(0),
            wal_open_attempts: AtomicUsize::new(0),
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
        ManagedTestVfsCounts {
            main_opens: context.main_opens.load(Ordering::SeqCst),
            journal_opens: context.journal_opens.load(Ordering::SeqCst),
            wal_open_attempts: context.wal_open_attempts.load(Ordering::SeqCst),
        }
    }

    fn unregister(mut self) -> Result<(), c_int> {
        self.unregister_in_place()
    }

    fn unregister_in_place(&mut self) -> Result<(), c_int> {
        if !self.registered {
            return Ok(());
        }
        let table = self.table.as_mut().expect("registered VFS table");
        // SAFETY: the fixture calls this only after its SQLite connection has closed.
        let code = unsafe { ffi::sqlite3_vfs_unregister(&mut **table) };
        if code == ffi::SQLITE_OK {
            self.registered = false;
            Ok(())
        } else {
            self.retain_registered_parts();
            Err(code)
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

impl Drop for ManagedTestVfsRegistration {
    fn drop(&mut self) {
        let _ = self.unregister_in_place();
    }
}
