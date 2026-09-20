use anyhow::Result;
use rusqlite::Connection;

pub(crate) fn migrate(conn: &Connection) -> Result<()> {
    let tx = conn.unchecked_transaction()?;
    // Preserve the only child table while replacing the legacy per-message uniqueness.
    tx.execute_batch(
        "CREATE TEMP TABLE saved_group_ai_work_options AS SELECT * FROM group_ai_work_options;
         DROP TABLE group_ai_work_options;
         ALTER TABLE group_ai_reply_requests RENAME TO group_ai_reply_requests_legacy;
         CREATE TABLE group_ai_reply_requests (
           id TEXT PRIMARY KEY,
           group_id TEXT NOT NULL REFERENCES friend_groups(id) ON DELETE CASCADE,
           trigger_message_id TEXT NOT NULL,
           requester_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
           state TEXT NOT NULL CHECK(state IN ('prepared','server_ready','dispatched','completed','indeterminate','cancelled')),
           result_message_id TEXT, created_at TEXT NOT NULL, updated_at TEXT NOT NULL,
           engine TEXT NOT NULL DEFAULT 'server_api' CHECK(engine IN ('server_api','chatgpt_web')),
           operation_hash TEXT UNIQUE, context_prompt TEXT,
           web_provider TEXT NOT NULL DEFAULT 'chatgpt_web' CHECK(web_provider IN ('chatgpt_web','google_web')),
           context_scope TEXT NOT NULL DEFAULT 'recent' CHECK(context_scope IN ('recent','selected')),
           selection_question TEXT NOT NULL DEFAULT '',
           CHECK((state = 'completed') = (result_message_id IS NOT NULL))
         );
         INSERT INTO group_ai_reply_requests
           (id,group_id,trigger_message_id,requester_id,state,result_message_id,created_at,updated_at,
            engine,operation_hash,context_prompt,web_provider)
           SELECT id,group_id,trigger_message_id,requester_id,state,result_message_id,created_at,updated_at,
            engine,operation_hash,context_prompt,web_provider FROM group_ai_reply_requests_legacy;
         DROP TABLE group_ai_reply_requests_legacy;
         CREATE UNIQUE INDEX group_ai_recent_owner ON group_ai_reply_requests(group_id,trigger_message_id)
           WHERE context_scope='recent';
         CREATE TABLE group_ai_work_options (
           request_id TEXT PRIMARY KEY REFERENCES group_ai_reply_requests(id) ON DELETE CASCADE,
           agent TEXT, allow_fallback INTEGER NOT NULL CHECK(allow_fallback IN (0,1))
         );
         INSERT INTO group_ai_work_options SELECT * FROM saved_group_ai_work_options;
         DROP TABLE saved_group_ai_work_options;
         CREATE TABLE group_ai_selected_sources (
           request_id TEXT NOT NULL REFERENCES group_ai_reply_requests(id) ON DELETE CASCADE,
           message_id TEXT NOT NULL, revision INTEGER NOT NULL,
           PRIMARY KEY(request_id,message_id)
         );",
    )?;
    tx.commit()?;
    Ok(())
}
