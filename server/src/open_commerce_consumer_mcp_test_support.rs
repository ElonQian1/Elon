use serde_json::json;
use uuid::Uuid;

use crate::{
    open_commerce_app_block_model::BlockOpenCommerceAppRequest,
    open_commerce_app_block_service, open_commerce_consumer,
    open_commerce_developer_model::{CreateAuthorizationRequest, CreateDeveloperAppRequest},
    open_commerce_directory_service,
    open_commerce_model::{
        CreateCapabilityRequest, CreateGrantRequest, CreateMerchantRequest, OpenCommerceGrant,
        ACCESS_AUTHORIZED, ACCESS_PUBLIC, HANDLER_STATIC_JSON,
    },
    open_commerce_service::{self, OpenCommerceActor},
    store::Store,
};

pub(super) const PUBLIC_QUERY: &str = "menu.public";
pub(super) const PUBLIC_ACTION: &str = "order.public";
pub(super) const AUTHORIZED_QUERY: &str = "menu.authorized";
pub(super) const AUTHORIZED_ACTION: &str = "order.authorized";
pub(super) const FREE_AUTHORIZED_QUERY: &str = "menu.free";

#[derive(Debug, PartialEq, Eq)]
pub(super) struct SideEffectSnapshot {
    counts: Vec<(&'static str, i64)>,
    grant_usage: Vec<(String, i64, i64, String)>,
}

pub(super) struct Fixture {
    pub(super) store: Store,
    pub(super) merchant_owner_id: String,
    pub(super) merchant_project_id: String,
    pub(super) merchant_id: String,
    pub(super) consumer_id: String,
    pub(super) consumer_project_id: String,
    pub(super) app_record_id: String,
    pub(super) app_id: String,
    pub(super) other_user_id: String,
    pub(super) other_app_id: String,
}

pub(super) fn fixture() -> Fixture {
    let path = std::env::temp_dir().join(format!(
        "elon_consumer_execution_plan_{}.db",
        Uuid::new_v4().simple()
    ));
    let store = Store::open(&path).expect("consumer execution plan store should open");
    let merchant_owner = store
        .create_user(
            "execution-plan-merchant@example.com",
            "secret1",
            Some("Execution Plan Merchant"),
            None,
        )
        .unwrap();
    let merchant_project = store
        .create_project(&merchant_owner.id, "Execution Plan Merchant", None, None)
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
            display_name: "执行计划咖啡店".to_string(),
            slug: Some("execution-plan-cafe".to_string()),
            description: String::new(),
            node_mode: "platform_hosted".to_string(),
            public_profile: json!({"category":"cafe"}),
        },
    )
    .unwrap();
    for (key, kind, access_level, unit_price_micros) in [
        (PUBLIC_QUERY, "query", ACCESS_PUBLIC, 1_000),
        (PUBLIC_ACTION, "action", ACCESS_PUBLIC, 1_000),
        (AUTHORIZED_QUERY, "query", ACCESS_AUTHORIZED, 1_000),
        (AUTHORIZED_ACTION, "action", ACCESS_AUTHORIZED, 1_000),
        (FREE_AUTHORIZED_QUERY, "query", ACCESS_AUTHORIZED, 0),
    ] {
        open_commerce_service::publish_capability(
            &store,
            &merchant_project.id,
            &merchant.id,
            &merchant_actor,
            CreateCapabilityRequest {
                capability_key: key.to_string(),
                display_name: key.to_string(),
                description: String::new(),
                kind: kind.to_string(),
                access_level: access_level.to_string(),
                input_schema: json!({
                    "type":"object",
                    "required":["sku"],
                    "properties":{"sku":{"type":"string","minLength":1}},
                    "additionalProperties":false
                }),
                output_schema: json!({}),
                handler_type: HANDLER_STATIC_JSON.to_string(),
                handler_config: Some(json!({"response":{"ok":true}})),
                unit_price_micros,
                currency: "CNY".to_string(),
                freshness_seconds: 30,
            },
        )
        .unwrap();
    }
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
            "execution-plan-consumer@example.com",
            "secret1",
            Some("Execution Plan Consumer"),
            None,
        )
        .unwrap();
    let consumer_project = store
        .create_project(&consumer.id, "Execution Plan Consumer", None, None)
        .unwrap()
        .project;
    let app = create_app(
        &store,
        &consumer_project.id,
        &consumer.id,
        "consumer.execution-plan",
    );
    let other_user = store
        .create_user(
            "execution-plan-other@example.com",
            "secret1",
            Some("Other Consumer"),
            None,
        )
        .unwrap();
    let other_project = store
        .create_project(&other_user.id, "Other Consumer", None, None)
        .unwrap()
        .project;
    let other_app = create_app(
        &store,
        &other_project.id,
        &other_user.id,
        "consumer.execution-plan-other",
    );

    Fixture {
        store,
        merchant_owner_id: merchant_owner.id,
        merchant_project_id: merchant_project.id,
        merchant_id: merchant.id,
        consumer_id: consumer.id,
        consumer_project_id: consumer_project.id,
        app_record_id: app.app.id,
        app_id: app.app.app_id,
        other_user_id: other_user.id,
        other_app_id: other_app.app.app_id,
    }
}

impl Fixture {
    pub(super) fn grant(
        &self,
        capability_key: &str,
        max_invocations: Option<i64>,
        max_amount_micros: Option<i64>,
        budget_currency: &str,
    ) -> OpenCommerceGrant {
        let actor = self.merchant_actor();
        open_commerce_service::create_grant(
            &self.store,
            &self.merchant_project_id,
            &actor,
            CreateGrantRequest {
                merchant_id: self.merchant_id.clone(),
                grantee_app_id: self.app_id.clone(),
                scopes: vec![capability_key.to_string()],
                purpose: "验证消费者执行计划".to_string(),
                expires_at: None,
                max_invocations,
                max_amount_micros,
                budget_currency: budget_currency.to_string(),
            },
        )
        .unwrap()
    }

    pub(super) fn set_grant_usage(
        &self,
        grant_id: &str,
        used_invocations: i64,
        used_amount_micros: i64,
    ) {
        self.store
            .conn()
            .unwrap()
            .execute(
                "UPDATE open_commerce_grants
                 SET used_invocations=?1, used_amount_micros=?2 WHERE id=?3",
                rusqlite::params![used_invocations, used_amount_micros, grant_id],
            )
            .unwrap();
    }

    pub(super) fn set_grant_created_at(&self, grant_id: &str, created_at: &str) {
        self.store
            .conn()
            .unwrap()
            .execute(
                "UPDATE open_commerce_grants SET created_at=?1, updated_at=?1 WHERE id=?2",
                rusqlite::params![created_at, grant_id],
            )
            .unwrap();
    }

    pub(super) fn expire_grant(&self, grant_id: &str) {
        self.store
            .conn()
            .unwrap()
            .execute(
                "UPDATE open_commerce_grants
                 SET expires_at='2000-01-01T00:00:00Z' WHERE id=?1",
                [grant_id],
            )
            .unwrap();
    }

    pub(super) fn create_request(
        &self,
        capability_key: &str,
    ) -> crate::open_commerce_developer_model::OpenCommerceAuthorizationRequest {
        open_commerce_consumer::create_authorization_request(
            &self.store,
            &self.consumer_id,
            CreateAuthorizationRequest {
                merchant_id: self.merchant_id.clone(),
                requester_app_id: self.app_id.clone(),
                scopes: vec![capability_key.to_string()],
                purpose: "消费者执行计划授权".to_string(),
            },
        )
        .unwrap()
    }

    pub(super) fn approve_request(
        &self,
        request_id: &str,
        grant_id: &str,
    ) -> crate::open_commerce_developer_model::OpenCommerceAuthorizationRequest {
        self.store
            .decide_open_commerce_authorization_request(
                &self.merchant_project_id,
                request_id,
                &self.merchant_owner_id,
                "approved",
                "批准执行计划测试",
                Some(grant_id),
            )
            .unwrap()
    }

    pub(super) fn block_app(&self) {
        open_commerce_app_block_service::block_app(
            &self.store,
            &self.merchant_project_id,
            &self.merchant_owner_id,
            "pc-web",
            "owner",
            BlockOpenCommerceAppRequest {
                merchant_id: self.merchant_id.clone(),
                requester_app_id: self.app_id.clone(),
                reason_code: "merchant_request".to_string(),
                reason_note: "执行计划封禁验证".to_string(),
            },
        )
        .unwrap();
    }

    pub(super) fn set_published(&self, published: bool) {
        let actor = self.merchant_actor();
        open_commerce_directory_service::set_publication(
            &self.store,
            &self.merchant_project_id,
            &self.merchant_id,
            &actor,
            published,
        )
        .unwrap();
    }

    pub(super) fn snapshot(&self) -> SideEffectSnapshot {
        let conn = self.store.conn().unwrap();
        let counts = [
            "open_commerce_authorization_requests",
            "open_commerce_grants",
            "open_commerce_action_confirmations",
            "open_commerce_invocations",
            "open_commerce_grant_budget_reservations",
            "open_commerce_audit_events",
        ]
        .into_iter()
        .map(|table| {
            let count = conn
                .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                    row.get(0)
                })
                .unwrap();
            (table, count)
        })
        .collect();
        let mut statement = conn
            .prepare(
                "SELECT id, used_invocations, used_amount_micros, updated_at
                   FROM open_commerce_grants ORDER BY id",
            )
            .unwrap();
        let grant_usage = statement
            .query_map([], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
            })
            .unwrap()
            .collect::<rusqlite::Result<Vec<_>>>()
            .unwrap();
        SideEffectSnapshot {
            counts,
            grant_usage,
        }
    }

    fn merchant_actor(&self) -> OpenCommerceActor<'_> {
        OpenCommerceActor {
            user_id: &self.merchant_owner_id,
            app_id: "pc-web",
            project_role: Some("owner"),
        }
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
                display_name: "执行计划消费者".to_string(),
            },
        )
        .unwrap()
}
