use chrono::Utc;
use rusqlite::params;
use uuid::Uuid;

use crate::{
    compute_federation::provider::{
        ComputeProvider, ComputeProviderCapabilities, ComputeProviderEvidenceProfile,
        COMPUTE_PROVIDER_SCHEMA,
    },
    compute_federation_settlement_withdrawal_request_service::{
        self, CreateMyComputeSettlementWithdrawalBody,
    },
    compute_federation_settlement_withdrawal_terminal_service::{
        self, AdminTerminalizeComputeSettlementWithdrawalBody,
        CancelMyComputeSettlementWithdrawalBody,
    },
    store::Store,
};

const INITIAL_BALANCE_MICROS: i64 = 1_000_000;

struct Fixture {
    root: std::path::PathBuf,
    database: std::path::PathBuf,
    store: Option<Store>,
    owner_id: String,
    other_owner_id: String,
    admin_id: String,
    provider_id: String,
}

impl Fixture {
    fn new() -> Self {
        let suffix = Uuid::new_v4().simple().to_string();
        let root = std::env::temp_dir().join(format!("elon-settlement-withdrawal-{suffix}"));
        std::fs::create_dir_all(&root).unwrap();
        let database = root.join("state.sqlite");
        let store = Store::open(&database).unwrap();
        let fixture = Self {
            root,
            database,
            store: Some(store),
            owner_id: format!("withdrawal-owner-{suffix}"),
            other_owner_id: format!("withdrawal-other-{suffix}"),
            admin_id: format!("withdrawal-admin-{suffix}"),
            provider_id: format!("withdrawal-provider-{suffix}"),
        };
        fixture.seed_provider_and_balance();
        fixture
    }

    fn store(&self) -> &Store {
        self.store.as_ref().unwrap()
    }

    fn reopen(&mut self) {
        self.store.take();
        self.store = Some(Store::open(&self.database).unwrap());
    }

    fn seed_provider_and_balance(&self) {
        let now = Utc::now().to_rfc3339();
        let release_id = format!("withdrawal-test-release-{}", self.provider_id);
        let release_posting_id = format!("withdrawal-test-release-posting-{}", self.provider_id);
        let settlement_receipt_id = format!("withdrawal-test-settlement-{}", self.provider_id);
        let provider = ComputeProvider {
            schema: COMPUTE_PROVIDER_SCHEMA.to_string(),
            provider_id: self.provider_id.clone(),
            provider_kind: "user_node".to_string(),
            owner_account_id: self.owner_id.clone(),
            settlement_account_id: Some(self.owner_id.clone()),
            display_name: "Withdrawal test provider".to_string(),
            status: "registering".to_string(),
            trust_tier: "self_declared".to_string(),
            home_region: Some("test-local".to_string()),
            policy_revision: 1,
            capabilities: ComputeProviderCapabilities {
                task_kinds: vec!["llm_chat".to_string()],
                accelerator_kinds: vec!["consumer_gpu".to_string()],
                regions: vec!["test-local".to_string()],
                allowed_data_classes: vec!["public".to_string()],
                supports_streaming: true,
                supports_checkpointing: false,
            },
            endpoint: None,
            adapter: None,
            evidence_profile: ComputeProviderEvidenceProfile {
                declared_hardware_digest: None,
                observed_hardware_digest: None,
                verified_hardware_digest: None,
                last_observed_at: None,
                last_verified_at: None,
            },
            created_at: now.clone(),
            updated_at: now.clone(),
        };
        self.store().register_compute_provider(&provider).unwrap();
        let conn = self.store().conn().unwrap();
        conn.execute_batch("PRAGMA foreign_keys=OFF;").unwrap();
        conn.execute(
            "INSERT INTO compute_settlement_release_postings (
               posting_id, release_id, settlement_receipt_id, currency,
               provider_released_micros, platform_released_micros,
               posting_digest, posted_at
             ) VALUES (?1,?2,?3,'CNY',?4,0,?5,?6)",
            params![
                release_posting_id,
                release_id,
                settlement_receipt_id,
                INITIAL_BALANCE_MICROS,
                "0".repeat(64),
                now,
            ],
        )
        .unwrap();
        for (line_no, leg_kind, direction, balance_state, balance_after) in [
            (1, "provider_pending_release", "debit", "pending", 0),
            (
                2,
                "provider_available_credit",
                "credit",
                "available",
                INITIAL_BALANCE_MICROS,
            ),
        ] {
            conn.execute(
                "INSERT INTO compute_settlement_release_ledger_legs (
                   posting_id, line_no, account_kind, leg_kind, account_id,
                   currency, direction, amount_micros, balance_state,
                   balance_after_micros, account_revision_after
                 ) VALUES (?1,?2,'provider',?3,?4,'CNY',?5,?6,?7,?8,1)",
                params![
                    release_posting_id,
                    line_no,
                    leg_kind,
                    &self.owner_id,
                    direction,
                    INITIAL_BALANCE_MICROS,
                    balance_state,
                    balance_after,
                ],
            )
            .unwrap();
        }
        conn.execute(
            "INSERT INTO compute_settlement_account_balances (
               account_kind, account_id, currency, pending_micros,
               available_micros, disputed_micros, withdrawn_micros,
               revision, updated_at
             ) VALUES ('provider',?1,'CNY',0,?2,0,0,1,?3)",
            params![&self.owner_id, INITIAL_BALANCE_MICROS, now],
        )
        .unwrap();
        conn.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
    }

    fn balance(&self) -> (i64, i64, i64) {
        self.store()
            .conn()
            .unwrap()
            .query_row(
                "SELECT available_micros, withdrawn_micros, revision
                   FROM compute_settlement_account_balances
                  WHERE account_kind='provider' AND account_id=?1 AND currency='CNY'",
                params![&self.owner_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap()
    }

    fn row_count(&self, table: &str) -> i64 {
        assert!(matches!(
            table,
            "compute_settlement_withdrawal_requests"
                | "compute_settlement_withdrawal_request_postings"
                | "compute_settlement_withdrawal_request_ledger_legs"
                | "compute_settlement_withdrawal_terminals"
                | "compute_settlement_withdrawal_terminal_postings"
                | "compute_settlement_withdrawal_terminal_ledger_legs"
        ));
        self.store()
            .conn()
            .unwrap()
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                row.get(0)
            })
            .unwrap()
    }

    fn create_body(
        &self,
        amount_micros: i64,
        idempotency_key: &str,
    ) -> CreateMyComputeSettlementWithdrawalBody {
        CreateMyComputeSettlementWithdrawalBody {
            amount_micros,
            destination_kind: "bank_account_vault_ref".to_string(),
            destination_ref: "vault://withdrawal-destination/test".to_string(),
            idempotency_key: idempotency_key.to_string(),
            confirm_internal_reserve_only: true,
            confirm_destination_ref_contains_no_secret: true,
        }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.store.take();
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

#[test]
fn withdrawal_request_is_atomic_idempotent_and_owner_scoped() {
    let fixture = Fixture::new();
    let mut missing_confirmation = fixture.create_body(300_000, "request-confirmation");
    missing_confirmation.confirm_internal_reserve_only = false;
    assert!(
        compute_federation_settlement_withdrawal_request_service::create_for_provider_owner(
            fixture.store(),
            &fixture.owner_id,
            &fixture.provider_id,
            missing_confirmation,
        )
        .is_err()
    );
    assert_eq!(fixture.balance(), (INITIAL_BALANCE_MICROS, 0, 1));

    let body = fixture.create_body(300_000, "request-idempotent");
    let created =
        compute_federation_settlement_withdrawal_request_service::create_for_provider_owner(
            fixture.store(),
            &fixture.owner_id,
            &fixture.provider_id,
            body.clone(),
        )
        .unwrap();
    assert!(!created.replayed);
    assert_eq!(created.available_balance_after_micros, 700_000);
    assert_eq!(created.withdrawn_balance_after_micros, 300_000);
    assert_eq!(
        created.fund_effect,
        "provider_available_moved_to_withdrawn_reserve"
    );
    assert_eq!(created.external_transfer_effect, "not_executed");
    assert_eq!(fixture.balance(), (700_000, 300_000, 2));

    let replayed =
        compute_federation_settlement_withdrawal_request_service::create_for_provider_owner(
            fixture.store(),
            &fixture.owner_id,
            &fixture.provider_id,
            body,
        )
        .unwrap();
    assert!(replayed.replayed);
    assert_eq!(replayed.withdrawal_id, created.withdrawal_id);
    assert_eq!(replayed.request_posting_id, created.request_posting_id);
    assert_eq!(fixture.balance(), (700_000, 300_000, 2));
    assert_eq!(
        fixture.row_count("compute_settlement_withdrawal_requests"),
        1
    );
    assert_eq!(
        fixture.row_count("compute_settlement_withdrawal_request_postings"),
        1
    );
    assert_eq!(
        fixture.row_count("compute_settlement_withdrawal_request_ledger_legs"),
        2
    );

    let listed = compute_federation_settlement_withdrawal_request_service::list_for_provider_owner(
        fixture.store(),
        &fixture.owner_id,
        &fixture.provider_id,
        10,
    )
    .unwrap();
    assert_eq!(listed.len(), 1);
    assert!(
        compute_federation_settlement_withdrawal_request_service::get_for_provider_owner(
            fixture.store(),
            &fixture.other_owner_id,
            &fixture.provider_id,
            &created.withdrawal_id,
        )
        .is_err()
    );

    let conflicting = fixture.create_body(200_000, "request-idempotent");
    assert!(
        compute_federation_settlement_withdrawal_request_service::create_for_provider_owner(
            fixture.store(),
            &fixture.owner_id,
            &fixture.provider_id,
            conflicting,
        )
        .is_err()
    );
    let insufficient = fixture.create_body(800_000, "request-insufficient");
    assert!(
        compute_federation_settlement_withdrawal_request_service::create_for_provider_owner(
            fixture.store(),
            &fixture.owner_id,
            &fixture.provider_id,
            insufficient,
        )
        .is_err()
    );
    assert_eq!(fixture.balance(), (700_000, 300_000, 2));
    assert_eq!(
        fixture.row_count("compute_settlement_withdrawal_requests"),
        1
    );
}

#[test]
fn provider_cancel_returns_internal_balance_once_and_survives_reopen() {
    let mut fixture = Fixture::new();
    let withdrawal =
        compute_federation_settlement_withdrawal_request_service::create_for_provider_owner(
            fixture.store(),
            &fixture.owner_id,
            &fixture.provider_id,
            fixture.create_body(400_000, "cancel-request"),
        )
        .unwrap();
    let cancel = CancelMyComputeSettlementWithdrawalBody {
        expected_withdrawal_event_digest: withdrawal.event_digest.clone(),
        expected_request_posting_id: withdrawal.request_posting_id.clone(),
        expected_request_posting_digest: withdrawal.request_posting_digest.clone(),
        reason_code: "provider_changed_destination".to_string(),
        reason_detail: Some("local test cancellation".to_string()),
        idempotency_key: "cancel-idempotent".to_string(),
        confirm_internal_refund_only: true,
    };
    let mut missing_confirmation = cancel.clone();
    missing_confirmation.confirm_internal_refund_only = false;
    assert!(
        compute_federation_settlement_withdrawal_terminal_service::cancel_for_provider_owner(
            fixture.store(),
            &fixture.owner_id,
            &fixture.provider_id,
            &withdrawal.withdrawal_id,
            missing_confirmation,
        )
        .is_err()
    );
    assert_eq!(fixture.balance(), (600_000, 400_000, 2));

    let terminal =
        compute_federation_settlement_withdrawal_terminal_service::cancel_for_provider_owner(
            fixture.store(),
            &fixture.owner_id,
            &fixture.provider_id,
            &withdrawal.withdrawal_id,
            cancel.clone(),
        )
        .unwrap();
    assert_eq!(terminal.action, "cancelled");
    assert_eq!(terminal.balance_returned_micros, 400_000);
    assert_eq!(
        terminal.fund_effect,
        "provider_withdrawn_returned_to_available"
    );
    assert_eq!(terminal.external_transfer_effect, "not_executed");
    assert_eq!(fixture.balance(), (INITIAL_BALANCE_MICROS, 0, 3));

    let replayed =
        compute_federation_settlement_withdrawal_terminal_service::cancel_for_provider_owner(
            fixture.store(),
            &fixture.owner_id,
            &fixture.provider_id,
            &withdrawal.withdrawal_id,
            cancel,
        )
        .unwrap();
    assert!(replayed.replayed);
    assert_eq!(replayed.terminal_id, terminal.terminal_id);
    assert_eq!(fixture.balance(), (INITIAL_BALANCE_MICROS, 0, 3));
    assert_eq!(
        fixture.row_count("compute_settlement_withdrawal_terminals"),
        1
    );
    assert_eq!(
        fixture.row_count("compute_settlement_withdrawal_terminal_postings"),
        1
    );
    assert_eq!(
        fixture.row_count("compute_settlement_withdrawal_terminal_ledger_legs"),
        2
    );

    fixture.reopen();
    let reopened =
        compute_federation_settlement_withdrawal_terminal_service::get_for_provider_owner(
            fixture.store(),
            &fixture.owner_id,
            &fixture.provider_id,
            &withdrawal.withdrawal_id,
        )
        .unwrap();
    assert_eq!(reopened.terminal_id, terminal.terminal_id);
    assert_eq!(reopened.event_digest, terminal.event_digest);
    assert_eq!(fixture.balance(), (INITIAL_BALANCE_MICROS, 0, 3));
}

#[test]
fn admin_external_paid_attestation_records_claim_without_executing_payment() {
    let fixture = Fixture::new();
    let withdrawal =
        compute_federation_settlement_withdrawal_request_service::create_for_provider_owner(
            fixture.store(),
            &fixture.owner_id,
            &fixture.provider_id,
            fixture.create_body(250_000, "attestation-request"),
        )
        .unwrap();
    let body = AdminTerminalizeComputeSettlementWithdrawalBody {
        expected_withdrawal_event_digest: withdrawal.event_digest.clone(),
        expected_request_posting_id: withdrawal.request_posting_id.clone(),
        expected_request_posting_digest: withdrawal.request_posting_digest.clone(),
        action: "external_paid_attested".to_string(),
        reason_code: "operator_payment_attestation".to_string(),
        reason_detail: Some("payment is asserted by an administrator only".to_string()),
        external_evidence_kind: Some("bank_receipt".to_string()),
        external_evidence_ref: Some("vault://external-payment-evidence/test".to_string()),
        external_evidence_digest: Some("a".repeat(64)),
        idempotency_key: "attestation-idempotent".to_string(),
        confirm_refund_or_attestation_only: true,
        confirm_external_payment_already_completed: true,
        confirm_evidence_ref_contains_no_secret: true,
    };
    let mut missing_completed_confirmation = body.clone();
    missing_completed_confirmation.confirm_external_payment_already_completed = false;
    assert!(
        compute_federation_settlement_withdrawal_terminal_service::terminalize_for_platform_admin(
            fixture.store(),
            &fixture.admin_id,
            &withdrawal.withdrawal_id,
            missing_completed_confirmation,
        )
        .is_err()
    );
    let mut missing_secret_confirmation = body.clone();
    missing_secret_confirmation.confirm_evidence_ref_contains_no_secret = false;
    assert!(
        compute_federation_settlement_withdrawal_terminal_service::terminalize_for_platform_admin(
            fixture.store(),
            &fixture.admin_id,
            &withdrawal.withdrawal_id,
            missing_secret_confirmation,
        )
        .is_err()
    );
    assert_eq!(fixture.balance(), (750_000, 250_000, 2));

    let terminal =
        compute_federation_settlement_withdrawal_terminal_service::terminalize_for_platform_admin(
            fixture.store(),
            &fixture.admin_id,
            &withdrawal.withdrawal_id,
            body.clone(),
        )
        .unwrap();
    assert_eq!(terminal.action, "external_paid_attested");
    assert_eq!(terminal.balance_returned_micros, 0);
    assert_eq!(terminal.fund_effect, "provider_withdrawn_balance_retained");
    assert_eq!(
        terminal.external_transfer_effect,
        "external_payment_attested_not_executed_or_verified"
    );
    assert_eq!(fixture.balance(), (750_000, 250_000, 2));

    let replayed =
        compute_federation_settlement_withdrawal_terminal_service::terminalize_for_platform_admin(
            fixture.store(),
            &fixture.admin_id,
            &withdrawal.withdrawal_id,
            body,
        )
        .unwrap();
    assert!(replayed.replayed);
    assert_eq!(replayed.terminal_id, terminal.terminal_id);
    assert_eq!(fixture.balance(), (750_000, 250_000, 2));
    assert_eq!(
        fixture.row_count("compute_settlement_withdrawal_terminals"),
        1
    );
}
