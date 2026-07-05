// server/src/store/task_recovery_tests.rs

use crate::store::Store;
use std::collections::HashSet;
use uuid::Uuid;

fn temp_store() -> Store {
    let path = std::env::temp_dir().join(format!(
        "elon_task_recovery_test_{}.db",
        Uuid::new_v4().simple()
    ));
    Store::open(&path).expect("store should open")
}

fn temp_channel_task(store: &Store, account: &str) -> (String, String, String, String) {
    let user = store
        .create_user(account, "secret1", None, None)
        .expect("user should be created");
    let project = store
        .create_project(&user.id, "Channel Task Recovery", None, None)
        .expect("project should be created")
        .project;
    let channel = store
        .list_project_space_channels(&user.id, &project.id)
        .expect("channels should list")
        .into_iter()
        .find(|channel| channel.kind == "ai_development")
        .expect("ai development channel should exist");
    let task_id = store
        .create_task(&project.id, &user.id, Some("channel-dev"), "修复任务")
        .expect("task should be created");
    store
        .insert_project_channel_message(
            &project.id,
            &channel.id,
            Some(&user.id),
            "ai_task",
            "发起 AI 开发任务：修复任务",
            Some(&task_id),
            None,
        )
        .expect("ai task message should insert");

    (user.id, project.id, channel.id, task_id)
}

fn result_messages_for_task(
    store: &Store,
    user_id: &str,
    project_id: &str,
    channel_id: &str,
    task_id: &str,
) -> Vec<String> {
    store
        .list_project_channel_messages(user_id, project_id, channel_id, 50)
        .expect("messages should list")
        .into_iter()
        .filter(|message| {
            message.kind == "ai_result" && message.task_id.as_deref() == Some(task_id)
        })
        .map(|message| message.content)
        .collect()
}

fn progress_messages_for_task(
    store: &Store,
    user_id: &str,
    project_id: &str,
    channel_id: &str,
    task_id: &str,
) -> Vec<String> {
    store
        .list_project_channel_messages(user_id, project_id, channel_id, 50)
        .expect("messages should list")
        .into_iter()
        .filter(|message| {
            message.kind == "ai_progress" && message.task_id.as_deref() == Some(task_id)
        })
        .map(|message| message.content)
        .collect()
}

fn task_status_and_error(store: &Store, task_id: &str) -> (String, Option<String>) {
    store
        .conn()
        .expect("store lock should be healthy")
        .query_row(
            "SELECT status, error FROM tasks WHERE id = ?1",
            rusqlite::params![task_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("task should exist")
}

#[test]
fn running_channel_task_enters_recovering_once_after_server_restart() {
    let store = temp_store();
    let (user_id, project_id, channel_id, task_id) =
        temp_channel_task(&store, "interrupted-channel-task@example.com");

    let changed = store
        .mark_recovering_running_tasks_after_server_restart()
        .expect("running tasks should enter recovery");
    assert_eq!(changed, 1);
    let (status, error) = task_status_and_error(&store, &task_id);
    assert_eq!(status, "recovering");
    assert_eq!(error.as_deref(), Some("server update recovery pending"));

    let results = result_messages_for_task(&store, &user_id, &project_id, &channel_id, &task_id);
    assert!(results.is_empty());
    let progress = progress_messages_for_task(&store, &user_id, &project_id, &channel_id, &task_id);
    assert_eq!(progress.len(), 1);
    assert!(progress[0].contains("\"phase\":\"server_updating\""));
    assert!(progress[0].contains("服务器正在更新升级"));
    assert!(progress[0].contains("会自动恢复"));

    let changed_again = store
        .mark_recovering_running_tasks_after_server_restart()
        .expect("second pass should be safe");
    assert_eq!(changed_again, 0);
    let progress = progress_messages_for_task(&store, &user_id, &project_id, &channel_id, &task_id);
    assert_eq!(progress.len(), 1);
}

#[test]
fn channel_messages_include_persisted_task_recovering_state() {
    let store = temp_store();
    let (user_id, project_id, channel_id, task_id) =
        temp_channel_task(&store, "channel-message-task-state@example.com");

    store
        .mark_recovering_running_tasks_after_server_restart()
        .expect("running task should enter recovery");

    let messages = store
        .list_project_channel_messages(&user_id, &project_id, &channel_id, 50)
        .expect("messages should list");
    let task_message = messages
        .iter()
        .find(|message| {
            message.kind == "ai_task" && message.task_id.as_deref() == Some(task_id.as_str())
        })
        .expect("task message should exist");
    assert_eq!(task_message.task_status.as_deref(), Some("recovering"));
    assert_eq!(
        task_message.task_error.as_deref(),
        Some("server update recovery pending")
    );
}

#[test]
fn stale_running_channel_task_gets_terminal_result_once() {
    let store = temp_store();
    let (user_id, project_id, channel_id, task_id) =
        temp_channel_task(&store, "stale-channel-task@example.com");
    let old_created_at = (chrono::Utc::now() - chrono::Duration::minutes(20)).to_rfc3339();
    store
        .conn()
        .expect("store lock should be healthy")
        .execute(
            "UPDATE tasks SET created_at = ?1 WHERE id = ?2",
            rusqlite::params![old_created_at, task_id],
        )
        .expect("task should become stale");

    let changed = store
        .mark_stale_running_tasks_with_channel_results(10 * 60)
        .expect("stale tasks should fail");
    assert_eq!(changed, 1);
    let (status, error) = task_status_and_error(&store, &task_id);
    assert_eq!(status, "failed");
    assert_eq!(error.as_deref(), Some("PC节点通信自动恢复超时"));

    let results = result_messages_for_task(&store, &user_id, &project_id, &channel_id, &task_id);
    assert_eq!(results.len(), 1);
    assert!(results[0].contains("通信自动恢复超时"));
    assert!(results[0].contains("继续"));

    let changed_again = store
        .mark_stale_running_tasks_with_channel_results(10 * 60)
        .expect("second pass should be safe");
    assert_eq!(changed_again, 0);
    let results = result_messages_for_task(&store, &user_id, &project_id, &channel_id, &task_id);
    assert_eq!(results.len(), 1);
}

#[test]
fn stale_running_channel_task_can_be_excluded_from_cleanup() {
    let store = temp_store();
    let (user_id, project_id, channel_id, task_id) =
        temp_channel_task(&store, "active-channel-task@example.com");
    let old_created_at = (chrono::Utc::now() - chrono::Duration::minutes(20)).to_rfc3339();
    store
        .conn()
        .expect("store lock should be healthy")
        .execute(
            "UPDATE tasks SET created_at = ?1 WHERE id = ?2",
            rusqlite::params![old_created_at, task_id],
        )
        .expect("task should become stale");

    let changed = store
        .mark_stale_running_tasks_with_channel_results_excluding(
            10 * 60,
            std::slice::from_ref(&task_id),
        )
        .expect("active task should be excluded");
    assert_eq!(changed, 0);
    let (status, error) = task_status_and_error(&store, &task_id);
    assert_eq!(status, "running");
    assert!(error.is_none());
    let results = result_messages_for_task(&store, &user_id, &project_id, &channel_id, &task_id);
    assert!(results.is_empty());

    let changed = store
        .mark_stale_running_tasks_with_channel_results(10 * 60)
        .expect("non-excluded stale task should fail");
    assert_eq!(changed, 1);
}

#[test]
fn stale_recovering_channel_task_gets_recovery_failure_result() {
    let store = temp_store();
    let (user_id, project_id, channel_id, task_id) =
        temp_channel_task(&store, "stale-recovering-channel-task@example.com");

    store
        .mark_recovering_running_tasks_after_server_restart()
        .expect("running task should enter recovery");
    let recent_changed = store
        .mark_stale_running_tasks_with_channel_results(10 * 60)
        .expect("fresh recovery should not fail immediately");
    assert_eq!(recent_changed, 0);
    let (status, _) = task_status_and_error(&store, &task_id);
    assert_eq!(status, "recovering");

    let old_updated_at = (chrono::Utc::now() - chrono::Duration::minutes(20)).to_rfc3339();
    store
        .conn()
        .expect("store lock should be healthy")
        .execute(
            "UPDATE tasks SET updated_at = ?1 WHERE id = ?2",
            rusqlite::params![old_updated_at, task_id],
        )
        .expect("recovery should become stale");

    let changed = store
        .mark_stale_running_tasks_with_channel_results(10 * 60)
        .expect("stale recovering task should fail");
    assert_eq!(changed, 1);
    let (status, error) = task_status_and_error(&store, &task_id);
    assert_eq!(status, "failed");
    assert_eq!(error.as_deref(), Some("server update recovery timed out"));

    let results = result_messages_for_task(&store, &user_id, &project_id, &channel_id, &task_id);
    assert_eq!(results.len(), 1);
    assert!(results[0].contains("恢复失败"));
    assert!(results[0].contains("更新升级"));
    assert!(results[0].contains("继续"));
}

#[test]
fn stale_cleanup_bulk_pressure_keeps_terminal_results_idempotent() {
    let store = temp_store();
    let old_created_at = (chrono::Utc::now() - chrono::Duration::minutes(20)).to_rfc3339();
    let mut tasks = Vec::new();

    for index in 0..32 {
        let (user_id, project_id, channel_id, task_id) =
            temp_channel_task(&store, &format!("bulk-stale-{index}@example.com"));
        store
            .conn()
            .expect("store lock should be healthy")
            .execute(
                "UPDATE tasks SET created_at = ?1 WHERE id = ?2",
                rusqlite::params![old_created_at, task_id],
            )
            .expect("task should become stale");
        tasks.push((user_id, project_id, channel_id, task_id));
    }

    let excluded_task_ids = tasks
        .iter()
        .enumerate()
        .filter(|(index, _)| index % 7 == 0)
        .map(|(_, (_, _, _, task_id))| task_id.clone())
        .collect::<Vec<_>>();
    let excluded = excluded_task_ids.iter().cloned().collect::<HashSet<_>>();

    let changed = store
        .mark_stale_running_tasks_with_channel_results_excluding(10 * 60, &excluded_task_ids)
        .expect("bulk stale cleanup should succeed");
    assert_eq!(changed, tasks.len() - excluded_task_ids.len());

    for (user_id, project_id, channel_id, task_id) in &tasks {
        let (status, error) = task_status_and_error(&store, task_id);
        let results = result_messages_for_task(&store, user_id, project_id, channel_id, task_id);
        if excluded.contains(task_id) {
            assert_eq!(status, "running");
            assert!(error.is_none());
            assert!(results.is_empty());
        } else {
            assert_eq!(status, "failed");
            assert_eq!(error.as_deref(), Some("PC节点通信自动恢复超时"));
            assert_eq!(results.len(), 1);
            assert!(results[0].contains("通信自动恢复超时"));
        }
    }

    let changed_again = store
        .mark_stale_running_tasks_with_channel_results_excluding(10 * 60, &excluded_task_ids)
        .expect("second bulk pass should be idempotent");
    assert_eq!(changed_again, 0);
    for (user_id, project_id, channel_id, task_id) in &tasks {
        let results = result_messages_for_task(&store, user_id, project_id, channel_id, task_id);
        assert_eq!(
            results.len(),
            if excluded.contains(task_id) { 0 } else { 1 }
        );
    }

    let changed_without_exclusion = store
        .mark_stale_running_tasks_with_channel_results(10 * 60)
        .expect("excluded stale tasks should be cleanable later");
    assert_eq!(changed_without_exclusion, excluded_task_ids.len());
    for (user_id, project_id, channel_id, task_id) in &tasks {
        let (status, _) = task_status_and_error(&store, task_id);
        let results = result_messages_for_task(&store, user_id, project_id, channel_id, task_id);
        assert_eq!(status, "failed");
        assert_eq!(results.len(), 1);
    }
}

#[test]
fn interrupted_channel_task_with_existing_result_is_not_duplicated() {
    let store = temp_store();
    let (user_id, project_id, channel_id, task_id) =
        temp_channel_task(&store, "interrupted-existing-result@example.com");
    assert!(store
        .insert_project_channel_ai_result_once(&project_id, &channel_id, "已有终态结果", &task_id)
        .expect("existing result should insert"));

    let changed = store
        .mark_recovering_running_tasks_after_server_restart()
        .expect("running task should enter recovery");
    assert_eq!(changed, 1);
    let (status, _) = task_status_and_error(&store, &task_id);
    assert_eq!(status, "recovering");
    let results = result_messages_for_task(&store, &user_id, &project_id, &channel_id, &task_id);
    assert_eq!(results, vec!["已有终态结果".to_string()]);
}

#[test]
fn recovering_task_can_be_finished_by_late_runner() {
    let store = temp_store();
    let (_, _, _, task_id) = temp_channel_task(&store, "late-runner-finish@example.com");

    store
        .mark_recovering_running_tasks_after_server_restart()
        .expect("running task should enter recovery");

    let changed = store
        .finish_running_task(&task_id, "done", Some("迟到的完成消息"), None, None)
        .expect("late finish should complete recovering task");
    assert!(changed);

    let (status, error) = task_status_and_error(&store, &task_id);
    assert_eq!(status, "done");
    assert!(error.is_none());
}

#[test]
fn stale_channel_task_with_existing_result_is_not_duplicated() {
    let store = temp_store();
    let (user_id, project_id, channel_id, task_id) =
        temp_channel_task(&store, "stale-existing-result@example.com");
    let old_created_at = (chrono::Utc::now() - chrono::Duration::minutes(20)).to_rfc3339();
    store
        .conn()
        .expect("store lock should be healthy")
        .execute(
            "UPDATE tasks SET created_at = ?1 WHERE id = ?2",
            rusqlite::params![old_created_at, task_id],
        )
        .expect("task should become stale");
    assert!(store
        .insert_project_channel_ai_result_once(&project_id, &channel_id, "已有失败结果", &task_id)
        .expect("existing result should insert"));

    let changed = store
        .mark_stale_running_tasks_with_channel_results(10 * 60)
        .expect("stale task should fail");
    assert_eq!(changed, 1);
    let (status, _) = task_status_and_error(&store, &task_id);
    assert_eq!(status, "failed");
    let results = result_messages_for_task(&store, &user_id, &project_id, &channel_id, &task_id);
    assert_eq!(results, vec!["已有失败结果".to_string()]);
}

#[test]
fn fresh_running_channel_task_is_not_stale_cleaned() {
    let store = temp_store();
    let (user_id, project_id, channel_id, task_id) =
        temp_channel_task(&store, "fresh-channel-task@example.com");

    let changed = store
        .mark_stale_running_tasks_with_channel_results(10 * 60)
        .expect("fresh task should remain running");
    assert_eq!(changed, 0);
    let (status, error) = task_status_and_error(&store, &task_id);
    assert_eq!(status, "running");
    assert!(error.is_none());
    let results = result_messages_for_task(&store, &user_id, &project_id, &channel_id, &task_id);
    assert!(results.is_empty());
}

#[test]
fn channel_ai_result_is_inserted_once_per_task() {
    let store = temp_store();
    let (user_id, project_id, channel_id, task_id) =
        temp_channel_task(&store, "ai-result-once@example.com");

    let inserted = store
        .insert_project_channel_ai_result_once(&project_id, &channel_id, "第一次结果", &task_id)
        .expect("first result should insert");
    let inserted_again = store
        .insert_project_channel_ai_result_once(&project_id, &channel_id, "第二次结果", &task_id)
        .expect("second result should be ignored");

    assert!(inserted);
    assert!(!inserted_again);
    let results = result_messages_for_task(&store, &user_id, &project_id, &channel_id, &task_id);
    assert_eq!(results, vec!["第一次结果".to_string()]);
}
