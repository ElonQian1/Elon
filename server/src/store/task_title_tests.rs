use rusqlite::params;

use super::{PcCliTaskCompletionApply, Store};

fn temp_store() -> Store {
    let path = std::env::temp_dir().join(format!(
        "elon-task-title-{}.sqlite",
        uuid::Uuid::new_v4().simple()
    ));
    Store::open(&path).expect("store should open")
}

fn local_task_context(store: &Store) -> (String, String, String) {
    let user = store
        .create_user("task-title@example.com", "secret1", None, None)
        .expect("user");
    let project = store
        .create_project(&user.id, "Task title project", None, None)
        .expect("project")
        .project;
    let channel = store
        .list_project_space_channels(&user.id, &project.id)
        .expect("channels")
        .into_iter()
        .find(|channel| channel.kind == "ai_development")
        .expect("development channel");
    (user.id, project.id, channel.id)
}

fn replay_local_task(
    store: &Store,
    user_id: &str,
    project_id: &str,
    channel_id: &str,
    prompt: &str,
) {
    store
        .apply_pc_cli_task_completion(PcCliTaskCompletionApply {
            completion_event_id: "event-readable-title",
            task_id: None,
            project_id,
            channel_id: Some(channel_id),
            conversation_id: "offline-readable-title",
            user_id,
            prompt: Some(prompt),
            final_reply: "任务已完成",
            channel_result: "任务已完成",
            status: "done",
            error: None,
            codex_session_id: Some("session-readable-title"),
        })
        .expect("completion should replay");
}

fn stored_title(store: &Store, project_id: &str, user_id: &str) -> String {
    store
        .conn()
        .unwrap()
        .query_row(
            "SELECT title FROM conversations
              WHERE project_id = ?1 AND user_id = ?2 AND id = 'offline-readable-title'",
            params![project_id, user_id],
            |row| row.get(0),
        )
        .unwrap()
}

#[test]
fn local_completion_creates_readable_title_and_repairs_only_exact_placeholder() {
    let store = temp_store();
    let (user_id, project_id, channel_id) = local_task_context(&store);
    let prompt = r#"codex://threads/019-title
<elon-pc-executor version="1"></elon-pc-executor>
<user-request>
用户原始需求：“用户希望的是，有适合人阅读且可区分的任务标题。”
桌面监督分析结论：不需要 Goal 模式。
</user-request>"#;

    replay_local_task(&store, &user_id, &project_id, &channel_id, prompt);
    assert_eq!(
        stored_title(&store, &project_id, &user_id),
        "适合人阅读且可区分的任务标题"
    );

    store
        .conn()
        .unwrap()
        .execute(
            "UPDATE conversations SET title = '本机离线任务'
              WHERE project_id = ?1 AND user_id = ?2 AND id = 'offline-readable-title'",
            params![project_id, user_id],
        )
        .unwrap();

    let own_list = store
        .list_user_conversations(&project_id, &user_id, 10)
        .unwrap();
    assert_eq!(
        own_list[0].title.as_deref(),
        Some("适合人阅读且可区分的任务标题")
    );
    let member_list = store
        .list_project_member_conversations(&user_id, &project_id, &user_id, 10)
        .unwrap();
    assert_eq!(
        member_list[0].title.as_deref(),
        Some("适合人阅读且可区分的任务标题")
    );
    assert_eq!(stored_title(&store, &project_id, &user_id), "本机离线任务");

    replay_local_task(&store, &user_id, &project_id, &channel_id, prompt);
    assert_eq!(
        stored_title(&store, &project_id, &user_id),
        "适合人阅读且可区分的任务标题"
    );

    store
        .conn()
        .unwrap()
        .execute(
            "UPDATE conversations SET title = '用户手工标题'
              WHERE project_id = ?1 AND user_id = ?2 AND id = 'offline-readable-title'",
            params![project_id, user_id],
        )
        .unwrap();
    replay_local_task(
        &store,
        &user_id,
        &project_id,
        &channel_id,
        "这次不应覆盖手工标题",
    );
    assert_eq!(stored_title(&store, &project_id, &user_id), "用户手工标题");
}
