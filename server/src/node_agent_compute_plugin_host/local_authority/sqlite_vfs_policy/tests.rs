use rusqlite::ffi;

use super::{
    abi::{
        ManagedSqliteAuthorizerAbiAdapter, ManagedSqliteAuthorizerAbiRejection,
        ManagedSqliteAuthorizerRawField, ManagedSqliteRawAuthorizerRequest,
        ManagedSqliteVfsAccessRequestRejection, ManagedSqliteVfsDeleteRequestRejection,
        ManagedSqliteVfsFullPathnameRequestRejection, ManagedSqliteVfsRequestAbiAdapter,
    },
    authorizer::{ManagedSqliteAuthorizerPhase, ManagedSqliteAuthorizerPolicy},
    ManagedSqliteAuthorizerAction as Action, ManagedSqliteAuthorizerDecision as Decision,
    ManagedSqliteAuthorizerRequest, ManagedSqliteAuthorizerTransitionError,
    ManagedSqliteLogicalFileRole as Role, ManagedSqliteLogicalNameRejection,
    ManagedSqliteVfsAccess, ManagedSqliteVfsOpenFlagRejection, SealedHandleBoundSqlitePolicy,
};

mod abi;

fn policy() -> SealedHandleBoundSqlitePolicy {
    SealedHandleBoundSqlitePolicy::from_registry_nonce([0x5a; 16])
        .expect("non-zero test nonce must seal")
}

fn name(policy: &SealedHandleBoundSqlitePolicy, role: Role) -> Vec<u8> {
    policy.logical_name(role).to_bytes().to_vec()
}

fn request<'a>(
    action: Action<'a>,
    database_name: Option<&'a str>,
) -> ManagedSqliteAuthorizerRequest<'a> {
    ManagedSqliteAuthorizerRequest::new(action, database_name, None)
}

fn raw<'a>(
    action_code: i32,
    argument_one: Option<&'a [u8]>,
    argument_two: Option<&'a [u8]>,
    argument_three: Option<&'a [u8]>,
) -> ManagedSqliteRawAuthorizerRequest<'a> {
    ManagedSqliteRawAuthorizerRequest::new(
        action_code,
        argument_one,
        argument_two,
        argument_three,
        None,
    )
}

#[test]
fn logical_names_are_opaque_exact_and_nonce_bound() {
    assert!(matches!(
        SealedHandleBoundSqlitePolicy::from_registry_nonce([0; 16]),
        Err(ManagedSqliteLogicalNameRejection::InvalidRegistryNonce)
    ));

    let policy = policy();
    let main = name(&policy, Role::Main);
    let journal = name(&policy, Role::Journal);
    let wal = name(&policy, Role::Wal);
    assert_eq!(main.len(), "elon-hbsql-v1-".len() + 32);
    assert_eq!(journal, [main.as_slice(), b"-journal"].concat());
    assert_eq!(wal, [main.as_slice(), b"-wal"].concat());
    assert_eq!(policy.classify_logical_name(Some(&main)), Ok(Role::Main));
    assert_eq!(
        policy.classify_logical_name(Some(&journal)),
        Ok(Role::Journal)
    );
    assert_eq!(policy.classify_logical_name(Some(&wal)), Ok(Role::Wal));

    let other = SealedHandleBoundSqlitePolicy::from_registry_nonce([0x6b; 16])
        .expect("second nonce must seal");
    assert_eq!(
        other.classify_logical_name(Some(&main)),
        Err(ManagedSqliteLogicalNameRejection::NotExact)
    );
}

#[test]
fn logical_name_parser_rejects_paths_uris_temporary_and_ambiguous_bytes() {
    let policy = policy();
    for (candidate, expected) in [
        (None, ManagedSqliteLogicalNameRejection::MissingOrTemporary),
        (Some(&b""[..]), ManagedSqliteLogicalNameRejection::Empty),
        (
            Some(&b"name\0tail"[..]),
            ManagedSqliteLogicalNameRejection::EmbeddedNul,
        ),
        (
            Some(&b"FiLe:opaque"[..]),
            ManagedSqliteLogicalNameRejection::UriSyntax,
        ),
        (
            Some(&b"folder/name"[..]),
            ManagedSqliteLogicalNameRejection::PathSyntax,
        ),
        (
            Some(&b"folder\\name"[..]),
            ManagedSqliteLogicalNameRejection::PathSyntax,
        ),
        (
            Some(&b"drive:name"[..]),
            ManagedSqliteLogicalNameRejection::PathSyntax,
        ),
        (
            Some(&b"UPPER"[..]),
            ManagedSqliteLogicalNameRejection::SpecialCharacter,
        ),
    ] {
        assert_eq!(policy.classify_logical_name(candidate), Err(expected));
    }
}

#[test]
fn root_open_flags_are_fixed_to_private_fullmutex_nofollow_mode() {
    let expected = ffi::SQLITE_OPEN_READWRITE
        | ffi::SQLITE_OPEN_CREATE
        | ffi::SQLITE_OPEN_FULLMUTEX
        | ffi::SQLITE_OPEN_PRIVATECACHE
        | ffi::SQLITE_OPEN_NOFOLLOW
        | ffi::SQLITE_OPEN_EXRESCODE;
    assert_eq!(policy().root_open_flags().bits(), expected);
    assert_eq!(expected & ffi::SQLITE_OPEN_URI, 0);
    assert_eq!(expected & ffi::SQLITE_OPEN_SHAREDCACHE, 0);
}

#[test]
fn xopen_accepts_only_the_exact_main_journal_and_wal_matrices() {
    let policy = policy();
    let main = name(&policy, Role::Main);
    let journal = name(&policy, Role::Journal);
    let wal = name(&policy, Role::Wal);
    let main_request = ManagedSqliteVfsRequestAbiAdapter::project_x_open(
        &policy,
        Some(&main),
        ffi::SQLITE_OPEN_READWRITE
            | ffi::SQLITE_OPEN_CREATE
            | ffi::SQLITE_OPEN_NOFOLLOW
            | ffi::SQLITE_OPEN_MAIN_DB,
    )
    .expect("main matrix");
    assert_eq!(main_request.role(), Role::Main);
    assert_eq!(main_request.access(), ManagedSqliteVfsAccess::ReadWrite);
    assert!(main_request.create());

    for (flags, access, create) in [
        (
            ffi::SQLITE_OPEN_READWRITE | ffi::SQLITE_OPEN_CREATE | ffi::SQLITE_OPEN_MAIN_JOURNAL,
            ManagedSqliteVfsAccess::ReadWrite,
            true,
        ),
        (
            ffi::SQLITE_OPEN_READWRITE | ffi::SQLITE_OPEN_MAIN_JOURNAL,
            ManagedSqliteVfsAccess::ReadWrite,
            false,
        ),
        (
            ffi::SQLITE_OPEN_READONLY | ffi::SQLITE_OPEN_MAIN_JOURNAL,
            ManagedSqliteVfsAccess::ReadOnly,
            false,
        ),
    ] {
        let request =
            ManagedSqliteVfsRequestAbiAdapter::project_x_open(&policy, Some(&journal), flags)
                .expect("journal matrix");
        assert_eq!(request.role(), Role::Journal);
        assert_eq!(request.access(), access);
        assert_eq!(request.create(), create);
    }

    let wal_request = ManagedSqliteVfsRequestAbiAdapter::project_x_open(
        &policy,
        Some(&wal),
        ffi::SQLITE_OPEN_READWRITE | ffi::SQLITE_OPEN_CREATE | ffi::SQLITE_OPEN_WAL,
    )
    .expect("wal matrix");
    assert_eq!(wal_request.role(), Role::Wal);
    assert_eq!(wal_request.access(), ManagedSqliteVfsAccess::ReadWrite);
    assert!(wal_request.create());
}

#[test]
fn xopen_rejects_each_unsafe_or_non_exact_flag_class() {
    let policy = policy();
    let main = name(&policy, Role::Main);
    let journal = name(&policy, Role::Journal);
    let base = ffi::SQLITE_OPEN_READWRITE
        | ffi::SQLITE_OPEN_CREATE
        | ffi::SQLITE_OPEN_NOFOLLOW
        | ffi::SQLITE_OPEN_MAIN_DB;
    for (flags, expected) in [
        (
            base | 0x4000_0000,
            ManagedSqliteVfsOpenFlagRejection::UnknownFlags,
        ),
        (
            base | ffi::SQLITE_OPEN_URI,
            ManagedSqliteVfsOpenFlagRejection::UriOrMemory,
        ),
        (
            base | ffi::SQLITE_OPEN_MEMORY,
            ManagedSqliteVfsOpenFlagRejection::UriOrMemory,
        ),
        (
            base | ffi::SQLITE_OPEN_SHAREDCACHE,
            ManagedSqliteVfsOpenFlagRejection::SharedCache,
        ),
        (
            base | ffi::SQLITE_OPEN_DELETEONCLOSE,
            ManagedSqliteVfsOpenFlagRejection::DeleteOnClose,
        ),
        (
            base | ffi::SQLITE_OPEN_TEMP_DB,
            ManagedSqliteVfsOpenFlagRejection::TemporaryOrAuxiliaryObject,
        ),
        (
            base | ffi::SQLITE_OPEN_NOMUTEX,
            ManagedSqliteVfsOpenFlagRejection::UnsupportedMutexMode,
        ),
        (
            base | ffi::SQLITE_OPEN_READONLY,
            ManagedSqliteVfsOpenFlagRejection::InvalidAccessMode,
        ),
        (
            base | ffi::SQLITE_OPEN_MAIN_JOURNAL,
            ManagedSqliteVfsOpenFlagRejection::ObjectRoleMismatch,
        ),
        (
            base | ffi::SQLITE_OPEN_FULLMUTEX,
            ManagedSqliteVfsOpenFlagRejection::UnsupportedKnownFlags,
        ),
    ] {
        assert_eq!(
            ManagedSqliteVfsRequestAbiAdapter::project_x_open(&policy, Some(&main), flags),
            Err(expected)
        );
    }

    let invalid_journal_matrix =
        ffi::SQLITE_OPEN_READONLY | ffi::SQLITE_OPEN_CREATE | ffi::SQLITE_OPEN_MAIN_JOURNAL;
    assert_eq!(
        ManagedSqliteVfsRequestAbiAdapter::project_x_open(
            &policy,
            Some(&journal),
            invalid_journal_matrix,
        ),
        Err(ManagedSqliteVfsOpenFlagRejection::InvalidRoleFlagMatrix)
    );
}

#[test]
fn bootstrap_authorizer_allows_only_exact_initialization_pragmas() {
    let policy = policy();
    assert_eq!(
        policy.authorize_sql(request(
            Action::Pragma {
                name: Some("journal_mode"),
                value: Some("wal"),
            },
            Some("main"),
        )),
        Decision::Allow
    );
    assert_eq!(
        policy.authorize_sql(request(
            Action::Pragma {
                name: Some("journal_mode"),
                value: Some("DELETE"),
            },
            Some("main"),
        )),
        Decision::Deny
    );
    assert_eq!(
        policy.authorize_sql(request(Action::Read, Some("main"))),
        Decision::Deny
    );
    assert_eq!(
        policy.authorize_sql(request(
            Action::Pragma {
                name: Some("trusted_schema"),
                value: Some("OFF"),
            },
            Some("temp"),
        )),
        Decision::Deny
    );
}

#[test]
fn authorizer_phase_transition_is_linear_and_strictly_reduces_privilege() {
    assert!(matches!(
        ManagedSqliteAuthorizerPolicy::bootstrap().into_runtime(),
        Err(ManagedSqliteAuthorizerTransitionError::InvalidPhaseTransition)
    ));
    let schema = policy().into_schema_migration().expect("schema phase");
    assert_eq!(
        schema.authorize_sql(request(Action::CreateTable, Some("main"))),
        Decision::Allow
    );
    assert_eq!(
        schema.authorize_sql(request(
            Action::Pragma {
                name: Some("user_version"),
                value: Some("7"),
            },
            Some("main"),
        )),
        Decision::Allow
    );
    assert_eq!(
        schema.authorize_sql(request(Action::CreateVirtualTable, Some("main"))),
        Decision::Deny
    );
    assert_eq!(
        schema.authorize_sql(request(Action::Attach, None)),
        Decision::Deny
    );

    let runtime = schema.into_runtime().expect("runtime phase");
    assert_eq!(
        runtime.authorize_sql(request(Action::Read, Some("main"))),
        Decision::Allow
    );
    assert_eq!(
        runtime.authorize_sql(request(Action::CreateTable, Some("main"))),
        Decision::Deny
    );
    assert_eq!(
        runtime.authorize_sql(request(
            Action::Pragma {
                name: Some("user_version"),
                value: None,
            },
            Some("main"),
        )),
        Decision::Deny
    );
}

#[test]
fn runtime_function_allowlist_is_ascii_exact_and_blocks_extension_loading() {
    let runtime = policy()
        .into_schema_migration()
        .expect("schema")
        .into_runtime()
        .expect("runtime");
    for allowed in ["count", "JSON_EXTRACT", "trim"] {
        assert_eq!(
            runtime.authorize_sql(request(
                Action::Function {
                    name: Some(allowed)
                },
                None
            )),
            Decision::Allow
        );
    }
    for denied in ["load_extension", "random", "count\u{301}"] {
        assert_eq!(
            runtime.authorize_sql(request(Action::Function { name: Some(denied) }, None)),
            Decision::Deny
        );
    }
    assert_eq!(
        runtime.authorize_sql(request(Action::Function { name: None }, None)),
        Decision::Deny
    );
    assert_eq!(
        runtime.authorize_sql(request(Action::Read, Some("temp"))),
        Decision::Deny
    );
}

#[test]
fn direct_authorizer_policy_rejects_invalid_phase_reentry() {
    let schema = ManagedSqliteAuthorizerPolicy::bootstrap()
        .into_schema_migration()
        .expect("schema");
    assert_eq!(
        schema.phase(),
        ManagedSqliteAuthorizerPhase::SchemaMigration
    );
    assert!(matches!(
        schema.into_schema_migration(),
        Err(ManagedSqliteAuthorizerTransitionError::InvalidPhaseTransition)
    ));
}
