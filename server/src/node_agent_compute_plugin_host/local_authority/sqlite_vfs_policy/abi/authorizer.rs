use std::str;

use rusqlite::ffi;

use super::super::{
    ManagedSqliteAuthorizerAction as Action, ManagedSqliteAuthorizerRequest,
    ManagedSqliteTempSchemaAction as TempAction,
};
use super::types::{
    ManagedSqliteAuthorizerAbiRejection as Rejection, ManagedSqliteAuthorizerRawField as Field,
    ManagedSqliteRawAuthorizerRequest,
};

pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy) struct ManagedSqliteAuthorizerAbiAdapter;

impl ManagedSqliteAuthorizerAbiAdapter {
    /// Strictly projects bundled SQLite 3.45 callback values into the policy-neutral action type.
    /// Unknown codes, invalid UTF-8 and any impossible NULL shape are denied by returning `Err`.
    /// A future callback boundary must translate every such rejection to `SQLITE_DENY`.
    pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy) fn project<
        'a,
    >(
        raw: ManagedSqliteRawAuthorizerRequest<'a>,
    ) -> Result<ManagedSqliteAuthorizerRequest<'a>, Rejection> {
        if !is_known_action_code(raw.action_code) {
            return Err(Rejection::UnknownActionCode(raw.action_code));
        }

        let argument_one = decode_optional(raw.argument_one, Field::ArgumentOne)?;
        let argument_two = decode_optional(raw.argument_two, Field::ArgumentTwo)?;
        let argument_three = decode_optional(raw.argument_three, Field::ArgumentThree)?;
        let accessor = decode_optional(raw.accessor, Field::Accessor)?;

        let (action, effective_database) =
            match (raw.action_code, argument_one, argument_two, argument_three) {
                (ffi::SQLITE_CREATE_INDEX, Some(_), Some(_), Some(database)) => {
                    (Action::CreateIndex, Some(database))
                }
                (ffi::SQLITE_CREATE_TABLE, Some(_), None, Some(database)) => {
                    (Action::CreateTable, Some(database))
                }
                (ffi::SQLITE_CREATE_TEMP_INDEX, Some(_), Some(_), Some(database)) => {
                    (Action::TempSchema(TempAction::CreateIndex), Some(database))
                }
                (ffi::SQLITE_CREATE_TEMP_TABLE, Some(_), None, Some(database)) => {
                    (Action::TempSchema(TempAction::CreateTable), Some(database))
                }
                (ffi::SQLITE_CREATE_TEMP_TRIGGER, Some(_), Some(_), Some(database)) => (
                    Action::TempSchema(TempAction::CreateTrigger),
                    Some(database),
                ),
                (ffi::SQLITE_CREATE_TEMP_VIEW, Some(_), None, Some(database)) => {
                    (Action::TempSchema(TempAction::CreateView), Some(database))
                }
                (ffi::SQLITE_CREATE_TRIGGER, Some(_), Some(_), Some(database)) => {
                    (Action::CreateTrigger, Some(database))
                }
                (ffi::SQLITE_CREATE_VIEW, Some(_), None, Some(database)) => {
                    (Action::CreateView, Some(database))
                }
                (ffi::SQLITE_DELETE, Some(_), None, Some(database)) => {
                    (Action::Delete, Some(database))
                }
                (ffi::SQLITE_DROP_INDEX, Some(_), Some(_), Some(database)) => {
                    (Action::DropIndex, Some(database))
                }
                (ffi::SQLITE_DROP_TABLE, Some(_), None, Some(database)) => {
                    (Action::DropTable, Some(database))
                }
                (ffi::SQLITE_DROP_TEMP_INDEX, Some(_), Some(_), Some(database)) => {
                    (Action::TempSchema(TempAction::DropIndex), Some(database))
                }
                (ffi::SQLITE_DROP_TEMP_TABLE, Some(_), None, Some(database)) => {
                    (Action::TempSchema(TempAction::DropTable), Some(database))
                }
                (ffi::SQLITE_DROP_TEMP_TRIGGER, Some(_), Some(_), Some(database)) => {
                    (Action::TempSchema(TempAction::DropTrigger), Some(database))
                }
                (ffi::SQLITE_DROP_TEMP_VIEW, Some(_), None, Some(database)) => {
                    (Action::TempSchema(TempAction::DropView), Some(database))
                }
                (ffi::SQLITE_DROP_TRIGGER, Some(_), Some(_), Some(database)) => {
                    (Action::DropTrigger, Some(database))
                }
                (ffi::SQLITE_DROP_VIEW, Some(_), None, Some(database)) => {
                    (Action::DropView, Some(database))
                }
                (ffi::SQLITE_INSERT, Some(_), None, Some(database)) => {
                    (Action::Insert, Some(database))
                }
                (ffi::SQLITE_PRAGMA, Some(name), value, database) => (
                    Action::Pragma {
                        name: Some(name),
                        value,
                    },
                    database,
                ),
                (ffi::SQLITE_READ, Some(_), Some(_), Some(database)) => {
                    (Action::Read, Some(database))
                }
                (ffi::SQLITE_SELECT, None, None, None) => (Action::Select, None),
                (ffi::SQLITE_TRANSACTION, Some(operation), None, None)
                    if is_transaction_operation(operation) =>
                {
                    (Action::Transaction, None)
                }
                (ffi::SQLITE_UPDATE, Some(_), Some(_), Some(database)) => {
                    (Action::Update, Some(database))
                }
                // Non-literal ATTACH/DETACH expressions reach the 3.45 callback with a NULL arg1.
                // Both actions remain denied by policy after their exact ABI shape is projected.
                (ffi::SQLITE_ATTACH, _, None, None) => (Action::Attach, None),
                (ffi::SQLITE_DETACH, _, None, None) => (Action::Detach, None),
                // SQLite 3.45 passes the database in argument one for ALTER. Argument three is either
                // NULL or, for DROP COLUMN, the affected column name.
                (ffi::SQLITE_ALTER_TABLE, Some(database), Some(_), _) => {
                    (Action::AlterTable, Some(database))
                }
                (ffi::SQLITE_REINDEX, Some(_), None, Some(database)) => {
                    (Action::Reindex, Some(database))
                }
                (ffi::SQLITE_ANALYZE, Some(_), None, Some(database)) => {
                    (Action::Analyze, Some(database))
                }
                (ffi::SQLITE_CREATE_VTABLE, Some(_), Some(_), Some(database)) => {
                    (Action::CreateVirtualTable, Some(database))
                }
                (ffi::SQLITE_DROP_VTABLE, Some(_), Some(_), Some(database)) => {
                    (Action::DropVirtualTable, Some(database))
                }
                (ffi::SQLITE_FUNCTION, None, Some(name), None) => {
                    (Action::Function { name: Some(name) }, None)
                }
                (ffi::SQLITE_SAVEPOINT, Some(operation), Some(_), None)
                    if is_savepoint_operation(operation) =>
                {
                    (Action::Savepoint, None)
                }
                (ffi::SQLITE_RECURSIVE, None, None, None) => (Action::Recursive, None),
                _ => return Err(Rejection::InvalidArgumentShape(raw.action_code)),
            };

        Ok(ManagedSqliteAuthorizerRequest::new(
            action,
            effective_database,
            accessor,
        ))
    }
}

fn decode_optional<'a>(raw: Option<&'a [u8]>, field: Field) -> Result<Option<&'a str>, Rejection> {
    match raw {
        Some(bytes) => str::from_utf8(bytes)
            .map(Some)
            .map_err(|_| Rejection::InvalidUtf8(field)),
        None => Ok(None),
    }
}

fn is_transaction_operation(operation: &str) -> bool {
    matches!(operation, "BEGIN" | "COMMIT" | "ROLLBACK")
}

fn is_savepoint_operation(operation: &str) -> bool {
    matches!(operation, "BEGIN" | "RELEASE" | "ROLLBACK")
}

fn is_known_action_code(code: i32) -> bool {
    matches!(
        code,
        ffi::SQLITE_CREATE_INDEX
            | ffi::SQLITE_CREATE_TABLE
            | ffi::SQLITE_CREATE_TEMP_INDEX
            | ffi::SQLITE_CREATE_TEMP_TABLE
            | ffi::SQLITE_CREATE_TEMP_TRIGGER
            | ffi::SQLITE_CREATE_TEMP_VIEW
            | ffi::SQLITE_CREATE_TRIGGER
            | ffi::SQLITE_CREATE_VIEW
            | ffi::SQLITE_DELETE
            | ffi::SQLITE_DROP_INDEX
            | ffi::SQLITE_DROP_TABLE
            | ffi::SQLITE_DROP_TEMP_INDEX
            | ffi::SQLITE_DROP_TEMP_TABLE
            | ffi::SQLITE_DROP_TEMP_TRIGGER
            | ffi::SQLITE_DROP_TEMP_VIEW
            | ffi::SQLITE_DROP_TRIGGER
            | ffi::SQLITE_DROP_VIEW
            | ffi::SQLITE_INSERT
            | ffi::SQLITE_PRAGMA
            | ffi::SQLITE_READ
            | ffi::SQLITE_SELECT
            | ffi::SQLITE_TRANSACTION
            | ffi::SQLITE_UPDATE
            | ffi::SQLITE_ATTACH
            | ffi::SQLITE_DETACH
            | ffi::SQLITE_ALTER_TABLE
            | ffi::SQLITE_REINDEX
            | ffi::SQLITE_ANALYZE
            | ffi::SQLITE_CREATE_VTABLE
            | ffi::SQLITE_DROP_VTABLE
            | ffi::SQLITE_FUNCTION
            | ffi::SQLITE_SAVEPOINT
            | ffi::SQLITE_RECURSIVE
    )
}
