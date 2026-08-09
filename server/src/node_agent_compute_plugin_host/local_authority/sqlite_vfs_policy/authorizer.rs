const AUTHORITY_DATABASE_VERSION: &str = "7";
const AUTHORITY_APPLICATION_ID: &str = "1162625872";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::node_agent_compute_plugin_host::local_authority) enum ManagedSqliteAuthorizerDecision
{
    Allow,
    Deny,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::node_agent_compute_plugin_host::local_authority) enum ManagedSqliteTempSchemaAction {
    CreateIndex,
    CreateTable,
    CreateTrigger,
    CreateView,
    DropIndex,
    DropTable,
    DropTrigger,
    DropView,
}

/// ABI-neutral projection of the SQLite authorizer actions used by this policy.
///
/// A future callback adapter must map missing strings, invalid UTF-8, and unrecognized raw action
/// codes to `Unknown`; it must never guess a known-safe action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::node_agent_compute_plugin_host::local_authority) enum ManagedSqliteAuthorizerAction<
    'a,
> {
    CreateIndex,
    CreateTable,
    CreateTrigger,
    CreateView,
    Delete,
    DropIndex,
    DropTable,
    DropTrigger,
    DropView,
    Insert,
    Read,
    Select,
    Transaction,
    Update,
    AlterTable,
    Reindex,
    Analyze,
    Function {
        name: Option<&'a str>,
    },
    Savepoint,
    Recursive,
    CreateVirtualTable,
    DropVirtualTable,
    Attach,
    Detach,
    TempSchema(ManagedSqliteTempSchemaAction),
    Pragma {
        name: Option<&'a str>,
        value: Option<&'a str>,
    },
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::node_agent_compute_plugin_host::local_authority) struct ManagedSqliteAuthorizerRequest<
    'a,
> {
    action: ManagedSqliteAuthorizerAction<'a>,
    database_name: Option<&'a str>,
    accessor: Option<&'a str>,
}

impl<'a> ManagedSqliteAuthorizerRequest<'a> {
    pub(in crate::node_agent_compute_plugin_host::local_authority) fn new(
        action: ManagedSqliteAuthorizerAction<'a>,
        database_name: Option<&'a str>,
        accessor: Option<&'a str>,
    ) -> Self {
        Self {
            action,
            database_name,
            accessor,
        }
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority) fn accessor(
        &self,
    ) -> Option<&'a str> {
        self.accessor
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ManagedSqliteAuthorizerPhase {
    Bootstrap,
    SchemaMigration,
    Runtime,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::node_agent_compute_plugin_host::local_authority) enum ManagedSqliteAuthorizerTransitionError
{
    InvalidPhaseTransition,
}

/// Linear phase policy. It is intentionally neither `Clone` nor `Copy`.
pub(super) struct ManagedSqliteAuthorizerPolicy {
    phase: ManagedSqliteAuthorizerPhase,
}

impl ManagedSqliteAuthorizerPolicy {
    pub(super) fn bootstrap() -> Self {
        Self {
            phase: ManagedSqliteAuthorizerPhase::Bootstrap,
        }
    }

    pub(super) fn phase(&self) -> ManagedSqliteAuthorizerPhase {
        self.phase
    }

    pub(super) fn enter_schema_migration(
        &mut self,
    ) -> Result<(), ManagedSqliteAuthorizerTransitionError> {
        if self.phase != ManagedSqliteAuthorizerPhase::Bootstrap {
            return Err(ManagedSqliteAuthorizerTransitionError::InvalidPhaseTransition);
        }
        self.phase = ManagedSqliteAuthorizerPhase::SchemaMigration;
        Ok(())
    }

    pub(super) fn enter_runtime(&mut self) -> Result<(), ManagedSqliteAuthorizerTransitionError> {
        if self.phase != ManagedSqliteAuthorizerPhase::SchemaMigration {
            return Err(ManagedSqliteAuthorizerTransitionError::InvalidPhaseTransition);
        }
        self.phase = ManagedSqliteAuthorizerPhase::Runtime;
        Ok(())
    }

    pub(super) fn into_schema_migration(
        mut self,
    ) -> Result<Self, ManagedSqliteAuthorizerTransitionError> {
        self.enter_schema_migration()?;
        Ok(self)
    }

    pub(super) fn into_runtime(mut self) -> Result<Self, ManagedSqliteAuthorizerTransitionError> {
        self.enter_runtime()?;
        Ok(self)
    }

    pub(super) fn authorize(
        &self,
        request: ManagedSqliteAuthorizerRequest<'_>,
    ) -> ManagedSqliteAuthorizerDecision {
        use ManagedSqliteAuthorizerAction as Action;
        use ManagedSqliteAuthorizerDecision::{Allow, Deny};

        if !has_exact_database_scope(request.action, request.database_name) {
            return Deny;
        }

        let allowed = match request.action {
            Action::Attach
            | Action::Detach
            | Action::TempSchema(_)
            | Action::CreateVirtualTable
            | Action::DropVirtualTable
            | Action::Unknown => false,
            Action::Pragma { name, value } => {
                is_allowed_pragma(self.phase, request.database_name, name, value)
            }
            action => match self.phase {
                ManagedSqliteAuthorizerPhase::Bootstrap => false,
                ManagedSqliteAuthorizerPhase::SchemaMigration => {
                    is_schema_action(action) || is_runtime_action(action)
                }
                ManagedSqliteAuthorizerPhase::Runtime => is_runtime_action(action),
            },
        };
        if allowed {
            Allow
        } else {
            Deny
        }
    }
}

fn has_exact_database_scope(
    action: ManagedSqliteAuthorizerAction<'_>,
    database_name: Option<&str>,
) -> bool {
    use ManagedSqliteAuthorizerAction as Action;

    match action {
        Action::CreateIndex
        | Action::CreateTable
        | Action::CreateTrigger
        | Action::CreateView
        | Action::Delete
        | Action::DropIndex
        | Action::DropTable
        | Action::DropTrigger
        | Action::DropView
        | Action::Insert
        | Action::Read
        | Action::Update
        | Action::AlterTable
        | Action::Reindex
        | Action::Analyze
        | Action::CreateVirtualTable
        | Action::DropVirtualTable => database_name.is_some_and(|database| database == "main"),
        Action::Select
        | Action::Transaction
        | Action::Function { .. }
        | Action::Savepoint
        | Action::Recursive
        | Action::Attach
        | Action::Detach => database_name.is_none(),
        Action::Pragma { .. } => database_name.is_none_or(|database| database == "main"),
        Action::TempSchema(_) | Action::Unknown => false,
    }
}

fn is_schema_action(action: ManagedSqliteAuthorizerAction<'_>) -> bool {
    use ManagedSqliteAuthorizerAction as Action;

    matches!(
        action,
        Action::CreateIndex
            | Action::CreateTable
            | Action::CreateTrigger
            | Action::CreateView
            | Action::DropIndex
            | Action::DropTable
            | Action::DropTrigger
            | Action::DropView
            | Action::AlterTable
            | Action::Reindex
            | Action::Analyze
    )
}

fn is_runtime_action(action: ManagedSqliteAuthorizerAction<'_>) -> bool {
    use ManagedSqliteAuthorizerAction as Action;

    match action {
        Action::Delete
        | Action::Insert
        | Action::Read
        | Action::Select
        | Action::Transaction
        | Action::Update
        | Action::Savepoint
        | Action::Recursive => true,
        Action::Function { name } => is_allowed_function(name),
        Action::CreateIndex
        | Action::CreateTable
        | Action::CreateTrigger
        | Action::CreateView
        | Action::DropIndex
        | Action::DropTable
        | Action::DropTrigger
        | Action::DropView
        | Action::AlterTable
        | Action::Reindex
        | Action::Analyze
        | Action::CreateVirtualTable
        | Action::DropVirtualTable
        | Action::Attach
        | Action::Detach
        | Action::TempSchema(_)
        | Action::Pragma { .. }
        | Action::Unknown => false,
    }
}

fn is_allowed_function(name: Option<&str>) -> bool {
    // `load_extension` and every unlisted function remain denied by construction.
    const ALLOWED: &[&str] = &[
        "changes",
        "coalesce",
        "count",
        "glob",
        "instr",
        "json_array_length",
        "json_each",
        "json_extract",
        "json_type",
        "json_valid",
        "length",
        "like",
        "max",
        "min",
        "raise",
        "sum",
        "trim",
    ];

    name.is_some_and(|actual| {
        actual.is_ascii()
            && ALLOWED
                .iter()
                .any(|allowed| actual.eq_ignore_ascii_case(allowed))
    })
}

fn is_allowed_pragma(
    phase: ManagedSqliteAuthorizerPhase,
    database_name: Option<&str>,
    name: Option<&str>,
    value: Option<&str>,
) -> bool {
    if database_name.is_some_and(|database| database != "main") {
        return false;
    }
    let Some(name) = name else {
        return false;
    };
    match phase {
        ManagedSqliteAuthorizerPhase::Bootstrap => match_pragma(
            name,
            value,
            &[
                ("journal_mode", Some("WAL")),
                ("journal_mode", None),
                ("synchronous", Some("FULL")),
                ("synchronous", None),
                ("foreign_keys", Some("ON")),
                ("foreign_keys", None),
                ("trusted_schema", Some("OFF")),
                ("trusted_schema", None),
                ("temp_store", Some("MEMORY")),
                ("temp_store", None),
                ("mmap_size", Some("0")),
                ("mmap_size", None),
            ],
        ),
        ManagedSqliteAuthorizerPhase::SchemaMigration => match_pragma(
            name,
            value,
            &[
                ("user_version", None),
                ("user_version", Some(AUTHORITY_DATABASE_VERSION)),
                ("application_id", None),
                ("application_id", Some(AUTHORITY_APPLICATION_ID)),
                ("foreign_key_check", None),
            ],
        ),
        ManagedSqliteAuthorizerPhase::Runtime => false,
    }
}

fn match_pragma(name: &str, value: Option<&str>, allowed: &[(&str, Option<&str>)]) -> bool {
    allowed.iter().any(|(allowed_name, allowed_value)| {
        name.eq_ignore_ascii_case(allowed_name)
            && match (value, allowed_value) {
                (None, None) => true,
                (Some(actual), Some(expected)) => actual.eq_ignore_ascii_case(expected),
                _ => false,
            }
    })
}
