//! Durable replay primitives shared by both sidecar recovery entry points.
//!
//! A replay batch is committed to the task journal before either the sidecar
//! registry or the update receipt may advance its cursor. Replaying the same
//! batch is idempotent by `(session_id, sequence)`.

use std::{
    collections::BTreeSet,
    fs::{File, OpenOptions},
    io::{BufRead, BufReader},
    path::Path,
};

use anyhow::{Context, Result};
use serde_json::Value;

use crate::{
    node_agent_cli_output_aggregate::{codex_json_event_at, progress_observation},
    node_agent_cli_sidecar_io::{read_new_output_records, CliSidecarOutputRecord},
    node_agent_cli_sidecar_runner::CliSidecarReplayCursor,
    node_agent_local_task_store::LocalTaskStore,
    node_agent_task_journal::TaskJournal,
    node_agent_task_journal_events::cli_chunk_event,
    node_agent_task_journal_lock::with_task_journal_io_lock,
};

pub(crate) fn record_receipt_resumed(
    journal: &TaskJournal,
    local_tasks: &LocalTaskStore,
    task_id: &str,
) -> Result<bool> {
    record_recovery_running(
        journal,
        local_tasks,
        task_id,
        "reasoning",
        None,
        "update_receipt_resumed",
    )
}

pub(crate) fn record_replayed_activity(
    journal: &TaskJournal,
    local_tasks: &LocalTaskStore,
    task_id: &str,
    records: &[CliSidecarOutputRecord],
) -> Result<bool> {
    let mut progress = None;
    let mut heartbeat = false;
    for record in records {
        if record.record_type == "chunk" {
            let observation = progress_observation(record.text.as_deref().unwrap_or_default());
            if observation.progress {
                progress = Some(observation);
            }
        } else if record.record_type == "runtime"
            && record
                .runtime
                .as_ref()
                .and_then(|value| value.get("heartbeat"))
                .and_then(Value::as_bool)
                == Some(true)
        {
            heartbeat = true;
        }
    }
    if let Some(progress) = progress {
        return record_recovery_running(
            journal,
            local_tasks,
            task_id,
            progress.phase.as_deref().unwrap_or("reasoning"),
            progress.current_command.as_deref(),
            "sidecar_output_replayed",
        );
    }
    if heartbeat {
        journal.record_runtime_heartbeat(task_id)?;
    }
    Ok(false)
}

fn record_recovery_running(
    journal: &TaskJournal,
    local_tasks: &LocalTaskStore,
    task_id: &str,
    phase: &str,
    current_command: Option<&str>,
    reason: &str,
) -> Result<bool> {
    if !journal.record_recovery_running(task_id, phase, current_command, reason)? {
        return Ok(false);
    }
    let _ = local_tasks.mark_recovery_running(task_id)?;
    Ok(true)
}

pub(crate) fn persist_batch_before_cursor(
    journal: &TaskJournal,
    task_id: &str,
    session_id: &str,
    records: &[CliSidecarOutputRecord],
    cursor: CliSidecarReplayCursor,
    mut commit_cursor: impl FnMut(CliSidecarReplayCursor) -> Result<()>,
) -> Result<()> {
    let first_sequence = cursor.sequence.saturating_sub(records.len() as u64);
    with_task_journal_io_lock(|| {
        let mut persisted = persisted_sequences(journal, task_id, session_id)?;
        for (index, record) in records.iter().enumerate() {
            let sequence = first_sequence.saturating_add(index as u64 + 1);
            if record.record_type != "chunk" || persisted.contains(&sequence) {
                continue;
            }
            let stream = record.stream.as_deref().unwrap_or("stdout");
            let text = record.text.as_deref().unwrap_or_default();
            let event = codex_json_event_at(task_id, stream, text, record.at_ms)
                .map(|(event, _)| event)
                .or_else(|| cli_chunk_event(task_id, stream, text, record.at_ms));
            let Some(mut event) = event else {
                continue;
            };
            event["sidecar_session_id"] = Value::String(session_id.to_string());
            event["sidecar_sequence"] = Value::Number(sequence.into());
            journal.append_event(event)?;
            persisted.insert(sequence);
        }
        if journal.events_path().exists() {
            OpenOptions::new()
                .read(true)
                .write(true)
                .open(journal.events_path())
                .with_context(|| format!("open persisted journal for {task_id}"))?
                .sync_all()
                .with_context(|| format!("persist sidecar journal batch for {task_id}"))?;
        }
        Ok::<(), anyhow::Error>(())
    })?;
    commit_cursor(cursor)
}

pub(crate) fn recovered_completion_output(
    journal: &TaskJournal,
    task_id: &str,
    sidecar_output_path: &Path,
    max_chars: usize,
) -> Result<(String, String)> {
    let journal_output = journal.completion_output(task_id, max_chars)?;
    let mut offset = 0;
    let records = read_new_output_records(sidecar_output_path, &mut offset)?;
    let mut stdout = String::new();
    let mut stderr = String::new();
    for record in records {
        if record.record_type != "chunk" {
            continue;
        }
        let text = record.text.unwrap_or_default();
        match record.stream.as_deref() {
            Some("stderr") => stderr.push_str(&text),
            Some("stdout") | Some("pty") | Some("runtime") => stdout.push_str(&text),
            _ => {}
        }
    }
    Ok((merge_journal_sidecar(&journal_output, &stdout), stderr))
}

fn persisted_sequences(
    journal: &TaskJournal,
    task_id: &str,
    session_id: &str,
) -> Result<BTreeSet<u64>> {
    let path = journal.events_path();
    if !path.exists() {
        return Ok(BTreeSet::new());
    }
    let file = File::open(&path).with_context(|| format!("read journal {:?}", path))?;
    let mut sequences = BTreeSet::new();
    for line in BufReader::new(file).lines() {
        let line = line.with_context(|| format!("read journal {:?}", path))?;
        let Ok(event) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        if event.get("req_id").and_then(Value::as_str) == Some(task_id)
            && event.get("sidecar_session_id").and_then(Value::as_str) == Some(session_id)
        {
            if let Some(sequence) = event.get("sidecar_sequence").and_then(Value::as_u64) {
                sequences.insert(sequence);
            }
        }
    }
    Ok(sequences)
}

fn merge_journal_sidecar(journal: &str, sidecar: &str) -> String {
    if journal.trim().is_empty() {
        return sidecar.to_string();
    }
    if sidecar.is_empty() || journal.contains(sidecar) {
        return journal.to_string();
    }
    let journal_prefix = journal.trim_end_matches(['\r', '\n']);
    if sidecar.starts_with(journal) || sidecar.starts_with(journal_prefix) {
        return sidecar.to_string();
    }
    if journal_prefix_matches_sidecar(journal_prefix, sidecar) {
        return sidecar.to_string();
    }
    let max_overlap = journal_prefix.len().min(sidecar.len());
    for bytes in (1..=max_overlap).rev() {
        if journal_prefix.is_char_boundary(journal_prefix.len() - bytes)
            && sidecar.is_char_boundary(bytes)
            && journal_prefix[journal_prefix.len() - bytes..] == sidecar[..bytes]
        {
            return format!(
                "{}{sidecar}",
                &journal_prefix[..journal_prefix.len() - bytes]
            );
        }
    }
    let separator = if journal.ends_with('\n') { "" } else { "\n" };
    format!("{journal}{separator}{sidecar}")
}

fn journal_prefix_matches_sidecar(journal: &str, sidecar: &str) -> bool {
    let mut sidecar = sidecar.chars();
    let mut current = sidecar.next();
    for journal_char in journal.chars() {
        if matches!(journal_char, '\r' | '\n') && current != Some(journal_char) {
            continue;
        }
        if current != Some(journal_char) {
            return false;
        }
        current = sidecar.next();
    }
    true
}

#[cfg(test)]
mod tests {
    use std::{cell::Cell, fs, path::PathBuf};
    use uuid::Uuid;

    use super::*;
    use crate::node_agent_cli_sidecar_io::append_output;

    #[test]
    fn append_failure_never_advances_cursor() {
        let temp = test_root("append-failure");
        fs::create_dir_all(&temp).unwrap();
        let blocked = temp.join("journal");
        fs::write(&blocked, b"not-a-directory").unwrap();
        let journal = TaskJournal::new(&blocked);
        let advanced = Cell::new(false);
        let records = vec![CliSidecarOutputRecord::chunk("stdout", "partial")];

        let result = persist_batch_before_cursor(
            &journal,
            "task-1",
            "session-1",
            &records,
            CliSidecarReplayCursor {
                offset: 10,
                sequence: 1,
            },
            |_| {
                advanced.set(true);
                Ok(())
            },
        );

        assert!(result.is_err());
        assert!(!advanced.get());
        let _ = fs::remove_dir_all(temp);
    }

    #[test]
    fn replay_is_idempotent_and_cursor_follows_persistence() {
        let temp = test_root("idempotent");
        let journal = TaskJournal::new(temp.join("journal"));
        let records = vec![CliSidecarOutputRecord::chunk("stdout", "partial\n")];
        let cursor = CliSidecarReplayCursor {
            offset: 10,
            sequence: 1,
        };
        for _ in 0..2 {
            persist_batch_before_cursor(&journal, "task-1", "session-1", &records, cursor, |_| {
                Ok(())
            })
            .unwrap();
        }

        let events = fs::read_to_string(journal.events_path()).unwrap();
        assert_eq!(events.matches("sidecar_sequence").count(), 1);
        assert!(events.contains("partial"));
        let _ = fs::remove_dir_all(temp);
    }

    #[test]
    fn replayed_chunk_persists_before_cursor_and_exposes_running_runtime() {
        let temp = test_root("running-visible");
        let journal = TaskJournal::new(temp.join("journal"));
        let local_tasks = LocalTaskStore::new(temp.join("local-tasks.sqlite3"));
        local_tasks
            .create(crate::node_agent_local_task_store::LocalTaskStart {
                task_id: "task-1",
                owner_user_id: "owner-a",
                agent_id: "agent-a",
                install_id: "install-a",
                project_id: "project-a",
                channel_id: None,
                conversation_id: "conversation-a",
                workspace_path: "D:/demo",
                prompt: "continue",
                cli: "codex",
                runtime_permission: "full_access",
            })
            .unwrap();
        local_tasks
            .mark_recovering("task-1", "节点更新完成，正在重接原 CLI 会话")
            .unwrap();
        journal
            .record_started(crate::node_agent_task_journal::TaskJournalStart {
                req_id: "task-1",
                cli_name: "codex",
                route: Some("route_a_external_cli"),
                run_handle_id: Some("task-1"),
                cwd: Some("D:/demo"),
                runtime_permission: Some("full_access"),
            })
            .unwrap();
        let records = vec![CliSidecarOutputRecord::chunk(
            "stdout",
            r#"{"type":"item.started","item":{"type":"command_execution","command":"cargo test --bin elon-pc-node"}}"#,
        )];
        let cursor_observed = Cell::new(false);

        persist_batch_before_cursor(
            &journal,
            "task-1",
            "session-1",
            &records,
            CliSidecarReplayCursor {
                offset: 20,
                sequence: 1,
            },
            |_| {
                record_replayed_activity(&journal, &local_tasks, "task-1", &records)?;
                let snapshot = journal.snapshot("task-1", 0, 20)?;
                assert!(snapshot.events.iter().any(|event| {
                    event.event["sidecar_sequence"] == 1
                        && event.event["sidecar_session_id"] == "session-1"
                }));
                assert_eq!(snapshot.record.as_ref().unwrap().phase, "verification");
                let task = local_tasks.get("task-1")?.unwrap();
                assert_eq!(task.status, "running");
                assert!(task.error.is_none());
                cursor_observed.set(true);
                Ok(())
            },
        )
        .unwrap();

        assert!(cursor_observed.get());
        let _ = fs::remove_dir_all(temp);
    }

    #[test]
    fn replayed_large_codex_item_stays_structured_and_parseable() {
        let temp = test_root("structured-large-item");
        let journal = TaskJournal::new(temp.join("journal"));
        let raw = serde_json::json!({
            "type":"item.completed",
            "item":{"id":"large","type":"command_execution","status":"completed","exit_code":0,
                "command":"cargo test","aggregated_output":"x".repeat(40_000)}
        })
        .to_string();
        let records = vec![CliSidecarOutputRecord::chunk("stdout", &raw)];
        persist_batch_before_cursor(
            &journal,
            "task-1",
            "session-1",
            &records,
            CliSidecarReplayCursor {
                offset: raw.len() as u64,
                sequence: 1,
            },
            |_| Ok(()),
        )
        .unwrap();
        let event = &journal.snapshot("task-1", 0, 10).unwrap().events[0].event;
        assert_eq!(event["type"], "codex_item");
        assert_eq!(event["item"]["output"]["raw_byte_count"], 40_000);
        assert_eq!(event["item"]["output"]["truncated"], true);
        assert!(serde_json::from_str::<serde_json::Value>(&event.to_string()).is_ok());
        let _ = fs::remove_dir_all(temp);
    }

    #[test]
    fn partial_journal_prefix_keeps_later_final_reply_from_full_sidecar() {
        let temp = test_root("partial-final");
        let journal = TaskJournal::new(temp.join("journal"));
        let first = CliSidecarOutputRecord::chunk("stdout", "working");
        persist_batch_before_cursor(
            &journal,
            "task-1",
            "session-1",
            std::slice::from_ref(&first),
            CliSidecarReplayCursor {
                offset: 10,
                sequence: 1,
            },
            |_| Ok(()),
        )
        .unwrap();
        let output = temp.join("sidecar.jsonl");
        append_output(&output, first).unwrap();
        append_output(
            &output,
            CliSidecarOutputRecord::chunk("stdout", " final reply\n"),
        )
        .unwrap();

        let (stdout, stderr) =
            recovered_completion_output(&journal, "task-1", &output, 200_000).unwrap();

        assert_eq!(stdout, "working final reply\n");
        assert!(stderr.is_empty());
        let _ = fs::remove_dir_all(temp);
    }

    fn test_root(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "elon-sidecar-replay-{label}-{}",
            Uuid::new_v4().simple()
        ))
    }
}
