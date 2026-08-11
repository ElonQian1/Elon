use std::path::PathBuf;

use chrono::{Duration, Utc};
use serde_json::json;
use uuid::Uuid;

use crate::{
    compute_federation_capacity_bucket_service::{
        self, CreateMyComputeCapacityBucketRequest, MyComputeCapacityBucketView,
    },
    compute_federation_capacity_pool_service::{
        self, CreateMyComputeCapacityMeterPolicyRequest, CreateMyComputeCapacityPoolRequest,
    },
    compute_federation_capacity_supply_service::{
        self, AddMyComputeCapacitySupplyLineRequest, AddMyComputeCapacitySupplyRequest,
        WithdrawMyComputeCapacitySupplyLineRequest, WithdrawMyComputeCapacitySupplyRequest,
    },
    compute_federation_provider_service::{self, CreateMyComputeProviderRequest},
    store::Store,
};

#[test]
fn owner_supply_control_plane_is_idempotent_auditable_and_durable() {
    let fixture = Fixture::new();
    let chain = create_control_plane(&fixture);

    let provider_replay = compute_federation_provider_service::create_for_user(
        &fixture.store,
        &fixture.owner_id,
        provider_request(&chain.provider_id),
    )
    .unwrap();
    assert!(provider_replay.replayed);
    assert_eq!(provider_replay.status, "registering");
    assert_eq!(provider_replay.trust_tier, "self_declared");

    let pool_replay = compute_federation_capacity_pool_service::create_for_user(
        &fixture.store,
        &fixture.owner_id,
        &chain.provider_id,
        pool_request(&chain.pool_id),
    )
    .unwrap();
    assert!(pool_replay.replayed);
    assert_eq!(pool_replay.capacity_epoch, 1);
    assert_eq!(pool_replay.pool_revision, 1);

    let add_request = add_supply_request(&chain, "supply-add-001", 100, 4, true);
    let added = compute_federation_capacity_supply_service::add_for_user(
        &fixture.store,
        &fixture.owner_id,
        &chain.provider_id,
        &chain.pool_id,
        add_request.clone(),
    )
    .unwrap();
    assert!(!added.replayed);
    assert_eq!(added.event_kind, "supply_added");
    assert_balance(&added.current_balances, &chain.token_bucket_id, 100, 100, 0);
    assert_balance(
        &added.current_balances,
        &chain.concurrency_bucket_id,
        4,
        4,
        0,
    );

    let replay = compute_federation_capacity_supply_service::add_for_user(
        &fixture.store,
        &fixture.owner_id,
        &chain.provider_id,
        &chain.pool_id,
        add_request,
    )
    .unwrap();
    assert!(replay.replayed);
    assert_eq!(replay.transaction_id, added.transaction_id);

    let withdrawn = compute_federation_capacity_supply_service::withdraw_for_user(
        &fixture.store,
        &fixture.owner_id,
        &chain.provider_id,
        &chain.pool_id,
        withdraw_supply_request(&chain, "supply-withdraw-001", 30, 1, true),
    )
    .unwrap();
    assert!(!withdrawn.replayed);
    assert_eq!(withdrawn.event_kind, "supply_withdrawn");
    assert_balance(
        &withdrawn.current_balances,
        &chain.token_bucket_id,
        100,
        70,
        30,
    );
    assert_balance(
        &withdrawn.current_balances,
        &chain.concurrency_bucket_id,
        4,
        3,
        1,
    );

    let audit = compute_federation_capacity_pool_service::audit_for_user(
        &fixture.store,
        &fixture.owner_id,
        &chain.provider_id,
        &chain.pool_id,
    )
    .unwrap();
    assert!(audit.healthy, "{:?}", audit.issues);
    assert_eq!(audit.transaction_count, 2);
    assert_eq!(audit.buckets.len(), 2);

    let history = compute_federation_capacity_pool_service::list_ledger_history_for_user(
        &fixture.store,
        &fixture.owner_id,
        &chain.provider_id,
        &chain.pool_id,
        None,
        20,
    )
    .unwrap();
    assert_eq!(history.transactions.len(), 2);
    assert_eq!(history.transactions[0].event_kind, "supply_withdrawn");
    assert_eq!(history.transactions[1].event_kind, "supply_added");

    let Fixture {
        path,
        store,
        owner_id,
        ..
    } = fixture;
    drop(store);
    let reopened = Store::open(&path).unwrap();
    let buckets = compute_federation_capacity_bucket_service::list_for_user(
        &reopened,
        &owner_id,
        &chain.provider_id,
        &chain.pool_id,
        20,
    )
    .unwrap();
    assert_eq!(buckets.len(), 2);
    assert_bucket_view(&buckets, &chain.token_bucket_id, 100, 70, 30);
    assert_bucket_view(&buckets, &chain.concurrency_bucket_id, 4, 3, 1);
    assert!(
        compute_federation_capacity_pool_service::audit_for_user(
            &reopened,
            &owner_id,
            &chain.provider_id,
            &chain.pool_id,
        )
        .unwrap()
        .healthy
    );
}

#[test]
fn owner_supply_control_plane_fails_closed_without_partial_ledger_writes() {
    let fixture = Fixture::new();
    let chain = create_control_plane(&fixture);

    let error = compute_federation_provider_service::get_for_user(
        &fixture.store,
        &fixture.outsider_id,
        &chain.provider_id,
    )
    .unwrap_err();
    assert!(error.to_string().contains("不属于当前登录用户"));

    let mut changed_provider = provider_request(&chain.provider_id);
    changed_provider.display_name = "Different provider".to_string();
    assert!(compute_federation_provider_service::create_for_user(
        &fixture.store,
        &fixture.owner_id,
        changed_provider,
    )
    .is_err());

    let mut changed_pool = pool_request(&chain.pool_id);
    changed_pool.resource_profile = json!({"accelerator":"different"});
    assert!(compute_federation_capacity_pool_service::create_for_user(
        &fixture.store,
        &fixture.owner_id,
        &chain.provider_id,
        changed_pool,
    )
    .is_err());

    let unconfirmed = add_supply_request(&chain, "supply-unconfirmed", 100, 4, false);
    assert!(compute_federation_capacity_supply_service::add_for_user(
        &fixture.store,
        &fixture.owner_id,
        &chain.provider_id,
        &chain.pool_id,
        unconfirmed,
    )
    .is_err());
    let invalid_quantum = add_supply_request(&chain, "supply-invalid-quantum", 15, 4, true);
    assert!(compute_federation_capacity_supply_service::add_for_user(
        &fixture.store,
        &fixture.owner_id,
        &chain.provider_id,
        &chain.pool_id,
        invalid_quantum,
    )
    .is_err());
    assert!(ledger_history(&fixture, &chain).transactions.is_empty());

    compute_federation_capacity_supply_service::add_for_user(
        &fixture.store,
        &fixture.owner_id,
        &chain.provider_id,
        &chain.pool_id,
        add_supply_request(&chain, "supply-conflict", 100, 4, true),
    )
    .unwrap();
    let conflict = add_supply_request(&chain, "supply-conflict", 110, 4, true);
    assert!(compute_federation_capacity_supply_service::add_for_user(
        &fixture.store,
        &fixture.owner_id,
        &chain.provider_id,
        &chain.pool_id,
        conflict,
    )
    .is_err());
    let excessive = withdraw_supply_request(&chain, "supply-excessive", 110, 1, true);
    assert!(
        compute_federation_capacity_supply_service::withdraw_for_user(
            &fixture.store,
            &fixture.owner_id,
            &chain.provider_id,
            &chain.pool_id,
            excessive,
        )
        .is_err()
    );

    let history = ledger_history(&fixture, &chain);
    assert_eq!(history.transactions.len(), 1);
    assert_eq!(history.transactions[0].event_kind, "supply_added");
    let buckets = compute_federation_capacity_bucket_service::list_for_user(
        &fixture.store,
        &fixture.owner_id,
        &chain.provider_id,
        &chain.pool_id,
        20,
    )
    .unwrap();
    assert_bucket_view(&buckets, &chain.token_bucket_id, 100, 100, 0);
    assert_bucket_view(&buckets, &chain.concurrency_bucket_id, 4, 4, 0);
}

struct Fixture {
    path: PathBuf,
    store: Store,
    owner_id: String,
    outsider_id: String,
    suffix: String,
}

impl Fixture {
    fn new() -> Self {
        let suffix = Uuid::new_v4().simple().to_string();
        let path = std::env::temp_dir().join(format!("elon-compute-supply-{suffix}.sqlite"));
        let store = Store::open(&path).unwrap();
        let owner = store
            .create_user(
                &format!("compute-owner-{suffix}@example.com"),
                "secret1",
                None,
                None,
            )
            .unwrap();
        let outsider = store
            .create_user(
                &format!("compute-outsider-{suffix}@example.com"),
                "secret1",
                None,
                None,
            )
            .unwrap();
        Self {
            path,
            store,
            owner_id: owner.id,
            outsider_id: outsider.id,
            suffix,
        }
    }
}

struct ControlPlane {
    provider_id: String,
    pool_id: String,
    token_bucket_id: String,
    concurrency_bucket_id: String,
}

fn create_control_plane(fixture: &Fixture) -> ControlPlane {
    let provider_id = format!("provider-{}", fixture.suffix);
    let pool_id = format!("pool-{}", fixture.suffix);
    let token_bucket_id = format!("bucket-token-{}", fixture.suffix);
    let concurrency_bucket_id = format!("bucket-concurrency-{}", fixture.suffix);
    compute_federation_provider_service::create_for_user(
        &fixture.store,
        &fixture.owner_id,
        provider_request(&provider_id),
    )
    .unwrap();
    compute_federation_capacity_pool_service::create_for_user(
        &fixture.store,
        &fixture.owner_id,
        &provider_id,
        pool_request(&pool_id),
    )
    .unwrap();
    let starts_at = (Utc::now() + Duration::hours(1)).to_rfc3339();
    let ends_at = (Utc::now() + Duration::hours(3)).to_rfc3339();
    for (bucket_id, meter) in [
        (&token_bucket_id, "tokens"),
        (&concurrency_bucket_id, "concurrency"),
    ] {
        compute_federation_capacity_bucket_service::create_for_user(
            &fixture.store,
            &fixture.owner_id,
            &provider_id,
            &pool_id,
            CreateMyComputeCapacityBucketRequest {
                bucket_id: bucket_id.clone(),
                window_id: format!("window-{}", fixture.suffix),
                starts_at_utc: starts_at.clone(),
                ends_at_utc: ends_at.clone(),
                meter: meter.to_string(),
            },
        )
        .unwrap();
    }
    ControlPlane {
        provider_id,
        pool_id,
        token_bucket_id,
        concurrency_bucket_id,
    }
}

fn provider_request(provider_id: &str) -> CreateMyComputeProviderRequest {
    CreateMyComputeProviderRequest {
        provider_id: provider_id.to_string(),
        provider_kind: "user_node".to_string(),
        display_name: "Owner compute node".to_string(),
        home_region: Some("cn-east".to_string()),
        task_kinds: vec!["llm_inference".to_string()],
        accelerator_kinds: vec!["consumer_gpu".to_string()],
        regions: vec!["cn-east".to_string()],
        allowed_data_classes: vec!["public".to_string()],
        supports_streaming: true,
        supports_checkpointing: false,
        declared_hardware_digest: Some("declared-hardware-digest".to_string()),
    }
}

fn pool_request(pool_id: &str) -> CreateMyComputeCapacityPoolRequest {
    CreateMyComputeCapacityPoolRequest {
        pool_id: pool_id.to_string(),
        resource_scope_key: "desktop-gpu-0".to_string(),
        region_or_data_zone: "cn-east".to_string(),
        resource_profile: json!({"accelerator":"consumer_gpu","count":1}),
        meter_policies: vec![
            CreateMyComputeCapacityMeterPolicyRequest {
                meter: "tokens".to_string(),
                meter_mode: "consumable".to_string(),
                quantum_units: 10,
            },
            CreateMyComputeCapacityMeterPolicyRequest {
                meter: "concurrency".to_string(),
                meter_mode: "reusable".to_string(),
                quantum_units: 1,
            },
        ],
    }
}

fn add_supply_request(
    chain: &ControlPlane,
    key: &str,
    tokens: i64,
    concurrency: i64,
    confirmed: bool,
) -> AddMyComputeCapacitySupplyRequest {
    AddMyComputeCapacitySupplyRequest {
        idempotency_key: key.to_string(),
        lines: vec![
            AddMyComputeCapacitySupplyLineRequest {
                bucket_id: chain.token_bucket_id.clone(),
                quantity_units: tokens,
            },
            AddMyComputeCapacitySupplyLineRequest {
                bucket_id: chain.concurrency_bucket_id.clone(),
                quantity_units: concurrency,
            },
        ],
        confirm_supply: confirmed,
    }
}

fn withdraw_supply_request(
    chain: &ControlPlane,
    key: &str,
    tokens: i64,
    concurrency: i64,
    confirmed: bool,
) -> WithdrawMyComputeCapacitySupplyRequest {
    WithdrawMyComputeCapacitySupplyRequest {
        idempotency_key: key.to_string(),
        lines: vec![
            WithdrawMyComputeCapacitySupplyLineRequest {
                bucket_id: chain.token_bucket_id.clone(),
                quantity_units: tokens,
            },
            WithdrawMyComputeCapacitySupplyLineRequest {
                bucket_id: chain.concurrency_bucket_id.clone(),
                quantity_units: concurrency,
            },
        ],
        confirm_withdrawal: confirmed,
    }
}

fn ledger_history(
    fixture: &Fixture,
    chain: &ControlPlane,
) -> crate::store::ComputeCapacityLedgerHistoryPage {
    compute_federation_capacity_pool_service::list_ledger_history_for_user(
        &fixture.store,
        &fixture.owner_id,
        &chain.provider_id,
        &chain.pool_id,
        None,
        20,
    )
    .unwrap()
}

fn assert_balance(
    balances: &[crate::compute_federation::capacity::ComputeCapacityBucketBalance],
    bucket_id: &str,
    issued: i64,
    available: i64,
    retired: i64,
) {
    let balance = balances
        .iter()
        .find(|balance| balance.binding.bucket_id == bucket_id)
        .unwrap();
    assert_eq!(balance.issued_units, issued);
    assert_eq!(balance.available_units, available);
    assert_eq!(balance.retired_units, retired);
    assert_eq!(balance.held_units, 0);
    assert_eq!(balance.active_units, 0);
    assert_eq!(balance.consumed_units, 0);
}

fn assert_bucket_view(
    buckets: &[MyComputeCapacityBucketView],
    bucket_id: &str,
    issued: i64,
    available: i64,
    retired: i64,
) {
    let bucket = buckets
        .iter()
        .find(|bucket| bucket.balance.binding.bucket_id == bucket_id)
        .unwrap();
    assert_balance(
        std::slice::from_ref(&bucket.balance),
        bucket_id,
        issued,
        available,
        retired,
    );
}
