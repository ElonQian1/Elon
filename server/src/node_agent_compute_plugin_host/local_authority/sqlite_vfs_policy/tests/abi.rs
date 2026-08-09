use super::*;

#[test]
fn raw_authorizer_projection_rejects_unknown_utf8_and_invalid_shapes() {
    assert_eq!(
        ManagedSqliteAuthorizerAbiAdapter::project(raw(999_999, None, None, None)),
        Err(ManagedSqliteAuthorizerAbiRejection::UnknownActionCode(
            999_999
        ))
    );
    assert_eq!(
        ManagedSqliteAuthorizerAbiAdapter::project(raw(
            ffi::SQLITE_READ,
            Some(&[0xff]),
            Some(b"column"),
            Some(b"main"),
        )),
        Err(ManagedSqliteAuthorizerAbiRejection::InvalidUtf8(
            ManagedSqliteAuthorizerRawField::ArgumentOne,
        ))
    );
    assert_eq!(
        ManagedSqliteAuthorizerAbiAdapter::project(raw(
            ffi::SQLITE_SELECT,
            Some(b"unexpected"),
            None,
            None,
        )),
        Err(ManagedSqliteAuthorizerAbiRejection::InvalidArgumentShape(
            ffi::SQLITE_SELECT,
        ))
    );
    assert_eq!(
        ManagedSqliteAuthorizerAbiAdapter::project(raw(
            ffi::SQLITE_TRANSACTION,
            Some(b"begin"),
            None,
            None,
        )),
        Err(ManagedSqliteAuthorizerAbiRejection::InvalidArgumentShape(
            ffi::SQLITE_TRANSACTION,
        ))
    );
}

#[test]
fn raw_alter_projection_uses_argument_one_as_the_effective_database() {
    let projected = ManagedSqliteAuthorizerAbiAdapter::project(raw(
        ffi::SQLITE_ALTER_TABLE,
        Some(b"main"),
        Some(b"orders"),
        Some(b"obsolete_column"),
    ))
    .expect("bundled alter shape");
    let schema = policy().into_schema_migration().expect("schema");
    assert_eq!(schema.authorize_sql(projected), Decision::Allow);

    let wrong_database = ManagedSqliteAuthorizerAbiAdapter::project(raw(
        ffi::SQLITE_ALTER_TABLE,
        Some(b"temp"),
        Some(b"orders"),
        Some(b"main"),
    ))
    .expect("shape remains valid");
    assert_eq!(schema.authorize_sql(wrong_database), Decision::Deny);
}

#[test]
fn raw_projection_preserves_accessor_but_never_uses_it_as_authority_scope() {
    let projected =
        ManagedSqliteAuthorizerAbiAdapter::project(ManagedSqliteRawAuthorizerRequest::new(
            ffi::SQLITE_READ,
            Some(b"orders"),
            Some(b"id"),
            Some(b"main"),
            Some(b"trigger_name"),
        ))
        .expect("read shape");
    assert_eq!(projected.accessor(), Some("trigger_name"));
    let runtime = policy()
        .into_schema_migration()
        .expect("schema")
        .into_runtime()
        .expect("runtime");
    assert_eq!(runtime.authorize_sql(projected), Decision::Allow);
}

#[test]
fn xaccess_accepts_only_sidecar_existence_checks() {
    let policy = policy();
    for role in [Role::Journal, Role::Wal] {
        let logical_name = name(&policy, role);
        let projected = ManagedSqliteVfsRequestAbiAdapter::project_x_access(
            &policy,
            Some(&logical_name),
            ffi::SQLITE_ACCESS_EXISTS,
        )
        .expect("sidecar existence");
        assert_eq!(projected.role(), role);
    }
    let main = name(&policy, Role::Main);
    assert_eq!(
        ManagedSqliteVfsRequestAbiAdapter::project_x_access(
            &policy,
            Some(&main),
            ffi::SQLITE_ACCESS_EXISTS,
        ),
        Err(ManagedSqliteVfsAccessRequestRejection::UnsupportedRole(
            Role::Main
        ))
    );
    assert_eq!(
        ManagedSqliteVfsRequestAbiAdapter::project_x_access(
            &policy,
            Some(&main),
            ffi::SQLITE_ACCESS_READWRITE,
        ),
        Err(
            ManagedSqliteVfsAccessRequestRejection::UnsupportedAccessFlag(
                ffi::SQLITE_ACCESS_READWRITE,
            )
        )
    );
}

#[test]
fn xdelete_enforces_role_and_parent_sync_matrix() {
    let policy = policy();
    let journal = name(&policy, Role::Journal);
    for sync in [0, 1] {
        let request =
            ManagedSqliteVfsRequestAbiAdapter::project_x_delete(&policy, Some(&journal), sync)
                .expect("journal delete");
        assert_eq!(request.role(), Role::Journal);
        assert_eq!(request.sync_parent(), sync == 1);
    }
    let wal = name(&policy, Role::Wal);
    assert_eq!(
        ManagedSqliteVfsRequestAbiAdapter::project_x_delete(&policy, Some(&wal), 1),
        Err(ManagedSqliteVfsDeleteRequestRejection::InvalidRoleSyncMatrix)
    );
    let main = name(&policy, Role::Main);
    assert_eq!(
        ManagedSqliteVfsRequestAbiAdapter::project_x_delete(&policy, Some(&main), 0),
        Err(ManagedSqliteVfsDeleteRequestRejection::UnsupportedRole(
            Role::Main
        ))
    );
    assert_eq!(
        ManagedSqliteVfsRequestAbiAdapter::project_x_delete(&policy, Some(&journal), 2),
        Err(ManagedSqliteVfsDeleteRequestRejection::InvalidSyncDirectoryFlag(2))
    );
}

#[test]
fn xfullpathname_returns_only_the_exact_main_name_with_nul_capacity() {
    let policy = policy();
    let main = name(&policy, Role::Main);
    let required = main.len() + 1;
    let projected = ManagedSqliteVfsRequestAbiAdapter::project_x_full_pathname(
        &policy,
        Some(&main),
        required as i32,
    )
    .expect("exact main full pathname");
    assert_eq!(projected.output().to_bytes(), main);

    assert!(matches!(
        ManagedSqliteVfsRequestAbiAdapter::project_x_full_pathname(
            &policy,
            Some(&main),
            (required - 1) as i32,
        ),
        Err(ManagedSqliteVfsFullPathnameRequestRejection::InvalidOutputCapacity(_))
    ));
    let wal = name(&policy, Role::Wal);
    assert!(matches!(
        ManagedSqliteVfsRequestAbiAdapter::project_x_full_pathname(&policy, Some(&wal), i32::MAX,),
        Err(ManagedSqliteVfsFullPathnameRequestRejection::UnsupportedRole(Role::Wal))
    ));
}
