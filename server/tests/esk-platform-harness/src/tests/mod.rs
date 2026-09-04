use crate::{esk_asset::platform::*, platform_migration, store::Store};
use rusqlite::{params, Connection};
use std::{fs, path::PathBuf};

mod auth;
mod cancellation;
mod recovery;
mod transactions;
mod validation;

pub(super) struct Fixture {
    pub store: Store,
    path: PathBuf,
}

impl Fixture {
    pub fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "elon-esk-platform-harness-{}.sqlite",
            uuid::Uuid::new_v4().simple(),
        ));
        let store = Store { path: path.clone() };
        let conn = store.conn().unwrap();
        // Use the exact production users table DDL, not a permissive role mock.
        let schema = include_str!("../../../../src/store_migrations/migrations_v1_v16.rs");
        for table in ["users", "sessions"] {
            let start = schema
                .find(&format!("CREATE TABLE IF NOT EXISTS {table} ("))
                .unwrap();
            let end = schema[start..].find(");").unwrap() + start + 2;
            conn.execute_batch(&schema[start..end]).unwrap();
        }
        // Production account_security_migration adds this nullable revocation field.
        conn.execute_batch("ALTER TABLE sessions ADD COLUMN revoked_at TEXT")
            .unwrap();
        for (id, role, status) in [
            ("admin-1", "admin", "active"),
            ("owner-1", "owner", "active"),
            ("alice", "user", "active"),
            ("bob", "user", "active"),
            ("inactive-admin", "admin", "disabled"),
            ("inactive-user", "user", "disabled"),
            // Reserved virtual identity stays forbidden even if a corrupt fixture contains a row.
            ("local-owner", "owner", "active"),
        ] {
            conn.execute(
                "INSERT INTO users(id,password_hash,role,status,created_at,updated_at)
                 VALUES(?1,'synthetic-non-login-hash',?2,?3,'fixture','fixture')",
                params![id, role, status],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO sessions(id,user_id,token_hash,expires_at,created_at)
                 VALUES(?1,?1,?2,'2099-01-01T00:00:00Z','fixture')",
                params![id, crate::store::common::hash_token(&token(id))],
            )
            .unwrap();
        }
        platform_migration::migration_v287(&conn).unwrap();
        crate::paper_migration::migration_v281(&conn).unwrap();
        conn.execute(
            "INSERT INTO esk_asset_ledger_entries(entry_id,user_id,amount_base_units,
             entry_kind,reference,idempotency_key,actor,created_at)
             VALUES('paper-fixture','alice',123000000,'paper_allocation',
             'fixture-paper','fixture-paper','platform_admin','fixture')",
            [],
        )
        .unwrap();
        Self { store, path }
    }

    pub fn count(&self, table: &str) -> i64 {
        // All table names are hardcoded test constants, never user input.
        self.store
            .conn()
            .unwrap()
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                row.get(0)
            })
            .unwrap()
    }

    pub fn paper_total(&self) -> i64 {
        self.store
            .conn()
            .unwrap()
            .query_row(
                "SELECT SUM(amount_base_units) FROM esk_asset_ledger_entries",
                [],
                |row| row.get(0),
            )
            .unwrap()
    }

    pub fn assert_empty_posting(&self) {
        assert_eq!(self.count("esk_platform_approvals"), 0);
        assert_eq!(self.count("esk_platform_ledger_entries"), 0);
        assert_eq!(
            self.store
                .esk_platform_account("alice", 20)
                .unwrap()
                .total_base_units,
            0
        );
        assert_eq!(self.paper_total(), 123000000);
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        // Only exact files created by this fixture; never recursive workspace cleanup.
        let _ = fs::remove_file(&self.path);
        for suffix in ["-journal", "-wal", "-shm"] {
            let _ = fs::remove_file(PathBuf::from(format!("{}{suffix}", self.path.display())));
        }
    }
}

pub(super) fn source() -> PaymentSource {
    serde_json::from_value(serde_json::json!({
        "namespace":"fixture.operator-ledger", "network":"synthetic",
        "asset_symbol":"USDT", "asset_reference":"fixture-usdt-contract",
        "decimals":6, "reference_format":"hex32",
    }))
    .unwrap()
}

pub(super) fn policy(limit: i64) -> PlatformPolicy {
    validate_policy(PolicyBody {
        source: source(),
        issuance_limit_base_units: limit.to_string(),
    })
    .unwrap()
}

pub(super) fn body() -> PrepareBody {
    serde_json::from_value(body_json()).unwrap()
}

pub(super) fn body_json() -> serde_json::Value {
    serde_json::json!({
        "schema": PREPARE_SCHEMA, "user_id":"alice",
        "external_payment_reference":"a".repeat(64), "transfer_index":0,
        "payment_amount":"20.000000", "amount":"10.000000",
        "commercial_purpose":"esk_purchase",
        "sale": { "sale_batch_id":"synthetic-sale", "payment_base_units_per_lot":"2000000",
            "esk_base_units_per_lot":"1000000", "disclosure_revision":"synthetic-disclosure",
            "terms_digest":"3".repeat(64) },
        "payment_evidence_digest":"4".repeat(64), "consent_digest":"5".repeat(64),
        "history_evidence_digest":"6".repeat(64), "history_complete":true,
        "review_reference":"synthetic-review",
    })
}

pub(super) fn input(policy: &PlatformPolicy) -> PlatformAllocationInput {
    prepare_input(policy, body()).unwrap()
}

pub(super) fn token(user: &str) -> String {
    format!("synthetic-harness-session-only-{user}")
}

pub(super) fn prepare(fixture: &Fixture, policy: &PlatformPolicy) -> PlatformAllocationRecord {
    fixture
        .store
        .prepare_esk_platform_allocation(policy, &input(policy), "admin-1", &token("admin-1"))
        .unwrap()
}

pub(super) fn record(
    fixture: &Fixture,
    policy: &PlatformPolicy,
    prepared: &PlatformAllocationRecord,
) -> PlatformAllocationRecord {
    fixture
        .store
        .record_esk_platform_allocation(
            policy,
            &prepared.allocation_id,
            &prepared.input.request_digest,
            "admin-1",
            &token("admin-1"),
        )
        .unwrap()
}

pub(super) fn assert_error<T: std::fmt::Debug>(result: anyhow::Result<T>, code: PlatformError) {
    let error = result.unwrap_err();
    assert_eq!(
        error.downcast_ref::<PlatformError>(),
        Some(&code),
        "{error:#}"
    );
}
