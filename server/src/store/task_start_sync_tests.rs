use super::*;
use crate::store::PcCliTaskCompletionApply;

#[test]
fn local_start_is_idempotent_and_binds_codex_thread() {
    let path = std::env::temp_dir().join(format!(
        "elon-local-task-start-{}.sqlite",
        uuid::Uuid::new_v4().simple()
    ));
    let store = Store::open(&path).unwrap();
    let user = store
        .create_user("local-start@example.com", "secret1", None, None)
        .unwrap();
    let project = store
        .create_project(&user.id, "Local start", None, None)
        .unwrap()
        .project;
    let channel = store
        .list_project_space_channels(&user.id, &project.id)
        .unwrap()
        .into_iter()
        .find(|value| value.kind == "ai_development")
        .unwrap();
    let first = store
        .apply_pc_local_task_start(start_input(
            "local-task-a",
            "r1",
            &project.id,
            &channel.id,
            "desktop-supervised-a",
            &user.id,
            None,
        ))
        .unwrap();
    let duplicate = store
        .apply_pc_local_task_start(start_input(
            "local-task-a",
            "r1",
            &project.id,
            &channel.id,
            "desktop-supervised-a",
            &user.id,
            None,
        ))
        .unwrap();
    let bound = store
        .apply_pc_local_task_start(start_input(
            "local-task-a",
            "r2",
            &project.id,
            &channel.id,
            "desktop-supervised-a",
            &user.id,
            Some("thread-a"),
        ))
        .unwrap();
    assert!(first.created);
    assert!(!duplicate.changed);
    assert_eq!(first.task_id, duplicate.task_id);
    assert_eq!(first.task_id, bound.task_id);
    assert_eq!(
        store
            .latest_task_codex_thread_id(&project.id, &user.id, "desktop-supervised-a")
            .unwrap()
            .as_deref(),
        Some("thread-a")
    );
    let conn = store.conn().unwrap();
    let user_messages: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM messages WHERE task_id = ?1 AND role = 'user'",
            params![first.task_id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(user_messages, 1);
}

#[test]
fn start_and_completion_converge_in_both_arrival_orders() {
    let path = std::env::temp_dir().join(format!(
        "elon-local-task-order-{}.sqlite",
        uuid::Uuid::new_v4().simple()
    ));
    let store = Store::open(&path).unwrap();
    let user = store
        .create_user("local-order@example.com", "secret1", None, None)
        .unwrap();
    let project = store
        .create_project(&user.id, "Local order", None, None)
        .unwrap()
        .project;
    let channel = store
        .list_project_space_channels(&user.id, &project.id)
        .unwrap()
        .into_iter()
        .find(|value| value.kind == "ai_development")
        .unwrap();

    let started = store
        .apply_pc_local_task_start(start_input(
            "local-task-start-first",
            "r1",
            &project.id,
            &channel.id,
            "desktop-start-first",
            &user.id,
            Some("thread-start-first"),
        ))
        .unwrap();
    let completed = store
        .apply_pc_cli_task_completion(completion_input(
            "event-start-first",
            "local-task-start-first",
            &project.id,
            &channel.id,
            "desktop-start-first",
            &user.id,
            "thread-start-first",
        ))
        .unwrap();
    assert_eq!(started.task_id, completed.task_id);

    let completed_early = store
        .apply_pc_cli_task_completion(completion_input(
            "event-completion-first",
            "local-task-completion-first",
            &project.id,
            &channel.id,
            "desktop-completion-first",
            &user.id,
            "thread-completion-first",
        ))
        .unwrap();
    let synced_late = store
        .apply_pc_local_task_start(start_input(
            "local-task-completion-first",
            "r1",
            &project.id,
            &channel.id,
            "desktop-completion-first",
            &user.id,
            Some("thread-completion-first"),
        ))
        .unwrap();
    assert_eq!(completed_early.task_id, synced_late.task_id);

    let conn = store.conn().unwrap();
    for request_id in ["local-task-start-first", "local-task-completion-first"] {
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM tasks WHERE client_request_id = ?1",
                params![format!("pc_local_task:{request_id}")],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }
}

fn start_input<'a>(
    request_id: &'a str,
    revision: &'a str,
    project_id: &'a str,
    channel_id: &'a str,
    conversation_id: &'a str,
    user_id: &'a str,
    session_id: Option<&'a str>,
) -> PcLocalTaskStartApply<'a> {
    PcLocalTaskStartApply {
        request_id,
        revision,
        project_id,
        channel_id,
        conversation_id,
        user_id,
        node_id: "node-a",
        prompt: "修复会话同步",
        workspace_path: "D:\\repo",
        cli: "codex",
        status: "running",
        codex_session_id: session_id,
    }
}

fn completion_input<'a>(
    event_id: &'a str,
    request_id: &'a str,
    project_id: &'a str,
    channel_id: &'a str,
    conversation_id: &'a str,
    user_id: &'a str,
    session_id: &'a str,
) -> PcCliTaskCompletionApply<'a> {
    PcCliTaskCompletionApply {
        completion_event_id: event_id,
        task_id: None,
        local_request_id: Some(request_id),
        project_id,
        channel_id: Some(channel_id),
        conversation_id,
        user_id,
        prompt: Some("修复会话同步"),
        final_reply: "已完成",
        channel_result: "已完成",
        status: "done",
        error: None,
        codex_session_id: Some(session_id),
    }
}
