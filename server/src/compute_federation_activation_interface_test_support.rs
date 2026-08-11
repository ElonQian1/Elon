use std::sync::Arc;

use axum::{
    body::{to_bytes, Body},
    http::{header, Method, Request, StatusCode},
    Router,
};
use chrono::{Duration, Utc};
use serde_json::Value;
use tower::ServiceExt;
use uuid::Uuid;

use crate::{
    compute_federation::provider::ComputeProviderEndpointRef,
    compute_federation_capacity_bucket_service::{self, CreateMyComputeCapacityBucketRequest},
    compute_federation_capacity_pool_service::{
        self, CreateMyComputeCapacityMeterPolicyRequest, CreateMyComputeCapacityPoolRequest,
    },
    compute_federation_provider_service::{self, CreateMyComputeProviderRequest},
    open_commerce_developer_production_test_support::test_app_state,
    store::Store,
    types::AppState,
};

pub(super) struct InterfaceFixture {
    pub(super) state: Arc<AppState>,
    pub(super) router: Router,
    pub(super) root: std::path::PathBuf,
    pub(super) owner_id: String,
    pub(super) admin_one_id: String,
    pub(super) admin_two_id: String,
    pub(super) outsider_id: String,
    pub(super) owner_token: String,
    pub(super) admin_one_token: String,
    pub(super) admin_two_token: String,
    pub(super) outsider_token: String,
    pub(super) provider_id: String,
    pub(super) pool_id: String,
}

impl InterfaceFixture {
    pub(super) fn new() -> Self {
        let suffix = Uuid::new_v4().simple().to_string();
        let root = std::env::temp_dir().join(format!("elon-activation-interface-{suffix}"));
        std::fs::create_dir_all(&root).unwrap();
        let store = Store::open(&root.join("state.sqlite")).unwrap();
        let owner = create_user(&store, "activation-owner", None);
        let admin_one = create_user(&store, "activation-admin-one", Some("admin"));
        let admin_two = create_user(&store, "activation-admin-two", Some("admin"));
        let outsider = create_user(&store, "activation-outsider", None);
        let provider_id = format!("provider-{suffix}");
        let pool_id = format!("pool-{suffix}");
        register_supply_contract(&store, &owner.id, &provider_id, &pool_id, &suffix);
        let owner_token = session(&store, &owner.id);
        let admin_one_token = session(&store, &admin_one.id);
        let admin_two_token = session(&store, &admin_two.id);
        let outsider_token = session(&store, &outsider.id);
        let state = Arc::new(test_app_state(store, &root));
        let router = crate::compute_federation_activation_api::routes().with_state(state.clone());
        Self {
            state,
            router,
            root,
            owner_id: owner.id,
            admin_one_id: admin_one.id,
            admin_two_id: admin_two.id,
            outsider_id: outsider.id,
            owner_token,
            admin_one_token,
            admin_two_token,
            outsider_token,
            provider_id,
            pool_id,
        }
    }

    pub(super) fn close(self) -> std::path::PathBuf {
        let root = self.root.clone();
        drop(self);
        root
    }
}

pub(super) fn endpoint(provider_id: &str) -> ComputeProviderEndpointRef {
    ComputeProviderEndpointRef {
        endpoint_id: format!("endpoint-{provider_id}"),
        transport: "https".into(),
        address_hint: Some("provider.test.invalid".into()),
        gateway_id: Some("gateway-test".into()),
        credential_ref: Some("vault://activation-test".into()),
    }
}

pub(super) fn endpoint_json(provider_id: &str) -> Value {
    serde_json::to_value(endpoint(provider_id)).unwrap()
}

pub(super) fn digest(byte: char) -> String {
    byte.to_string().repeat(64)
}

pub(super) async fn call_http(
    router: &Router,
    method: Method,
    path: &str,
    bearer: Option<&str>,
    body: Value,
) -> (StatusCode, Value) {
    let mut builder = Request::builder().method(method).uri(path);
    let request_body = if body.is_null() {
        Body::empty()
    } else {
        builder = builder.header(header::CONTENT_TYPE, "application/json");
        Body::from(body.to_string())
    };
    if let Some(token) = bearer {
        builder = builder.header(header::AUTHORIZATION, format!("Bearer {token}"));
    }
    let response = router
        .clone()
        .oneshot(builder.body(request_body).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let value = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap()
    };
    (status, value)
}

fn register_supply_contract(
    store: &Store,
    owner_id: &str,
    provider_id: &str,
    pool_id: &str,
    suffix: &str,
) {
    compute_federation_provider_service::create_for_user(
        store,
        owner_id,
        CreateMyComputeProviderRequest {
            provider_id: provider_id.into(),
            provider_kind: "user_node".into(),
            display_name: "Activation interface provider".into(),
            home_region: Some("cn-east".into()),
            task_kinds: vec!["llm_inference".into()],
            accelerator_kinds: vec!["consumer_gpu".into()],
            regions: vec!["cn-east".into()],
            allowed_data_classes: vec!["public".into()],
            supports_streaming: true,
            supports_checkpointing: false,
            declared_hardware_digest: Some(digest('0')),
        },
    )
    .unwrap();
    compute_federation_capacity_pool_service::create_for_user(
        store,
        owner_id,
        provider_id,
        CreateMyComputeCapacityPoolRequest {
            pool_id: pool_id.into(),
            resource_scope_key: "desktop-gpu-0".into(),
            region_or_data_zone: "cn-east".into(),
            resource_profile: serde_json::json!({"accelerator":"consumer_gpu","count":1}),
            meter_policies: vec![
                meter_policy("tokens", "consumable", 10),
                meter_policy("concurrency", "reusable", 1),
            ],
        },
    )
    .unwrap();
    let window_id = format!("window-{suffix}");
    let starts_at_utc = (Utc::now() + Duration::hours(1)).to_rfc3339();
    let ends_at_utc = (Utc::now() + Duration::hours(3)).to_rfc3339();
    for (bucket_suffix, meter) in [("tokens", "tokens"), ("concurrency", "concurrency")] {
        compute_federation_capacity_bucket_service::create_for_user(
            store,
            owner_id,
            provider_id,
            pool_id,
            CreateMyComputeCapacityBucketRequest {
                bucket_id: format!("bucket-{bucket_suffix}-{provider_id}"),
                window_id: window_id.clone(),
                starts_at_utc: starts_at_utc.clone(),
                ends_at_utc: ends_at_utc.clone(),
                meter: meter.into(),
            },
        )
        .unwrap();
    }
}

fn meter_policy(
    meter: &str,
    meter_mode: &str,
    quantum_units: i64,
) -> CreateMyComputeCapacityMeterPolicyRequest {
    CreateMyComputeCapacityMeterPolicyRequest {
        meter: meter.into(),
        meter_mode: meter_mode.into(),
        quantum_units,
    }
}

fn create_user(store: &Store, prefix: &str, role: Option<&str>) -> crate::store::PublicUser {
    store
        .create_user(
            &format!("{prefix}-{}@example.com", Uuid::new_v4().simple()),
            "secret1",
            None,
            role,
        )
        .unwrap()
}

fn session(store: &Store, user_id: &str) -> String {
    store
        .create_session(user_id, Some("compute-activation-interface"), None)
        .unwrap()
        .0
}
