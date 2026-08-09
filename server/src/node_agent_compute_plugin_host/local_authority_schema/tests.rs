use rusqlite::Connection;

use super::ensure_schema;

mod versioning_v7;

#[test]
fn schema_installs_and_reopens_with_candidate_health_and_cleanup_objects() {
    let mut connection = Connection::open_in_memory().unwrap();
    connection
        .pragma_update(None, "foreign_keys", "ON")
        .unwrap();
    connection
        .pragma_update(None, "trusted_schema", "OFF")
        .unwrap();

    ensure_schema(&mut connection).unwrap();
    ensure_schema(&mut connection).unwrap();

    let table_count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'candidate_health_receipts'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    let trigger_count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'trigger' AND tbl_name = 'candidate_health_receipts'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    let quarantine_table_count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'candidate_health_quarantine_receipts'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    let quarantine_trigger_count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'trigger' AND tbl_name = 'candidate_health_quarantine_receipts'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    let cleanup_table_count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name IN ('candidate_cleanup_authorizations', 'candidate_cleanup_completions')",
            [],
            |row| row.get(0),
        )
        .unwrap();
    let cleanup_trigger_count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'trigger' AND tbl_name IN ('candidate_cleanup_authorizations', 'candidate_cleanup_completions')",
            [],
            |row| row.get(0),
        )
        .unwrap();
    let cleanup_execution_table_count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name IN ('candidate_cleanup_execution_plans', 'candidate_cleanup_expected_objects', 'candidate_cleanup_execution_plan_seals', 'candidate_cleanup_step_events')",
            [],
            |row| row.get(0),
        )
        .unwrap();
    let cleanup_execution_trigger_count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'trigger' AND tbl_name IN ('candidate_cleanup_execution_plans', 'candidate_cleanup_expected_objects', 'candidate_cleanup_execution_plan_seals', 'candidate_cleanup_step_events')",
            [],
            |row| row.get(0),
        )
        .unwrap();
    let cleanup_completion_journal_gate_count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'trigger' AND name = 'candidate_cleanup_completion_requires_execution_journal'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    let owner_schema: String = connection
        .query_row(
            "SELECT sql FROM sqlite_master WHERE type = 'table' AND name = 'candidate_owners'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    let owner_transition_count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'trigger' AND name IN ('candidate_cleanup_pending_requires_authorization', 'candidate_cleaned_requires_completion')",
            [],
            |row| row.get(0),
        )
        .unwrap();
    let active_candidate_index: String = connection
        .query_row(
            "SELECT sql FROM sqlite_master WHERE type = 'index' AND name = 'one_owned_candidate_per_plugin'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    let owner_transition_schema: String = connection
        .query_row(
            "SELECT sql FROM sqlite_master WHERE type = 'trigger' AND name = 'candidate_state_transition'",
            [],
            |row| row.get(0),
        )
        .unwrap();

    assert_eq!(table_count, 1);
    assert_eq!(trigger_count, 3);
    assert_eq!(quarantine_table_count, 1);
    assert_eq!(quarantine_trigger_count, 3);
    assert_eq!(cleanup_table_count, 2);
    assert_eq!(cleanup_trigger_count, 7);
    assert_eq!(cleanup_execution_table_count, 4);
    assert_eq!(cleanup_execution_trigger_count, 12);
    assert_eq!(cleanup_completion_journal_gate_count, 1);
    assert!(owner_schema.contains("cleanup_pending"));
    assert!(owner_schema.contains("cleaned"));
    assert_eq!(owner_transition_count, 2);
    assert!(active_candidate_index.contains("cleanup_pending"));
    assert!(owner_transition_schema.contains("OLD.state = 'cleanup_pending'"));
    assert!(owner_transition_schema.contains("NEW.state = 'cleaned'"));
}
