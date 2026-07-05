#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn temp_store() -> Store {
        let path = std::env::temp_dir().join(format!(
            "elon_project_delete_{}.db",
            Uuid::new_v4().simple()
        ));
        Store::open(&path).expect("store should open")
    }

    #[test]
    fn create_project_reuses_existing_owner_project_name() {
        let store = temp_store();
        let user = store
            .create_user("reuse-project-name@example.com", "secret1", None, None)
            .expect("user should be created");

        let first = store
            .create_project(&user.id, "Reusable Project", None, None)
            .expect("project should be created");
        let second = store
            .create_project(&user.id, "Reusable Project", None, None)
            .expect("existing project should be reused");

        assert!(!first.reused_existing);
        assert!(second.reused_existing);
        assert_eq!(second.project.id, first.project.id);
        assert_eq!(
            store
                .list_projects_for_user(&user.id)
                .expect("projects should list")
                .len(),
            1
        );
    }

    #[test]
    fn invite_projects_are_hidden_from_store_but_joinable_by_card() {
        let store = temp_store();
        let owner = store
            .create_user("invite-owner@example.com", "secret1", None, None)
            .expect("owner should be created");
        let invited = store
            .create_user("invite-member@example.com", "secret1", None, None)
            .expect("invited user should be created");
        let project = store
            .create_project(&owner.id, "Invite Only", None, None)
            .expect("project should be created")
            .project;

        store
            .set_project_visibility(&project.id, true, "invite")
            .expect("project should become invite-only");

        assert!(store
            .list_public_projects(None, None, None, None, 10, 0)
            .expect("store projects should list")
            .is_empty());
        assert!(store.get_public_project(&project.id).is_err());

        store
            .join_project(&invited.id, &project.id)
            .expect("invited card recipient should join");
        let access = store
            .get_project_access(&invited.id, &project.id)
            .expect("joined user should have project access");
        assert_eq!(access.role, "member");
    }

    #[test]
    fn elon_self_is_public_store_project_with_approval_join() {
        let store = temp_store();
        let owner = store
            .create_user("elon-self-owner@example.com", "secret1", None, None)
            .expect("owner should be created");
        let applicant = store
            .create_user("elon-self-applicant@example.com", "secret1", None, None)
            .expect("applicant should be created");

        store
            .ensure_project_for_user(
                &owner.id,
                "elon-self",
                "一龙项目",
                Some("一龙项目主仓库"),
                "template",
                "android",
                None,
            )
            .expect("elon self project should be ensured");
        store
            .set_project_visibility("elon-self", false, "open")
            .expect("visibility changes should keep elon self in approval mode");

        let public_project = store
            .get_public_project("elon-self")
            .expect("elon self should appear in the store");
        assert!(public_project.is_public);
        assert_eq!(public_project.join_mode, "approval");
        assert_eq!(public_project.display_name.as_deref(), Some("一龙项目"));
        assert!(public_project
            .icon_data_url
            .as_deref()
            .is_some_and(|icon| icon.starts_with("data:image/png;base64,")));

        let join_error = store
            .join_project(&applicant.id, "elon-self")
            .expect_err("approval project should not join directly");
        assert!(join_error.to_string().contains("需要审批"));

        let request = store
            .create_join_request(&applicant.id, "elon-self", Some("想加入"))
            .expect("applicant should be able to request approval");
        assert_eq!(request.status, "pending");
    }

    #[test]
    fn add_project_member_by_account_supports_admin_role() {
        let store = temp_store();
        let owner = store
            .create_user("member-admin-owner@example.com", "secret1", None, None)
            .expect("owner should be created");
        let target = store
            .create_user("15692409892", "secret1", Some("钱一龙"), None)
            .expect("target user should be created");
        let project = store
            .create_project(&owner.id, "Admin Invite", None, None)
            .expect("project should be created")
            .project;

        let member = store
            .add_project_member_by_account(&project.id, "15692409892", "admin")
            .expect("phone account should be invited as admin");
        assert_eq!(member.user_id, target.id);
        assert_eq!(member.role, "admin");

        store
            .update_member_role(&project.id, &target.id, "observer")
            .expect("admin member should be demotable");
        let access = store
            .get_project_access(&target.id, &project.id)
            .expect("target should keep project access");
        assert_eq!(access.role, "observer");

        let member = store
            .add_project_member_by_account(&project.id, "钱一龙", "admin")
            .expect("nickname account should update the member role");
        assert_eq!(member.user_id, target.id);
        assert_eq!(member.role, "admin");

        store
            .update_member_role(&project.id, &target.id, "admin")
            .expect("member should be promotable to admin");
        let access = store
            .get_project_access(&target.id, &project.id)
            .expect("target should keep project access");
        assert_eq!(access.role, "admin");
    }

    #[test]
    fn project_member_moderation_blocks_banned_members_and_marks_muted() {
        let store = temp_store();
        let owner = store
            .create_user("moderation-owner@example.com", "secret1", None, None)
            .expect("owner should be created");
        let target = store
            .create_user(
                "moderation-target@example.com",
                "secret1",
                Some("被管理成员"),
                None,
            )
            .expect("target should be created");
        let project = store
            .create_project(&owner.id, "Moderation Project", None, None)
            .expect("project should be created")
            .project;

        store
            .add_project_member_by_account(&project.id, "moderation-target@example.com", "member")
            .expect("target should be invited");

        let mute = store
            .update_project_member_moderation(
                &project.id,
                &target.id,
                &owner.id,
                "mute",
                Some(60),
                None,
            )
            .expect("target should be muted");
        assert!(mute.is_muted);
        assert!(mute.muted_until.is_some());
        assert!(store
            .active_project_member_muted_until(&project.id, &target.id)
            .expect("mute lookup should work")
            .is_some());
        let members = store
            .list_project_members(&project.id)
            .expect("members should list");
        let muted_member = members
            .iter()
            .find(|member| member.user_id == target.id)
            .expect("target should be listed");
        assert!(muted_member.is_muted);

        let ban = store
            .update_project_member_moderation(&project.id, &target.id, &owner.id, "ban", None, None)
            .expect("target should be banned");
        assert!(ban.is_banned);
        assert!(store
            .project_member_is_banned(&project.id, &target.id)
            .expect("ban lookup should work"));
        assert!(store
            .get_project_access(&target.id, &project.id)
            .expect_err("banned target should not access project")
            .to_string()
            .contains("封禁"));
        assert!(store
            .add_project_member_by_account(&project.id, "moderation-target@example.com", "member")
            .expect_err("banned target should not be re-invited")
            .to_string()
            .contains("封禁"));

        store
            .update_project_member_moderation(
                &project.id,
                &target.id,
                &owner.id,
                "unban",
                None,
                None,
            )
            .expect("target should be unbanned");
        assert!(store.get_project_access(&target.id, &project.id).is_ok());

        store
            .update_project_member_moderation(
                &project.id,
                &target.id,
                &owner.id,
                "unmute",
                None,
                None,
            )
            .expect("target should be unmuted");
        assert!(store
            .active_project_member_muted_until(&project.id, &target.id)
            .expect("mute lookup should work")
            .is_none());
    }

    #[test]
    fn readonly_projects_are_public_but_join_as_observer() {
        let store = temp_store();
        let owner = store
            .create_user("readonly-owner@example.com", "secret1", None, None)
            .expect("owner should be created");
        let viewer = store
            .create_user("readonly-viewer@example.com", "secret1", None, None)
            .expect("viewer should be created");
        let project = store
            .create_project(&owner.id, "Readonly Project", None, None)
            .expect("project should be created")
            .project;

        store
            .set_project_visibility(&project.id, true, "readonly")
            .expect("project should become readonly public");

        let public_projects = store
            .list_public_projects(None, None, None, None, 10, 0)
            .expect("store projects should list");
        assert_eq!(public_projects.len(), 1);
        assert_eq!(public_projects[0].join_mode, "readonly");

        store
            .join_project(&viewer.id, &project.id)
            .expect("readonly viewer should join");
        let access = store
            .get_project_access(&viewer.id, &project.id)
            .expect("viewer should have project access");
        assert_eq!(access.role, "observer");
    }

    #[test]
    fn project_owner_display_prefers_nickname() {
        let store = temp_store();
        let owner = store
            .create_user("named-owner@example.com", "secret1", Some("项目主人"), None)
            .expect("owner should be created");
        let member = store
            .create_user("named-member@example.com", "secret1", None, None)
            .expect("member should be created");
        let project = store
            .create_project(&owner.id, "Named Owner Project", None, None)
            .expect("project should be created")
            .project;

        store
            .set_project_visibility(&project.id, true, "open")
            .expect("project should become public");

        let public_projects = store
            .list_public_projects(None, None, None, None, 10, 0)
            .expect("store projects should list");
        assert_eq!(public_projects[0].owner_account, "项目主人");
        assert_eq!(
            store
                .get_public_project(&project.id)
                .expect("public project should load")
                .owner_account,
            "项目主人"
        );

        let account_matches = store
            .list_public_projects(Some("named-owner"), None, None, None, 10, 0)
            .expect("owner account search should still list");
        assert_eq!(account_matches.len(), 1);

        let first_join = store
            .join_project(&member.id, &project.id)
            .expect("member should join");
        assert!(!first_join);
        let second_join = store
            .join_project(&member.id, &project.id)
            .expect("repeated join should be idempotent");
        assert!(second_join);
        let joined = store
            .list_joined_projects(&member.id)
            .expect("joined projects should list");
        assert_eq!(joined[0].owner_account, "项目主人");

        let owner_joined = store
            .list_joined_projects(&owner.id)
            .expect("owner projects should count as joined in store");
        assert_eq!(owner_joined[0].id, project.id);
        assert_eq!(owner_joined[0].viewer_role.as_deref(), Some("owner"));

        let member_count: i64 = store
            .conn()
            .expect("db connection")
            .query_row(
                "SELECT COUNT(*) FROM project_members WHERE project_id = ?1 AND user_id = ?2",
                params![project.id, member.id],
                |row| row.get(0),
            )
            .expect("member count should load");
        assert_eq!(member_count, 1);
    }

    #[test]
    fn public_projects_include_latest_apk_url() {
        let store = temp_store();
        let owner = store
            .create_user("apk-owner@example.com", "secret1", None, None)
            .expect("owner should be created");
        let project = store
            .create_project(&owner.id, "APK Project", None, None)
            .expect("project should be created")
            .project;

        store
            .set_project_visibility(&project.id, true, "open")
            .expect("project should become public");
        let task = store
            .create_task(&project.id, &owner.id, Some("conv"), "build apk")
            .expect("task should be created");
        store
            .finish_task(
                &task,
                "done",
                Some("done"),
                Some("https://example.test/latest.apk"),
                None,
            )
            .expect("task should finish with apk url");

        let public_projects = store
            .list_public_projects(None, None, None, None, 10, 0)
            .expect("store projects should list");
        assert_eq!(
            public_projects[0].latest_apk_url.as_deref(),
            Some("https://example.test/latest.apk")
        );
    }

    #[test]
    fn public_projects_include_viewer_role_when_authenticated() {
        let store = temp_store();
        let owner = store
            .create_user("viewer-role-owner@example.com", "secret1", None, None)
            .expect("owner should be created");
        let member = store
            .create_user("viewer-role-member@example.com", "secret1", None, None)
            .expect("member should be created");
        let project = store
            .create_project(&owner.id, "Viewer Role Project", None, None)
            .expect("project should be created")
            .project;

        store
            .set_project_visibility(&project.id, true, "open")
            .expect("project should become public");
        store
            .join_project(&member.id, &project.id)
            .expect("member should join");

        let anonymous_projects = store
            .list_public_projects(None, None, None, None, 10, 0)
            .expect("anonymous store projects should list");
        assert_eq!(anonymous_projects[0].viewer_role, None);

        let owner_projects = store
            .list_public_projects_for_viewer(None, None, None, None, 10, 0, Some(&owner.id))
            .expect("owner store projects should list");
        assert_eq!(owner_projects[0].viewer_role.as_deref(), Some("owner"));

        let member_projects = store
            .list_public_projects_for_viewer(None, None, None, None, 10, 0, Some(&member.id))
            .expect("member store projects should list");
        assert_eq!(member_projects[0].viewer_role.as_deref(), Some("member"));

        let detail = store
            .get_public_project_for_viewer(&project.id, Some(&member.id))
            .expect("public project should load for member");
        assert_eq!(detail.viewer_role.as_deref(), Some("member"));
    }

    #[test]
    fn public_projects_can_filter_join_mode_and_apk() {
        let store = temp_store();
        let owner = store
            .create_user("filter-owner@example.com", "secret1", None, None)
            .expect("owner should be created");
        let open_project = store
            .create_project(&owner.id, "Open APK Project", None, None)
            .expect("open project should be created")
            .project;
        let readonly_project = store
            .create_project(&owner.id, "Readonly Demo", None, None)
            .expect("readonly project should be created")
            .project;

        store
            .set_project_visibility(&open_project.id, true, "open")
            .expect("open project should become public");
        store
            .set_project_visibility(&readonly_project.id, true, "readonly")
            .expect("readonly project should become public");
        let task = store
            .create_task(&open_project.id, &owner.id, Some("conv"), "build apk")
            .expect("task should be created");
        store
            .finish_task(
                &task,
                "done",
                Some("done"),
                Some("https://example.test/open.apk"),
                None,
            )
            .expect("task should finish with apk url");

        let readonly_only = store
            .list_public_projects(None, Some("readonly"), None, None, 10, 0)
            .expect("readonly filter should list");
        assert_eq!(readonly_only.len(), 1);
        assert_eq!(readonly_only[0].id, readonly_project.id);

        let apk_only = store
            .list_public_projects(None, None, Some(true), None, 10, 0)
            .expect("apk filter should list");
        assert_eq!(apk_only.len(), 1);
        assert_eq!(apk_only[0].id, open_project.id);

        let owner_matches = store
            .list_public_projects(Some("filter-owner"), None, None, None, 10, 0)
            .expect("owner search should list");
        assert_eq!(owner_matches.len(), 2);
    }

    #[test]
    fn deletion_target_rejects_running_tasks() {
        let store = temp_store();
        let user = store
            .create_user("delete-running@example.com", "secret1", None, None)
            .expect("user should be created");
        let project = store
            .create_project(&user.id, "Running Delete", None, None)
            .expect("project should be created")
            .project;
        store
            .create_task(&project.id, &user.id, Some("conv"), "run")
            .expect("task should be created");

        let err = store
            .project_deletion_target(&user.id, &project.id)
            .expect_err("running task should block deletion")
            .to_string();

        assert!(err.contains("正在运行"));
    }

    #[test]
    fn system_projects_cannot_be_deleted_or_published() {
        let store = temp_store();
        let user = store
            .create_user("system-guard@example.com", "secret1", None, None)
            .expect("user should be created");
        let member = store
            .create_user(
                "system-guard-member@example.com",
                "secret1",
                Some("成员"),
                None,
            )
            .expect("member should be created");
        let (project_id, _) = store
            .ensure_balloon_project_for_user(&user.id)
            .expect("system project should exist");

        let delete_err = store
            .project_deletion_target(&user.id, &project_id)
            .expect_err("system project deletion should be rejected")
            .to_string();
        assert!(delete_err.contains("系统归档项目"));

        let visibility_err = store
            .set_project_visibility(&project_id, true, "open")
            .expect_err("system project visibility should be rejected")
            .to_string();
        assert!(visibility_err.contains("系统归档项目"));

        let join_err = store
            .join_project(&member.id, &project_id)
            .expect_err("system project join should be rejected")
            .to_string();
        assert!(join_err.contains("系统归档项目不支持加入"));

        let add_err = store
            .add_project_member_by_account(&project_id, "system-guard-member@example.com", "member")
            .expect_err("system project member add should be rejected")
            .to_string();
        assert!(add_err.contains("系统归档项目不能添加成员"));

        let role_err = store
            .update_member_role(&project_id, &member.id, "admin")
            .expect_err("system project role change should be rejected")
            .to_string();
        assert!(role_err.contains("系统归档项目不能修改成员角色"));

        let remove_err = store
            .remove_member(&project_id, &member.id)
            .expect_err("system project member removal should be rejected")
            .to_string();
        assert!(remove_err.contains("系统归档项目不能移除成员"));

        let bind_err = store
            .bind_project_to_pc_workspace(
                &user.id,
                &project_id,
                "D:/Elon/workspaces/user/project/repo",
                "node-a",
                None,
                None,
                None,
            )
            .expect_err("system project PC binding should be rejected")
            .to_string();
        assert!(bind_err.contains("系统归档项目不能绑定"));
    }

    #[test]
    fn bind_project_to_pc_workspace_persists_git_remote() {
        let store = temp_store();
        let user = store
            .create_user("pc-bind-git@example.com", "secret1", None, None)
            .expect("user should be created");
        let project = store
            .create_project(&user.id, "Portable PC Project", None, None)
            .expect("project should be created")
            .project;

        let rebound = store
            .bind_project_to_pc_workspace(
                &user.id,
                &project.id,
                "D:/Elon/workspaces/user/project/repo",
                "node-a",
                Some("abc123"),
                Some("git@github.com:owner/portable.git"),
                Some("main"),
            )
            .expect("project should bind to pc workspace");

        assert_eq!(rebound.source_type, "pc_managed");
        assert_eq!(
            rebound.repo_url.as_deref(),
            Some("git@github.com:owner/portable.git")
        );
        assert_eq!(rebound.branch.as_deref(), Some("main"));
    }

    #[test]
    fn purge_project_records_removes_project_children() {
        let store = temp_store();
        let user = store
            .create_user("delete-purge@example.com", "secret1", None, None)
            .expect("user should be created");
        let project = store
            .create_project(&user.id, "Purge Delete", None, None)
            .expect("project should be created")
            .project;
        let task = store
            .create_task(&project.id, &user.id, Some("conv"), "run")
            .expect("task should be created");
        store
            .record_task_event(&task, r#"{"type":"progress","message":"step"}"#)
            .expect("event should be recorded");
        store
            .finish_task(&task, "done", Some("done"), None, None)
            .expect("task should finish");
        store
            .set_project_runtime_permission(&project.id, &user.id, "full_access")
            .expect("runtime permission should be recorded");
        let matter_id = "delete-matter";
        let assignment_id = "delete-assignment";
        {
            let conn = store.conn().expect("connection should open");
            let ts = now();
            conn.execute(
                "INSERT INTO project_join_requests
                   (id, project_id, user_id, message, status, created_at, updated_at)
                 VALUES ('delete-join-request', ?1, ?2, 'join', 'pending', ?3, ?3)",
                params![project.id, user.id, ts],
            )
            .expect("join request should insert");
            conn.execute(
                "INSERT INTO project_member_conversation_discussion_messages
                   (id, project_id, member_user_id, conversation_id, sender_user_id, content, created_at)
                 VALUES ('delete-discussion', ?1, ?2, 'conv', ?2, 'discussion', ?3)",
                params![project.id, user.id, ts],
            )
            .expect("discussion message should insert");
            conn.execute(
                "INSERT INTO project_execution_sessions
                   (id, project_id, conversation_id, user_id, node_id, request_id, status, created_at, updated_at)
                 VALUES ('delete-session', ?1, 'conv', ?2, 'node-delete', 'request-delete', 'done', ?3, ?3)",
                params![project.id, user.id, ts],
            )
            .expect("execution session should insert");
            conn.execute(
                "INSERT INTO project_landing_upload_tokens
                   (id, project_id, token_hash, created_by, created_at)
                 VALUES ('delete-landing-token', ?1, 'delete-token-hash', ?2, ?3)",
                params![project.id, user.id, ts],
            )
            .expect("landing token should insert");
            conn.execute(
                "INSERT OR REPLACE INTO project_dev_profiles (project_id, updated_at)
                 VALUES (?1, ?2)",
                params![project.id, ts],
            )
            .expect("dev profile should insert");
            conn.execute(
                "INSERT INTO project_identities
                   (id, project_id, owner_user_id, scope_key, identity_type, identity_value, created_at, updated_at)
                 VALUES ('delete-identity', ?1, ?2, 'delete-scope', 'package', 'com.elon.delete', ?3, ?3)",
                params![project.id, user.id, ts],
            )
            .expect("project identity should insert");
            conn.execute(
                "INSERT INTO project_ai_node_authorizations
                   (id, project_id, provider_user_id, node_id, allowed_clis_json, permission_level,
                    enabled, created_by_user_id, created_at, updated_at)
                 VALUES ('delete-ai-auth', ?1, ?2, 'node-delete', '[]', 'project_write', 1, ?2, ?3, ?3)",
                params![project.id, user.id, ts],
            )
            .expect("AI node authorization should insert");
            conn.execute(
                "INSERT INTO project_ai_bots
                   (id, project_id, provider_user_id, node_id, display_name, runtime_route, cli_name,
                    capabilities_json, risk_level, enabled, created_at, updated_at)
                 VALUES ('delete-ai-bot', ?1, ?2, 'node-delete', 'Delete Bot', 'codex', 'codex',
                         '[]', 'project_write', 1, ?3, ?3)",
                params![project.id, user.id, ts],
            )
            .expect("AI bot should insert");
            conn.execute(
                "INSERT INTO project_ai_matters
                   (id, project_id, channel_id, requester_user_id, title, brief, collaboration_mode,
                    status, participant_user_ids_json, node_policy_json, acceptance_criteria_json,
                    plan_json, created_at, updated_at)
                 VALUES (?1, ?2, 'general', ?3, 'Delete Matter', 'brief', 'solo',
                         'done', '[]', '{}', '[]', '{}', ?4, ?4)",
                params![matter_id, project.id, user.id, ts],
            )
            .expect("AI matter should insert");
            conn.execute(
                "INSERT INTO project_ai_matter_assignments
                   (id, matter_id, bot_id, provider_user_id, node_id, role, runtime_route,
                    cli_name, status, created_at, updated_at)
                 VALUES (?1, ?2, 'delete-ai-bot', ?3, 'node-delete', 'builder',
                         'codex', 'codex', 'done', ?4, ?4)",
                params![assignment_id, matter_id, user.id, ts],
            )
            .expect("AI assignment should insert");
            conn.execute(
                "INSERT INTO project_ai_reviews
                   (id, matter_id, target_assignment_id, severity, finding_json, status, created_at, updated_at)
                 VALUES ('delete-ai-review', ?1, ?2, 'info', '{}', 'open', ?3, ?3)",
                params![matter_id, assignment_id, ts],
            )
            .expect("AI review should insert");
            conn.execute(
                "INSERT INTO project_ai_events
                   (id, matter_id, project_id, actor_user_id, event_type, payload_json, created_at)
                 VALUES ('delete-ai-event', ?1, ?2, ?3, 'created', '{}', ?4)",
                params![matter_id, project.id, user.id, ts],
            )
            .expect("AI event should insert");
        }

        let target = store
            .project_deletion_target(&user.id, &project.id)
            .expect("target should be available");
        assert_eq!(target.id, project.id);

        store
            .purge_project_records(&user.id, &project.id)
            .expect("project records should purge");

        assert!(store.get_project_access(&user.id, &project.id).is_err());
        assert!(store
            .list_task_events(&task, 10)
            .expect("task events query should work")
            .is_empty());
        let conn = store.conn().expect("connection should open");
        for table in [
            "project_join_requests",
            "project_member_conversation_discussion_messages",
            "project_execution_sessions",
            "project_runtime_permission_audit",
            "project_runtime_permissions",
            "project_landing_upload_tokens",
            "project_dev_profiles",
            "project_identities",
            "project_ai_node_authorizations",
            "project_ai_bots",
            "project_ai_matters",
            "project_ai_events",
        ] {
            let sql = format!("SELECT COUNT(*) FROM {table} WHERE project_id = ?1");
            let count: i64 = conn
                .query_row(&sql, params![project.id], |row| row.get(0))
                .expect("project child count should query");
            assert_eq!(count, 0, "{table} should be purged");
        }
        let assignment_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM project_ai_matter_assignments WHERE matter_id = ?1",
                params![matter_id],
                |row| row.get(0),
            )
            .expect("assignment count should query");
        assert_eq!(assignment_count, 0);
        let review_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM project_ai_reviews
                  WHERE matter_id = ?1 OR target_assignment_id = ?2",
                params![matter_id, assignment_id],
                |row| row.get(0),
            )
            .expect("review count should query");
        assert_eq!(review_count, 0);
    }
}
