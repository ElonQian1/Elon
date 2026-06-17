use crate::external_app_registry::{external_app_by_id, group_seeds};

use super::{ExternalAccountSessionInput, Store};

fn temp_store() -> Store {
    let path = std::env::temp_dir().join(format!(
        "elon_external_apps_{}.db",
        uuid::Uuid::new_v4().simple()
    ));
    Store::open(&path).expect("store should open")
}

#[test]
fn external_app_session_links_user_and_auto_joins_default_group() {
    let store = temp_store();
    let app = external_app_by_id("fb2").expect("fb2 app should exist");
    let seeds = group_seeds(app);

    let session = store
        .create_external_app_session(
            app.id,
            &seeds,
            ExternalAccountSessionInput {
                external_user_id: "fb2-user-1".into(),
                account: "13800138000".into(),
                display_name: Some("fb2用户".into()),
                avatar_url: None,
                device_name: Some("fb2 user app".into()),
                apk_version: Some("1.2.0".into()),
            },
        )
        .expect("external app session should create");

    assert!(!session.token.is_empty());
    assert_eq!(session.user.account, "13800138000");
    assert_eq!(
        session.account.main_user_id.as_deref(),
        Some(session.user.id.as_str())
    );
    assert_eq!(session.default_groups.len(), 5);

    let groups = store
        .list_friend_groups(&session.user.id)
        .expect("groups should load");
    assert!(groups.iter().any(|group| group.id == "ext_fb2_official"));
    assert!(!groups.iter().any(|group| group.id == "ext_fb2_expert"));

    let hint = store
        .external_account_origin_hint("13800138000")
        .expect("lookup should run")
        .expect("origin should exist");
    assert_eq!(hint.app_id, "fb2");
    assert_eq!(hint.external_user_id, "fb2-user-1");

    let second = store
        .create_external_app_session(
            app.id,
            &seeds,
            ExternalAccountSessionInput {
                external_user_id: "fb2-user-1".into(),
                account: "13800138000".into(),
                display_name: Some("fb2用户新昵称".into()),
                avatar_url: None,
                device_name: None,
                apk_version: None,
            },
        )
        .expect("second session should reuse link");
    assert_eq!(second.user.id, session.user.id);
}

#[test]
fn external_app_authorization_code_is_single_use() {
    let store = temp_store();
    let user = store
        .create_user("main-user@example.com", "secret1", Some("主项目用户"), None)
        .expect("user should create");

    let code = store
        .create_external_app_authorization_code(
            "fb2",
            &user.id,
            vec!["profile".into(), "chat_center".into()],
            Some("fb2://auth/callback"),
        )
        .expect("auth code should create");
    assert!(code.code.starts_with("eac_"));

    let exchange = store
        .exchange_external_app_authorization_code("fb2", &code.code)
        .expect("first exchange should succeed");
    assert_eq!(exchange.user.id, user.id);
    assert_eq!(
        exchange.redirect_uri.as_deref(),
        Some("fb2://auth/callback")
    );

    let second = store.exchange_external_app_authorization_code("fb2", &code.code);
    assert!(second.is_err());
}
