#[cfg(test)]
mod tests {
    use super::super::*;
    use uuid::Uuid;

    fn temp_store() -> super::super::super::Store {
        let path = std::env::temp_dir().join(format!(
            "elon_project_suggestions_{}.db",
            Uuid::new_v4().simple()
        ));
        super::super::super::Store::open(&path).expect("store should open")
    }

    #[test]
    fn default_channels_include_suggestions() {
        let store = temp_store();
        let owner = store
            .create_user("suggestions-owner@example.com", "secret1", None, None)
            .expect("owner should be created");
        let project = store
            .create_project(&owner.id, "Suggestions Project", None, None)
            .expect("project should be created")
            .project;

        let channels = store
            .list_project_space_channels(&owner.id, &project.id)
            .expect("channels should list");

        assert!(channels.iter().any(|channel| channel.kind == "suggestions"));
        assert!(channels.iter().any(|channel| channel.kind == "docs"));
    }

    #[test]
    fn suggestion_message_can_be_marked_updated() {
        let store = temp_store();
        let owner = store
            .create_user("suggestions-resolver@example.com", "secret1", None, None)
            .expect("owner should be created");
        let project = store
            .create_project(&owner.id, "Suggestion Resolve", None, None)
            .expect("project should be created")
            .project;
        let channel = store
            .list_project_space_channels(&owner.id, &project.id)
            .expect("channels should list")
            .into_iter()
            .find(|channel| channel.kind == "suggestions")
            .expect("suggestions channel should exist");

        let message = store
            .insert_project_channel_message(
                &project.id,
                &channel.id,
                Some(&owner.id),
                "suggestion",
                "希望增加深色模式",
                None,
                None,
            )
            .expect("suggestion should insert");
        assert_eq!(message.suggestion_status.as_deref(), Some("open"));

        let updated = store
            .mark_project_suggestion_updated(&owner.id, &project.id, &channel.id, &message.id)
            .expect("suggestion should update");

        assert_eq!(updated.suggestion_status.as_deref(), Some("updated"));
        assert_eq!(
            updated.suggestion_resolved_by.as_deref(),
            Some(owner.id.as_str())
        );
    }

    #[test]
    fn project_description_can_be_updated_and_cleared() {
        let store = temp_store();
        let owner = store
            .create_user("intro-owner@example.com", "secret1", None, None)
            .expect("owner should be created");
        let project = store
            .create_project(&owner.id, "Intro Project", Some("旧简介"), None)
            .expect("project should be created")
            .project;

        let updated = store
            .update_project_description(&owner.id, &project.id, "  一款太逃杀类型的卡牌游戏  ")
            .expect("description should update");
        assert_eq!(
            updated.description.as_deref(),
            Some("一款太逃杀类型的卡牌游戏")
        );

        let cleared = store
            .update_project_description(&owner.id, &project.id, "   ")
            .expect("description should clear");
        assert!(cleared.description.is_none());
    }

    #[test]
    fn channel_role_permission_overrides_hide_and_allow_channels() {
        let store = temp_store();
        let owner = store
            .create_user("channel-perm-owner@example.com", "secret1", None, None)
            .expect("owner should be created");
        let viewer = store
            .create_user("channel-perm-viewer@example.com", "secret1", None, None)
            .expect("viewer should be created");
        let project = store
            .create_project(&owner.id, "Channel Permissions", None, None)
            .expect("project should be created")
            .project;
        store
            .add_project_member_by_account(
                &project.id,
                "channel-perm-viewer@example.com",
                "observer",
            )
            .expect("viewer should be invited");
        let channel = store
            .list_project_space_channels(&owner.id, &project.id)
            .expect("owner channels should list")
            .into_iter()
            .find(|channel| channel.kind == "discussion")
            .expect("discussion channel should exist");

        store
            .set_project_channel_role_permission_override(
                &project.id,
                &channel.id,
                "observer",
                &[],
                &[CHANNEL_PERMISSION_VIEW.to_string()],
                Some(&owner.id),
            )
            .expect("deny override should save");
        let permissions = store
            .project_member_channel_permissions(&project.id, &channel.id, &viewer.id)
            .expect("permissions should load");
        assert!(!permissions.can_view);
        assert!(!store
            .list_project_space_channels(&viewer.id, &project.id)
            .expect("viewer channels should list")
            .iter()
            .any(|item| item.id == channel.id));

        store
            .set_project_channel_role_permission_override(
                &project.id,
                &channel.id,
                "observer",
                &[
                    CHANNEL_PERMISSION_VIEW.to_string(),
                    CHANNEL_PERMISSION_SEND.to_string(),
                ],
                &[],
                Some(&owner.id),
            )
            .expect("allow override should save");
        let permissions = store
            .project_member_channel_permissions(&project.id, &channel.id, &viewer.id)
            .expect("permissions should reload");
        assert!(permissions.can_view);
        assert!(permissions.can_send);
    }

    #[test]
    fn channel_category_permissions_are_inherited_before_channel_overrides() {
        let store = temp_store();
        let owner = store
            .create_user("category-perm-owner@example.com", "secret1", None, None)
            .expect("owner should be created");
        let viewer = store
            .create_user("category-perm-viewer@example.com", "secret1", None, None)
            .expect("viewer should be created");
        let project = store
            .create_project(&owner.id, "Category Permissions", None, None)
            .expect("project should be created")
            .project;
        store
            .add_project_member_by_account(
                &project.id,
                "category-perm-viewer@example.com",
                "observer",
            )
            .expect("viewer should be invited");
        let categories = store
            .list_project_channel_categories(&project.id)
            .expect("categories should list");
        let feedback = categories
            .iter()
            .find(|category| category.kind == "feedback")
            .expect("feedback category should exist");
        let channel = store
            .list_project_space_channels(&owner.id, &project.id)
            .expect("owner channels should list")
            .into_iter()
            .find(|channel| channel.kind == "requirements")
            .expect("requirements channel should exist");

        store
            .set_project_channel_category_role_permission_override(
                &project.id,
                &feedback.id,
                "observer",
                &[],
                &[CHANNEL_PERMISSION_VIEW.to_string()],
                Some(&owner.id),
            )
            .expect("category deny should save");
        assert!(
            !store
                .project_member_channel_permissions(&project.id, &channel.id, &viewer.id)
                .expect("permissions should load")
                .can_view
        );

        store
            .set_project_channel_role_permission_override(
                &project.id,
                &channel.id,
                "observer",
                &[CHANNEL_PERMISSION_VIEW.to_string()],
                &[],
                Some(&owner.id),
            )
            .expect("channel allow should save");
        assert!(
            store
                .project_member_channel_permissions(&project.id, &channel.id, &viewer.id)
                .expect("permissions should reload")
                .can_view
        );
    }

    #[test]
    fn channel_member_permission_overrides_can_override_role_denies() {
        let store = temp_store();
        let owner = store
            .create_user(
                "channel-member-perm-owner@example.com",
                "secret1",
                None,
                None,
            )
            .expect("owner should be created");
        let viewer = store
            .create_user(
                "channel-member-perm-viewer@example.com",
                "secret1",
                None,
                None,
            )
            .expect("viewer should be created");
        let project = store
            .create_project(&owner.id, "Channel Member Permissions", None, None)
            .expect("project should be created")
            .project;
        store
            .add_project_member_by_account(
                &project.id,
                "channel-member-perm-viewer@example.com",
                "observer",
            )
            .expect("viewer should be invited");
        let channel = store
            .list_project_space_channels(&owner.id, &project.id)
            .expect("owner channels should list")
            .into_iter()
            .find(|channel| channel.kind == "discussion")
            .expect("discussion channel should exist");

        store
            .set_project_channel_role_permission_override(
                &project.id,
                &channel.id,
                "observer",
                &[],
                &[CHANNEL_PERMISSION_VIEW.to_string()],
                Some(&owner.id),
            )
            .expect("role deny override should save");
        assert!(
            !store
                .project_member_channel_permissions(&project.id, &channel.id, &viewer.id)
                .expect("permissions should load")
                .can_view
        );

        store
            .set_project_channel_member_permission_override(
                &project.id,
                &channel.id,
                &viewer.id,
                &[
                    CHANNEL_PERMISSION_VIEW.to_string(),
                    CHANNEL_PERMISSION_SEND.to_string(),
                ],
                &[],
                Some(&owner.id),
            )
            .expect("member allow override should save");
        let permissions = store
            .project_member_channel_permissions(&project.id, &channel.id, &viewer.id)
            .expect("permissions should reload");
        assert!(permissions.can_view);
        assert!(permissions.can_send);
    }
}
