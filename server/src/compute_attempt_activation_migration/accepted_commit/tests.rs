use rusqlite::Connection;

use super::backfill::ensure_no_unsafe_backfill;

#[test]
fn accepted_commit_backfill_allows_an_empty_v214_projection() {
    let connection = backfill_projection();

    ensure_no_unsafe_backfill(&connection).expect("empty v214 projection should be safe");
}

#[test]
fn accepted_commit_backfill_rejects_cleanup_with_accepted_ack() {
    let connection = backfill_projection();
    connection
        .execute(
            "INSERT INTO compute_attempt_start_outbox (command_id, operation_kind)
             VALUES (?1, 'cancel')",
            ["cmd_conflict"],
        )
        .expect("legacy cleanup should insert");
    connection
        .execute(
            "INSERT INTO compute_attempt_dispatch_acks (command_id, outcome, disposition)
             VALUES (?1, 'accepted', 'accepted_applied')",
            ["cmd_conflict"],
        )
        .expect("legacy accepted ACK should insert");

    let error = ensure_no_unsafe_backfill(&connection)
        .expect_err("accepted custody must not coexist with cleanup custody");
    assert!(error
        .to_string()
        .contains("COMPUTE_ATTEMPT_ACCEPTED_COMMIT_BACKFILL_REQUIRED"));
}

fn backfill_projection() -> Connection {
    let connection = Connection::open_in_memory().expect("in-memory database should open");
    connection
        .execute_batch(
            "CREATE TABLE compute_attempt_start_outbox (
                command_id TEXT,
                command_digest TEXT,
                operation_kind TEXT,
                operation_generation INTEGER,
                ack_id TEXT,
                ack_digest TEXT,
                application_id TEXT,
                application_digest TEXT,
                route_authorization_id TEXT,
                route_authorization_digest TEXT,
                lease_id TEXT,
                actor_receipt_id TEXT,
                actor_receipt_digest TEXT,
                lease_authority_id TEXT,
                lease_authority_revision INTEGER,
                lease_authority_digest TEXT,
                created_at TEXT,
                not_before TEXT,
                not_after TEXT
             );
             CREATE TABLE compute_attempt_dispatch_acks (
                command_id TEXT,
                outcome TEXT,
                disposition TEXT
             );
             CREATE TABLE compute_attempt_dispatch_applications (
                command_id TEXT,
                application_id TEXT,
                application_digest TEXT,
                ack_id TEXT,
                lease_id TEXT,
                created_at TEXT,
                applied_at TEXT
             );
             CREATE TABLE compute_attempt_dispatch_commands (
                command_id TEXT,
                command_digest TEXT,
                lease_id TEXT,
                reservation_id TEXT,
                provider_id TEXT,
                activated_by_user_id TEXT,
                executor_id TEXT,
                adapter_id TEXT,
                adapter_binding_digest TEXT,
                lease_expires_at TEXT
             );
             CREATE TABLE compute_attempt_activations (
                lease_id TEXT,
                reservation_id TEXT
             );
             CREATE TABLE compute_attempt_dispatch_actor_receipts (
                actor_receipt_id TEXT,
                actor_receipt_digest TEXT,
                actor_phase TEXT,
                command_id TEXT,
                command_digest TEXT,
                provider_id TEXT,
                provider_owner_account_id TEXT,
                service_actor_id TEXT,
                route_authorization_id TEXT,
                route_authorization_digest TEXT,
                actor_authorization_id TEXT,
                actor_authorization_digest TEXT,
                ack_id TEXT,
                ack_digest TEXT,
                application_id TEXT,
                application_digest TEXT,
                issued_at TEXT,
                recorded_at TEXT,
                valid_until TEXT
             );
             CREATE TABLE compute_route_authorization_receipts (
                route_authorization_id TEXT,
                route_authorization_digest TEXT,
                provider_id TEXT,
                provider_owner_account_id TEXT,
                executor_id TEXT,
                adapter_id TEXT,
                adapter_binding_digest TEXT,
                verified_by_service_actor_id TEXT,
                actor_authorization_id TEXT,
                actor_authorization_digest TEXT,
                authorized_at TEXT,
                recorded_at TEXT,
                expires_at TEXT
             );
             CREATE TABLE compute_service_actor_authorizations (
                actor_authorization_id TEXT,
                actor_authorization_digest TEXT,
                provider_id TEXT,
                provider_owner_account_id TEXT,
                service_actor_id TEXT,
                issued_at TEXT,
                recorded_at TEXT,
                valid_until TEXT,
                allowed_actor_phases_json TEXT
             );
             CREATE TABLE compute_attempt_lease_authority_bindings (
                lease_authority_id TEXT,
                authority_revision INTEGER,
                lease_authority_digest TEXT,
                command_id TEXT,
                command_digest TEXT,
                ack_id TEXT,
                ack_digest TEXT,
                application_id TEXT,
                application_digest TEXT,
                lease_id TEXT,
                route_authorization_id TEXT,
                route_authorization_digest TEXT,
                application_actor_receipt_id TEXT,
                application_actor_receipt_digest TEXT,
                recorded_at TEXT,
                expires_at TEXT
             );",
        )
        .expect("backfill projection should initialize");
    connection
}
