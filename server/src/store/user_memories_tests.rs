    use super::*;
    use uuid::Uuid;

    fn temp_store() -> Store {
        let path =
            std::env::temp_dir().join(format!("elon_user_memories_{}.db", Uuid::new_v4().simple()));
        Store::open(&path).expect("store should open")
    }

    #[test]
    fn scoped_memories_include_global_but_not_other_scopes() {
        let store = temp_store();
        let user = store
            .create_user("memory-scope@example.com", "secret1", None, None)
            .expect("user should be created");

        store
            .insert_user_memory(&user.id, "全局偏好", "preference", 9, None)
            .expect("global memory should insert");
        store
            .insert_user_memory_scoped(
                &user.id,
                "项目 A 偏好",
                "fact",
                8,
                Some("conv-a"),
                MEMORY_SCOPE_PROJECT,
                Some("project-a"),
            )
            .expect("project A memory should insert");
        store
            .insert_user_memory_scoped(
                &user.id,
                "项目 B 偏好",
                "fact",
                8,
                Some("conv-b"),
                MEMORY_SCOPE_PROJECT,
                Some("project-b"),
            )
            .expect("project B memory should insert");

        let project_a = store
            .get_user_memories_for_scope(&user.id, MEMORY_SCOPE_PROJECT, Some("project-a"), 10)
            .expect("project A memories should list");
        let contents = project_a
            .iter()
            .map(|memory| memory.content.as_str())
            .collect::<Vec<_>>();
        assert!(contents.contains(&"全局偏好"));
        assert!(contents.contains(&"项目 A 偏好"));
        assert!(!contents.contains(&"项目 B 偏好"));

        let exact_project_a = store
            .list_user_memories_for_scope(&user.id, MEMORY_SCOPE_PROJECT, Some("project-a"), 1, 10)
            .expect("exact project A memories should list");
        assert_eq!(exact_project_a.len(), 1);
        assert_eq!(exact_project_a[0].content, "项目 A 偏好");

        let global_only = store
            .get_user_memories(&user.id, 10)
            .expect("global memories should list");
        assert_eq!(global_only.len(), 1);
        assert_eq!(global_only[0].content, "全局偏好");
    }
