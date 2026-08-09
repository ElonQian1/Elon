//! Safe, dormant projections of SQLite 3.45 callback inputs.
//!
//! These modules contain no raw pointer dereference, FFI callback, connection, VFS
//! registration, filesystem operation, or SQLite open. A future ABI layer must first convert
//! SQLite-owned pointers into bounded borrowed byte slices, then call these projections.

mod authorizer;
mod types;
mod vfs_requests;

pub(super) use authorizer::ManagedSqliteAuthorizerAbiAdapter;
pub(super) use types::{
    ManagedSqliteAuthorizerAbiRejection, ManagedSqliteAuthorizerRawField,
    ManagedSqliteRawAuthorizerRequest, ManagedSqliteVfsAccessRequest,
    ManagedSqliteVfsAccessRequestRejection, ManagedSqliteVfsDeleteRequest,
    ManagedSqliteVfsDeleteRequestRejection, ManagedSqliteVfsFullPathnameRequest,
    ManagedSqliteVfsFullPathnameRequestRejection,
};
pub(super) use vfs_requests::ManagedSqliteVfsRequestAbiAdapter;
