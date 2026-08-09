use serde_json::json;
use uuid::Uuid;

use crate::{
    open_commerce_consumer,
    open_commerce_developer_model::{CreateAuthorizationRequest, CreateDeveloperAppRequest},
    open_commerce_directory_service,
    open_commerce_model::{
        CreateCapabilityRequest, CreateGrantRequest, CreateMerchantRequest, ACCESS_AUTHORIZED,
        HANDLER_STATIC_JSON,
    },
    open_commerce_service::{self, OpenCommerceActor},
    store::Store,
};

pub(super) struct Fixture {
    pub(super) store: Store,
    pub(super) merchant_owner_id: String,
    pub(super) merchant_project_id: String,
    pub(super) merchant_id: String,
    pub(super) consumer_id: String,
    pub(super) consumer_project_id: String,
    pub(super) app_id: String,
    pub(super) second_app_id: String,
    pub(super) test_token: String,
    pub(super) teammate_id: String,
    pub(super) teammate_app_id: String,
    pub(super) other_project_id: String,
    pub(super) pending_request_id: String,
    pub(super) rejected_request_id: String,
    pub(super) approved_request_id: String,
    pub(super) canceled_request_id: String,
    pub(super) approved_grant_id: String,
    pub(super) count_exhausted_grant_id: String,
    pub(super) amount_exhausted_grant_id: String,
    pub(super) unlimited_grant_id: String,
    pub(super) second_app_grant_id: String,
    pub(super) expired_grant_id: String,
    pub(super) revoked_grant_id: String,
}

pub(super) fn fixture() -> Fixture {
    let path = std::env::temp_dir().join(format!(
        "elon_consumer_authorization_mcp_{}.db",
        Uuid::new_v4().simple()
    ));
    let store = Store::open(&path).expect("consumer authorization MCP store should open");
    let merchant_owner = store
        .create_user(
            "authorization-mcp-merchant@example.com",
            "secret1",
            Some("Merchant Owner"),
            None,
        )
        .unwrap();
    let merchant_project = store
        .create_project(&merchant_owner.id, "Authorization Merchant", None, None)
        .unwrap()
        .project;
    let merchant_actor = OpenCommerceActor {
        user_id: &merchant_owner.id,
        app_id: "pc-web",
        project_role: Some("owner"),
    };
    let merchant = open_commerce_service::create_merchant(
        &store,
        &merchant_project.id,
        &merchant_actor,
        CreateMerchantRequest {
            display_name: "授权状态咖啡店".to_string(),
            slug: Some("authorization-status-cafe".to_string()),
            description: String::new(),
            node_mode: "platform_hosted".to_string(),
            public_profile: json!({}),
        },
    )
    .unwrap();
    open_commerce_service::publish_capability(
        &store,
        &merchant_project.id,
        &merchant.id,
        &merchant_actor,
        CreateCapabilityRequest {
            capability_key: "menu.preview".to_string(),
            display_name: "菜单预览".to_string(),
            description: String::new(),
            kind: "query".to_string(),
            access_level: ACCESS_AUTHORIZED.to_string(),
            input_schema: json!({}),
            output_schema: json!({}),
            handler_type: HANDLER_STATIC_JSON.to_string(),
            handler_config: Some(json!({"response":{"items":["latte"]}})),
            unit_price_micros: 1_000,
            currency: "CNY".to_string(),
            freshness_seconds: 30,
        },
    )
    .unwrap();
    open_commerce_directory_service::set_publication(
        &store,
        &merchant_project.id,
        &merchant.id,
        &merchant_actor,
        true,
    )
    .unwrap();

    let consumer = store
        .create_user(
            "authorization-mcp-consumer@example.com",
            "secret1",
            Some("Consumer"),
            None,
        )
        .unwrap();
    let consumer_project = store
        .create_project(&consumer.id, "Authorization Consumer", None, None)
        .unwrap()
        .project;
    let app = create_app(
        &store,
        &consumer_project.id,
        &consumer.id,
        "consumer.auth-a",
    );
    let second_app = create_app(
        &store,
        &consumer_project.id,
        &consumer.id,
        "consumer.auth-b",
    );
    let teammate = store
        .create_user(
            "authorization-mcp-teammate@example.com",
            "secret1",
            Some("Teammate"),
            None,
        )
        .unwrap();
    let teammate_app = create_app(
        &store,
        &consumer_project.id,
        &teammate.id,
        "consumer.auth-teammate",
    );
    let other_project = store
        .create_project(&consumer.id, "Authorization Other", None, None)
        .unwrap()
        .project;
    let other_project_app = create_app(
        &store,
        &other_project.id,
        &consumer.id,
        "consumer.auth-other-project",
    );

    create_request(
        &store,
        &teammate.id,
        &merchant.id,
        &teammate_app.app.app_id,
        "同项目其他用户申请",
    );
    create_request(
        &store,
        &consumer.id,
        &merchant.id,
        &other_project_app.app.app_id,
        "同用户其他项目申请",
    );

    let rejected = create_request(
        &store,
        &consumer.id,
        &merchant.id,
        &app.app.app_id,
        "拒绝申请",
    );
    store
        .decide_open_commerce_authorization_request(
            &merchant_project.id,
            &rejected.id,
            &merchant_owner.id,
            "rejected",
            "用途不清晰",
            None,
        )
        .unwrap();

    let approved = create_request(
        &store,
        &consumer.id,
        &merchant.id,
        &app.app.app_id,
        "批准申请",
    );
    let approved_grant = create_grant(
        &store,
        &merchant_project.id,
        &merchant_owner.id,
        &merchant.id,
        &app.app.app_id,
        Some(5),
        Some(5_000),
        "批准申请",
    );
    store
        .conn()
        .unwrap()
        .execute(
            "UPDATE open_commerce_grants
             SET used_invocations=2, used_amount_micros=2000 WHERE id=?1",
            [&approved_grant],
        )
        .unwrap();
    store
        .decide_open_commerce_authorization_request(
            &merchant_project.id,
            &approved.id,
            &merchant_owner.id,
            "approved",
            "用途清晰",
            Some(&approved_grant),
        )
        .unwrap();

    let canceled = create_request(
        &store,
        &consumer.id,
        &merchant.id,
        &second_app.app.app_id,
        "已取消申请",
    );
    store
        .cancel_requester_open_commerce_authorization_request(&consumer_project.id, &canceled.id)
        .unwrap();
    let pending = create_request(
        &store,
        &consumer.id,
        &merchant.id,
        &app.app.app_id,
        "待处理申请",
    );

    let count_exhausted = create_grant(
        &store,
        &merchant_project.id,
        &merchant_owner.id,
        &merchant.id,
        &app.app.app_id,
        Some(1),
        Some(5_000),
        "次数耗尽",
    );
    set_grant_usage(&store, &count_exhausted, 1, 1_000);
    let amount_exhausted = create_grant(
        &store,
        &merchant_project.id,
        &merchant_owner.id,
        &merchant.id,
        &app.app.app_id,
        Some(5),
        Some(1_000),
        "金额耗尽",
    );
    set_grant_usage(&store, &amount_exhausted, 0, 1_000);
    let unlimited = create_grant(
        &store,
        &merchant_project.id,
        &merchant_owner.id,
        &merchant.id,
        &app.app.app_id,
        None,
        None,
        "无限额度",
    );
    let second_app_grant = create_grant(
        &store,
        &merchant_project.id,
        &merchant_owner.id,
        &merchant.id,
        &second_app.app.app_id,
        Some(3),
        Some(3_000),
        "第二应用授权",
    );
    let expired = create_grant(
        &store,
        &merchant_project.id,
        &merchant_owner.id,
        &merchant.id,
        &app.app.app_id,
        Some(3),
        Some(3_000),
        "已过期",
    );
    store
        .conn()
        .unwrap()
        .execute(
            "UPDATE open_commerce_grants SET expires_at='2000-01-01T00:00:00Z' WHERE id=?1",
            [&expired],
        )
        .unwrap();
    let revoked = create_grant(
        &store,
        &merchant_project.id,
        &merchant_owner.id,
        &merchant.id,
        &app.app.app_id,
        Some(3),
        Some(3_000),
        "已撤销",
    );
    store
        .revoke_open_commerce_grant(&merchant_project.id, &revoked)
        .unwrap();
    create_grant(
        &store,
        &merchant_project.id,
        &merchant_owner.id,
        &merchant.id,
        &teammate_app.app.app_id,
        Some(3),
        Some(3_000),
        "其他用户授权",
    );

    Fixture {
        store,
        merchant_owner_id: merchant_owner.id,
        merchant_project_id: merchant_project.id,
        merchant_id: merchant.id,
        consumer_id: consumer.id,
        consumer_project_id: consumer_project.id,
        app_id: app.app.app_id,
        second_app_id: second_app.app.app_id,
        test_token: app.test_token,
        teammate_id: teammate.id,
        teammate_app_id: teammate_app.app.app_id,
        other_project_id: other_project.id,
        pending_request_id: pending.id,
        rejected_request_id: rejected.id,
        approved_request_id: approved.id,
        canceled_request_id: canceled.id,
        approved_grant_id: approved_grant,
        count_exhausted_grant_id: count_exhausted,
        amount_exhausted_grant_id: amount_exhausted,
        unlimited_grant_id: unlimited,
        second_app_grant_id: second_app_grant,
        expired_grant_id: expired,
        revoked_grant_id: revoked,
    }
}

fn create_app(
    store: &Store,
    project_id: &str,
    owner_user_id: &str,
    app_id: &str,
) -> crate::open_commerce_developer_model::OpenCommerceDeveloperAppCredential {
    store
        .create_open_commerce_developer_app(
            project_id,
            owner_user_id,
            CreateDeveloperAppRequest {
                app_id: app_id.to_string(),
                display_name: "相同显示名称".to_string(),
            },
        )
        .unwrap()
}

fn create_request(
    store: &Store,
    user_id: &str,
    merchant_id: &str,
    app_id: &str,
    purpose: &str,
) -> crate::open_commerce_developer_model::OpenCommerceAuthorizationRequest {
    open_commerce_consumer::create_authorization_request(
        store,
        user_id,
        CreateAuthorizationRequest {
            merchant_id: merchant_id.to_string(),
            requester_app_id: app_id.to_string(),
            scopes: vec!["menu.preview".to_string()],
            purpose: purpose.to_string(),
        },
    )
    .unwrap()
}

fn create_grant(
    store: &Store,
    merchant_project_id: &str,
    merchant_owner_id: &str,
    merchant_id: &str,
    app_id: &str,
    max_invocations: Option<i64>,
    max_amount_micros: Option<i64>,
    purpose: &str,
) -> String {
    let actor = OpenCommerceActor {
        user_id: merchant_owner_id,
        app_id: "pc-web",
        project_role: Some("owner"),
    };
    open_commerce_service::create_grant(
        store,
        merchant_project_id,
        &actor,
        CreateGrantRequest {
            merchant_id: merchant_id.to_string(),
            grantee_app_id: app_id.to_string(),
            scopes: vec!["menu.preview".to_string()],
            purpose: purpose.to_string(),
            expires_at: None,
            max_invocations,
            max_amount_micros,
            budget_currency: "CNY".to_string(),
        },
    )
    .unwrap()
    .id
}

fn set_grant_usage(store: &Store, grant_id: &str, calls: i64, amount_micros: i64) {
    store
        .conn()
        .unwrap()
        .execute(
            "UPDATE open_commerce_grants
             SET used_invocations=?1, used_amount_micros=?2 WHERE id=?3",
            rusqlite::params![calls, amount_micros, grant_id],
        )
        .unwrap();
}
