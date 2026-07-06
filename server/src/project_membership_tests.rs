    use super::{
        can_update_project_brand, can_update_project_icon, clean_project_display_name_update,
        clean_project_icon_data_url_update, ensure_role_management_allowed_by_level,
    };
    use serde_json::{json, Value};

    #[test]
    fn project_icon_update_is_owner_only() {
        assert!(can_update_project_icon("owner"));
        assert!(!can_update_project_icon("admin"));
        assert!(!can_update_project_icon("editor"));
        assert!(!can_update_project_icon("member"));
        assert!(!can_update_project_icon("observer"));
    }

    #[test]
    fn project_brand_update_is_owner_only() {
        assert!(can_update_project_brand("owner"));
        assert!(!can_update_project_brand("admin"));
        assert!(!can_update_project_brand("editor"));
    }

    #[test]
    fn project_display_name_update_distinguishes_missing_clear_and_set() {
        assert_eq!(clean_project_display_name_update(None).unwrap(), None);
        assert_eq!(
            clean_project_display_name_update(Some(&Value::Null)).unwrap(),
            Some(None)
        );
        assert_eq!(
            clean_project_display_name_update(Some(&json!(" 一龙网游加速器 "))).unwrap(),
            Some(Some("一龙网游加速器".to_string()))
        );
    }

    #[test]
    fn project_icon_update_accepts_null_as_clear() {
        assert_eq!(
            clean_project_icon_data_url_update(Some(&Value::Null)).unwrap(),
            Some(None)
        );
        assert_eq!(
            clean_project_icon_data_url_update(Some(&json!("null"))).unwrap(),
            Some(None)
        );
    }

    #[test]
    fn role_hierarchy_blocks_same_or_higher_management() {
        assert!(ensure_role_management_allowed_by_level(
            100,
            Some(80),
            Some(60),
            Some("editor"),
            "修改"
        )
        .is_ok());
        assert!(ensure_role_management_allowed_by_level(
            80,
            Some(60),
            Some(40),
            Some("member"),
            "修改"
        )
        .is_ok());
        assert!(ensure_role_management_allowed_by_level(80, Some(80), None, None, "移除").is_err());
        assert!(ensure_role_management_allowed_by_level(
            80,
            Some(60),
            Some(80),
            Some("admin"),
            "修改"
        )
        .is_err());
        assert!(ensure_role_management_allowed_by_level(0, Some(40), None, None, "移除").is_err());
    }
