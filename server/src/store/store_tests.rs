    use crate::store::*;
    use uuid::Uuid;

    fn temp_store() -> Store {
        let path = std::env::temp_dir().join(format!(
            "elon_store_external_project_{}.db",
            Uuid::new_v4().simple()
        ));
        Store::open(&path).expect("store should open")
    }

    #[test]
    fn register_external_project_persists_and_updates_node_binding() {
        let store = temp_store();
        let user = store
            .create_user("pc-project-owner@example.com", "secret1", None, None)
            .expect("user should be created");

        let first = store
            .register_external_project(
                &user.id,
                None,
                "PC Project",
                Some("from pc"),
                r"D:\rust\active-projects\one",
                Some("node-a"),
                None,
                None,
            )
            .expect("external project should register");
        assert!(!first.reused_existing);
        assert_eq!(first.project.source_type, "local_path");
        assert_eq!(first.project.node_id.as_deref(), Some("node-a"));

        let same_path = store
            .register_external_project(
                &user.id,
                None,
                "Different Project Name",
                Some("same pc path"),
                r"D:\rust\active-projects\one",
                Some("node-a"),
                None,
                None,
            )
            .expect("same external path should reuse identity");
        assert!(same_path.reused_existing);
        assert_eq!(same_path.project.id, first.project.id);
        assert_eq!(
            same_path.project.workspace_path.as_deref(),
            Some(r"D:\rust\active-projects\one")
        );
        assert_eq!(same_path.project.runtime_permission, "project_write");

        let second = store
            .register_external_project(
                &user.id,
                None,
                "PC Project",
                Some("from pc"),
                r"D:\rust\active-projects\two",
                Some("node-b"),
                Some("git@github.com:owner/pc-project.git"),
                Some("main"),
            )
            .expect("same external project should update");
        assert!(second.reused_existing);
        assert_eq!(
            second.project.workspace_path.as_deref(),
            Some(r"D:\rust\active-projects\two")
        );
        assert_eq!(second.project.node_id.as_deref(), Some("node-b"));
        assert_eq!(
            second.project.repo_url.as_deref(),
            Some("git@github.com:owner/pc-project.git")
        );
        assert_eq!(second.project.branch.as_deref(), Some("main"));

        let access = store
            .get_project_access(&user.id, &second.project.id)
            .expect("project access should include node binding");
        assert_eq!(access.node_id.as_deref(), Some("node-b"));
    }

    #[test]
    fn project_landing_snapshot_is_normalized_and_readable() {
        let store = temp_store();
        let user = store
            .create_user("pc-project-landing@example.com", "secret1", None, None)
            .expect("user should be created");
        let project = store
            .register_external_project(
                &user.id,
                None,
                "Landing Project",
                Some("from pc"),
                r"D:\rust\active-projects\landing",
                Some("node-a"),
                None,
                None,
            )
            .expect("external project should register");

        let snapshot = store
            .update_project_landing_snapshot(
                &user.id,
                &project.project.id,
                &serde_json::json!({
                    "title": "Landing Project",
                    "downloads": {
                        "windows": "https://example.com/app.exe",
                        "ios": "javascript:alert(1)"
                    }
                }),
            )
            .expect("landing snapshot should update")
            .expect("landing snapshot should have display content");
        assert_eq!(snapshot["source"]["mode"], "node_agent_snapshot");
        let downloads = snapshot["downloads"].as_array().unwrap();
        let windows = downloads
            .iter()
            .find(|download| download["platform"] == "windows")
            .unwrap();
        let ios = downloads
            .iter()
            .find(|download| download["platform"] == "ios")
            .unwrap();
        assert_eq!(windows["url"], "https://example.com/app.exe");
        assert!(ios.get("url").is_none());

        let loaded = store
            .project_landing_snapshot(&user.id, &project.project.id)
            .expect("landing snapshot should load")
            .expect("landing snapshot should exist");
        assert_eq!(loaded["title"], "Landing Project");
        assert_eq!(loaded["source"]["mode"], "node_agent_snapshot");
    }

    #[test]
    fn project_landing_upload_token_is_project_scoped() {
        let store = temp_store();
        let user = store
            .create_user(
                "pc-project-landing-token@example.com",
                "secret1",
                None,
                None,
            )
            .expect("user should be created");
        let project = store
            .register_external_project(
                &user.id,
                None,
                "Landing Token Project",
                Some("from pc"),
                r"D:\rust\active-projects\landing-token",
                Some("node-a"),
                None,
                None,
            )
            .expect("external project should register");
        let other = store
            .register_external_project(
                &user.id,
                None,
                "Other Landing Token Project",
                Some("from pc"),
                r"D:\rust\active-projects\landing-token-other",
                Some("node-a"),
                None,
                None,
            )
            .expect("other project should register");
        let token = "plt_test_project_scoped_token";

        let record = store
            .rotate_project_landing_upload_token(&project.project.id, &user.id, token)
            .expect("token should rotate");
        assert!(store
            .authenticate_project_landing_upload_token(&other.project.id, token)
            .expect("token auth should run")
            .is_none());
        assert!(store
            .authenticate_project_landing_upload_token(&project.project.id, "wrong-token")
            .expect("token auth should run")
            .is_none());

        let authed = store
            .authenticate_project_landing_upload_token(&project.project.id, token)
            .expect("token auth should run")
            .expect("token should authenticate for its project");
        assert_eq!(authed.id, record.id);
        let snapshot = store
            .update_project_landing_snapshot_with_upload_token(
                &project.project.id,
                &authed.id,
                &serde_json::json!({
                    "title": "Landing Token Project",
                    "release_manifest_url": "https://example.com/project-downloads.json",
                    "downloads": [{
                        "platform": "windows",
                        "status": "available",
                        "url": "https://example.com/app.exe"
                    }]
                }),
            )
            .expect("landing snapshot should update with upload token")
            .expect("snapshot should have display content");
        assert_eq!(
            snapshot["release_manifest_url"],
            "https://example.com/project-downloads.json"
        );
    }

    #[test]
    fn register_external_project_can_bind_existing_shared_project_by_id() {
        let store = temp_store();
        let owner = store
            .create_user("shared-owner@example.com", "secret1", None, None)
            .expect("user should be created");
        store
            .ensure_project_for_user(
                &owner.id,
                "elon-self",
                "一龙项目",
                None,
                "template",
                "android",
                None,
            )
            .expect("shared project should exist");

        let bound = store
            .register_external_project(
                &owner.id,
                Some("elon-self"),
                "一龙项目",
                Some("PC local repo"),
                r"D:\rust\active-projects\elon cli",
                Some("node-owner"),
                None,
                None,
            )
            .expect("existing shared project should bind");

        assert!(bound.reused_existing);
        assert_eq!(bound.project.id, "elon-self");
        assert_eq!(
            bound.project.workspace_path.as_deref(),
            Some(r"D:\rust\active-projects\elon cli")
        );
        assert_eq!(bound.project.node_id.as_deref(), Some("node-owner"));
    }

    #[test]
    fn register_external_project_reuses_existing_workspace_path_with_different_name() {
        let store = temp_store();
        let user = store
            .create_user("pc-project-path-owner@example.com", "secret1", None, None)
            .expect("user should be created");
        let workspace_path = r"D:\rust\active-projects\江西吉安商会";

        let first = store
            .register_external_project(
                &user.id,
                None,
                "江西吉安商会",
                None,
                workspace_path,
                Some("node-a"),
                None,
                None,
            )
            .expect("first project should register");
        let second = store
            .register_external_project(
                &user.id,
                None,
                "jxjssh",
                None,
                "D:/rust/active-projects/江西吉安商会/",
                Some("node-b"),
                None,
                None,
            )
            .expect("same workspace path should reuse existing project");

        assert!(second.reused_existing);
        assert_eq!(second.project.id, first.project.id);
        assert_eq!(second.project.name, "江西吉安商会");
        assert_eq!(second.project.node_id.as_deref(), Some("node-b"));
        assert_eq!(
            store
                .get_project_pc_workspace_binding(&user.id, &second.project.id, "node-a")
                .expect("node-a binding lookup")
                .expect("node-a binding")
                .workspace_path,
            workspace_path
        );
        assert_eq!(
            store
                .get_project_pc_workspace_binding(&user.id, &second.project.id, "node-b")
                .expect("node-b binding lookup")
                .expect("node-b binding")
                .workspace_path,
            "D:/rust/active-projects/江西吉安商会/"
        );

        let count: i64 = store
            .conn()
            .expect("db connection")
            .query_row(
                "SELECT COUNT(*) FROM projects WHERE created_by = ?1 AND status != 'deleted'",
                params![user.id],
                |row| row.get(0),
            )
            .expect("project count");
        assert_eq!(count, 1);
    }

    #[test]
    fn register_external_project_reuses_existing_git_remote_with_different_path_and_name() {
        let store = temp_store();
        let user = store
            .create_user("pc-project-git-owner@example.com", "secret1", None, None)
            .expect("user should be created");

        let first = store
            .register_external_project(
                &user.id,
                None,
                "江西吉安商会",
                None,
                r"D:\rust\active-projects\江西吉安商会",
                Some("node-a"),
                Some("git@github.com:Owner/Jiangxi-Jian.git"),
                Some("main"),
            )
            .expect("first project should register");
        let second = store
            .register_external_project(
                &user.id,
                None,
                "本地git项目",
                None,
                r"D:\rust\active-projects\jx-ja-copy",
                Some("node-b"),
                Some("https://github.com/owner/jiangxi-jian"),
                Some("refs/heads/main"),
            )
            .expect("same git remote should reuse existing project");

        assert!(second.reused_existing);
        assert_eq!(second.project.id, first.project.id);
        assert_eq!(second.project.name, "江西吉安商会");
        assert_eq!(
            second.project.workspace_path.as_deref(),
            Some(r"D:\rust\active-projects\jx-ja-copy")
        );
        assert_eq!(
            store
                .get_project_pc_workspace_binding(&user.id, &second.project.id, "node-a")
                .expect("node-a binding lookup")
                .expect("node-a binding")
                .workspace_path,
            r"D:\rust\active-projects\江西吉安商会"
        );
        assert_eq!(
            store
                .get_project_pc_workspace_binding(&user.id, &second.project.id, "node-b")
                .expect("node-b binding lookup")
                .expect("node-b binding")
                .workspace_path,
            r"D:\rust\active-projects\jx-ja-copy"
        );
        let conn = store.conn().expect("db connection");
        let project_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM projects WHERE created_by = ?1 AND status != 'deleted'",
                params![user.id],
                |row| row.get(0),
            )
            .expect("project count");
        let identity_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM project_identities WHERE project_id = ?1",
                params![second.project.id],
                |row| row.get(0),
            )
            .expect("identity count");
        assert_eq!(project_count, 1);
        assert_eq!(identity_count, 4);
    }

    #[test]
    fn register_external_project_prefers_existing_path_over_same_name() {
        let store = temp_store();
        let user = store
            .create_user(
                "pc-project-path-priority@example.com",
                "secret1",
                None,
                None,
            )
            .expect("user should be created");
        let project_a = store
            .register_external_project(
                &user.id,
                None,
                "项目A",
                None,
                r"D:\rust\active-projects\a",
                Some("node-a"),
                None,
                None,
            )
            .expect("project A should register");
        let project_b = store
            .register_external_project(
                &user.id,
                None,
                "项目B",
                None,
                r"D:\rust\active-projects\b",
                Some("node-b"),
                None,
                None,
            )
            .expect("project B should register");

        let reused = store
            .register_external_project(
                &user.id,
                None,
                "项目A",
                None,
                r"D:/rust/active-projects/b/",
                Some("node-c"),
                None,
                None,
            )
            .expect("path match should win over name match");

        assert!(reused.reused_existing);
        assert_eq!(reused.project.id, project_b.project.id);
        assert_eq!(reused.project.name, "项目B");
        assert_ne!(reused.project.id, project_a.project.id);
        let project_count: i64 = store
            .conn()
            .expect("db connection")
            .query_row(
                "SELECT COUNT(*) FROM projects WHERE created_by = ?1 AND status != 'deleted'",
                params![user.id],
                |row| row.get(0),
            )
            .expect("project count");
        assert_eq!(project_count, 2);
    }

    #[test]
    fn register_external_project_rejects_binding_to_path_owned_by_another_project() {
        let store = temp_store();
        let user = store
            .create_user(
                "pc-project-path-conflict@example.com",
                "secret1",
                None,
                None,
            )
            .expect("user should be created");
        let workspace_path = r"D:\rust\active-projects\江西吉安商会";
        let existing = store
            .register_external_project(
                &user.id,
                None,
                "江西吉安商会",
                None,
                workspace_path,
                Some("node-a"),
                None,
                None,
            )
            .expect("first project should register");
        let other = store
            .ensure_project_for_user(
                &user.id,
                "prj-other",
                "其他项目",
                None,
                "template",
                "android",
                None,
            )
            .expect("other project should exist");

        let err = store
            .register_external_project(
                &user.id,
                Some(&other.id),
                "其他项目",
                None,
                workspace_path,
                Some("node-b"),
                None,
                None,
            )
            .expect_err("binding duplicate workspace path should fail");

        assert!(err.to_string().contains("该本地路径已绑定到项目"));
        assert!(err.to_string().contains(&existing.project.name));
    }

    #[test]
    fn register_external_project_rejects_binding_to_git_identity_owned_by_another_project() {
        let store = temp_store();
        let user = store
            .create_user("pc-project-git-conflict@example.com", "secret1", None, None)
            .expect("user should be created");
        let existing = store
            .register_external_project(
                &user.id,
                None,
                "江西吉安商会",
                None,
                r"D:\rust\active-projects\江西吉安商会",
                Some("node-a"),
                Some("git@github.com:Owner/Jiangxi-Jian.git"),
                Some("main"),
            )
            .expect("first project should register");
        let other = store
            .ensure_project_for_user(
                &user.id,
                "prj-other-git",
                "其他项目",
                None,
                "template",
                "android",
                None,
            )
            .expect("other project should exist");

        let err = store
            .register_external_project(
                &user.id,
                Some(&other.id),
                "其他项目",
                None,
                r"D:\rust\active-projects\other",
                Some("node-b"),
                Some("https://github.com/owner/jiangxi-jian.git"),
                Some("main"),
            )
            .expect_err("binding duplicate git remote should fail");

        assert!(err.to_string().contains("该代码项目已绑定到项目"));
        assert!(err.to_string().contains(&existing.project.name));
    }

    #[test]
    fn ensure_project_for_user_preserves_pc_bound_workspace_path() {
        let store = temp_store();
        let owner = store
            .create_user("pc-bound-owner@example.com", "secret1", None, None)
            .expect("user should be created");
        store
            .ensure_project_for_user(
                &owner.id,
                "elon-self",
                "一龙项目",
                None,
                "template",
                "android",
                None,
            )
            .expect("shared project should exist");
        store
            .register_external_project(
                &owner.id,
                Some("elon-self"),
                "一龙项目",
                Some("PC local repo"),
                r"D:\rust\active-projects\elon cli",
                Some("node-owner"),
                None,
                None,
            )
            .expect("existing shared project should bind");

        let ensured = store
            .ensure_project_for_user(
                &owner.id,
                "elon-self",
                "一龙项目",
                Some("server fallback"),
                "local_path",
                "local",
                Some("/opt/elon/data/projects/elon-self"),
            )
            .expect("ensure should keep project accessible");

        assert_eq!(
            ensured.workspace_path.as_deref(),
            Some(r"D:\rust\active-projects\elon cli")
        );
        assert_eq!(ensured.node_id.as_deref(), Some("node-owner"));
    }
