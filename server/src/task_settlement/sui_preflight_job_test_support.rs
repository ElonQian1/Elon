use std::{
    ffi::OsString,
    path::PathBuf,
    sync::{Mutex, MutexGuard},
};

use uuid::Uuid;

use crate::{
    store::Store,
    task_settlement::{
        ledger::LedgerPosting,
        model::{
            CreateSettlementIntent, CreateSettlementReceipt, CreateUsageReceipt, RECEIPT_RECONCILED,
        },
        sui_preflight_model::{CreateSuiPreflightAdapterRequest, SuiPreflightAdapter},
        sui_preflight_service, sui_projection_service,
    },
};

const RUNTIME_FLAG: &str = "ELON_SUI_OFFLINE_PREFLIGHT_ENABLED";
static RUNTIME_ENV_LOCK: Mutex<()> = Mutex::new(());

pub(crate) struct SuiPreflightJobFixture {
    pub(crate) store: Store,
    pub(crate) path: PathBuf,
    pub(crate) owner_user_id: String,
    pub(crate) project_id: String,
    pub(crate) receipt_id: String,
    pub(crate) projection_id: String,
    pub(crate) adapter: SuiPreflightAdapter,
    pub(crate) adapter_token: String,
    pub(crate) owner_token: String,
    pub(crate) outsider_token: String,
}

pub(crate) struct RuntimeFlagGuard {
    previous: Option<OsString>,
    _lock: MutexGuard<'static, ()>,
}

impl RuntimeFlagGuard {
    pub(crate) fn enabled() -> Self {
        let lock = RUNTIME_ENV_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let previous = std::env::var_os(RUNTIME_FLAG);
        std::env::set_var(RUNTIME_FLAG, "1");
        Self {
            previous,
            _lock: lock,
        }
    }
}

impl Drop for RuntimeFlagGuard {
    fn drop(&mut self) {
        match self.previous.take() {
            Some(value) => std::env::set_var(RUNTIME_FLAG, value),
            None => std::env::remove_var(RUNTIME_FLAG),
        }
    }
}

pub(crate) fn fixture() -> SuiPreflightJobFixture {
    let suffix = Uuid::new_v4().simple();
    let path = std::env::temp_dir().join(format!("elon-sui-preflight-job-{suffix}.sqlite"));
    let store = Store::open(&path).unwrap();
    let owner = store
        .create_user(
            &format!("sui-preflight-owner-{suffix}@example.com"),
            "secret1",
            None,
            None,
        )
        .unwrap();
    let outsider = store
        .create_user(
            &format!("sui-preflight-outsider-{suffix}@example.com"),
            "secret1",
            None,
            None,
        )
        .unwrap();
    let project = store
        .create_project(&owner.id, "Sui preflight job fixture", None, None)
        .unwrap()
        .project;
    let usage = store
        .insert_task_usage_receipt(CreateUsageReceipt {
            project_id: &project.id,
            subject_type: "task_assignment",
            subject_id: "assignment-sui-preflight-job",
            source_type: "test",
            source_id: "source-sui-preflight-job",
            source_digest: "source-digest-sui-preflight-job",
            consumer_user_id: &owner.id,
            provider_user_id: Some(&owner.id),
            units: 0,
            amount_micros: 0,
            provider_amount_micros: 0,
            currency: "CNY",
            billing_source: "test",
            source_status: "settled",
            occurred_at: "2026-08-11T00:00:00Z",
        })
        .unwrap();
    let intent = store
        .create_task_settlement_intent(CreateSettlementIntent {
            project_id: &project.id,
            matter_id: Some("matter-sui-preflight-job"),
            assignment_id: Some("assignment-sui-preflight-job"),
            payer_user_id: &owner.id,
            payee_user_id: Some(&owner.id),
            idempotency_key: "sui-preflight-job-intent",
            policy_version: "test.v1",
            policy_digest: "policy-digest-sui-preflight-job",
            usage_receipt_id: &usage.id,
        })
        .unwrap();
    let receipt = store
        .post_task_shadow_settlement(
            CreateSettlementReceipt {
                project_id: &project.id,
                intent_id: &intent.id,
                posting_key: "sui-preflight-job-posting",
                status: RECEIPT_RECONCILED,
                compute_amount_micros: 0,
                provider_amount_micros: 0,
                platform_amount_micros: 0,
                outcome_reward_micros: 0,
                review_reward_micros: 0,
                currency: "CNY",
                accepted_matter_id: Some("matter-sui-preflight-job"),
                reason: "Sui preflight job fixture",
                receipt_kind: "standard",
                correction_id: None,
            },
            &Vec::<LedgerPosting>::new(),
        )
        .unwrap();
    let projection =
        sui_projection_service::prepare(&store, &project.id, &receipt.id, &owner.id, "testnet")
            .unwrap();
    let adapter_issue = sui_preflight_service::create_adapter(
        &store,
        &project.id,
        &owner.id,
        "owner",
        &CreateSuiPreflightAdapterRequest {
            display_name: "Local Sui preflight worker".to_string(),
            allowed_networks: vec!["testnet".to_string()],
            allowed_package_kinds: vec!["standard".to_string()],
            expires_in_days: 30,
            confirmed_by_user: true,
        },
    )
    .unwrap();
    let owner_token = session(&store, &owner.id);
    let outsider_token = session(&store, &outsider.id);

    SuiPreflightJobFixture {
        store,
        path,
        owner_user_id: owner.id,
        project_id: project.id,
        receipt_id: receipt.id,
        projection_id: projection.id,
        adapter: adapter_issue.adapter,
        adapter_token: adapter_issue.adapter_token,
        owner_token,
        outsider_token,
    }
}

fn session(store: &Store, user_id: &str) -> String {
    store
        .create_session(user_id, Some("sui-preflight-job-test"), None)
        .unwrap()
        .0
}
