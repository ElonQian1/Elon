    use super::*;
    use uuid::Uuid;

    fn temp_store() -> Store {
        let path =
            std::env::temp_dir().join(format!("elon_project_roles_{}.db", Uuid::new_v4().simple()));
        Store::open(&path).expect("store should open")
    }

    #[test]
    fn custom_project_role_can_be_assigned_and_checked() {
        let store = temp_store();
        let owner = store
            .create_user("role-owner@example.com", "secret1", None, None)
            .expect("owner should be created");
        let member = store
            .create_user("role-member@example.com", "secret1", None, None)
            .expect("member should be created");
        let project = store
            .create_project(&owner.id, "Role Project", None, None)
            .expect("project should be created")
            .project;
        let permissions = vec![PERMISSION_INVITE_MEMBERS.to_string()];

        let role = store
            .create_project_role(
                &project.id,
                "审核员",
                Some("#43b581"),
                Some(55),
                Some(&permissions),
                Some(&owner.id),
            )
            .expect("custom role should be created");

        let assigned = store
            .add_project_member_by_account(&project.id, "role-member@example.com", &role.id)
            .expect("custom role should be assignable");
        assert_eq!(assigned.role, role.id);
        assert_eq!(assigned.roles.len(), 1);
        assert_eq!(assigned.roles[0].id, role.id);
        assert_eq!(
            store
                .get_project_access(&member.id, &project.id)
                .expect("member access should load")
                .role,
            role.id
        );
        assert_eq!(
            store
                .project_role_level(&project.id, &role.id)
                .expect("role level should load"),
            55
        );
        assert!(store
            .project_role_has_permission(&project.id, &role.id, PERMISSION_INVITE_MEMBERS)
            .expect("role permission should load"));

        let roles = store
            .list_project_roles(&project.id)
            .expect("roles should list");
        let custom = roles
            .iter()
            .find(|item| item.id == role.id)
            .expect("custom role should be listed");
        assert_eq!(custom.member_count, 1);
        assert!(store
            .delete_project_role(&project.id, &role.id)
            .expect_err("role in use should not delete")
            .to_string()
            .contains("成员"));
    }

    #[test]
    fn project_member_roles_stack_permissions_and_effective_role() {
        let store = temp_store();
        let owner = store
            .create_user("multi-role-owner@example.com", "secret1", None, None)
            .expect("owner should be created");
        let member = store
            .create_user("multi-role-member@example.com", "secret1", None, None)
            .expect("member should be created");
        let project = store
            .create_project(&owner.id, "Multi Role Project", None, None)
            .expect("project should be created")
            .project;
        store
            .add_project_member_by_account(&project.id, "multi-role-member@example.com", "member")
            .expect("member should be invited");

        let permissions = vec![PERMISSION_INVITE_MEMBERS.to_string()];
        let reviewer = store
            .create_project_role(
                &project.id,
                "审核员",
                Some("#43b581"),
                Some(55),
                Some(&permissions),
                Some(&owner.id),
            )
            .expect("custom reviewer role should be created");

        let roles = vec!["member".to_string(), reviewer.id.clone()];
        let updated = store
            .set_project_member_roles(&project.id, &member.id, &roles, Some(&owner.id))
            .expect("member should accept stacked roles");
        assert_eq!(updated.role, reviewer.id);
        assert_eq!(updated.roles[0].id, reviewer.id);
        assert!(updated.roles.iter().any(|role| role.id == "member"));
        assert_eq!(
            store
                .get_project_access(&member.id, &project.id)
                .expect("access should load")
                .role,
            reviewer.id
        );
        assert!(store
            .project_member_has_permission(&project.id, &member.id, PERMISSION_INVITE_MEMBERS)
            .expect("custom permission should apply"));
        assert!(store
            .project_member_has_permission(&project.id, &member.id, PERMISSION_SEND_MESSAGES)
            .expect("builtin permission should still apply"));

        let listed_roles = store
            .list_project_roles(&project.id)
            .expect("roles should list");
        let reviewer_entry = listed_roles
            .iter()
            .find(|role| role.id == reviewer.id)
            .expect("reviewer should be listed");
        assert_eq!(reviewer_entry.member_count, 1);
        assert!(store
            .delete_project_role(&project.id, &reviewer.id)
            .expect_err("assigned role should not delete")
            .to_string()
            .contains("成员"));

        store
            .update_member_role(&project.id, &member.id, "member")
            .expect("single-role update should clear stacked roles");
        store
            .delete_project_role(&project.id, &reviewer.id)
            .expect("unused custom role should delete");
    }
