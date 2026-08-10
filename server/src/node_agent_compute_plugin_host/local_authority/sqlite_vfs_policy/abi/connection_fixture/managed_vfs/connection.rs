use std::{
    mem,
    os::raw::{c_char, c_int, c_void},
    panic::{catch_unwind, AssertUnwindSafe},
    path::Path,
    ptr,
    sync::{atomic::AtomicUsize, Arc},
};

use anyhow::{anyhow, Context};
use rusqlite::{ffi, Connection, OpenFlags};

use super::*;

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

    pub(super) fn close(mut self) -> anyhow::Result<ManagedTestVfsCounts> {
        let counts = self.close_connection()?;
        if let Some(registration) = self.registration.take() {
            registration.unregister()?;
        }
        Ok(counts)
    }

    fn close_connection(&mut self) -> anyhow::Result<ManagedTestVfsCounts> {
        if let Err(error) = self.uninstall_authorizer() {
            if let Some(connection) = self.connection.take() {
                mem::forget(connection);
            }
            return Err(error);
        }
        drop(self.authorizer.take());
        let connection = self
            .connection
            .take()
            .ok_or_else(|| anyhow!("managed routed SQLite connection already consumed"))?;
        if let Err((connection, error)) = connection.close() {
            mem::forget(connection);
            return Err(anyhow!(
                "close managed routed SQLite connection: {error}; connection retained"
            ));
        }
        let route_entry = self
            .route_entry
            .as_ref()
            .expect("managed fixture route entry");
        let logical_removal = self.routes.retire_closed_route(route_entry)?;
        drop(logical_removal);
        self.route_entry.take();
        Ok(self.counters.snapshot())
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

impl Drop for ManagedSqliteRoutedConnectionFixture {
    fn drop(&mut self) {
        if self.connection.is_some() {
            let _ = self.close_connection();
        }
    }
}
