#[cfg(test)]
mod tests {
    use super::super::*;
    use rusqlite::{params, OptionalExtension};

    #[test]
    fn migration_v47_promotes_qian_yilong_only_for_elon_self() {
        let conn = Connection::open_in_memory().expect("in-memory db should open");
        migration_v1(&conn).expect("base schema should apply");

        conn.execute(
            "INSERT INTO users (id, phone, email, password_hash, nickname, role, status, created_at, updated_at)
             VALUES (?1, ?2, NULL, 'hash', ?3, 'user', 'active', 'now', 'now')",
            params!["usr_old_owner", "18800000000", "旧 owner"],
        )
        .expect("legacy owner should insert");
        conn.execute(
            "INSERT INTO users (id, phone, email, password_hash, nickname, role, status, created_at, updated_at)
             VALUES (?1, ?2, NULL, 'hash', ?3, 'user', 'active', 'now', 'now')",
            params!["usr_qian", "15692409892", "旧昵称"],
        )
        .expect("qian user should insert");

        insert_project(&conn, "elon-self", "一龙项目", "usr_old_owner");
        insert_project(&conn, "prj_joint", "普通联合项目", "usr_old_owner");
        insert_member(&conn, "elon-self", "usr_old_owner", "owner");
        insert_member(&conn, "elon-self", "usr_qian", "admin");
        insert_member(&conn, "prj_joint", "usr_old_owner", "owner");

        migration_v47(&conn).expect("migration should apply");

        let elon_created_by: String = conn
            .query_row(
                "SELECT created_by FROM projects WHERE id = 'elon-self'",
                [],
                |row| row.get(0),
            )
            .expect("elon self project should load");
        let elon_role: String = conn
            .query_row(
                "SELECT role FROM project_members WHERE project_id = 'elon-self' AND user_id = 'usr_qian'",
                [],
                |row| row.get(0),
            )
            .expect("qian membership should load");
        let nickname: String = conn
            .query_row(
                "SELECT nickname FROM users WHERE id = 'usr_qian'",
                [],
                |row| row.get(0),
            )
            .expect("qian user should load");
        let joint_created_by: String = conn
            .query_row(
                "SELECT created_by FROM projects WHERE id = 'prj_joint'",
                [],
                |row| row.get(0),
            )
            .expect("joint project should load");
        let qian_joint_role: Option<String> = conn
            .query_row(
                "SELECT role FROM project_members WHERE project_id = 'prj_joint' AND user_id = 'usr_qian'",
                [],
                |row| row.get(0),
            )
            .optional()
            .expect("joint membership query should run");

        assert_eq!(elon_created_by, "usr_qian");
        assert_eq!(elon_role, "owner");
        assert_eq!(nickname, "钱一龙");
        assert_eq!(joint_created_by, "usr_old_owner");
        assert!(qian_joint_role.is_none());
    }

    #[test]
    fn migration_v51_publishes_elon_self_with_approval() {
        let conn = Connection::open_in_memory().expect("in-memory db should open");
        migration_v1(&conn).expect("base schema should apply");
        migration_v2(&conn).expect("visibility columns should apply");

        conn.execute(
            "INSERT INTO users (id, phone, email, password_hash, nickname, role, status, created_at, updated_at)
             VALUES (?1, ?2, NULL, 'hash', ?3, 'user', 'active', 'now', 'now')",
            params!["usr_owner", "18800000000", "owner"],
        )
        .expect("owner should insert");
        insert_project(&conn, "elon-self", "一龙项目", "usr_owner");
        insert_project(&conn, "prj_joint", "普通联合项目", "usr_owner");
        conn.execute(
            "UPDATE projects SET is_public = 0, join_mode = 'readonly' WHERE id = 'elon-self'",
            [],
        )
        .expect("elon self should update");
        conn.execute(
            "UPDATE projects SET is_public = 1, join_mode = 'open' WHERE id = 'prj_joint'",
            [],
        )
        .expect("joint project should update");

        migration_v51(&conn).expect("migration should apply");

        let elon_visibility: (i64, String) = conn
            .query_row(
                "SELECT is_public, join_mode FROM projects WHERE id = 'elon-self'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("elon self project should load");
        let joint_visibility: (i64, String) = conn
            .query_row(
                "SELECT is_public, join_mode FROM projects WHERE id = 'prj_joint'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("joint project should load");

        assert_eq!(elon_visibility, (1, "approval".to_string()));
        assert_eq!(joint_visibility, (1, "open".to_string()));
    }

    #[test]
    fn migration_v55_backfills_workspace_project_identities() {
        let conn = Connection::open_in_memory().expect("in-memory db should open");
        migration_v1(&conn).expect("base schema should apply");
        migration_v20(&conn).expect("node_id column should apply");

        conn.execute(
            "INSERT INTO users (id, phone, email, password_hash, nickname, role, status, created_at, updated_at)
             VALUES (?1, ?2, NULL, 'hash', ?3, 'user', 'active', 'now', 'now')",
            params!["usr_owner", "18800000000", "owner"],
        )
        .expect("owner should insert");
        insert_project(&conn, "prj_jian", "江西吉安商会", "usr_owner");
        conn.execute(
            "UPDATE projects
             SET workspace_path = ?1,
                 node_id = ?2
             WHERE id = 'prj_jian'",
            params![r"D:\rust\active-projects\江西吉安商会\", "node-a"],
        )
        .expect("project workspace should update");

        migration_v55(&conn).expect("identity migration should apply");

        let identity: (String, String, String) = conn
            .query_row(
                "SELECT scope_key, identity_type, identity_value
                 FROM project_identities
                 WHERE project_id = 'prj_jian'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("identity should be backfilled");
        assert_eq!(identity.0, "node:node-a");
        assert_eq!(identity.1, "workspace_path");
        assert_eq!(identity.2, "d:/rust/active-projects/江西吉安商会");
    }

    #[test]
    fn migration_v62_removes_legacy_default_joint_memberships() {
        let conn = Connection::open_in_memory().expect("in-memory db should open");
        migration_v1(&conn).expect("base schema should apply");
        migration_v19(&conn).expect("join request table should exist");
        migration_v48(&conn).expect("storage columns should exist");
        migration_v49(&conn).expect("storage worktree column should exist");
        migration_v50(&conn).expect("display name column should exist");

        conn.execute(
            "INSERT INTO users (id, phone, email, password_hash, nickname, role, status, created_at, updated_at)
             VALUES (?1, ?2, NULL, 'hash', ?3, 'user', 'active', 'now', 'now')",
            params!["usr_owner", "18800000000", "owner"],
        )
        .expect("owner should insert");
        conn.execute(
            "INSERT INTO users (id, phone, email, password_hash, nickname, role, status, created_at, updated_at)
             VALUES (?1, ?2, NULL, 'hash', ?3, 'user', 'active', 'now', 'now')",
            params!["usr_member", "18800000001", "member"],
        )
        .expect("member should insert");
        conn.execute(
            "INSERT INTO users (id, phone, email, password_hash, nickname, role, status, created_at, updated_at)
             VALUES (?1, ?2, NULL, 'hash', ?3, 'user', 'active', 'now', 'now')",
            params!["usr_admin", "18800000002", "admin"],
        )
        .expect("admin should insert");
        conn.execute(
            "INSERT INTO users (id, phone, email, password_hash, nickname, role, status, created_at, updated_at)
             VALUES (?1, ?2, NULL, 'hash', ?3, 'user', 'active', 'now', 'now')",
            params!["usr_approved", "18800000003", "approved"],
        )
        .expect("approved should insert");

        insert_project(&conn, "prj_bb64a", "bb64a", "usr_owner");
        insert_project(&conn, "prj_regular", "普通联合项目", "usr_owner");
        insert_member(&conn, "prj_bb64a", "usr_owner", "owner");
        insert_member(&conn, "prj_bb64a", "usr_member", "member");
        insert_member(&conn, "prj_bb64a", "usr_admin", "admin");
        insert_member(&conn, "prj_bb64a", "usr_approved", "member");
        insert_member(&conn, "prj_regular", "usr_member", "member");
        conn.execute(
            "INSERT INTO project_join_requests (
                id, project_id, user_id, message, status, reviewed_by,
                reviewed_at, created_at, updated_at
             )
             VALUES ('req_approved', 'prj_bb64a', 'usr_approved', 'ok', 'approved',
                     'usr_owner', 'now', 'now', 'now')",
            [],
        )
        .expect("approved request should insert");

        migration_v62(&conn).expect("cleanup migration should apply");

        let default_member_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM project_members WHERE project_id = 'prj_bb64a' AND user_id = 'usr_member'",
                [],
                |row| row.get(0),
            )
            .expect("default member count should load");
        let admin_role: String = conn
            .query_row(
                "SELECT role FROM project_members WHERE project_id = 'prj_bb64a' AND user_id = 'usr_admin'",
                [],
                |row| row.get(0),
            )
            .expect("admin role should load");
        let approved_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM project_members WHERE project_id = 'prj_bb64a' AND user_id = 'usr_approved'",
                [],
                |row| row.get(0),
            )
            .expect("approved count should load");
        let regular_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM project_members WHERE project_id = 'prj_regular' AND user_id = 'usr_member'",
                [],
                |row| row.get(0),
            )
            .expect("regular count should load");

        assert_eq!(default_member_count, 0);
        assert_eq!(admin_role, "admin");
        assert_eq!(approved_count, 1);
        assert_eq!(regular_count, 1);
    }

    #[test]
    fn migration_v64_creates_route_c_budget_events() {
        let conn = Connection::open_in_memory().expect("in-memory db should open");
        migration_v1(&conn).expect("base schema should apply");
        migration_v64(&conn).expect("route c budget table should apply");

        conn.execute(
            "INSERT INTO users (id, phone, email, password_hash, nickname, role, status, created_at, updated_at)
             VALUES ('usr_route_c_budget', NULL, 'route-c-budget@example.com', 'hash', 'Route C', 'user', 'active', 'now', 'now')",
            [],
        )
        .expect("user should insert");
        conn.execute(
            "INSERT INTO route_c_runtime_budget_events (
               id, user_id, request_fingerprint, route_day, created_at
             ) VALUES ('rcb_test', 'usr_route_c_budget', 'fp', '2026-06-23', 'now')",
            [],
        )
        .expect("budget event should insert");

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM route_c_runtime_budget_events WHERE route_day='2026-06-23'",
                [],
                |row| row.get(0),
            )
            .expect("budget event should count");
        assert_eq!(count, 1);
    }

    #[test]
    fn migration_v65_adds_route_c_budget_day_user_index() {
        let conn = Connection::open_in_memory().expect("in-memory db should open");
        migration_v1(&conn).expect("base schema should apply");
        migration_v64(&conn).expect("route c budget table should apply");
        migration_v65(&conn).expect("route c user budget index should apply");

        let exists: i64 = conn
            .query_row(
                "SELECT COUNT(*)
                   FROM sqlite_master
                  WHERE type='index'
                    AND name='idx_route_c_budget_day_user'",
                [],
                |row| row.get(0),
            )
            .expect("index should be queryable");
        assert_eq!(exists, 1);
    }

    #[test]
    fn migration_v66_adds_route_c_completion_audit_columns() {
        let conn = Connection::open_in_memory().expect("in-memory db should open");
        migration_v1(&conn).expect("base schema should apply");
        migration_v64(&conn).expect("route c budget table should apply");
        migration_v66(&conn).expect("completion audit columns should apply");

        conn.execute(
            "INSERT INTO users (id, phone, email, password_hash, nickname, role, status, created_at, updated_at)
             VALUES ('usr_route_c_completion', NULL, 'route-c-completion@example.com', 'hash', 'Route C', 'user', 'active', 'now', 'now')",
            [],
        )
        .expect("user should insert");
        conn.execute(
            "INSERT INTO route_c_runtime_budget_events (
               id, user_id, request_fingerprint, route_day, created_at
             ) VALUES ('rcb_completion', 'usr_route_c_completion', 'fp', '2026-06-23', 'now')",
            [],
        )
        .expect("budget event should insert with default outcome");

        let row: (
            String,
            Option<String>,
            Option<String>,
            Option<i64>,
            Option<String>,
        ) = conn
            .query_row(
                "SELECT outcome, completed_at, model, total_tokens, error_summary
                   FROM route_c_runtime_budget_events
                  WHERE id = 'rcb_completion'",
                [],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                    ))
                },
            )
            .expect("completion audit columns should load");
        assert_eq!(row, ("admitted".to_string(), None, None, None, None));

        let exists: i64 = conn
            .query_row(
                "SELECT COUNT(*)
                   FROM sqlite_master
                  WHERE type='index'
                    AND name='idx_route_c_budget_outcome_time'",
                [],
                |row| row.get(0),
            )
            .expect("index should be queryable");
        assert_eq!(exists, 1);
    }

    #[test]
    fn migration_v68_allows_danger_full_access_runtime_permission() {
        let conn = Connection::open_in_memory().expect("in-memory db should open");
        migration_v1(&conn).expect("base schema should apply");
        migration_v52(&conn).expect("runtime permission table should apply");
        conn.execute(
            "INSERT INTO users (id, phone, email, password_hash, nickname, role, status, created_at, updated_at)
             VALUES ('usr_danger', NULL, 'danger@example.invalid', 'hash', 'Danger', 'user', 'active', 'now', 'now')",
            [],
        )
        .expect("user should insert");
        insert_project(&conn, "prj_danger", "Danger", "usr_danger");

        let before = conn.execute(
            "INSERT INTO project_runtime_permissions (project_id, mode, updated_by, updated_at)
             VALUES ('prj_danger', 'danger_full_access', 'usr_danger', 'now')",
            [],
        );
        assert!(before.is_err());

        migration_v68(&conn).expect("danger permission migration should apply");
        conn.execute(
            "INSERT INTO project_runtime_permissions (project_id, mode, updated_by, updated_at)
             VALUES ('prj_danger', 'danger_full_access', 'usr_danger', 'now')",
            [],
        )
        .expect("danger permission should insert after v68");
        let mode: String = conn
            .query_row(
                "SELECT mode FROM project_runtime_permissions WHERE project_id = 'prj_danger'",
                [],
                |row| row.get(0),
            )
            .expect("mode should read");
        assert_eq!(mode, "danger_full_access");
    }

    fn insert_project(conn: &Connection, id: &str, name: &str, created_by: &str) {
        conn.execute(
            "INSERT INTO projects (id, name, description, workspace_key, template, source_type, status, created_by, created_at, updated_at)
             VALUES (?1, ?2, '测试项目', ?1, 'local', 'local_path', 'active', ?3, 'now', 'now')",
            params![id, name, created_by],
        )
        .expect("project should insert");
    }

    fn insert_member(conn: &Connection, project_id: &str, user_id: &str, role: &str) {
        conn.execute(
            "INSERT INTO project_members (project_id, user_id, role, created_at)
             VALUES (?1, ?2, ?3, 'now')",
            params![project_id, user_id, role],
        )
        .expect("membership should insert");
    }
}
