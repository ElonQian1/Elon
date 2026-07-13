use std::{fs, path::Path};

use homecli_proto::{CliCompletionEnvelope, CliCompletionProducerIdentity, CliProjectContext};
use rusqlite::{params, Connection};

use super::{CliCompletionOutbox, STATUS_PENDING};

fn completion(event_id: &str, req_id: &str) -> CliCompletionEnvelope {
    CliCompletionEnvelope {
        event_id: event_id.to_string(),
        req_id: req_id.to_string(),
        cli: "codex".to_string(),
        origin: "cloud_dispatch".to_string(),
        producer_identity: Some(CliCompletionProducerIdentity {
            owner_user_id: "owner-a".to_string(),
            agent_id: "node-a".to_string(),
            install_id: "install-a".to_string(),
        }),
        project_context: Some(CliProjectContext {
            project_id: "project-a".to_string(),
            conversation_id: "conversation-a".to_string(),
            runtime_permission: Some("project_write".to_string()),
        }),
        channel_id: None,
        prompt: None,
        final_output: "done".to_string(),
        exit_ok: true,
        error: None,
        session_id: None,
        prompt_tokens: Some(1),
        cached_input_tokens: Some(0),
        completion_tokens: Some(1),
        reasoning_tokens: Some(0),
        total_tokens: Some(2),
        model: Some("test".to_string()),
        workspace_status: None,
        created_at_ms: 1,
    }
}

#[test]
fn legacy_unscoped_rows_are_migrated_but_never_replayed_under_current_identity() {
    let path = std::env::temp_dir().join(format!(
        "elon-outbox-identity-migration-{}.sqlite3",
        uuid::Uuid::new_v4().simple()
    ));
    let conn = Connection::open(&path).unwrap();
    conn.execute_batch(
        "CREATE TABLE cli_completion_outbox (
            event_id TEXT PRIMARY KEY,
            req_id TEXT NOT NULL UNIQUE,
            payload_json TEXT NOT NULL,
            status TEXT NOT NULL,
            created_at_ms INTEGER NOT NULL,
            attempt_count INTEGER NOT NULL DEFAULT 0,
            last_attempt_at_ms INTEGER,
            last_error TEXT,
            acked_at_ms INTEGER
         );",
    )
    .unwrap();
    let legacy = completion("legacy-event", "legacy-req");
    conn.execute(
        "INSERT INTO cli_completion_outbox
         (event_id, req_id, payload_json, status, created_at_ms)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            legacy.event_id,
            legacy.req_id,
            serde_json::to_string(&legacy).unwrap(),
            STATUS_PENDING,
            1_i64
        ],
    )
    .unwrap();
    drop(conn);

    let outbox = CliCompletionOutbox::new(&path);
    let identity = legacy.producer_identity.as_ref().unwrap();
    assert!(outbox
        .list_pending_for_producer(identity, 10)
        .unwrap()
        .is_empty());
    let fresh = completion("fresh-event", "fresh-req");
    outbox.enqueue(&fresh).unwrap();
    let pending = outbox.list_pending_for_producer(identity, 10).unwrap();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].completion.event_id, "fresh-event");

    cleanup(&path);
}

fn cleanup(path: &Path) {
    let _ = fs::remove_file(path);
    let _ = fs::remove_file(path.with_extension("sqlite3-wal"));
    let _ = fs::remove_file(path.with_extension("sqlite3-shm"));
}
