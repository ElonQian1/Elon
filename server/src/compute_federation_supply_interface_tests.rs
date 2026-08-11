use std::sync::Arc;

use axum::{
    body::{to_bytes, Body},
    http::{header, Method, Request, StatusCode},
    Router,
};
use chrono::{Duration, Utc};
use serde_json::{json, Value};
use tower::ServiceExt;
use uuid::Uuid;

use crate::{
    open_commerce_developer_production_test_support::test_app_state, store::Store, types::AppState,
};

const CREATE_PROVIDER: &str = "compute_create_my_provider";
const CREATE_POOL: &str = "compute_create_my_capacity_pool";
const CREATE_BUCKET: &str = "compute_create_my_capacity_bucket";
const ADD_SUPPLY: &str = "compute_add_my_capacity_supply";
const WITHDRAW_SUPPLY: &str = "compute_withdraw_my_capacity_supply";

#[tokio::test]
async fn owner_http_supply_flow_requires_bearer_and_is_auditable() {
    let fixture = Fixture::new();
    let provider_path = "/api/me/compute/providers";

    assert_eq!(
        call_http(
            &fixture.router,
            Method::POST,
            provider_path,
            None,
            provider_body(&fixture.provider_id),
        )
        .await
        .0,
        StatusCode::UNAUTHORIZED
    );

    let (status, provider) = call_http(
        &fixture.router,
        Method::POST,
        provider_path,
        Some(&fixture.owner_token),
        provider_body(&fixture.provider_id),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{provider}");
    assert_eq!(provider["provider_id"], fixture.provider_id);

    let pools_path = fixture.pools_path();
    let (status, pool) = call_http(
        &fixture.router,
        Method::POST,
        &pools_path,
        Some(&fixture.owner_token),
        pool_body(&fixture.pool_id),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{pool}");
    assert_eq!(pool["capacity_epoch"], 1);

    for (bucket_id, meter) in [
        (&fixture.token_bucket_id, "tokens"),
        (&fixture.concurrency_bucket_id, "concurrency"),
    ] {
        let (status, bucket) = call_http(
            &fixture.router,
            Method::POST,
            &fixture.buckets_path(),
            Some(&fixture.owner_token),
            bucket_body(&fixture, bucket_id, meter),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{bucket}");
        assert_eq!(bucket["balance"]["available_units"], 0);
    }

    let supply_body = supply_body(&fixture, "http-add", 100, 4, true);
    let (status, supplied) = call_http(
        &fixture.router,
        Method::POST,
        &fixture.supply_path(),
        Some(&fixture.owner_token),
        supply_body.clone(),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{supplied}");
    assert_eq!(supplied["event_kind"], "supply_added");
    assert_eq!(supplied["replayed"], false);

    let (status, replayed) = call_http(
        &fixture.router,
        Method::POST,
        &fixture.supply_path(),
        Some(&fixture.owner_token),
        supply_body,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{replayed}");
    assert_eq!(replayed["replayed"], true);
    assert_eq!(replayed["transaction_id"], supplied["transaction_id"]);

    let (status, audit) = call_http(
        &fixture.router,
        Method::GET,
        &format!("{}/audit", fixture.pool_path()),
        Some(&fixture.owner_token),
        Value::Null,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{audit}");
    assert_eq!(audit["healthy"], true);
    assert_eq!(audit["transaction_count"], 1);

    let (status, history) = call_http(
        &fixture.router,
        Method::GET,
        &format!("{}/ledger-transactions", fixture.pool_path()),
        Some(&fixture.owner_token),
        Value::Null,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{history}");
    assert_eq!(history["transactions"].as_array().unwrap().len(), 1);
    assert_eq!(history["transactions"][0]["event_kind"], "supply_added");
}

#[tokio::test]
async fn http_supply_flow_rejects_outsiders_and_unconfirmed_mutations() {
    let fixture = Fixture::new();
    fixture.create_chain_via_mcp();

    let (status, body) = call_http(
        &fixture.router,
        Method::GET,
        &fixture.pool_path(),
        Some(&fixture.outsider_token),
        Value::Null,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert!(body["error"]
        .as_str()
        .unwrap()
        .contains("不属于当前登录用户"));

    let (status, body) = call_http(
        &fixture.router,
        Method::POST,
        &fixture.supply_path(),
        Some(&fixture.owner_token),
        supply_body(&fixture, "http-unconfirmed", 100, 4, false),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");

    let (status, buckets) = call_http(
        &fixture.router,
        Method::GET,
        &fixture.buckets_path(),
        Some(&fixture.owner_token),
        Value::Null,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{buckets}");
    assert_eq!(buckets.as_array().unwrap().len(), 2);
    assert!(buckets
        .as_array()
        .unwrap()
        .iter()
        .all(|bucket| bucket["balance"]["issued_units"] == 0));
}

#[test]
fn mcp_aggregator_exposes_and_executes_owner_supply_tools() {
    let fixture = Fixture::new();
    let definitions = super::definitions();
    for (name, read_only, destructive) in [
        (CREATE_PROVIDER, false, false),
        ("compute_get_my_provider", true, false),
        ("compute_audit_my_capacity_pool", true, false),
        (ADD_SUPPLY, false, true),
        (WITHDRAW_SUPPLY, false, true),
    ] {
        let tool = definitions
            .iter()
            .find(|tool| tool["name"] == name)
            .unwrap_or_else(|| panic!("missing MCP tool {name}"));
        assert_eq!(tool["annotations"]["readOnlyHint"], read_only);
        assert_eq!(tool["annotations"]["destructiveHint"], destructive);
    }

    fixture.create_chain_via_mcp();
    let supplied = fixture
        .mcp(
            &fixture.owner_id,
            ADD_SUPPLY,
            supply_arguments(&fixture, "mcp-add", 100, 4, true),
        )
        .unwrap();
    assert_eq!(supplied["event_kind"], "supply_added");
    let replayed = fixture
        .mcp(
            &fixture.owner_id,
            ADD_SUPPLY,
            supply_arguments(&fixture, "mcp-add", 100, 4, true),
        )
        .unwrap();
    assert_eq!(replayed["replayed"], true);
    assert_eq!(replayed["transaction_id"], supplied["transaction_id"]);

    let withdrawn = fixture
        .mcp(
            &fixture.owner_id,
            WITHDRAW_SUPPLY,
            withdraw_arguments(&fixture, "mcp-withdraw", 30, 1, true),
        )
        .unwrap();
    assert_eq!(withdrawn["event_kind"], "supply_withdrawn");
    let audit = fixture
        .mcp(
            &fixture.owner_id,
            "compute_audit_my_capacity_pool",
            pool_arguments(&fixture),
        )
        .unwrap();
    assert_eq!(audit["healthy"], true);
    assert_eq!(audit["transaction_count"], 2);

    let outsider = fixture.mcp(
        &fixture.outsider_id,
        "compute_get_my_provider",
        json!({"provider_id":fixture.provider_id}),
    );
    assert!(outsider
        .unwrap_err()
        .to_string()
        .contains("不属于当前登录用户"));
    assert!(super::call_if_handled(
        &fixture.store,
        "project",
        &fixture.owner_id,
        "compute_unknown_supply_tool",
        json!({}),
    )
    .unwrap()
    .is_none());
}

struct Fixture {
    store: Store,
    router: Router,
    owner_id: String,
    outsider_id: String,
    owner_token: String,
    outsider_token: String,
    provider_id: String,
    pool_id: String,
    token_bucket_id: String,
    concurrency_bucket_id: String,
    window_id: String,
    starts_at_utc: String,
    ends_at_utc: String,
}

impl Fixture {
    fn new() -> Self {
        let suffix = Uuid::new_v4().simple().to_string();
        let starts_at_utc = (Utc::now() + Duration::hours(1)).to_rfc3339();
        let ends_at_utc = (Utc::now() + Duration::hours(3)).to_rfc3339();
        let root = std::env::temp_dir().join(format!("elon-compute-interface-{suffix}"));
        std::fs::create_dir_all(&root).unwrap();
        let store = Store::open(&root.join("state.sqlite")).unwrap();
        let owner = store
            .create_user(
                &format!("compute-interface-owner-{suffix}@example.com"),
                "secret1",
                None,
                None,
            )
            .unwrap();
        let outsider = store
            .create_user(
                &format!("compute-interface-outsider-{suffix}@example.com"),
                "secret1",
                None,
                None,
            )
            .unwrap();
        let (owner_token, _) = store.create_session(&owner.id, Some("test"), None).unwrap();
        let (outsider_token, _) = store
            .create_session(&outsider.id, Some("test"), None)
            .unwrap();
        let database_path = root.join("state.sqlite");
        let state = Arc::new(test_app_state(store, &root));
        let store = Store::open(&database_path).unwrap();
        let router = Router::new()
            .merge(crate::compute_federation_provider_api::routes())
            .merge(crate::compute_federation_capacity_pool_api::routes())
            .merge(crate::compute_federation_capacity_bucket_api::routes())
            .merge(crate::compute_federation_capacity_supply_api::routes())
            .with_state(state);
        Self {
            store,
            router,
            owner_id: owner.id,
            outsider_id: outsider.id,
            owner_token,
            outsider_token,
            provider_id: format!("provider-{suffix}"),
            pool_id: format!("pool-{suffix}"),
            token_bucket_id: format!("bucket-token-{suffix}"),
            concurrency_bucket_id: format!("bucket-concurrency-{suffix}"),
            window_id: format!("window-{suffix}"),
            starts_at_utc,
            ends_at_utc,
        }
    }

    fn mcp(&self, user_id: &str, name: &str, arguments: Value) -> anyhow::Result<Value> {
        super::call_if_handled(&self.store, "project", user_id, name, arguments)?
            .ok_or_else(|| anyhow::anyhow!("MCP tool was not handled: {name}"))
    }

    fn create_chain_via_mcp(&self) {
        self.mcp(
            &self.owner_id,
            CREATE_PROVIDER,
            provider_body(&self.provider_id),
        )
        .unwrap();
        self.mcp(&self.owner_id, CREATE_POOL, pool_arguments_create(self))
            .unwrap();
        for (bucket_id, meter) in [
            (&self.token_bucket_id, "tokens"),
            (&self.concurrency_bucket_id, "concurrency"),
        ] {
            self.mcp(
                &self.owner_id,
                CREATE_BUCKET,
                bucket_arguments(self, bucket_id, meter),
            )
            .unwrap();
        }
    }

    fn pools_path(&self) -> String {
        format!(
            "/api/me/compute/providers/{}/capacity-pools",
            self.provider_id
        )
    }

    fn pool_path(&self) -> String {
        format!("{}/{}", self.pools_path(), self.pool_id)
    }

    fn buckets_path(&self) -> String {
        format!("{}/buckets", self.pool_path())
    }

    fn supply_path(&self) -> String {
        format!("{}/supply", self.pool_path())
    }
}

async fn call_http(
    router: &Router,
    method: Method,
    path: &str,
    token: Option<&str>,
    body: Value,
) -> (StatusCode, Value) {
    let mut request = Request::builder().method(method).uri(path);
    if let Some(token) = token {
        request = request.header(header::AUTHORIZATION, format!("Bearer {token}"));
    }
    let body = if body.is_null() {
        Body::empty()
    } else {
        request = request.header(header::CONTENT_TYPE, "application/json");
        Body::from(body.to_string())
    };
    let response = router
        .clone()
        .oneshot(request.body(body).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, value)
}

fn provider_body(provider_id: &str) -> Value {
    json!({
        "provider_id":provider_id,
        "provider_kind":"user_node",
        "display_name":"Owner compute node",
        "home_region":"cn-east",
        "task_kinds":["llm_inference"],
        "accelerator_kinds":["consumer_gpu"],
        "regions":["cn-east"],
        "allowed_data_classes":["public"],
        "supports_streaming":true,
        "supports_checkpointing":false,
        "declared_hardware_digest":"declared-hardware-digest"
    })
}

fn pool_body(pool_id: &str) -> Value {
    json!({
        "pool_id":pool_id,
        "resource_scope_key":"desktop-gpu-0",
        "region_or_data_zone":"cn-east",
        "resource_profile":{"accelerator":"consumer_gpu","count":1},
        "meter_policies":[
            {"meter":"tokens","meter_mode":"consumable","quantum_units":10},
            {"meter":"concurrency","meter_mode":"reusable","quantum_units":1}
        ]
    })
}

fn pool_arguments_create(fixture: &Fixture) -> Value {
    let mut value = pool_body(&fixture.pool_id);
    value["provider_id"] = json!(fixture.provider_id);
    value
}

fn pool_arguments(fixture: &Fixture) -> Value {
    json!({"provider_id":fixture.provider_id,"pool_id":fixture.pool_id})
}

fn bucket_body(fixture: &Fixture, bucket_id: &str, meter: &str) -> Value {
    json!({
        "bucket_id":bucket_id,
        "window_id":fixture.window_id,
        "starts_at_utc":fixture.starts_at_utc,
        "ends_at_utc":fixture.ends_at_utc,
        "meter":meter
    })
}

fn bucket_arguments(fixture: &Fixture, bucket_id: &str, meter: &str) -> Value {
    let mut value = bucket_body(fixture, bucket_id, meter);
    value["provider_id"] = json!(fixture.provider_id);
    value["pool_id"] = json!(fixture.pool_id);
    value
}

fn supply_body(
    fixture: &Fixture,
    key: &str,
    tokens: i64,
    concurrency: i64,
    confirm: bool,
) -> Value {
    json!({
        "idempotency_key":key,
        "lines":[
            {"bucket_id":fixture.token_bucket_id,"quantity_units":tokens},
            {"bucket_id":fixture.concurrency_bucket_id,"quantity_units":concurrency}
        ],
        "confirm_supply":confirm
    })
}

fn supply_arguments(
    fixture: &Fixture,
    key: &str,
    tokens: i64,
    concurrency: i64,
    confirm: bool,
) -> Value {
    let mut value = supply_body(fixture, key, tokens, concurrency, confirm);
    value["provider_id"] = json!(fixture.provider_id);
    value["pool_id"] = json!(fixture.pool_id);
    value
}

fn withdraw_arguments(
    fixture: &Fixture,
    key: &str,
    tokens: i64,
    concurrency: i64,
    confirm: bool,
) -> Value {
    json!({
        "provider_id":fixture.provider_id,
        "pool_id":fixture.pool_id,
        "idempotency_key":key,
        "lines":[
            {"bucket_id":fixture.token_bucket_id,"quantity_units":tokens},
            {"bucket_id":fixture.concurrency_bucket_id,"quantity_units":concurrency}
        ],
        "confirm_withdrawal":confirm
    })
}
