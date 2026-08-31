use std::{
    os::raw::{c_char, c_int, c_void},
    panic::{catch_unwind, AssertUnwindSafe},
    path::Path,
    ptr,
    sync::{atomic::AtomicUsize, Arc},
};

use anyhow::{anyhow, Context};
use rusqlite::{ffi, Connection, OpenFlags};

use super::*;
#[cfg(all(test, windows))]
use crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_abi::{
    HandleBoundSqliteAbiRawCloseWitness, HandleBoundSqliteAbiRawSlotSnapshot,
};

mod registry_lifecycle;
pub(super) use registry_lifecycle::{
    ManagedTestRegistryLifecycleCloseOutcome, ManagedTestRegistryLifecycleRouteObserver,
};
#[cfg(all(test, windows))]
mod joint_close;
#[cfg(all(test, windows))]
pub(super) use joint_close::ManagedTestCapturedMainCloseCall;
#[cfg(all(test, windows))]
mod unmap;
#[cfg(all(test, windows))]
pub(super) use unmap::{
    ManagedTestShmLockCallbackObservation, ManagedTestShmMapCallbackObservation,
    ManagedTestUnmapCallbackObservation,
};

struct ManagedVfsAuthorizerContext {
    route: Arc<TestRoute>,
}

impl ManagedVfsAuthorizerContext {
    fn authorize(
        &self,
        action_code: c_int,
        argument_one: *const c_char,
        argument_two: *const c_char,
        argument_three: *const c_char,
        accessor: *const c_char,
    ) -> c_int {
        // SAFETY: SQLite owns all callback strings for this callback invocation.
        let decoded = unsafe {
            (
                bounded_bytes(argument_one),
                bounded_bytes(argument_two),
                bounded_bytes(argument_three),
                bounded_bytes(accessor),
            )
        };
        let (Ok(argument_one), Ok(argument_two), Ok(argument_three), Ok(accessor)) = decoded else {
            return ffi::SQLITE_DENY;
        };
        let request = match ManagedSqliteAuthorizerAbiAdapter::project(
            ManagedSqliteRawAuthorizerRequest::new(
                action_code,
                argument_one,
                argument_two,
                argument_three,
                accessor,
            ),
        ) {
            Ok(request) => request,
            Err(_) => return ffi::SQLITE_DENY,
        };
        match self.route.authorize_sql(request) {
            Ok(ManagedSqliteAuthorizerDecision::Allow) => ffi::SQLITE_OK,
            Ok(ManagedSqliteAuthorizerDecision::Deny) | Err(()) => ffi::SQLITE_DENY,
        }
    }
}

unsafe extern "C" fn managed_authorizer_callback(
    context: *mut c_void,
    action_code: c_int,
    argument_one: *const c_char,
    argument_two: *const c_char,
    argument_three: *const c_char,
    accessor: *const c_char,
) -> c_int {
    catch_unwind(AssertUnwindSafe(|| {
        let Some(context) = context.cast::<ManagedVfsAuthorizerContext>().as_ref() else {
            return ffi::SQLITE_DENY;
        };
        context.authorize(
            action_code,
            argument_one,
            argument_two,
            argument_three,
            accessor,
        )
    }))
    .unwrap_or(ffi::SQLITE_DENY)
}

pub(super) struct ManagedSqliteRoutedConnectionFixture {
    connection: Option<Connection>,
    authorizer: Option<Box<ManagedVfsAuthorizerContext>>,
    authorizer_installed: bool,
    registration: Option<ManagedTestVfsRegistration>,
    routes: Arc<ManagedTestVfsRouteCollection>,
    route_entry: Option<Arc<ManagedTestVfsRouteEntry>>,
    route: Arc<TestRoute>,
    counters: Arc<ManagedTestVfsCounters>,
}

impl ManagedSqliteRoutedConnectionFixture {
    pub(super) fn open(root: &Path, nonce: [u8; 16]) -> anyhow::Result<Self> {
        let registration = ManagedTestVfsRegistration::register(root, nonce)?;
        let mut fixture = Self::open_registered(&registration)?;
        fixture.registration = Some(registration);
        Ok(fixture)
    }

    pub(super) fn open_registered(
        registration: &ManagedTestVfsRegistration,
    ) -> anyhow::Result<Self> {
        let vfs_name = registration.name()?.to_owned();
        let routes = registration.routes();
        let custody_drops = Arc::new(AtomicUsize::new(0));
        let route_entry = routes.register_route(custody_drops)?;
        let route = Arc::clone(route_entry.route());
        let logical_name = match route_entry.main_name().to_str() {
            Ok(logical_name) => logical_name.to_owned(),
            Err(error) => {
                route.abort_unopened_for_test();
                routes.retire_route(&route_entry)?;
                return Err(error).context("managed VFS logical main name is UTF-8");
            }
        };
        let flags = OpenFlags::SQLITE_OPEN_READ_WRITE
            | OpenFlags::SQLITE_OPEN_CREATE
            | OpenFlags::SQLITE_OPEN_FULL_MUTEX
            | OpenFlags::SQLITE_OPEN_PRIVATE_CACHE
            | OpenFlags::SQLITE_OPEN_NOFOLLOW;
        let connection = match Connection::open_with_flags_and_vfs(&logical_name, flags, &vfs_name)
        {
            Ok(connection) => connection,
            Err(error) => {
                route.abort_unopened_for_test();
                routes
                    .retire_route(&route_entry)
                    .with_context(|| format!("retire route after SQLite open failed: {error}"))?;
                return Err(error).context("open managed routed SQLite connection");
            }
        };
        let mut fixture = Self {
            connection: Some(connection),
            authorizer: Some(Box::new(ManagedVfsAuthorizerContext {
                route: Arc::clone(&route),
            })),
            authorizer_installed: false,
            registration: None,
            routes,
            route_entry: Some(route_entry),
            route,
            counters: registration.counters(),
        };
        if let Err(error) = configure_connection(fixture.connection()) {
            return Err(fixture.cancel_open(error.context("configure managed SQLite connection")));
        }
        if let Err(error) = fixture.install_authorizer() {
            return Err(fixture.cancel_open(error));
        }
        Ok(fixture)
    }

    pub(super) fn connection(&self) -> &Connection {
        self.connection
            .as_ref()
            .expect("managed fixture connection")
    }

    pub(super) fn into_schema_migration(&self) -> anyhow::Result<()> {
        self.route
            .enter_schema_migration()
            .map_err(|()| anyhow!("enter managed VFS schema migration"))
    }

    pub(super) fn into_runtime(&self) -> anyhow::Result<()> {
        self.route
            .enter_runtime()
            .map_err(|()| anyhow!("enter managed VFS runtime"))
    }

    pub(super) fn security_snapshot(&self) -> anyhow::Result<ConnectionSecuritySnapshot> {
        let db = self.raw_handle();
        Ok(ConnectionSecuritySnapshot {
            defensive: read_db_config(db, ffi::SQLITE_DBCONFIG_DEFENSIVE)?,
            trusted_schema: read_db_config(db, ffi::SQLITE_DBCONFIG_TRUSTED_SCHEMA)?,
            dqs_dml: read_db_config(db, ffi::SQLITE_DBCONFIG_DQS_DML)?,
            dqs_ddl: read_db_config(db, ffi::SQLITE_DBCONFIG_DQS_DDL)?,
            load_extension: read_db_config(db, ffi::SQLITE_DBCONFIG_ENABLE_LOAD_EXTENSION)?,
            // SAFETY: the live fixture owns this valid SQLite connection handle.
            attached_limit: unsafe { ffi::sqlite3_limit(db, ffi::SQLITE_LIMIT_ATTACHED, -1) },
            // SAFETY: the live fixture owns this valid SQLite connection handle.
            worker_thread_limit: unsafe {
                ffi::sqlite3_limit(db, ffi::SQLITE_LIMIT_WORKER_THREADS, -1)
            },
        })
    }

    pub(super) fn counts(&self) -> ManagedTestVfsCounts {
        self.counters.snapshot()
    }

    pub(super) fn route_ordinal(&self) -> ManagedTestRouteOrdinal {
        self.route_entry
            .as_ref()
            .expect("managed fixture route entry")
            .ordinal()
    }

    #[cfg(all(test, windows))]
    pub(super) fn registration_id_for_test(&self) -> u64 {
        self.registration
            .as_ref()
            .expect("managed fixture owns its registration")
            .registration_id()
            .counter_value()
    }

    #[cfg(all(test, windows))]
    pub(super) fn live_registration_snapshot_for_test(
        &self,
    ) -> anyhow::Result<ManagedTestVfsLiveRegistrationSnapshot> {
        self.registration
            .as_ref()
            .expect("managed fixture owns its registration")
            .live_registration_snapshot()
    }

    #[cfg(all(test, windows))]
    pub(super) fn route_custody_snapshot(
        &self,
    ) -> Result<ManagedSqliteTestVfsRouteCustodySnapshot, &'static str> {
        self.route
            .registration_shutdown_custody_snapshot()
            .map_err(|()| "managed route custody snapshot unavailable")
    }

    #[cfg(all(test, windows))]
    pub(super) fn barrier_logical_route_snapshot(
        &self,
    ) -> anyhow::Result<ManagedTestBarrierLogicalRouteSnapshot> {
        self.routes.barrier_logical_route_snapshot(
            self.route_entry
                .as_ref()
                .expect("managed fixture route entry"),
        )
    }

    #[cfg(all(test, windows))]
    pub(super) fn terminal_custody_test_snapshot(
        &self,
    ) -> Result<
        crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::registry::ManagedSqliteRegistryTerminalCustodyTestSnapshot,
        &'static str,
    >{
        self.route
            .terminal_custody_test_snapshot()
            .map_err(|()| "managed terminal custody snapshot unavailable")
    }

    #[cfg(all(test, windows))]
    pub(super) fn quarantine_for_barrier_admission_test(&self) -> Result<(), &'static str> {
        self.route
            .retain_failure("managed barrier admission rejection sentinel")
            .map_err(|()| "managed barrier admission route quarantine failed")
    }

    #[cfg(all(test, windows))]
    pub(super) fn install_shm_fault_script(
        &self,
        before_call: &[(
            crate::node_agent_managed_fs::ManagedSqliteShmFailurePhase,
            u32,
        )],
        after_success: &[(
            crate::node_agent_managed_fs::ManagedSqliteShmFailurePhase,
            u32,
            crate::node_agent_managed_fs::ManagedSqliteShmFailureClass,
        )],
    ) -> Result<(), &'static str> {
        self.route_entry
            .as_ref()
            .ok_or("managed fixture route entry is not live")?
            .install_shm_fault_script(before_call, after_success)
    }

    #[cfg(all(test, windows))]
    pub(super) fn installed_shm_fault_witness(
        &self,
    ) -> Result<ManagedTestShmFaultPlanBinding, &'static str> {
        self.route_entry
            .as_ref()
            .ok_or("managed fixture route entry is not live")?
            .installed_shm_fault_witness()
    }

    #[cfg(all(test, windows))]
    pub(super) fn exact_main_shm_target_presence(&self) -> Result<bool, &'static str> {
        self.route_entry
            .as_ref()
            .ok_or("managed fixture route entry is not live")?
            .exact_main_shm_target_presence()
    }

    #[cfg(all(test, windows))]
    pub(super) fn observe_main_raw_slots(
        &self,
    ) -> Result<HandleBoundSqliteAbiRawSlotSnapshot, &'static str> {
        let file = self.main_file_pointer()?;
        // SAFETY: `main_file_pointer` returned this test VFS's live allocation and performs no
        // callback or ownership mutation.
        unsafe { observe_test_vfs_file_raw_slots(file) }
            .ok_or("managed main-file raw slots unavailable")
    }

    #[cfg(all(test, windows))]
    pub(super) fn observe_main_raw_close_witness(
        &self,
    ) -> Result<HandleBoundSqliteAbiRawCloseWitness, &'static str> {
        let file = self.main_file_pointer()?;
        // SAFETY: `main_file_pointer` returned this test VFS's live serialized allocation. The
        // returned witness clones only its durable atomic observation state.
        unsafe { observe_test_vfs_file_raw_close_witness(file) }
            .ok_or("managed main-file raw close witness unavailable")
    }

    #[cfg(all(test, windows))]
    pub(super) fn call_main_shm_barrier(
        &self,
    ) -> Result<ManagedTestVoidCallbackObservation, &'static str> {
        let file = self.main_file_pointer()?;
        // SAFETY: file_control returned this test VFS's live allocation and callbacks are invoked
        // serially by the owning FULL_MUTEX connection.
        let before = unsafe { observe_test_vfs_file_raw_slots(file) }
            .ok_or("managed barrier raw slots unavailable before callback")?;
        if !before.methods_installed || !before.state_installed {
            return Err("managed barrier raw state was not installed before callback");
        }
        // SAFETY: the live allocation exposed its installed method table above.
        let methods = unsafe { (*file).pMethods };
        let barrier = if methods.is_null() {
            None
        } else {
            // SAFETY: methods belongs to the same live main-file allocation.
            unsafe { (*methods).xShmBarrier }
        }
        .ok_or("managed barrier callback unavailable")?;
        // SAFETY: the callback receives its owning live sqlite3_file. xShmBarrier is void, so this
        // helper deliberately creates no SQLite result-code channel.
        unsafe { barrier(file) };
        // SAFETY: the allocation remains owned by the live Connection even when the callback has
        // fail-closed and cleared both installed ownership slots.
        let after = unsafe { observe_test_vfs_file_raw_slots(file) }
            .ok_or("managed barrier raw slots unavailable after callback")?;
        Ok(ManagedTestVoidCallbackObservation { before, after })
    }

    #[cfg(all(test, windows))]
    fn main_file_pointer(&self) -> Result<*mut ffi::sqlite3_file, &'static str> {
        let mut file = ptr::null_mut::<ffi::sqlite3_file>();
        // SAFETY: the fixture owns this live connection. SQLite writes only its current main-file
        // pointer to `file`; the pointer remains inside this sealed Windows-test helper.
        let code = unsafe {
            ffi::sqlite3_file_control(
                self.raw_handle(),
                b"main\0".as_ptr().cast(),
                ffi::SQLITE_FCNTL_FILE_POINTER,
                (&mut file as *mut *mut ffi::sqlite3_file).cast(),
            )
        };
        if code != ffi::SQLITE_OK || file.is_null() {
            return Err("managed barrier main-file pointer unavailable");
        }
        Ok(file)
    }

    pub(super) fn close(self) -> anyhow::Result<ManagedTestVfsCounts> {
        registry_lifecycle::close(self)
    }

    #[cfg(all(test, windows))]
    pub(super) fn close_registry_lifecycle_once(
        mut self,
    ) -> anyhow::Result<ManagedTestRegistryLifecycleCloseOutcome> {
        registry_lifecycle::close_connection_detailed(&mut self)
    }

    #[cfg(all(test, windows))]
    pub(super) fn registry_lifecycle_binding(
        &self,
    ) -> anyhow::Result<ManagedTestLifecycleFaultBinding> {
        registry_lifecycle::lifecycle_binding(self)
    }

    #[cfg(all(test, windows))]
    pub(super) fn registry_lifecycle_route_observer(
        &self,
    ) -> anyhow::Result<ManagedTestRegistryLifecycleRouteObserver> {
        registry_lifecycle::route_observer(self)
    }

    #[cfg(all(test, windows))]
    pub(super) fn retain_outstanding_journal_sidecar(
        &self,
        runtime: &Arc<PinnedManagedSqliteWalRuntime>,
    ) -> anyhow::Result<()> {
        registry_lifecycle::retain_outstanding_journal_sidecar(self, runtime)
    }

    fn close_connection(&mut self) -> anyhow::Result<ManagedTestVfsCounts> {
        registry_lifecycle::close_connection(self)
    }

    fn cancel_open(mut self, open_error: anyhow::Error) -> anyhow::Error {
        match self.close_connection() {
            Ok(_) => open_error,
            Err(close_error) => open_error.context(format!(
                "managed SQLite open cancellation retained custody: {close_error}"
            )),
        }
    }

    fn install_authorizer(&mut self) -> anyhow::Result<()> {
        let context = self.authorizer.as_deref_mut().expect("managed authorizer")
            as *mut ManagedVfsAuthorizerContext;
        // SAFETY: the boxed context remains stable until explicit uninstall or permanent retain.
        let code = unsafe {
            ffi::sqlite3_set_authorizer(
                self.raw_handle(),
                Some(managed_authorizer_callback),
                context.cast::<c_void>(),
            )
        };
        if code == ffi::SQLITE_OK {
            self.authorizer_installed = true;
            Ok(())
        } else {
            Err(anyhow!(
                "install managed VFS authorizer failed with SQLite code {code}"
            ))
        }
    }

    fn uninstall_authorizer(&mut self) -> anyhow::Result<()> {
        if !self.authorizer_installed {
            return Ok(());
        }
        if self.connection.is_none() || self.authorizer.is_none() {
            return Err(anyhow!(
                "managed VFS authorizer ownership missing while callback is installed"
            ));
        }
        // SAFETY: the fixture still owns the live connection and callback context.
        let code = unsafe { ffi::sqlite3_set_authorizer(self.raw_handle(), None, ptr::null_mut()) };
        if code == ffi::SQLITE_OK {
            self.authorizer_installed = false;
            Ok(())
        } else {
            if let Some(context) = self.authorizer.take() {
                Box::leak(context);
            }
            self.authorizer_installed = false;
            Err(anyhow!(
                "uninstall managed VFS authorizer failed with SQLite code {code}; context retained"
            ))
        }
    }

    fn raw_handle(&self) -> *mut ffi::sqlite3 {
        // SAFETY: used only while this fixture owns the live Connection.
        unsafe { self.connection().handle() }
    }
}

#[cfg(all(test, windows))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ManagedTestVoidCallbackObservation {
    pub(super) before: HandleBoundSqliteAbiRawSlotSnapshot,
    pub(super) after: HandleBoundSqliteAbiRawSlotSnapshot,
}

impl Drop for ManagedSqliteRoutedConnectionFixture {
    fn drop(&mut self) {
        if self.connection.is_some() {
            let _ = self.close_connection();
        }
    }
}
