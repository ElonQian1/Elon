use std::{
    collections::HashSet,
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

use super::{
    now_ms, protect_for_current_user, sanitize_stored_conversations_with_ttl,
    unprotect_for_current_user, StoredConversationSnapshot,
};

const SCHEMA: &str = "elon.local_ai_web_conversation_snapshot.v1";
const MAX_FILE_BYTES: usize = 2 * 1024 * 1024;
const MAX_PROTECTED_BYTES: usize = MAX_FILE_BYTES * 2;
const MAX_TOTAL_BYTES: u64 = 24 * 1024 * 1024;
const MAX_ENTRIES: usize = 48;
const CACHE_TTL_MS: u64 = 30 * 24 * 60 * 60 * 1_000;

#[derive(Deserialize, Serialize)]
struct ConversationEnvelope {
    schema: String,
    provider_id: String,
    entry: StoredConversationSnapshot,
}

pub(super) fn load(path: &Path, provider_id: &str) -> Vec<StoredConversationSnapshot> {
    if !cfg!(windows) {
        return Vec::new();
    }
    let directory = directory(path);
    let mut values = Vec::new();
    let Ok(files) = fs::read_dir(&directory) else {
        return values;
    };
    for file in files.flatten() {
        let path = file.path();
        if !accepted_cache_file(&path) {
            continue;
        }
        match load_entry(&path, provider_id) {
            Ok(Some(entry)) => values.push(entry),
            Ok(None) | Err(_) => {
                let _ = fs::remove_file(path);
            }
        }
    }
    sanitize_stored_conversations_with_ttl(provider_id, values, CACHE_TTL_MS)
}

pub(super) fn store(
    path: &Path,
    provider_id: &str,
    entries: &[StoredConversationSnapshot],
) -> Result<()> {
    if !cfg!(windows) {
        return Ok(());
    }
    let entries =
        sanitize_stored_conversations_with_ttl(provider_id, entries.to_vec(), CACHE_TTL_MS);
    if entries.is_empty() {
        trim(path, &[]);
        return Ok(());
    }
    let directory = directory(path);
    fs::create_dir_all(&directory).context("create local AI conversation snapshot directory")?;
    for entry in &entries {
        let target = cache_file(&directory, &entry.id);
        if file_is_current(&target, entry.updated_at_ms) {
            continue;
        }
        let envelope = ConversationEnvelope {
            schema: SCHEMA.to_string(),
            provider_id: provider_id.to_string(),
            entry: entry.clone(),
        };
        let plaintext = encode_bounded(envelope)?;
        if plaintext.len() > MAX_FILE_BYTES {
            // The provider envelope still keeps the current bounded message
            // window. An exceptional oversized historical body must not block
            // persistence for other conversations.
            let _ = fs::remove_file(target);
            continue;
        }
        let protected = protect_for_current_user(&plaintext)?;
        if protected.len() > MAX_PROTECTED_BYTES {
            let _ = fs::remove_file(target);
            continue;
        }
        atomic_replace(&target, &protected)?;
    }
    trim(path, &entries);
    Ok(())
}

pub(super) fn clear(path: &Path) {
    let directory = directory(path);
    let Ok(files) = fs::read_dir(&directory) else {
        return;
    };
    for file in files.flatten() {
        let path = file.path();
        if accepted_cache_file(&path) || accepted_temporary_file(&path) {
            let _ = fs::remove_file(path);
        }
    }
    let _ = fs::remove_dir(directory);
}

fn load_entry(path: &Path, provider_id: &str) -> Result<Option<StoredConversationSnapshot>> {
    let protected = fs::read(path).context("read local AI conversation snapshot")?;
    if protected.is_empty() || protected.len() > MAX_PROTECTED_BYTES {
        bail!("local AI conversation snapshot protected size is invalid");
    }
    let plaintext = unprotect_for_current_user(&protected)?;
    if plaintext.is_empty() || plaintext.len() > MAX_FILE_BYTES {
        bail!("local AI conversation snapshot plaintext size is invalid");
    }
    let envelope: ConversationEnvelope =
        serde_json::from_slice(&plaintext).context("decode local AI conversation snapshot")?;
    if envelope.schema != SCHEMA || envelope.provider_id != provider_id {
        bail!("local AI conversation snapshot identity is invalid");
    }
    if now_ms().saturating_sub(envelope.entry.updated_at_ms) > CACHE_TTL_MS {
        return Ok(None);
    }
    Ok(
        sanitize_stored_conversations_with_ttl(provider_id, vec![envelope.entry], CACHE_TTL_MS)
            .pop(),
    )
}

fn encode_bounded(mut envelope: ConversationEnvelope) -> Result<Vec<u8>> {
    loop {
        let plaintext =
            serde_json::to_vec(&envelope).context("encode local AI conversation snapshot")?;
        if plaintext.len() <= MAX_FILE_BYTES
            || !drop_oldest_message(&mut envelope.entry.semantic_event)
        {
            return Ok(plaintext);
        }
    }
}

fn drop_oldest_message(snapshot: &mut serde_json::Value) -> bool {
    let Some(snapshot) = snapshot.as_object_mut() else {
        return false;
    };
    let message_count = snapshot
        .get("messages")
        .and_then(serde_json::Value::as_array)
        .map_or(0, Vec::len);
    if message_count <= 1 {
        return false;
    }
    let observed = snapshot
        .get("observedMessageCount")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(message_count as u64);
    let window_start = snapshot
        .get("messageWindowStart")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or_default();
    snapshot
        .get_mut("messages")
        .and_then(serde_json::Value::as_array_mut)
        .expect("message array was validated")
        .remove(0);
    snapshot.insert(
        "messageWindowStart".to_string(),
        serde_json::Value::from(window_start.saturating_add(1)),
    );
    snapshot.insert(
        "observedMessageCount".to_string(),
        serde_json::Value::from(observed),
    );
    true
}

fn trim(path: &Path, entries: &[StoredConversationSnapshot]) {
    let directory = directory(path);
    let mut retained = HashSet::new();
    let mut total_bytes = 0_u64;
    for entry in entries.iter().take(MAX_ENTRIES) {
        let target = cache_file(&directory, &entry.id);
        let Ok(metadata) = fs::metadata(&target) else {
            continue;
        };
        let next_total = total_bytes.saturating_add(metadata.len());
        if next_total > MAX_TOTAL_BYTES {
            continue;
        }
        total_bytes = next_total;
        retained.insert(target);
    }
    let Ok(files) = fs::read_dir(&directory) else {
        return;
    };
    for file in files.flatten() {
        let path = file.path();
        if accepted_cache_file(&path) && !retained.contains(&path) {
            let _ = fs::remove_file(path);
        } else if accepted_temporary_file(&path) {
            let _ = fs::remove_file(path);
        }
    }
    let _ = fs::remove_dir(directory);
}

fn directory(path: &Path) -> PathBuf {
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("snapshot.dpapi");
    path.with_file_name(format!("{file_name}.conversations"))
}

fn cache_file(directory: &Path, id: &str) -> PathBuf {
    directory.join(format!("conversation-{id}.dpapi"))
}

fn temporary_file(path: &Path) -> PathBuf {
    path.with_extension("dpapi.tmp")
}

fn accepted_cache_file(path: &Path) -> bool {
    path.file_name()
        .and_then(|value| value.to_str())
        .and_then(|value| value.strip_prefix("conversation-"))
        .and_then(|value| value.strip_suffix(".dpapi"))
        .is_some_and(|id| id.len() == 16 && id.bytes().all(|byte| byte.is_ascii_hexdigit()))
}

fn accepted_temporary_file(path: &Path) -> bool {
    path.file_name()
        .and_then(|value| value.to_str())
        .and_then(|value| value.strip_prefix("conversation-"))
        .and_then(|value| value.strip_suffix(".dpapi.tmp"))
        .is_some_and(|id| id.len() == 16 && id.bytes().all(|byte| byte.is_ascii_hexdigit()))
}

fn file_is_current(path: &Path, updated_at_ms: u64) -> bool {
    fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .ok()
        .and_then(system_time_ms)
        .is_some_and(|modified_at_ms| modified_at_ms >= updated_at_ms)
}

fn system_time_ms(value: SystemTime) -> Option<u64> {
    value
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|duration| duration.as_millis() as u64)
}

fn atomic_replace(path: &Path, bytes: &[u8]) -> Result<()> {
    let temporary = temporary_file(path);
    let mut file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(&temporary)
        .context("open local AI conversation snapshot temporary file")?;
    file.write_all(bytes)
        .context("write local AI conversation snapshot temporary file")?;
    file.sync_all()
        .context("flush local AI conversation snapshot temporary file")?;
    drop(file);
    super::replace_file(&temporary, path).inspect_err(|_| {
        let _ = fs::remove_file(&temporary);
    })
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn independent_dpapi_files_restore_rich_conversations_and_ignore_one_corruption() {
        let root = std::env::temp_dir().join(format!(
            "elon-local-ai-conversation-cache-{}-{}",
            std::process::id(),
            now_ms()
        ));
        let main = root.join("chatgpt.dpapi");
        let first = snapshot("0000000000000001", "First", now_ms().saturating_sub(2));
        let second = snapshot("0000000000000002", "Second", now_ms().saturating_sub(1));

        store(&main, "chatgpt", &[second.clone(), first.clone()]).unwrap();
        let restored = load(&main, "chatgpt");
        assert_eq!(restored.len(), 2);
        assert_eq!(restored[0].id, second.id);
        assert_eq!(
            restored[0].semantic_event["messages"][1]["content"][1]["type"],
            "rich_card"
        );

        fs::write(cache_file(&directory(&main), &second.id), b"corrupt").unwrap();
        let restored = load(&main, "chatgpt");
        assert_eq!(restored.len(), 1);
        assert_eq!(restored[0].id, first.id);

        clear(&main);
        let _ = fs::remove_dir(root);
    }

    #[test]
    fn oversized_conversation_keeps_the_recent_bounded_window() {
        let root = std::env::temp_dir().join(format!(
            "elon-local-ai-conversation-window-{}-{}",
            std::process::id(),
            now_ms()
        ));
        let main = root.join("chatgpt.dpapi");
        let mut value = snapshot("0000000000000003", "Long", now_ms());
        value.semantic_event["messages"] = serde_json::Value::Array(
            (0..80)
                .map(|index| {
                    json!({
                        "id":format!("message-{index}"),
                        "role":if index % 2 == 0 { "user" } else { "assistant" },
                        "state":"completed",
                        "content":[{"type":"markdown","text":format!("{index}:{}", "x".repeat(40_000))}]
                    })
                })
                .collect(),
        );
        value.semantic_event["observedMessageCount"] = json!(80);

        store(&main, "chatgpt", &[value]).unwrap();
        let restored = load(&main, "chatgpt");
        let messages = restored[0].semantic_event["messages"].as_array().unwrap();
        assert!(messages.len() < 80);
        assert_eq!(restored[0].semantic_event["observedMessageCount"], 80);
        assert_eq!(messages.last().unwrap()["id"], "message-79");

        clear(&main);
        let _ = fs::remove_dir(root);
    }

    fn snapshot(id: &str, title: &str, updated_at_ms: u64) -> StoredConversationSnapshot {
        StoredConversationSnapshot {
            id: id.to_string(),
            title: title.to_string(),
            restorable_url: format!("https://chatgpt.com/c/{id}"),
            semantic_event: json!({
                "type":"message_snapshot",
                "streaming":false,
                "messages":[
                    {"id":"u1","role":"user","state":"completed","content":[{"type":"text","text":"price"}]},
                    {"id":"a1","role":"assistant","state":"completed","content":[
                        {"type":"markdown","text":"answer"},
                        {"type":"rich_card","text":"Bitcoin (BTC)","kind":"finance","richContent":{
                            "schema":"yilong.rich-content.v1","kind":"finance","source":"private_response",
                            "payload":{"title":"Bitcoin (BTC)","primaryValue":"US$77,000","trend":"positive"}
                        }}
                    ]}
                ]
            }),
            updated_at_ms,
        }
    }
}
