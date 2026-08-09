//! Real Connection fixture for the dormant SQLite authority boundary.
//!
//! The transport VFS is a test-only alias over SQLite's default VFS. The authorizer projection,
//! phase policy and connection hardening are the project implementations under test. Nothing in
//! this module is present in production builds.

use std::{
    ffi::CStr,
    os::raw::{c_char, c_int, c_void},
    panic::{catch_unwind, AssertUnwindSafe},
    path::Path,
    ptr,
};

use anyhow::{anyhow, Context};
use rusqlite::{ffi, Connection, OpenFlags};

use super::{ManagedSqliteAuthorizerAbiAdapter, ManagedSqliteRawAuthorizerRequest};
use crate::node_agent_compute_plugin_host::local_authority::{
    sqlite_vfs_abi::{
        ensure_test_transport_vfs, test_transport_open_count, TEST_TRANSPORT_VFS_NAME,
    },
    sqlite_vfs_policy::{
        registry::{ManagedSqliteRegistryProcessOwner, ManagedSqliteRegistryRouteHandle},
        ManagedSqliteAuthorizerDecision, ManagedSqliteRegistryCustody,
        ManagedSqliteRegistryNonceSource,
    },
};

const MAX_AUTHORIZER_FIELD_BYTES: usize = 4 * 1024;

struct TestCustody;

impl ManagedSqliteRegistryCustody for TestCustody {
    fn ensure_registry_current(&self) -> anyhow::Result<()> {
        Ok(())
    }
}

struct FixedNonceSource([u8; 16]);

impl ManagedSqliteRegistryNonceSource for FixedNonceSource {
    fn fill_nonce(&self, output: &mut [u8; 16]) -> Result<(), ()> {
        *output = self.0;
        Ok(())
    }
}

type TestProcessOwner = ManagedSqliteRegistryProcessOwner<TestCustody, FixedNonceSource>;

struct AuthorizerContext {
    owner: &'static TestProcessOwner,
    route: ManagedSqliteRegistryRouteHandle,
}

impl AuthorizerContext {
    fn new(owner: &'static TestProcessOwner, route: ManagedSqliteRegistryRouteHandle) -> Self {
        Self { owner, route }
    }

    fn into_schema_migration(&self) -> anyhow::Result<()> {
        self.owner
            .enter_schema_migration(self.route)
            .map_err(|reason| anyhow!("enter schema migration failed: {reason:?}"))
    }

    fn into_runtime(&self) -> anyhow::Result<()> {
        self.owner
            .enter_runtime(self.route)
            .map_err(|reason| anyhow!("enter runtime failed: {reason:?}"))
    }

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
        let raw = ManagedSqliteRawAuthorizerRequest::new(
            action_code,
            argument_one,
            argument_two,
            argument_three,
            accessor,
        );
        let request = match ManagedSqliteAuthorizerAbiAdapter::project(raw) {
            Ok(request) => request,
            Err(_) => return ffi::SQLITE_DENY,
        };
        match self.owner.authorize_sql(self.route, request) {
            Ok(ManagedSqliteAuthorizerDecision::Allow) => ffi::SQLITE_OK,
            Ok(ManagedSqliteAuthorizerDecision::Deny) | Err(_) => ffi::SQLITE_DENY,
        }
    }
}

unsafe fn bounded_bytes<'a>(value: *const c_char) -> Result<Option<&'a [u8]>, ()> {
    if value.is_null() {
        return Ok(None);
    }
    // SAFETY: caller forwards a non-null SQLite-owned authorizer string.
    let bytes = unsafe { CStr::from_ptr(value) }.to_bytes();
    if bytes.len() > MAX_AUTHORIZER_FIELD_BYTES {
        return Err(());
    }
    Ok(Some(bytes))
}

unsafe extern "C" fn authorizer_callback(
    context: *mut c_void,
    action_code: c_int,
    argument_one: *const c_char,
    argument_two: *const c_char,
    argument_three: *const c_char,
    accessor: *const c_char,
) -> c_int {
    catch_unwind(AssertUnwindSafe(|| {
        let Some(context) = context.cast::<AuthorizerContext>().as_ref() else {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ConnectionSecuritySnapshot {
    defensive: c_int,
    trusted_schema: c_int,
    dqs_dml: c_int,
    dqs_ddl: c_int,
    load_extension: c_int,
    attached_limit: c_int,
    worker_thread_limit: c_int,
}

struct ManagedSqliteConnectionFixture {
    connection: Option<Connection>,
    authorizer: Option<Box<AuthorizerContext>>,
}

impl ManagedSqliteConnectionFixture {
    fn open(path: &Path, nonce: [u8; 16]) -> anyhow::Result<Self> {
        ensure_test_transport_vfs().map_err(|code| {
            anyhow!("register test transport VFS failed with SQLite code {code}")
        })?;
        let flags = OpenFlags::SQLITE_OPEN_READ_WRITE
            | OpenFlags::SQLITE_OPEN_CREATE
            | OpenFlags::SQLITE_OPEN_FULL_MUTEX
            | OpenFlags::SQLITE_OPEN_PRIVATE_CACHE
            | OpenFlags::SQLITE_OPEN_NOFOLLOW;
        let connection = Connection::open_with_flags_and_vfs(path, flags, TEST_TRANSPORT_VFS_NAME)
            .with_context(|| format!("open SQLite test connection at {}", path.display()))?;
        configure_connection(&connection)?;
        let owner = ManagedSqliteRegistryProcessOwner::leak(FixedNonceSource(nonce));
        let route = owner
            .register(TestCustody)
            .map_err(|failure| anyhow!("register test authorizer route failed: {failure:?}"))?;
        let mut fixture = Self {
            connection: Some(connection),
            authorizer: Some(Box::new(AuthorizerContext::new(owner, route))),
        };
        fixture.install_authorizer()?;
        Ok(fixture)
    }

    fn connection(&self) -> &Connection {
        self.connection.as_ref().expect("fixture connection")
    }

    fn into_schema_migration(&mut self) -> anyhow::Result<()> {
        self.authorizer
            .as_deref_mut()
            .expect("fixture authorizer")
            .into_schema_migration()
    }

    fn into_runtime(&mut self) -> anyhow::Result<()> {
        self.authorizer
            .as_deref_mut()
            .expect("fixture authorizer")
            .into_runtime()
    }

    fn security_snapshot(&self) -> anyhow::Result<ConnectionSecuritySnapshot> {
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

    fn close(mut self) -> anyhow::Result<()> {
        let uninstall = self.uninstall_authorizer();
        if let Err(code) = uninstall {
            if let Some(context) = self.authorizer.take() {
                Box::leak(context);
            }
            let connection = self.connection.take().expect("fixture connection");
            let _ = connection.close();
            return Err(anyhow!(
                "uninstall authorizer failed with SQLite code {code}; context retained"
            ));
        }
        let context = self.authorizer.take().expect("fixture authorizer");
        let owner = context.owner;
        let route = context.route;
        drop(context);
        let connection = self.connection.take().expect("fixture connection");
        connection
            .close()
            .map_err(|(_, error)| anyhow!("close SQLite test connection: {error}"))?;
        owner
            .retire_pending_for_test(route)
            .map_err(|reason| anyhow!("retire test authorizer route failed: {reason:?}"))?;
        Ok(())
    }

    fn install_authorizer(&mut self) -> anyhow::Result<()> {
        let db = self.raw_handle();
        let context =
            self.authorizer.as_deref_mut().expect("fixture authorizer") as *mut AuthorizerContext;
        // SAFETY: the boxed context is stable until the callback is uninstalled or permanently
        // retained. The fixture owns the connection for the same interval.
        let code = unsafe {
            ffi::sqlite3_set_authorizer(db, Some(authorizer_callback), context.cast::<c_void>())
        };
        if code == ffi::SQLITE_OK {
            Ok(())
        } else {
            Err(anyhow!("install authorizer failed with SQLite code {code}"))
        }
    }

    fn uninstall_authorizer(&mut self) -> Result<(), c_int> {
        if self.connection.is_none() || self.authorizer.is_none() {
            return Ok(());
        }
        // SAFETY: the fixture still owns a valid live connection and context.
        let code = unsafe { ffi::sqlite3_set_authorizer(self.raw_handle(), None, ptr::null_mut()) };
        if code == ffi::SQLITE_OK {
            Ok(())
        } else {
            Err(code)
        }
    }

    fn raw_handle(&self) -> *mut ffi::sqlite3 {
        // SAFETY: the pointer is used only while the fixture exclusively owns the Connection.
        unsafe { self.connection().handle() }
    }
}

impl Drop for ManagedSqliteConnectionFixture {
    fn drop(&mut self) {
        if self.uninstall_authorizer().is_err() {
            if let Some(context) = self.authorizer.take() {
                Box::leak(context);
            }
        }
    }
}

fn configure_connection(connection: &Connection) -> anyhow::Result<()> {
    // SAFETY: all calls are connection-local and happen before any statement is prepared.
    let db = unsafe { connection.handle() };
    for (operation, value) in [
        (ffi::SQLITE_DBCONFIG_ENABLE_LOAD_EXTENSION, 0),
        (ffi::SQLITE_DBCONFIG_DEFENSIVE, 1),
        (ffi::SQLITE_DBCONFIG_TRUSTED_SCHEMA, 0),
        (ffi::SQLITE_DBCONFIG_DQS_DML, 0),
        (ffi::SQLITE_DBCONFIG_DQS_DDL, 0),
    ] {
        set_db_config(db, operation, value)?;
    }
    // SAFETY: disabling extension loading on this valid connection narrows authority.
    let code = unsafe { ffi::sqlite3_enable_load_extension(db, 0) };
    if code != ffi::SQLITE_OK {
        return Err(anyhow!(
            "disable extension loading failed with SQLite code {code}"
        ));
    }
    // SAFETY: both limits are scoped to this valid connection.
    unsafe {
        ffi::sqlite3_limit(db, ffi::SQLITE_LIMIT_ATTACHED, 0);
        ffi::sqlite3_limit(db, ffi::SQLITE_LIMIT_WORKER_THREADS, 0);
    }
    Ok(())
}

fn set_db_config(db: *mut ffi::sqlite3, operation: c_int, value: c_int) -> anyhow::Result<()> {
    let mut observed = -1;
    // SAFETY: these db_config operations all use the documented `(int, int*)` varargs shape.
    let code = unsafe { ffi::sqlite3_db_config(db, operation, value, &mut observed) };
    if code != ffi::SQLITE_OK || observed != value {
        return Err(anyhow!(
            "SQLite db_config {operation}={value} failed: code={code}, observed={observed}"
        ));
    }
    Ok(())
}

fn read_db_config(db: *mut ffi::sqlite3, operation: c_int) -> anyhow::Result<c_int> {
    let mut observed = -1;
    // SAFETY: `-1` reads without changing these `(int, int*)` db_config operations.
    let code = unsafe { ffi::sqlite3_db_config(db, operation, -1, &mut observed) };
    if code != ffi::SQLITE_OK {
        return Err(anyhow!(
            "read SQLite db_config {operation} failed with code {code}"
        ));
    }
    Ok(observed)
}

#[cfg(test)]
mod tests;
