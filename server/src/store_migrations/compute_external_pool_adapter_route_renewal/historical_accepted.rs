//! Admit receipt-authenticated historical Accepted closure without weakening fresh guards.

use anyhow::{ensure, Result};
use rusqlite::Connection;

mod source;

const MARKER: &str = "external_pool_adapter_task_receipt.v1";
const UDF: &str = "elon_v278_external_pool_adapter_task_reachability_pending_plan_matches";

const ACTOR: &str = "trg_compute_attempt_dispatch_actor_exact_source";
const AUTHORITY: &str = "trg_compute_attempt_lease_authority_projection";
const APPLICATION_LIVE: &str = "trg_compute_attempt_application_live_authority_v215";
const COMMIT_LIVE: &str = "trg_compute_attempt_commit_live_authority_v215";
const APPLICATION_CLOSURE: &str = "trg_compute_attempt_application_commit_closure_v213";

pub(super) fn install(connection: &Connection) -> Result<()> {
    append_admission(
        connection,
        ACTOR,
        "NEW.actor_phase='application'",
        &source::actor(),
        Some(actor_pending()),
    )?;
    append_projection_guard(connection, &source::authority(), authority_pending())?;
    append_admission(
        connection,
        COMMIT_LIVE,
        "NEW.operation_kind='commit'",
        &source::commit(),
        Some(commit_pending()),
    )?;
    append_admission(
        connection,
        APPLICATION_LIVE,
        "commit_intent.lease_id=NEW.lease_id",
        &source::application(),
        None,
    )?;
    append_admission(
        connection,
        APPLICATION_CLOSURE,
        "commit_intent.operation_kind='commit'",
        &source::application(),
        Some(application_pending()),
    )
}

fn append_admission(
    connection: &Connection,
    trigger: &str,
    legacy_marker: &str,
    historical_source: &str,
    pending: Option<&str>,
) -> Result<()> {
    let sql = trigger_sql(connection, trigger)?;
    if sql.contains(MARKER) {
        ensure_installed(&sql, trigger, legacy_marker, pending.is_some())?;
        return Ok(());
    }
    ensure!(
        sql.contains("NOT EXISTS (") && sql.contains(legacy_marker) && sql.contains("BEGIN"),
        "V278 historical Accepted predecessor guard drifted: {trigger}"
    );
    let sql = sql.replacen("NOT EXISTS (", "NOT (EXISTS (", 1);
    let begin = sql
        .rfind("BEGIN")
        .ok_or_else(|| anyhow::anyhow!("V278 historical Accepted guard lost BEGIN: {trigger}"))?;
    let branch = match pending {
        Some(pending) => format!(
            " OR (CASE WHEN EXISTS (\n{historical_source}\n        ) \
             THEN ({pending} IS 1) ELSE 0 END))\n"
        ),
        None => format!(" OR EXISTS (\n{historical_source}\n        ))\n"),
    };
    replace_trigger(
        connection,
        trigger,
        &format!("{}{branch}{}", &sql[..begin], &sql[begin..]),
    )?;
    let installed = trigger_sql(connection, trigger)?;
    ensure_installed(&installed, trigger, legacy_marker, pending.is_some())
}

fn append_projection_guard(
    connection: &Connection,
    historical_source: &str,
    pending: &str,
) -> Result<()> {
    let sql = trigger_sql(connection, AUTHORITY)?;
    let legacy_marker = "json_extract(NEW.authority_json,'$.lease_authority_id')";
    if sql.contains(MARKER) {
        ensure_installed(&sql, AUTHORITY, legacy_marker, true)?;
        return Ok(());
    }
    ensure!(
        sql.contains("BEFORE INSERT ON compute_attempt_lease_authority_bindings")
            && sql.contains(legacy_marker)
            && sql.contains("projection mismatch"),
        "V278 historical Accepted authority projection guard drifted"
    );
    let begin = sql
        .rfind("BEGIN")
        .ok_or_else(|| anyhow::anyhow!("V278 authority projection guard lost BEGIN"))?;
    let branch = format!(
        " OR (CASE WHEN EXISTS (\n{historical_source}\n        ) \
         THEN {pending} ELSE 1 END IS NOT 1)\n"
    );
    replace_trigger(
        connection,
        AUTHORITY,
        &format!("{}{branch}{}", &sql[..begin], &sql[begin..]),
    )?;
    let installed = trigger_sql(connection, AUTHORITY)?;
    ensure_installed(&installed, AUTHORITY, legacy_marker, true)
}

fn ensure_installed(
    sql: &str,
    trigger: &str,
    legacy_marker: &str,
    expects_pending: bool,
) -> Result<()> {
    ensure!(
        sql.matches(MARKER).count() == 1 && sql.contains(legacy_marker),
        "V278 historical Accepted branch is not exact: {trigger}"
    );
    ensure!(
        sql.matches(UDF).count() == usize::from(expects_pending),
        "V278 historical Accepted pending-plan guard drifted: {trigger}"
    );
    Ok(())
}

fn trigger_sql(connection: &Connection, trigger: &str) -> Result<String> {
    connection
        .query_row(
            "SELECT sql FROM sqlite_master WHERE type='trigger' AND name=?1",
            [trigger],
            |row| row.get(0),
        )
        .map_err(Into::into)
}

fn replace_trigger(connection: &Connection, trigger: &str, replacement: &str) -> Result<()> {
    connection.execute_batch(&format!(
        "DROP TRIGGER IF EXISTS {trigger};\n{replacement};"
    ))?;
    Ok(())
}

fn actor_pending() -> &'static str {
    "elon_v278_external_pool_adapter_task_reachability_pending_plan_matches(\
        'historical_accepted_actor',NEW.actor_receipt_id,NEW.actor_receipt_digest,\
        NEW.actor_receipt_json,NEW.command_id,NEW.ack_id,NEW.application_id,NEW.recorded_at)"
}

fn authority_pending() -> &'static str {
    "elon_v278_external_pool_adapter_task_reachability_pending_plan_matches(\
        'historical_accepted_lease_authority',NEW.lease_authority_id,\
        NEW.authority_revision,NEW.lease_authority_digest,NEW.authority_json,\
        NEW.command_id,NEW.application_id,NEW.recorded_at)"
}

fn commit_pending() -> &'static str {
    "elon_v278_external_pool_adapter_task_reachability_pending_plan_matches(\
        'historical_accepted_commit',NEW.outbox_id,NEW.outbox_digest,NEW.outbox_json,\
        NEW.command_id,NEW.application_id,NEW.created_at)"
}

fn application_pending() -> &'static str {
    "elon_v278_external_pool_adapter_task_reachability_pending_plan_matches(\
        'historical_accepted_application',NEW.application_id,NEW.application_digest,\
        NEW.command_id,NEW.ack_id,NEW.applied_at,NEW.created_at)"
}

#[cfg(test)]
mod tests {
    use rusqlite::Connection;

    use super::{append_admission, trigger_sql, MARKER};

    #[test]
    fn admission_accepts_a_live_guard_with_a_business_predicate_before_not_exists() {
        let connection = Connection::open_in_memory().expect("open migration fixture");
        connection
            .execute_batch(
                "CREATE TABLE guarded(id INTEGER, actor_phase TEXT NOT NULL);
                 CREATE TRIGGER guarded_exact_source
                 BEFORE INSERT ON guarded
                 WHEN NEW.actor_phase='application' AND NOT EXISTS (SELECT 1 WHERE 0)
                 BEGIN
                   SELECT RAISE(ABORT, 'legacy guard rejected row');
                 END;",
            )
            .expect("install live predecessor guard");

        append_admission(
            &connection,
            "guarded_exact_source",
            "NEW.actor_phase='application'",
            "SELECT 1 WHERE 'external_pool_adapter_task_receipt.v1' IS NOT NULL",
            None,
        )
        .expect("append historical admission after the business predicate");

        let installed = trigger_sql(&connection, "guarded_exact_source").expect("read trigger");
        assert!(installed.contains("AND NOT (EXISTS ("));
        assert_eq!(installed.matches(MARKER).count(), 1);
        connection
            .execute(
                "INSERT INTO guarded(id,actor_phase) VALUES(1,'application')",
                [],
            )
            .expect("historical branch should admit the row");
    }
}
