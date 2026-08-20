//! V278 fail-closed trigger consumers for the connection-local ordered write plan.

use anyhow::{ensure, Result};
use rusqlite::Connection;

const UDF: &str = "elon_v278_external_pool_adapter_task_reachability_pending_plan_matches";

const INSERT_GUARDS: [(&str, &str, &str, usize); 7] = [
    (
        "v273_task_exchange_attempt_no_replace",
        "compute_external_pool_adapter_task_exchange_attempts",
        "exchange_attempt",
        52,
    ),
    (
        "v273_task_exchange_receipt_no_replace",
        "compute_external_pool_adapter_task_exchange_receipts",
        "exchange_receipt",
        65,
    ),
    (
        "v273_task_reconcile_poll_no_replace",
        "compute_external_pool_adapter_task_reconcile_polls",
        "reconcile_poll",
        39,
    ),
    (
        "v273_task_event_poll_no_replace",
        "compute_external_pool_adapter_task_event_polls",
        "event_poll",
        42,
    ),
    (
        "v273_task_event_batch_no_replace",
        "compute_external_pool_adapter_task_event_batches",
        "event_batch",
        35,
    ),
    (
        "v273_task_event_no_replace",
        "compute_external_pool_adapter_task_events",
        "event",
        21,
    ),
    (
        "trg_compute_attempt_start_send_no_replace",
        "compute_attempt_start_send_attempts",
        "start_send_attempt",
        18,
    ),
];

const LEGACY_PARALLEL_GUARDS: [&str; 10] = [
    "v278_task_reachability_exchange_attempt_insert",
    "v278_task_reachability_exchange_receipt_insert",
    "v278_task_reachability_reconcile_poll_insert",
    "v278_task_reachability_event_poll_insert",
    "v278_task_reachability_event_batch_insert",
    "v278_task_reachability_event_insert",
    "v278_task_reachability_start_send_insert",
    "v278_task_reachability_start_outbox_send_cas",
    "v278_task_reachability_reconcile_poll_cas",
    "v278_task_reachability_event_poll_cas",
];

pub(super) fn install(conn: &Connection) -> Result<()> {
    remove_legacy_parallel_guards(conn)?;
    for (trigger, table, kind, arity) in INSERT_GUARDS {
        install_insert_guard(conn, trigger, table, kind, arity)?;
    }
    install_start_outbox_cas_guard(conn)?;
    install_poll_cas_guard(
        conn,
        "v273_task_reconcile_poll_claim_cas",
        "compute_external_pool_adapter_task_reconcile_polls",
        "reconcile_poll_id",
        "reconcile_poll_digest",
        "reconcile_poll_cas",
    )?;
    install_poll_cas_guard(
        conn,
        "v273_task_event_poll_claim_cas",
        "compute_external_pool_adapter_task_event_polls",
        "event_poll_id",
        "event_poll_digest",
        "event_poll_cas",
    )
}

fn install_insert_guard(
    conn: &Connection,
    trigger: &str,
    table: &str,
    kind: &str,
    expected_arity: usize,
) -> Result<()> {
    let columns = table_columns(conn, table)?;
    ensure!(
        columns.len() == expected_arity,
        "V278 reachability insert guard arity drifted for {table}"
    );
    let arguments = columns
        .iter()
        .map(|column| format!("NEW.{column}"))
        .collect::<Vec<_>>()
        .join(",");
    let external_pool_scope = if table == "compute_attempt_start_send_attempts" {
        "EXISTS (SELECT 1 FROM compute_attempt_start_outbox outbox
                   JOIN compute_providers provider ON provider.provider_id=outbox.provider_id
                  WHERE outbox.outbox_id=NEW.outbox_id
                    AND outbox.outbox_digest=NEW.outbox_digest
                    AND provider.provider_kind='external_pool') AND "
    } else {
        ""
    };
    append_guard_condition(
        conn,
        trigger,
        &format!("{external_pool_scope}{UDF}('{kind}',{arguments}) IS NOT 1"),
    )
}

fn install_start_outbox_cas_guard(conn: &Connection) -> Result<()> {
    append_guard_condition(
        conn,
        "trg_compute_attempt_start_outbox_transition",
        &format!(
            "OLD.state='claimed' AND NEW.state='in_flight_unknown'
             AND EXISTS (SELECT 1 FROM compute_providers provider
                          WHERE provider.provider_id=OLD.provider_id
                            AND provider.provider_kind='external_pool')
             AND {UDF}('start_outbox_cas',
                 OLD.outbox_id,OLD.outbox_digest,OLD.state,NEW.state,
                 OLD.state_revision,NEW.state_revision,
                 OLD.attempt_count,NEW.attempt_count,
                 OLD.claim_owner_id,NEW.claim_owner_id,
                 OLD.claim_token_digest,NEW.claim_token_digest,
                 OLD.claim_generation,NEW.claim_generation,
                 OLD.claim_expires_at,NEW.claim_expires_at,NEW.updated_at) IS NOT 1"
        ),
    )
}

fn install_poll_cas_guard(
    conn: &Connection,
    trigger: &str,
    table: &str,
    id_column: &str,
    digest_column: &str,
    kind: &str,
) -> Result<()> {
    ensure!(
        table_columns(conn, table)?.contains(&id_column.to_owned()),
        "V278 poll CAS source table drifted for {table}"
    );
    append_guard_condition(
        conn,
        trigger,
        &format!(
            "{UDF}('{kind}',
             OLD.{id_column},OLD.{digest_column},
             OLD.claim_status,NEW.claim_status,
             OLD.claim_revision,NEW.claim_revision,
             OLD.claim_generation,NEW.claim_generation,
             OLD.claim_owner_id,NEW.claim_owner_id,
             OLD.claim_token_digest,NEW.claim_token_digest,
             OLD.claim_expires_at,NEW.claim_expires_at) IS NOT 1"
        ),
    )
}

fn append_guard_condition(conn: &Connection, trigger: &str, condition: &str) -> Result<()> {
    let sql: String = conn.query_row(
        "SELECT sql FROM sqlite_master WHERE type='trigger' AND name=?1",
        [trigger],
        |row| row.get(0),
    )?;
    if sql.contains(UDF) {
        ensure!(
            sql.matches(UDF).count() == 1,
            "V278 reachability plan was appended more than once to {trigger}"
        );
        return Ok(());
    }
    ensure!(
        sql.contains("WHEN") && sql.contains("BEFORE "),
        "V278 reachability source guard shape drifted for {trigger}"
    );
    let begin = sql
        .rfind("BEGIN")
        .ok_or_else(|| anyhow::anyhow!("V278 reachability source guard lost BEGIN: {trigger}"))?;
    let replacement = format!("{}OR ({condition})\n{}", &sql[..begin], &sql[begin..]);
    conn.execute_batch(&format!(
        "DROP TRIGGER IF EXISTS {trigger};\n{replacement};"
    ))?;
    let installed: String = conn.query_row(
        "SELECT sql FROM sqlite_master WHERE type='trigger' AND name=?1",
        [trigger],
        |row| row.get(0),
    )?;
    ensure!(
        installed.matches(UDF).count() == 1,
        "V278 reachability source guard was not installed exactly once: {trigger}"
    );
    Ok(())
}

fn remove_legacy_parallel_guards(conn: &Connection) -> Result<()> {
    for trigger in LEGACY_PARALLEL_GUARDS {
        conn.execute_batch(&format!("DROP TRIGGER IF EXISTS {trigger};"))?;
    }
    Ok(())
}

fn table_columns(conn: &Connection, table: &str) -> Result<Vec<String>> {
    let mut statement = conn.prepare(&format!("PRAGMA table_info({table})"))?;
    let columns = statement
        .query_map([], |row| row.get::<_, String>(1))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    ensure!(
        !columns.is_empty(),
        "V278 reachability table is missing: {table}"
    );
    Ok(columns)
}
