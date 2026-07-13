use rusqlite::params;

use super::super::{codex_vault_emergency::CodexVaultEmergencyLeaseCreate, Store};

const INPUT_TOKENS: i64 = 60;
const OUTPUT_TOKENS: i64 = 20;
const COST_FEN: i64 = 40;
const EARNED_FEN: i64 = 32;
const STATUS: &str = "billed";

struct Fixture {
    store: Store,
    consumer_id: String,
    provider_id: String,
    grant_id: String,
    lease_id: String,
    node_id: String,
    compute_call_id: String,
    allowance_id: String,
    token_event_id: String,
    billing_event_id: String,
    node_transaction_id: String,
}

impl Fixture {
    fn new() -> (Self, std::path::PathBuf) {
        let suffix = uuid::Uuid::new_v4().simple().to_string();
        let path = std::env::temp_dir().join(format!("elon-late-usage-{suffix}.sqlite"));
        let _ = std::fs::remove_file(&path);
        let store = Store::open(&path).expect("store should open");
        let consumer = store
            .create_user(
                &format!("late-consumer-{suffix}@example.com"),
                "secret1",
                None,
                None,
            )
            .unwrap();
        let provider = store
            .create_user(
                &format!("late-provider-{suffix}@example.com"),
                "secret1",
                None,
                None,
            )
            .unwrap();
        let grant = store
            .upsert_codex_vault_emergency_grant(
                &provider.id,
                &consumer.id,
                Some("late completion proof"),
                Some("test"),
                Some(900),
                None,
                &provider.id,
            )
            .unwrap();
        let node_id = format!("node-{suffix}");
        let lease = store
            .create_codex_vault_emergency_lease(CodexVaultEmergencyLeaseCreate {
                grant_id: &grant.id,
                provider_user_id: &provider.id,
                consumer_user_id: &consumer.id,
                consumer_node_id: &node_id,
                provider_slot_id: "slot-late-proof",
                account_hint_hash: None,
                purpose: Some("late_completion_test"),
                failure_reason: None,
                max_lease_seconds: 900,
            })
            .unwrap();
        let compute_call_id = format!("pc_agent_cli:late-{suffix}");
        let allowance_id = format!("brv_{suffix}");
        let token_event_id = format!("tok_{suffix}");
        let billing_event_id = format!("bev_{suffix}");
        let node_transaction_id = format!("ntx_{suffix}");
        let conn = store.conn().unwrap();
        conn.execute(
            "INSERT INTO billing_reservations
             (id, user_id, compute_call_id, feature, usage_mode, model,
              reserved_fen, settled_cost_fen, refunded_fen, status,
              created_at, updated_at, expires_at)
             VALUES (?1, ?2, ?3, 'pc_agent_cli_chat', 'pc_agent_cli', 'pc-cli/gpt-5-codex',
                     100, 0, 0, 'dispatch_hold', ?4, ?4, ?5)",
            params![
                allowance_id,
                consumer.id,
                compute_call_id,
                "2026-01-01T00:00:00Z",
                "2099-01-01T00:00:00Z"
            ],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO node_compute_runs
             (id, compute_call_id, consumer_user_id, provider_user_id, node_id, model_id,
              feature, usage_mode, billing_source, resource_owner_user_id, lease_id,
              offline_policy, replay_deadline, max_cost_rmb_fen, allowance_id, status,
              started_at, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, 'pc-cli/gpt-5-codex',
                     'pc_agent_cli_chat', 'pc_agent_cli', 'shared_codex', ?4, ?6,
                     'require_active_reservation', ?7, 100, ?8, 'started', ?9, ?9, ?9)",
            params![
                format!("nrun_{suffix}"),
                compute_call_id,
                consumer.id,
                provider.id,
                node_id,
                lease.id,
                lease.expires_at,
                allowance_id,
                "2026-01-01T00:00:00Z"
            ],
        )
        .unwrap();
        drop(conn);
        (
            Self {
                store,
                consumer_id: consumer.id,
                provider_id: provider.id,
                grant_id: grant.id,
                lease_id: lease.id,
                node_id,
                compute_call_id,
                allowance_id,
                token_event_id,
                billing_event_id,
                node_transaction_id,
            },
            path,
        )
    }

    fn finish_exact_accounting(&self) {
        let conn = self.store.conn().unwrap();
        conn.execute(
            "INSERT INTO token_usage_events
             (id, user_id, feature, usage_mode, model, input_tokens, cached_input_tokens,
              output_tokens, reasoning_tokens, total_tokens, created_at, accounting_status,
              billing_event_id, cost_rmb_fen, idempotency_key, billing_source,
              resource_owner_user_id)
             VALUES (?1, ?2, 'pc_agent_cli_chat', 'pc_agent_cli', 'pc-cli/gpt-5-codex',
                     ?3, 0, ?4, 0, ?5, ?6, ?7, ?8, ?9, ?10, 'shared_codex', ?11)",
            params![
                self.token_event_id,
                self.consumer_id,
                INPUT_TOKENS,
                OUTPUT_TOKENS,
                INPUT_TOKENS + OUTPUT_TOKENS,
                "2026-01-01T00:01:00Z",
                STATUS,
                self.billing_event_id,
                COST_FEN,
                self.compute_call_id,
                self.provider_id,
            ],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO billing_events
             (id, user_id, model, input_tokens, cached_input_tokens, output_tokens,
              cost_rmb_fen, exchange_rate_x10000, markup_x1000, created_at,
              token_usage_event_id)
             VALUES (?1, ?2, 'pc-cli/gpt-5-codex', ?3, 0, ?4, ?5, 73000, 1200, ?6, ?7)",
            params![
                self.billing_event_id,
                self.consumer_id,
                INPUT_TOKENS,
                OUTPUT_TOKENS,
                COST_FEN,
                "2026-01-01T00:01:00Z",
                self.token_event_id,
            ],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO node_transactions
             (id, consumer_user_id, provider_user_id, node_id, model_id, feature, usage_mode,
              compute_call_id, token_usage_event_id, billing_event_id, prompt_tokens,
              completion_tokens, charged_credits, settled_credits, platform_fee_rate,
              billed_cost_rmb_fen, provider_earned_fen, provider_revenue_share_x1000,
              settlement_status, created_at)
             VALUES (?1, ?2, ?3, ?4, 'pc-cli/gpt-5-codex', 'pc_agent_cli_chat',
                     'pc_agent_cli', ?5, ?6, ?7, ?8, ?9, 0.4, 0.32, 0.2,
                     ?10, ?11, 800, ?12, ?13)",
            params![
                self.node_transaction_id,
                self.consumer_id,
                self.provider_id,
                self.node_id,
                self.compute_call_id,
                self.token_event_id,
                self.billing_event_id,
                INPUT_TOKENS,
                OUTPUT_TOKENS,
                COST_FEN,
                EARNED_FEN,
                STATUS,
                "2026-01-01T00:01:00Z",
            ],
        )
        .unwrap();
        conn.execute(
            "UPDATE billing_reservations
                SET status = 'settled', settled_cost_fen = ?2, refunded_fen = reserved_fen - ?2,
                    token_usage_event_id = ?3, billing_event_id = ?4, updated_at = ?5
              WHERE id = ?1",
            params![
                self.allowance_id,
                COST_FEN,
                self.token_event_id,
                self.billing_event_id,
                "2026-01-01T00:01:00Z"
            ],
        )
        .unwrap();
    }

    fn attach(&self) -> anyhow::Result<()> {
        self.store.attach_codex_vault_emergency_usage_strict(
            &self.lease_id,
            &self.compute_call_id,
            &self.token_event_id,
            Some(&self.billing_event_id),
            &self.node_transaction_id,
            INPUT_TOKENS,
            OUTPUT_TOKENS,
            COST_FEN,
            EARNED_FEN,
            STATUS,
        )
    }

    fn assert_unattached(&self) {
        let conn = self.store.conn().unwrap();
        let events: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM codex_vault_emergency_lease_usage_events
                  WHERE lease_id = ?1",
                [&self.lease_id],
                |row| row.get(0),
            )
            .unwrap();
        let total: i64 = conn
            .query_row(
                "SELECT total_tokens FROM codex_vault_emergency_leases WHERE id = ?1",
                [&self.lease_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!((events, total), (0, 0));
    }
}

#[derive(Clone, Copy, Debug)]
enum LateState {
    Active,
    Expired,
    Cleared,
    Revoked,
    Superseded,
}

fn transition_before_completion(f: &Fixture, state: LateState) {
    match state {
        LateState::Active => {}
        LateState::Expired => {
            f.store
                .conn()
                .unwrap()
                .execute(
                    "UPDATE codex_vault_emergency_leases
                        SET status = 'expired', expires_at = '2000-01-01T00:00:00Z'
                      WHERE id = ?1",
                    [&f.lease_id],
                )
                .unwrap();
        }
        LateState::Cleared => {
            f.store
                .clear_codex_vault_emergency_lease_for_node(
                    &f.consumer_id,
                    &f.node_id,
                    Some(&f.lease_id),
                )
                .unwrap()
                .unwrap();
        }
        LateState::Revoked => {
            f.store
                .revoke_codex_vault_emergency_grant(&f.grant_id, &f.provider_id)
                .unwrap()
                .unwrap();
        }
        LateState::Superseded => {
            f.store
                .create_codex_vault_emergency_lease(CodexVaultEmergencyLeaseCreate {
                    grant_id: &f.grant_id,
                    provider_user_id: &f.provider_id,
                    consumer_user_id: &f.consumer_id,
                    consumer_node_id: &f.node_id,
                    provider_slot_id: "slot-newer",
                    account_hint_hash: None,
                    purpose: Some("superseding_test"),
                    failure_reason: None,
                    max_lease_seconds: 900,
                })
                .unwrap();
        }
    }
}

#[test]
fn exact_run_usage_attaches_after_every_terminal_lease_transition() {
    for state in [
        LateState::Active,
        LateState::Expired,
        LateState::Cleared,
        LateState::Revoked,
        LateState::Superseded,
    ] {
        let (fixture, path) = Fixture::new();
        transition_before_completion(&fixture, state);
        fixture.finish_exact_accounting();
        fixture
            .attach()
            .unwrap_or_else(|error| panic!("{state:?} should attach: {error:#}"));
        fixture.attach().expect("exact replay should be idempotent");
        let lease = fixture
            .store
            .get_codex_vault_emergency_lease(&fixture.lease_id)
            .unwrap()
            .unwrap();
        assert_eq!(lease.total_tokens, INPUT_TOKENS + OUTPUT_TOKENS);
        assert_eq!(lease.billed_cost_rmb_fen, COST_FEN);
        assert_eq!(lease.provider_earned_fen, EARNED_FEN);
        let event_count: i64 = fixture
            .store
            .conn()
            .unwrap()
            .query_row(
                "SELECT COUNT(*) FROM codex_vault_emergency_lease_usage_events
                  WHERE lease_id = ?1",
                [&fixture.lease_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(event_count, 1);
        drop(fixture);
        let _ = std::fs::remove_file(path);
    }
}

#[derive(Clone, Copy, Debug)]
enum Mismatch {
    Run,
    Lease,
    User,
    Node,
    Token,
    Transaction,
    Source,
    Allowance,
    UsageModel,
    NodeModel,
    AllowanceModel,
    BillingModel,
    OutputTokens,
}

#[test]
fn mismatched_audit_edge_rejects_without_partial_attach() {
    for mismatch in [
        Mismatch::Run,
        Mismatch::Lease,
        Mismatch::User,
        Mismatch::Node,
        Mismatch::Token,
        Mismatch::Transaction,
        Mismatch::Source,
        Mismatch::Allowance,
        Mismatch::UsageModel,
        Mismatch::NodeModel,
        Mismatch::AllowanceModel,
        Mismatch::BillingModel,
        Mismatch::OutputTokens,
    ] {
        let (mut fixture, path) = Fixture::new();
        fixture.finish_exact_accounting();
        match mismatch {
            Mismatch::Run => fixture.compute_call_id = "pc_agent_cli:wrong-run".into(),
            Mismatch::Lease => fixture.lease_id = "cvle_wrong-lease".into(),
            Mismatch::Token => fixture.token_event_id = "tok_wrong".into(),
            Mismatch::Transaction => fixture.node_transaction_id = "ntx_wrong".into(),
            Mismatch::User => {
                fixture
                    .store
                    .conn()
                    .unwrap()
                    .execute(
                        "UPDATE token_usage_events SET user_id = ?2 WHERE id = ?1",
                        params![fixture.token_event_id, fixture.provider_id],
                    )
                    .unwrap();
            }
            Mismatch::Node => {
                fixture
                    .store
                    .conn()
                    .unwrap()
                    .execute(
                        "UPDATE node_transactions SET node_id = 'wrong-node' WHERE id = ?1",
                        [&fixture.node_transaction_id],
                    )
                    .unwrap();
            }
            Mismatch::Source => {
                fixture
                    .store
                    .conn()
                    .unwrap()
                    .execute(
                        "UPDATE token_usage_events SET billing_source = 'own_codex' WHERE id = ?1",
                        [&fixture.token_event_id],
                    )
                    .unwrap();
            }
            Mismatch::Allowance => {
                fixture
                    .store
                    .conn()
                    .unwrap()
                    .execute(
                        "UPDATE billing_reservations SET refunded_fen = 59 WHERE id = ?1",
                        [&fixture.allowance_id],
                    )
                    .unwrap();
            }
            Mismatch::UsageModel => {
                fixture
                    .store
                    .conn()
                    .unwrap()
                    .execute(
                        "UPDATE token_usage_events SET model = 'pc-cli/cheaper-model' WHERE id = ?1",
                        [&fixture.token_event_id],
                    )
                    .unwrap();
            }
            Mismatch::NodeModel => {
                fixture
                    .store
                    .conn()
                    .unwrap()
                    .execute(
                        "UPDATE node_transactions SET model_id = 'pc-cli/cheaper-model' WHERE id = ?1",
                        [&fixture.node_transaction_id],
                    )
                    .unwrap();
            }
            Mismatch::AllowanceModel => {
                fixture
                    .store
                    .conn()
                    .unwrap()
                    .execute(
                        "UPDATE billing_reservations SET model = 'pc-cli/cheaper-model' WHERE id = ?1",
                        [&fixture.allowance_id],
                    )
                    .unwrap();
            }
            Mismatch::BillingModel => {
                fixture
                    .store
                    .conn()
                    .unwrap()
                    .execute(
                        "UPDATE billing_events SET model = 'pc-cli/cheaper-model' WHERE id = ?1",
                        [&fixture.billing_event_id],
                    )
                    .unwrap();
            }
            Mismatch::OutputTokens => {
                fixture
                    .store
                    .conn()
                    .unwrap()
                    .execute(
                        "UPDATE token_usage_events SET output_tokens = output_tokens - 1 WHERE id = ?1",
                        [&fixture.token_event_id],
                    )
                    .unwrap();
            }
        }
        assert!(fixture.attach().is_err(), "{mismatch:?} must fail closed");
        if !matches!(mismatch, Mismatch::Lease) {
            fixture.assert_unattached();
        }
        drop(fixture);
        let _ = std::fs::remove_file(path);
    }
}

#[test]
fn caller_cannot_attach_a_forged_lower_price() {
    let (fixture, path) = Fixture::new();
    fixture.finish_exact_accounting();
    assert!(fixture
        .store
        .attach_codex_vault_emergency_usage_strict(
            &fixture.lease_id,
            &fixture.compute_call_id,
            &fixture.token_event_id,
            Some(&fixture.billing_event_id),
            &fixture.node_transaction_id,
            INPUT_TOKENS,
            OUTPUT_TOKENS,
            COST_FEN - 1,
            EARNED_FEN - 1,
            STATUS,
        )
        .is_err());
    fixture.assert_unattached();
    drop(fixture);
    let _ = std::fs::remove_file(path);
}

#[test]
fn idempotent_event_rejects_changed_replay_amounts() {
    let (fixture, path) = Fixture::new();
    fixture.finish_exact_accounting();
    fixture.attach().unwrap();
    assert!(fixture
        .store
        .attach_codex_vault_emergency_usage_strict(
            &fixture.lease_id,
            &fixture.compute_call_id,
            &fixture.token_event_id,
            Some(&fixture.billing_event_id),
            &fixture.node_transaction_id,
            INPUT_TOKENS + 1,
            OUTPUT_TOKENS,
            COST_FEN,
            EARNED_FEN,
            STATUS,
        )
        .is_err());
    let lease = fixture
        .store
        .get_codex_vault_emergency_lease(&fixture.lease_id)
        .unwrap()
        .unwrap();
    assert_eq!(lease.total_tokens, INPUT_TOKENS + OUTPUT_TOKENS);
    drop(fixture);
    let _ = std::fs::remove_file(path);
}
