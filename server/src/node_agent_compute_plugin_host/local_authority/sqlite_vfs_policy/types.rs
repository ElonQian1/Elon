use rusqlite::ffi;

use super::name::ManagedSqliteLogicalNameRejection;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::node_agent_compute_plugin_host::local_authority) enum ManagedSqliteLogicalFileRole {
    Main,
    Journal,
    Wal,
}

/// The only flags the future root `sqlite3_open_v2` call may receive.
///
/// This is intentionally not a general-purpose bitflag builder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::node_agent_compute_plugin_host::local_authority) struct ManagedSqliteRootOpenFlags {
    bits: i32,
}

impl ManagedSqliteRootOpenFlags {
    pub(super) fn handle_bound_authority() -> Self {
        Self {
            bits: ROOT_CONNECTION_FLAGS as i32,
        }
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority) fn bits(self) -> i32 {
        self.bits
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::node_agent_compute_plugin_host::local_authority) struct ManagedSqliteVfsOpenRequest {
    role: ManagedSqliteLogicalFileRole,
    access: ManagedSqliteVfsAccess,
    create: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::node_agent_compute_plugin_host::local_authority) enum ManagedSqliteVfsAccess {
    ReadOnly,
    ReadWrite,
}

impl ManagedSqliteVfsOpenRequest {
    pub(in crate::node_agent_compute_plugin_host::local_authority) fn role(
        &self,
    ) -> ManagedSqliteLogicalFileRole {
        self.role
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority) fn access(
        &self,
    ) -> ManagedSqliteVfsAccess {
        self.access
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority) fn create(&self) -> bool {
        self.create
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::node_agent_compute_plugin_host::local_authority) enum ManagedSqliteVfsOpenFlagRejection
{
    LogicalName(ManagedSqliteLogicalNameRejection),
    UnknownFlags,
    UriOrMemory,
    SharedCache,
    DeleteOnClose,
    TemporaryOrAuxiliaryObject,
    UnsupportedMutexMode,
    InvalidAccessMode,
    InvalidRoleFlagMatrix,
    ObjectRoleMismatch,
    UnsupportedKnownFlags,
}

/// Checks the flags delivered to `sqlite3_vfs.xOpen`, not the flags supplied directly to
/// `sqlite3_open_v2`. SQLite consumes some root-only flags before calling the VFS and synthesizes
/// sidecar flags itself, so each role has its own exact bundled-SQLite-3.45 matrix. A SQLite
/// upgrade must explicitly revise this matrix instead of widening it for compatibility guesses.
pub(super) struct ManagedSqliteVfsOpenFlagPolicy;

impl ManagedSqliteVfsOpenFlagPolicy {
    pub(super) fn authorize(
        role: ManagedSqliteLogicalFileRole,
        raw_flags: i32,
    ) -> Result<ManagedSqliteVfsOpenRequest, ManagedSqliteVfsOpenFlagRejection> {
        let flags = raw_flags as u32;
        if flags & !KNOWN_FLAGS != 0 {
            return Err(ManagedSqliteVfsOpenFlagRejection::UnknownFlags);
        }
        if flags & (OPEN_URI | OPEN_MEMORY) != 0 {
            return Err(ManagedSqliteVfsOpenFlagRejection::UriOrMemory);
        }
        if flags & OPEN_SHAREDCACHE != 0 {
            return Err(ManagedSqliteVfsOpenFlagRejection::SharedCache);
        }
        if flags & OPEN_DELETEONCLOSE != 0 {
            return Err(ManagedSqliteVfsOpenFlagRejection::DeleteOnClose);
        }
        if flags & TEMPORARY_OR_AUXILIARY_OBJECT_FLAGS != 0 {
            return Err(ManagedSqliteVfsOpenFlagRejection::TemporaryOrAuxiliaryObject);
        }
        if flags & OPEN_NOMUTEX != 0 {
            return Err(ManagedSqliteVfsOpenFlagRejection::UnsupportedMutexMode);
        }
        let access = match (flags & OPEN_READONLY != 0, flags & OPEN_READWRITE != 0) {
            (true, false) => ManagedSqliteVfsAccess::ReadOnly,
            (false, true) => ManagedSqliteVfsAccess::ReadWrite,
            _ => return Err(ManagedSqliteVfsOpenFlagRejection::InvalidAccessMode),
        };
        let create = flags & OPEN_CREATE != 0;

        let expected_object_flag = match role {
            ManagedSqliteLogicalFileRole::Main => OPEN_MAIN_DB,
            ManagedSqliteLogicalFileRole::Journal => OPEN_MAIN_JOURNAL,
            ManagedSqliteLogicalFileRole::Wal => OPEN_WAL,
        };
        if flags & OBJECT_ROLE_FLAGS != expected_object_flag {
            return Err(ManagedSqliteVfsOpenFlagRejection::ObjectRoleMismatch);
        }

        let accepted_bits = match role {
            ManagedSqliteLogicalFileRole::Main => MAIN_VFS_OPEN_FLAGS,
            ManagedSqliteLogicalFileRole::Journal => JOURNAL_VFS_OPEN_FLAG_UNION,
            ManagedSqliteLogicalFileRole::Wal => WAL_VFS_OPEN_FLAGS,
        };
        if flags & !accepted_bits != 0 {
            return Err(ManagedSqliteVfsOpenFlagRejection::UnsupportedKnownFlags);
        }
        let role_matrix_matches = match role {
            ManagedSqliteLogicalFileRole::Main => flags == MAIN_VFS_OPEN_FLAGS,
            ManagedSqliteLogicalFileRole::Journal => matches!(
                flags,
                JOURNAL_VFS_CREATE_FLAGS
                    | JOURNAL_VFS_EXISTING_READ_WRITE_FLAGS
                    | JOURNAL_VFS_EXISTING_READ_ONLY_FLAGS
            ),
            ManagedSqliteLogicalFileRole::Wal => flags == WAL_VFS_OPEN_FLAGS,
        };
        if !role_matrix_matches {
            return Err(ManagedSqliteVfsOpenFlagRejection::InvalidRoleFlagMatrix);
        }

        Ok(ManagedSqliteVfsOpenRequest {
            role,
            access,
            create,
        })
    }
}

const OPEN_READONLY: u32 = ffi::SQLITE_OPEN_READONLY as u32;
const OPEN_READWRITE: u32 = ffi::SQLITE_OPEN_READWRITE as u32;
const OPEN_CREATE: u32 = ffi::SQLITE_OPEN_CREATE as u32;
const OPEN_DELETEONCLOSE: u32 = ffi::SQLITE_OPEN_DELETEONCLOSE as u32;
const OPEN_EXCLUSIVE: u32 = ffi::SQLITE_OPEN_EXCLUSIVE as u32;
const OPEN_AUTOPROXY: u32 = ffi::SQLITE_OPEN_AUTOPROXY as u32;
const OPEN_URI: u32 = ffi::SQLITE_OPEN_URI as u32;
const OPEN_MEMORY: u32 = ffi::SQLITE_OPEN_MEMORY as u32;
const OPEN_MAIN_DB: u32 = ffi::SQLITE_OPEN_MAIN_DB as u32;
const OPEN_TEMP_DB: u32 = ffi::SQLITE_OPEN_TEMP_DB as u32;
const OPEN_TRANSIENT_DB: u32 = ffi::SQLITE_OPEN_TRANSIENT_DB as u32;
const OPEN_MAIN_JOURNAL: u32 = ffi::SQLITE_OPEN_MAIN_JOURNAL as u32;
const OPEN_TEMP_JOURNAL: u32 = ffi::SQLITE_OPEN_TEMP_JOURNAL as u32;
const OPEN_SUBJOURNAL: u32 = ffi::SQLITE_OPEN_SUBJOURNAL as u32;
const OPEN_SUPER_JOURNAL: u32 = ffi::SQLITE_OPEN_SUPER_JOURNAL as u32;
const OPEN_NOMUTEX: u32 = ffi::SQLITE_OPEN_NOMUTEX as u32;
const OPEN_FULLMUTEX: u32 = ffi::SQLITE_OPEN_FULLMUTEX as u32;
const OPEN_SHAREDCACHE: u32 = ffi::SQLITE_OPEN_SHAREDCACHE as u32;
const OPEN_PRIVATECACHE: u32 = ffi::SQLITE_OPEN_PRIVATECACHE as u32;
const OPEN_WAL: u32 = ffi::SQLITE_OPEN_WAL as u32;
const OPEN_NOFOLLOW: u32 = ffi::SQLITE_OPEN_NOFOLLOW as u32;
const OPEN_EXRESCODE: u32 = ffi::SQLITE_OPEN_EXRESCODE as u32;

const ROOT_CONNECTION_FLAGS: u32 = OPEN_READWRITE
    | OPEN_CREATE
    | OPEN_FULLMUTEX
    | OPEN_PRIVATECACHE
    | OPEN_NOFOLLOW
    | OPEN_EXRESCODE;
const MAIN_VFS_OPEN_FLAGS: u32 = OPEN_READWRITE | OPEN_CREATE | OPEN_NOFOLLOW | OPEN_MAIN_DB;
const JOURNAL_VFS_CREATE_FLAGS: u32 = OPEN_READWRITE | OPEN_CREATE | OPEN_MAIN_JOURNAL;
const JOURNAL_VFS_EXISTING_READ_WRITE_FLAGS: u32 = OPEN_READWRITE | OPEN_MAIN_JOURNAL;
const JOURNAL_VFS_EXISTING_READ_ONLY_FLAGS: u32 = OPEN_READONLY | OPEN_MAIN_JOURNAL;
const JOURNAL_VFS_OPEN_FLAG_UNION: u32 =
    JOURNAL_VFS_CREATE_FLAGS | JOURNAL_VFS_EXISTING_READ_ONLY_FLAGS;
const WAL_VFS_OPEN_FLAGS: u32 = OPEN_READWRITE | OPEN_CREATE | OPEN_WAL;
const OBJECT_ROLE_FLAGS: u32 = OPEN_MAIN_DB | OPEN_MAIN_JOURNAL | OPEN_WAL;
const TEMPORARY_OR_AUXILIARY_OBJECT_FLAGS: u32 =
    OPEN_TEMP_DB | OPEN_TRANSIENT_DB | OPEN_TEMP_JOURNAL | OPEN_SUBJOURNAL | OPEN_SUPER_JOURNAL;
const KNOWN_FLAGS: u32 = OPEN_READONLY
    | OPEN_READWRITE
    | OPEN_CREATE
    | OPEN_DELETEONCLOSE
    | OPEN_EXCLUSIVE
    | OPEN_AUTOPROXY
    | OPEN_URI
    | OPEN_MEMORY
    | OPEN_MAIN_DB
    | OPEN_TEMP_DB
    | OPEN_TRANSIENT_DB
    | OPEN_MAIN_JOURNAL
    | OPEN_TEMP_JOURNAL
    | OPEN_SUBJOURNAL
    | OPEN_SUPER_JOURNAL
    | OPEN_NOMUTEX
    | OPEN_FULLMUTEX
    | OPEN_SHAREDCACHE
    | OPEN_PRIVATECACHE
    | OPEN_WAL
    | OPEN_NOFOLLOW
    | OPEN_EXRESCODE;
