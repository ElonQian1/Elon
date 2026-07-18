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
    node_agent_cli_sidecar_io::{read_new_output_records, CliSidecarOutputRecord},
    node_agent_cli_sidecar_runner::CliSidecarReplayCursor,
    node_agent_task_journal::TaskJournal,
    node_agent_task_journal_events::cli_chunk_event,
    node_agent_task_journal_lock::with_task_journal_io_lock,
};

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
            let Some(mut event) = cli_chunk_event(
                task_id,
                record.stream.as_deref().unwrap_or("stdout"),
                record.text.as_deref().unwrap_or_default(),
                record.at_ms,
            ) else {
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
    fn partial_journal_prefix_keeps_later_final_reply_from_full_sidecar() {
        let temp = test_root("partial-final");
        let journal = TaskJournal::new(temp.join("journal"));
        let first = CliSidecarOutputRecord::chunk("stdout", "working\n");
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
            CliSidecarOutputRecord::chunk("stdout", "final reply\n"),
        )
        .unwrap();

        let (stdout, stderr) =
            recovered_completion_output(&journal, "task-1", &output, 200_000).unwrap();

        assert_eq!(stdout, "working\nfinal reply\n");
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
