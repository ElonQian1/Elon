use std::path::PathBuf;

use super::{CreateUiTunerContextArtifact, Store};

fn temp_store() -> Store {
    let path = std::env::temp_dir().join(format!(
        "elon_project_module_test_{}_{}.db",
        std::process::id(),
        uuid::Uuid::new_v4()
    ));
    Store::open(&PathBuf::from(path)).expect("store should open")
}

fn setup() -> (Store, String, String) {
    let store = temp_store();
    let user = store
        .create_user("ui-tuner@example.com", "secret1", Some("UI Tuner"), None)
        .expect("user should be created");
    let project = store
        .create_project(&user.id, "UI Tuner Project", None, None)
        .expect("project should be created")
        .project;
    (store, user.id, project.id)
}

#[test]
fn ui_tuner_workspace_has_user_scoped_main_session_and_seed_memories() {
    let (store, user_id, project_id) = setup();
    let bundle = store
        .ensure_ui_tuner_workspace(&project_id, &user_id)
        .expect("workspace should be created");

    assert!(bundle
        .workspace
        .canonical_conversation_id
        .starts_with("ui-tuner-main-"));
    assert_eq!(
        bundle.workspace.active_conversation_id,
        bundle.workspace.canonical_conversation_id
    );
    assert_eq!(bundle.sessions.len(), 1);
    assert!(bundle.sessions[0].is_canonical);
    assert!(
        bundle
            .memories
            .iter()
            .filter(|memory| memory.status == "accepted")
            .count()
            >= 4
    );
}

#[test]
fn ui_tuner_workspace_recovers_latest_legacy_server_conversation() {
    let (store, user_id, project_id) = setup();
    store
        .ensure_conversation(
            &project_id,
            &user_id,
            Some("ui-tuner-legacy-session"),
            Some("微调画布 · 历史会话"),
        )
        .expect("legacy conversation should exist");

    let bundle = store
        .ensure_ui_tuner_workspace(&project_id, &user_id)
        .expect("workspace should recover");

    assert_eq!(
        bundle.workspace.active_conversation_id,
        "ui-tuner-legacy-session"
    );
    assert!(bundle
        .sessions
        .iter()
        .any(|session| session.conversation_id == "ui-tuner-legacy-session"));
}

#[test]
fn ui_tuner_workspace_imports_legacy_browser_memory_once() {
    let (store, user_id, project_id) = setup();
    let imported = store
        .import_legacy_ui_tuner_memory(
            &project_id,
            &user_id,
            "旧版长期目标",
            &["继续复用同一个项目会话".to_string()],
            &["间距使用统一 token".to_string()],
            &["低置信度节点待确认".to_string()],
        )
        .expect("legacy memory should import");

    assert_eq!(imported.workspace.memory_revision, 2);
    assert_eq!(imported.workspace.stable_summary, "旧版长期目标");
    assert!(imported.memories.iter().any(|memory| {
        memory.status == "accepted" && memory.content == "继续复用同一个项目会话"
    }));
    assert!(imported.memories.iter().any(|memory| {
        memory.status == "candidate" && memory.content == "低置信度节点待确认"
    }));
}

#[test]
fn ui_tuner_completion_creates_checkpoint_candidate_and_real_fork_point() {
    let (store, user_id, project_id) = setup();
    let bundle = store
        .ensure_ui_tuner_workspace(&project_id, &user_id)
        .expect("workspace should be created");
    let conversation_id = bundle.workspace.canonical_conversation_id;
    let payload =
        r#"{"version":1,"kind":"elon_ui_tuner_codex_context","selectedElement":{"name":"保存"}}"#;
    let artifact = store
        .create_ui_tuner_context_artifact(
            &project_id,
            &user_id,
            CreateUiTunerContextArtifact {
                conversation_id: &conversation_id,
                schema_version: "elon.ui_tuner.context.v1",
                payload_json: payload,
                payload_sha256: "abc123",
                selected_element_name: Some("保存"),
                resource_id: Some("com.elon:id/save"),
                source_file: Some("app/src/main/res/layout/page.xml"),
                user_intent: "把保存按钮统一成主按钮标准",
            },
        )
        .expect("artifact should be created");
    let task_id = store
        .create_task(
            &project_id,
            &user_id,
            Some(&conversation_id),
            "把保存按钮统一成主按钮标准",
        )
        .expect("task should be created");
    store
        .bind_ui_tuner_task(
            &project_id,
            &user_id,
            &conversation_id,
            &artifact.id,
            &task_id,
        )
        .expect("task should bind");
    assert!(store
        .finish_running_task(
            &task_id,
            "done",
            Some("已修改 components.json，并完成真机复采。"),
            None,
            None,
        )
        .expect("task should finish"));
    assert!(store
        .record_ui_tuner_task_completion(
            &task_id,
            "done",
            "已修改 components.json，并完成真机复采。"
        )
        .expect("completion should write back"));

    let next = store
        .ui_tuner_workspace_bundle(&project_id, &user_id)
        .expect("workspace should reload");
    assert_eq!(next.workspace.memory_revision, 2);
    assert!(next.workspace.last_checkpoint_id.is_some());
    assert!(next
        .latest_checkpoint
        .as_ref()
        .is_some_and(|item| item.status == "done"));
    let candidate = next
        .memories
        .iter()
        .find(|memory| memory.status == "candidate")
        .expect("candidate memory should be created");
    assert_eq!(candidate.content, "把保存按钮统一成主按钮标准");

    let (checkpoint_id, message_id) = store
        .latest_ui_tuner_fork_point(&project_id, &user_id, &conversation_id)
        .expect("fork point should load")
        .expect("fork point should exist");
    assert_eq!(checkpoint_id, next.workspace.last_checkpoint_id);
    let fork = store
        .fork_conversation_at_message(
            &project_id,
            &user_id,
            &conversation_id,
            &message_id,
            Some("ui-tuner-test-fork"),
            Some("微调画布 · 保存按钮分叉"),
        )
        .expect("conversation should fork");
    let session = store
        .register_ui_tuner_fork(
            &project_id,
            &user_id,
            &fork.conversation_id,
            "微调画布 · 保存按钮分叉",
            &conversation_id,
            Some(&message_id),
            checkpoint_id.as_deref(),
            Some("保存"),
        )
        .expect("fork should be indexed");
    assert_eq!(
        session.source_message_id.as_deref(),
        Some(message_id.as_str())
    );
    assert_eq!(session.source_checkpoint_id, checkpoint_id);
    let branch_checkpoint = store
        .ui_tuner_context_checkpoint(&project_id, &user_id, &session.conversation_id)
        .expect("branch context checkpoint should load")
        .expect("branch should inherit source checkpoint");
    assert_eq!(Some(branch_checkpoint.id), checkpoint_id);
}

#[test]
fn ui_tuner_memory_candidate_requires_explicit_review() {
    let (store, user_id, project_id) = setup();
    let bundle = store
        .ensure_ui_tuner_workspace(&project_id, &user_id)
        .expect("workspace should be created");
    let conversation_id = bundle.workspace.canonical_conversation_id;
    let artifact = store
        .create_ui_tuner_context_artifact(
            &project_id,
            &user_id,
            CreateUiTunerContextArtifact {
                conversation_id: &conversation_id,
                schema_version: "elon.ui_tuner.context.v1",
                payload_json: r#"{"version":1,"kind":"elon_ui_tuner_codex_context"}"#,
                payload_sha256: "def456",
                selected_element_name: None,
                resource_id: None,
                source_file: None,
                user_intent: "列表间距统一为 12dp",
            },
        )
        .unwrap();
    let task_id = store
        .create_task(
            &project_id,
            &user_id,
            Some(&conversation_id),
            "列表间距统一为 12dp",
        )
        .unwrap();
    store
        .bind_ui_tuner_task(
            &project_id,
            &user_id,
            &conversation_id,
            &artifact.id,
            &task_id,
        )
        .unwrap();
    store
        .finish_running_task(&task_id, "done", Some("完成"), None, None)
        .unwrap();
    store
        .record_ui_tuner_task_completion(&task_id, "done", "完成")
        .unwrap();
    let memory = store
        .list_ui_tuner_memories(&project_id, &user_id)
        .unwrap()
        .into_iter()
        .find(|item| item.status == "candidate")
        .unwrap();
    let reviewed = store
        .review_ui_tuner_memory(&project_id, &user_id, &memory.id, "accepted", "user")
        .expect("candidate should be accepted");
    assert_eq!(reviewed.status, "accepted");
    assert_eq!(reviewed.scope_type, "user");
}
