use super::*;
use uuid::Uuid;

fn temp_store() -> Store {
    let path = std::env::temp_dir().join(format!(
        "elon_project_execution_sessions_{}.db",
        Uuid::new_v4().simple()
    ));
    Store::open(&path).expect("store should open")
}

fn finish_session(
    store: &Store,
    project_id: &str,
    request_id: &str,
    user_id: &str,
    node_id: &str,
    status: &str,
    error: Option<&str>,
) -> bool {
    store
        .record_project_execution_finished(ProjectExecutionSessionFinish {
            request_id,
            project_id,
            conversation_id: request_id,
            user_id,
            node_id,
            base_workspace_path: None,
            active_workspace_path: None,
            branch: None,
            isolated: false,
            status,
            merge_status: Some(status),
            last_error: error,
            model: Some("codex"),
            prompt_tokens: Some(1),
            cached_input_tokens: Some(0),
            completion_tokens: Some(1),
            reasoning_tokens: Some(0),
            total_tokens: Some(2),
            token_usage_event_id: Some("terminal-token"),
            billing_event_id: None,
        })
        .unwrap()
}

#[test]
fn latest_session_tracks_workspace_finish() {
    let store = temp_store();
    let user = store
        .create_user("project-execution@example.com", "secret1", None, None)
        .expect("user should be created");
    let project = store
        .create_project(&user.id, "执行会话项目", None, Some("android"))
        .expect("project should be created")
        .project;
    store
        .record_project_execution_started(ProjectExecutionSessionStart {
            project_id: &project.id,
            conversation_id: "conv-a",
            user_id: &user.id,
            node_id: "node-a",
            request_id: "req-a",
            requested_workspace_path: Some("D:/repo"),
            model: Some("codex"),
        })
        .expect("start should record");
    store
        .record_project_execution_finished(ProjectExecutionSessionFinish {
            request_id: "req-a",
            project_id: &project.id,
            conversation_id: "conv-a",
            user_id: &user.id,
            node_id: "node-a",
            base_workspace_path: Some("D:/repo"),
            active_workspace_path: Some("D:/wt"),
            branch: Some("ai/session/prj-a/conv-a"),
            isolated: true,
            status: "done",
            merge_status: Some("merged"),
            last_error: None,
            model: Some("gpt-5"),
            prompt_tokens: Some(100),
            cached_input_tokens: Some(20),
            completion_tokens: Some(30),
            reasoning_tokens: Some(5),
            total_tokens: Some(130),
            token_usage_event_id: Some("tok-a"),
            billing_event_id: Some("bev-a"),
        })
        .expect("finish should update");

    let latest = store
        .latest_project_execution_session(&project.id)
        .expect("latest should query")
        .expect("latest should exist");
    assert_eq!(latest.status, "done");
    assert_eq!(latest.active_workspace_path.as_deref(), Some("D:/wt"));
    assert!(latest.isolated);
    assert_eq!(latest.model.as_deref(), Some("gpt-5"));
    assert_eq!(latest.prompt_tokens, 100);
    assert_eq!(latest.cached_input_tokens, 20);
    assert_eq!(latest.completion_tokens, 30);
    assert_eq!(latest.reasoning_tokens, 5);
    assert_eq!(latest.total_tokens, 130);
    assert_eq!(latest.token_usage_event_id.as_deref(), Some("tok-a"));
    assert_eq!(latest.billing_event_id.as_deref(), Some("bev-a"));

    assert!(store
        .bind_project_execution_task_id("req-a", "task-a")
        .expect("task binding should update"));
    assert!(!store
        .bind_project_execution_task_id("req-a", "task-other")
        .expect("different task binding should not overwrite"));

    let by_request = store
        .get_project_execution_session_by_request_id("req-a")
        .expect("request lookup should query")
        .expect("request lookup should find session");
    assert_eq!(by_request.id, latest.id);
    assert_eq!(by_request.task_id.as_deref(), Some("task-a"));
    assert_eq!(
        by_request.branch.as_deref(),
        Some("ai/session/prj-a/conv-a")
    );
    let by_task = store
        .get_project_execution_session_by_task_id("task-a")
        .expect("task lookup should query")
        .expect("task lookup should find session");
    assert_eq!(by_task.id, latest.id);
}

#[test]
fn startup_interrupts_running_project_execution_sessions() {
    let store = temp_store();
    let user = store
        .create_user(
            "project-execution-restart@example.com",
            "secret1",
            None,
            None,
        )
        .expect("user should be created");
    let project = store
        .create_project(&user.id, "执行会话重启项目", None, Some("android"))
        .expect("project should be created")
        .project;
    store
        .record_project_execution_started(ProjectExecutionSessionStart {
            project_id: &project.id,
            conversation_id: "conv-restart",
            user_id: &user.id,
            node_id: "node-a",
            request_id: "req-restart",
            requested_workspace_path: Some("D:/repo"),
            model: Some("codex"),
        })
        .expect("start should record");

    assert_eq!(
        store
            .mark_interrupted_running_project_execution_sessions()
            .expect("running sessions should be interrupted"),
        1
    );
    let session = store
        .get_project_execution_session_by_request_id("req-restart")
        .expect("request lookup should query")
        .expect("request lookup should find session");
    assert_eq!(session.status, "failed");
    assert_eq!(session.merge_status.as_deref(), Some("interrupted"));
    assert_eq!(
        session.last_error.as_deref(),
        Some("server restarted before PC CLI terminal event")
    );
}

#[test]
fn request_identity_conflicts_do_not_rebind_or_reopen_terminal_session() {
    let store = temp_store();
    let user = store
        .create_user("session-identity@example.com", "secret1", None, None)
        .unwrap();
    let project = store
        .create_project(&user.id, "Session Identity", None, None)
        .unwrap()
        .project;
    let other_project = store
        .create_project(&user.id, "Other Session Identity", None, None)
        .unwrap()
        .project;
    assert!(store
        .record_project_execution_started(ProjectExecutionSessionStart {
            project_id: &project.id,
            conversation_id: "conversation-a",
            user_id: &user.id,
            node_id: "node-a",
            request_id: "request-strict",
            requested_workspace_path: Some("D:/a"),
            model: Some("codex"),
        })
        .unwrap());
    store
        .record_project_execution_finished(ProjectExecutionSessionFinish {
            request_id: "request-strict",
            project_id: &project.id,
            conversation_id: "conversation-a",
            user_id: &user.id,
            node_id: "node-a",
            base_workspace_path: None,
            active_workspace_path: Some("D:/a"),
            branch: None,
            isolated: false,
            status: "done",
            merge_status: Some("merged"),
            last_error: None,
            model: Some("codex"),
            prompt_tokens: None,
            cached_input_tokens: None,
            completion_tokens: None,
            reasoning_tokens: None,
            total_tokens: None,
            token_usage_event_id: None,
            billing_event_id: None,
        })
        .unwrap();
    assert!(store
        .record_project_execution_started(ProjectExecutionSessionStart {
            project_id: &project.id,
            conversation_id: "conversation-a",
            user_id: &user.id,
            node_id: "node-a",
            request_id: "request-strict",
            requested_workspace_path: Some("D:/a"),
            model: Some("codex"),
        })
        .unwrap());
    assert!(!store
        .record_project_execution_started(ProjectExecutionSessionStart {
            project_id: &other_project.id,
            conversation_id: "conversation-a",
            user_id: &user.id,
            node_id: "node-other",
            request_id: "request-strict",
            requested_workspace_path: Some("D:/other"),
            model: Some("codex"),
        })
        .unwrap());
    let session = store
        .get_project_execution_session_by_request_id("request-strict")
        .unwrap()
        .unwrap();
    assert_eq!(session.project_id, project.id);
    assert_eq!(session.node_id, "node-a");
    assert_eq!(session.status, "done");
}

#[test]
fn late_finish_keeps_session_canceled_when_bound_task_was_canceled() {
    let store = temp_store();
    let user = store
        .create_user("session-canceled@example.com", "secret1", None, None)
        .unwrap();
    let project = store
        .create_project(&user.id, "Canceled Session", None, None)
        .unwrap()
        .project;
    let task_id = store
        .create_task(&project.id, &user.id, Some("conversation-canceled"), "work")
        .unwrap();
    assert!(store
        .record_project_execution_started(ProjectExecutionSessionStart {
            project_id: &project.id,
            conversation_id: "conversation-canceled",
            user_id: &user.id,
            node_id: "node-a",
            request_id: "request-canceled",
            requested_workspace_path: None,
            model: Some("codex"),
        })
        .unwrap());
    assert!(store
        .bind_project_execution_task_id("request-canceled", &task_id)
        .unwrap());
    store
        .finish_task(&task_id, "canceled", None, None, Some("用户主动取消"))
        .unwrap();
    store
        .record_project_execution_finished(ProjectExecutionSessionFinish {
            request_id: "request-canceled",
            project_id: &project.id,
            conversation_id: "conversation-canceled",
            user_id: &user.id,
            node_id: "node-a",
            base_workspace_path: None,
            active_workspace_path: None,
            branch: None,
            isolated: false,
            status: "done",
            merge_status: Some("completion_replayed"),
            last_error: None,
            model: Some("codex"),
            prompt_tokens: Some(10),
            cached_input_tokens: Some(0),
            completion_tokens: Some(5),
            reasoning_tokens: Some(0),
            total_tokens: Some(15),
            token_usage_event_id: Some("token-canceled"),
            billing_event_id: None,
        })
        .unwrap();
    let session = store
        .get_project_execution_session_by_request_id("request-canceled")
        .unwrap()
        .unwrap();
    assert_eq!(session.status, "canceled");
    assert_eq!(session.merge_status.as_deref(), Some("canceled"));
    assert_eq!(session.last_error.as_deref(), Some("用户主动取消"));
    assert_eq!(session.total_tokens, 15);
}

#[test]
fn finish_is_identity_strict_and_only_repairs_automatic_failures() {
    let store = temp_store();
    let user = store
        .create_user("session-terminal@example.com", "secret1", None, None)
        .unwrap();
    let project = store
        .create_project(&user.id, "Terminal Session", None, None)
        .unwrap()
        .project;
    let start = |request_id: &str| {
        store
            .record_project_execution_started(ProjectExecutionSessionStart {
                project_id: &project.id,
                conversation_id: request_id,
                user_id: &user.id,
                node_id: "node-terminal",
                request_id,
                requested_workspace_path: None,
                model: Some("codex"),
            })
            .unwrap()
    };

    assert!(start("terminal-canceled"));
    assert!(finish_session(
        &store,
        &project.id,
        "terminal-canceled",
        &user.id,
        "node-terminal",
        "canceled",
        Some("用户主动取消")
    ));
    assert!(finish_session(
        &store,
        &project.id,
        "terminal-canceled",
        &user.id,
        "node-terminal",
        "done",
        None
    ));
    assert_eq!(
        store
            .get_project_execution_session_by_request_id("terminal-canceled")
            .unwrap()
            .unwrap()
            .status,
        "canceled"
    );

    assert!(start("terminal-done"));
    assert!(finish_session(
        &store,
        &project.id,
        "terminal-done",
        &user.id,
        "node-terminal",
        "done",
        None
    ));
    assert!(finish_session(
        &store,
        &project.id,
        "terminal-done",
        &user.id,
        "node-terminal",
        "failed",
        Some("late failure")
    ));
    assert_eq!(
        store
            .get_project_execution_session_by_request_id("terminal-done")
            .unwrap()
            .unwrap()
            .status,
        "done"
    );

    assert!(start("terminal-business-failed"));
    assert!(finish_session(
        &store,
        &project.id,
        "terminal-business-failed",
        &user.id,
        "node-terminal",
        "failed",
        Some("构建测试失败")
    ));
    assert!(finish_session(
        &store,
        &project.id,
        "terminal-business-failed",
        &user.id,
        "node-terminal",
        "done",
        None
    ));
    assert_eq!(
        store
            .get_project_execution_session_by_request_id("terminal-business-failed")
            .unwrap()
            .unwrap()
            .status,
        "failed"
    );

    assert!(start("terminal-communication-failed"));
    assert!(finish_session(
        &store,
        &project.id,
        "terminal-communication-failed",
        &user.id,
        "node-terminal",
        "failed",
        Some("PC节点通信自动恢复超时")
    ));
    assert!(finish_session(
        &store,
        &project.id,
        "terminal-communication-failed",
        &user.id,
        "node-terminal",
        "done",
        None
    ));
    assert_eq!(
        store
            .get_project_execution_session_by_request_id("terminal-communication-failed")
            .unwrap()
            .unwrap()
            .status,
        "done"
    );

    assert!(!finish_session(
        &store,
        &project.id,
        "terminal-done",
        &user.id,
        "different-node",
        "failed",
        Some("identity mismatch")
    ));
}
