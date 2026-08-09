use std::{
    os::raw::{c_char, c_int, c_void},
    panic::{catch_unwind, AssertUnwindSafe},
    path::Path,
    ptr,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
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
    registration: Option<ManagedTestVfsRegistration>,
    route: Arc<TestRoute>,
    custody_drops: Arc<AtomicUsize>,
}

impl ManagedSqliteRoutedConnectionFixture {
    pub(super) fn open(root: &Path, nonce: [u8; 16]) -> anyhow::Result<Self> {
        let custody_drops = Arc::new(AtomicUsize::new(0));
        let owner = ManagedSqliteRegistryProcessOwner::leak(FixedNonceSource(nonce));
        let route = Arc::new(
            TestRoute::register(owner, TestCustody::tracked(Arc::clone(&custody_drops)))
                .map_err(|()| anyhow!("register managed VFS route"))?,
        );
        let registration = ManagedTestVfsRegistration::register(root, Arc::clone(&route))?;
        let logical_name = route
            .main_logical_name()
            .map_err(|()| anyhow!("read managed VFS logical main name"))?;
        let logical_name = logical_name
            .to_str()
            .context("managed VFS logical main name is UTF-8")?;
        let flags = OpenFlags::SQLITE_OPEN_READ_WRITE
            | OpenFlags::SQLITE_OPEN_CREATE
            | OpenFlags::SQLITE_OPEN_FULL_MUTEX
            | OpenFlags::SQLITE_OPEN_PRIVATE_CACHE
            | OpenFlags::SQLITE_OPEN_NOFOLLOW;
        let connection =
            match Connection::open_with_flags_and_vfs(logical_name, flags, registration.name()?) {
                Ok(connection) => connection,
                Err(error) => {
                    route.abort_unopened_for_test();
                    return Err(error).context("open managed routed SQLite connection");
                }
            };
        configure_connection(&connection)?;
        let mut fixture = Self {
            connection: Some(connection),
            authorizer: Some(Box::new(ManagedVfsAuthorizerContext {
                route: Arc::clone(&route),
            })),
            registration: Some(registration),
            route,
            custody_drops,
        };
        fixture.install_authorizer()?;
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
        self.registration
            .as_ref()
            .expect("managed VFS registration")
            .counts()
    }

    pub(super) fn close(mut self) -> anyhow::Result<ManagedTestVfsCounts> {
        self.uninstall_authorizer()?;
        drop(self.authorizer.take());
        let connection = self.connection.take().expect("managed fixture connection");
        connection
            .close()
            .map_err(|(_, error)| anyhow!("close managed routed SQLite connection: {error}"))?;
        if self.custody_drops.load(Ordering::SeqCst) != 1 {
            return Err(anyhow!(
                "managed VFS route custody was not retired exactly once"
            ));
        }
        let registration = self.registration.take().expect("managed VFS registration");
        let counts = registration.counts();
        registration
            .unregister()
            .map_err(|code| anyhow!("unregister managed test VFS: SQLite code {code}"))?;
        Ok(counts)
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
            Ok(())
        } else {
            Err(anyhow!(
                "install managed VFS authorizer failed with SQLite code {code}"
            ))
        }
    }

    fn uninstall_authorizer(&mut self) -> anyhow::Result<()> {
        if self.connection.is_none() || self.authorizer.is_none() {
            return Ok(());
        }
        // SAFETY: the fixture still owns the live connection and callback context.
        let code = unsafe { ffi::sqlite3_set_authorizer(self.raw_handle(), None, ptr::null_mut()) };
        if code == ffi::SQLITE_OK {
            Ok(())
        } else {
            if let Some(context) = self.authorizer.take() {
                Box::leak(context);
            }
            Err(anyhow!(
                "uninstall managed VFS authorizer failed with SQLite code {code}"
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
        let _ = self.uninstall_authorizer();
        drop(self.connection.take());
        drop(self.authorizer.take());
        drop(self.registration.take());
    }
}
