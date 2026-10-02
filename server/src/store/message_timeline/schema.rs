//! Bounded, body-free change journal. Triggers participate in the message transaction.
use anyhow::Result;
use rusqlite::Connection;

pub(crate) fn migrate(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS message_timeline_changes (
          seq INTEGER PRIMARY KEY AUTOINCREMENT, kind TEXT NOT NULL,
          scope TEXT NOT NULL, project TEXT NOT NULL, audience TEXT NOT NULL,
          message_id TEXT NOT NULL, operation TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS message_timeline_change_scope
          ON message_timeline_changes(kind, project, scope, seq);
        CREATE TRIGGER IF NOT EXISTS timeline_change_retention AFTER INSERT ON message_timeline_changes BEGIN
          DELETE FROM message_timeline_changes WHERE seq < new.seq-1000000;
        END;
        CREATE TABLE IF NOT EXISTS message_timeline_epoch (id INTEGER PRIMARY KEY CHECK(id=1), value TEXT NOT NULL);
        INSERT OR IGNORE INTO message_timeline_epoch VALUES (1, lower(hex(randomblob(16))));
        CREATE INDEX IF NOT EXISTS timeline_group_history ON friend_group_messages(group_id,created_at,id);
        CREATE INDEX IF NOT EXISTS timeline_friend_history ON friend_messages(sender_user_id,receiver_user_id,created_at,id);
        CREATE INDEX IF NOT EXISTS timeline_friend_context_history ON friend_messages(sender_user_id,receiver_user_id,context_user_id,created_at,id);
        CREATE INDEX IF NOT EXISTS timeline_ai_history ON messages(project_id,conversation_id,created_at,id);
        CREATE INDEX IF NOT EXISTS timeline_channel_history ON project_channel_messages(project_id,channel_id,created_at,id);",
    )?;
    for (table, kind, scope, project, audience) in [
        ("friend_group_messages", "group", "r.group_id", "''", "''"),
        ("messages", "ai", "COALESCE(r.conversation_id,'default')", "r.project_id", "''"),
        ("project_channel_messages", "channel", "r.channel_id", "r.project_id", "''"),
        ("friend_messages", "friend",
         "CASE WHEN r.sender_user_id='usr_elon_ai' AND r.context_user_id IS NOT NULL
           THEN json_array(min(r.receiver_user_id,r.context_user_id),max(r.receiver_user_id,r.context_user_id))
           ELSE json_array(min(r.sender_user_id,r.receiver_user_id),max(r.sender_user_id,r.receiver_user_id)) END",
         "''", "CASE WHEN r.sender_user_id='usr_elon_ai' AND r.context_user_id IS NOT NULL THEN r.receiver_user_id ELSE '' END"),
    ] {
        for (event, row) in [("INSERT", "new"), ("UPDATE", "new"), ("DELETE", "old")] {
            let scope = scope.replace("r.", &format!("{row}."));
            let project = project.replace("r.", &format!("{row}."));
            let audience = audience.replace("r.", &format!("{row}."));
            conn.execute_batch(&format!(
                "CREATE TRIGGER IF NOT EXISTS timeline_{kind}_{event} AFTER {event} ON {table} BEGIN
                 INSERT INTO message_timeline_changes(kind,scope,project,audience,message_id,operation)
                   VALUES('{kind}',{scope},{project},{audience},{row}.id,'{event}');
                 END;"
            ))?;
        }
    }
    // Joined projections must invalidate with the rows that supply their visible metadata.
    conn.execute_batch("CREATE INDEX IF NOT EXISTS timeline_task_messages ON project_channel_messages(task_id);
        CREATE INDEX IF NOT EXISTS timeline_selected_sources ON group_ai_selected_sources(message_id);
        CREATE TRIGGER IF NOT EXISTS timeline_task_metadata AFTER UPDATE OF status,error,apk_url,codex_thread_id ON tasks BEGIN
          INSERT INTO message_timeline_changes(kind,scope,project,audience,message_id,operation)
            SELECT 'channel',channel_id,project_id,'',id,'UPDATE' FROM project_channel_messages WHERE task_id=new.id;
        END;
        CREATE TRIGGER IF NOT EXISTS timeline_group_ai_metadata AFTER UPDATE ON group_ai_reply_requests WHEN new.result_message_id IS NOT NULL BEGIN
          INSERT INTO message_timeline_changes(kind,scope,project,audience,message_id,operation)
            SELECT 'group',group_id,'','',id,'UPDATE' FROM friend_group_messages WHERE id=new.result_message_id;
        END;
        CREATE TRIGGER IF NOT EXISTS timeline_group_ai_consent AFTER UPDATE ON group_ai_reply_contexts BEGIN
          INSERT INTO message_timeline_changes(kind,scope,project,audience,message_id,operation)
            SELECT 'group',m.group_id,'','',m.id,'UPDATE' FROM friend_group_messages m JOIN group_ai_reply_requests r ON r.result_message_id=m.id WHERE r.id=new.request_id;
        END;")?;
    for (kind, table) in [
        ("group", "friend_group_messages"),
        ("friend", "friend_messages"),
    ] {
        for event in ["UPDATE OF recalled_at", "DELETE"] {
            let name = if event == "DELETE" {
                "delete"
            } else {
                "recall"
            };
            // Touch only the identity so the ordinary, privacy-aware journal trigger is reused.
            conn.execute_batch(&format!("CREATE TRIGGER IF NOT EXISTS timeline_{kind}_quote_{name} AFTER {event} ON {table} BEGIN
                UPDATE {table} SET id=id WHERE id IN (SELECT message_id FROM social_message_quotes WHERE kind='{kind}' AND source_id=old.id);
            END;"))?;
        }
    }
    for event in ["UPDATE OF recalled_at", "DELETE"] {
        let name = if event == "DELETE" {
            "delete"
        } else {
            "recall"
        };
        conn.execute_batch(&format!("CREATE TRIGGER IF NOT EXISTS timeline_group_sources_{name} AFTER {event} ON friend_group_messages BEGIN
            UPDATE friend_group_messages SET id=id WHERE id IN (SELECT r.result_message_id FROM group_ai_selected_sources s JOIN group_ai_reply_requests r ON r.id=s.request_id WHERE s.message_id=old.id);
        END;"))?;
    }
    Ok(())
}
