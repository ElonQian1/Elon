//! Explicit group business commands, separate from the body-free diagnostic timeline.
use super::{lock, now_ms};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::VecDeque,
    path::{Path, PathBuf},
    sync::Mutex,
};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct Command {
    pub command_id: String,
    pub action: String,
    #[serde(default)]
    pub owner_binding: Option<String>,
    #[serde(default)]
    pub group_id: Option<String>,
    #[serde(default)]
    pub message_ids: Vec<String>,
    #[serde(default)]
    pub message_revisions: std::collections::BTreeMap<String, u64>,
    #[serde(default)]
    pub question: Option<String>,
    #[serde(default)]
    pub task_id: Option<String>,
    #[serde(default)]
    pub confirmed: bool,
    #[serde(default)]
    pub offset: usize,
}

fn identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 100
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
}
impl Command {
    fn validate(&self) -> Result<(), String> {
        if uuid::Uuid::parse_str(&self.command_id).is_err() || self.offset > 10000 {
            return Err("invalid_command_id_or_offset".into());
        }
        if !matches!(
            self.action.as_str(),
            "groups" | "messages" | "start" | "status" | "resume" | "cancel"
        ) {
            return Err("unsupported_group_action".into());
        }
        if self.action != "groups" && !self.owner_binding.as_deref().is_some_and(identifier) {
            return Err("owner_binding_required".into());
        }
        let selecting = matches!(self.action.as_str(), "messages" | "start");
        if selecting != self.group_id.is_some()
            || self.group_id.as_deref().is_some_and(|id| !identifier(id))
        {
            return Err("invalid_group_id".into());
        }
        let tracking = matches!(self.action.as_str(), "status" | "resume" | "cancel");
        if tracking != self.task_id.is_some()
            || self
                .task_id
                .as_deref()
                .is_some_and(|id| uuid::Uuid::parse_str(id).is_err())
        {
            return Err("invalid_task_id".into());
        }
        if self.action == "start" {
            if !self.confirmed
                || self.message_ids.is_empty()
                || self.message_ids.len() > 50
                || self.message_ids.len() != self.message_revisions.len()
                || self.message_ids.iter().any(|id| {
                    !identifier(id) || !self.message_revisions.get(id).is_some_and(|r| *r > 0)
                })
                || self
                    .message_ids
                    .iter()
                    .collect::<std::collections::HashSet<_>>()
                    .len()
                    != self.message_ids.len()
                || !self.question.as_deref().is_some_and(|q| {
                    !q.trim().is_empty() && q.chars().count() <= 2000 && !q.contains('\0')
                })
            {
                return Err("invalid_or_unconfirmed_selection".into());
            }
        } else if self.question.is_some()
            || !self.message_ids.is_empty()
            || !self.message_revisions.is_empty()
            || self.confirmed
        {
            return Err("unexpected_write_fields".into());
        }
        Ok(())
    }
}

#[derive(Clone)]
struct Entry {
    workspace: PathBuf,
    command: Command,
    fingerprint: String,
    worker: Option<String>,
    expires: u128,
    result: Option<Value>,
}

#[derive(Default)]
pub(crate) struct GroupAiControl(Mutex<VecDeque<Entry>>);
impl GroupAiControl {
    pub fn enqueue(&self, workspace: &Path, command: Command) -> Result<Value, String> {
        command.validate()?;
        use sha2::{Digest, Sha256};
        let fingerprint = format!(
            "{:x}",
            Sha256::digest(serde_json::to_vec(&command).map_err(|_| "invalid_command")?)
        );
        let mut entries = lock(&self.0);
        if let Some(entry) = entries
            .iter()
            .find(|e| e.command.command_id == command.command_id)
        {
            if entry.workspace != workspace || entry.fingerprint != fingerprint {
                return Err("command_id_conflict".into());
            }
            return Ok(snapshot(entry));
        }
        if entries.len() >= 256 {
            if entries
                .front()
                .is_some_and(|e| e.expires + 3600000 < now_ms())
            {
                entries.pop_front();
            } else {
                return Err("group_command_capacity".into());
            }
        }
        let entry = Entry {
            workspace: workspace.to_path_buf(),
            command,
            fingerprint,
            worker: None,
            expires: now_ms() + 120000,
            result: None,
        };
        let output = snapshot(&entry);
        entries.push_back(entry);
        Ok(output)
    }
    pub fn status(&self, workspace: &Path, id: &str) -> Result<Value, String> {
        lock(&self.0)
            .iter()
            .find(|e| e.workspace == workspace && e.command.command_id == id)
            .map(snapshot)
            .ok_or_else(|| "group_command_not_found".into())
    }
    pub fn pending(&self) -> Vec<String> {
        lock(&self.0)
            .iter()
            .filter(|e| e.worker.is_none() && e.result.is_none() && e.expires > now_ms())
            .take(10)
            .map(|e| e.command.command_id.clone())
            .collect()
    }
    pub fn claim(&self, id: &str, worker: &str) -> Result<Command, String> {
        if uuid::Uuid::parse_str(worker).is_err() {
            return Err("invalid_worker".into());
        }
        let mut entries = lock(&self.0);
        let e = entries
            .iter_mut()
            .find(|e| e.command.command_id == id)
            .ok_or("group_command_not_found")?;
        if e.expires <= now_ms() || e.worker.is_some() || e.result.is_some() {
            return Err("group_command_already_claimed_or_expired".into());
        }
        e.worker = Some(worker.into());
        Ok(e.command.clone())
    }
    pub fn receipt(&self, id: &str, worker: &str, result: Value) -> Result<(), String> {
        let result = sanitize_result(result)?;
        let mut entries = lock(&self.0);
        let e = entries
            .iter_mut()
            .find(|e| e.command.command_id == id)
            .ok_or("group_command_not_found")?;
        if e.worker.as_deref() != Some(worker) || e.expires <= now_ms() {
            return Err("group_receipt_not_owned_or_expired".into());
        }
        if let Some(previous) = &e.result {
            return if previous == &result {
                Ok(())
            } else {
                Err("group_receipt_conflict".into())
            };
        }
        e.result = Some(result);
        // The fingerprint handles retries without retaining prompt text after execution.
        e.command.question = None;
        Ok(())
    }
}
fn snapshot(e: &Entry) -> Value {
    json!({"schema":"elon.win_group_ai_command.v1", "command_id":e.command.command_id,
        "action":e.command.action, "status": if e.result.is_some() {"completed"} else if e.expires <= now_ms() {"expired"} else if e.worker.is_some() {"executing"} else {"queued"},
        "result":e.result, "expires_at_ms":e.expires})
}

fn bounded_text(v: &Value, key: &str, max: usize) -> Value {
    v.get(key)
        .and_then(Value::as_str)
        .map(|s| Value::String(s.chars().take(max).collect()))
        .unwrap_or(Value::Null)
}
fn safe_id(v: &Value, key: &str) -> Value {
    v.get(key)
        .and_then(Value::as_str)
        .filter(|s| identifier(s))
        .map(|s| json!(s))
        .unwrap_or(Value::Null)
}
fn sanitize_result(v: Value) -> Result<Value, String> {
    if serde_json::to_vec(&v)
        .map_err(|_| "invalid_group_result")?
        .len()
        > 32768
    {
        return Err("group_result_too_large".into());
    }
    if v["schema"] != "elon.win_group_ai_result.v1" {
        return Err("invalid_group_result_schema".into());
    }
    let mut out =
        json!({"schema":"elon.win_group_ai_result.v1", "ok":v["ok"].as_bool().unwrap_or(false)});
    for key in [
        "owner_binding",
        "group_id",
        "task_id",
        "request_id",
        "result_message_id",
        "error_code",
        "phase",
        "stage",
    ] {
        out[key] = safe_id(&v, key);
    }
    for key in [
        "busy",
        "dispatched",
        "has_answer",
        "delivery_verified",
        "source_verified",
        "task_found",
    ] {
        out[key] = json!(v[key].as_bool().unwrap_or(false));
    }
    for key in [
        "next_offset",
        "attachment_count",
        "answer_chars",
        "source_count",
        "message_count",
    ] {
        out[key] = v[key]
            .as_u64()
            .filter(|n| *n <= 1000000)
            .map(|n| json!(n))
            .unwrap_or(Value::Null);
    }
    for (key, limit) in [("groups", 20), ("messages", 30)] {
        if let Some(items) = v[key].as_array() {
            out[key] = json!(items.iter().take(limit).map(|item| {
                if key == "groups" { json!({"id":safe_id(item,"id"),"name":bounded_text(item,"name",80)}) }
                else { json!({"id":safe_id(item,"id"), "revision":item["revision"].as_u64(),
                    "preview":bounded_text(item,"preview",240), "created_at":bounded_text(item,"created_at",40),
                    "attachment_count":item["attachment_count"].as_u64().unwrap_or(0).min(100),
                    "has_image":item["has_image"].as_bool().unwrap_or(false), "recalled":item["recalled"].as_bool().unwrap_or(false)}) }
            }).collect::<Vec<_>>());
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn command(action: &str) -> Command {
        serde_json::from_value(
            json!({"command_id":uuid::Uuid::new_v4().to_string(),"action":action}),
        )
        .unwrap()
    }
    #[test]
    fn rejects_unconfirmed_writes_unknown_fields_and_cross_workspace_reads() {
        let q = GroupAiControl::default();
        assert!(q.enqueue(Path::new("a"), command("start")).is_err());
        assert!(serde_json::from_value::<Command>(
            json!({"command_id":"x","action":"groups","script":"bad"})
        )
        .is_err());
        let c = command("groups");
        q.enqueue(Path::new("a"), c.clone()).unwrap();
        assert!(q.status(Path::new("b"), &c.command_id).is_err());
        assert!(q.enqueue(Path::new("b"), c).is_err());
    }
    #[test]
    fn exactly_one_claim_and_same_command_deduplication() {
        let q = GroupAiControl::default();
        let c = command("groups");
        q.enqueue(Path::new("a"), c.clone()).unwrap();
        q.enqueue(Path::new("a"), c.clone()).unwrap();
        assert_eq!(q.pending().len(), 1);
        let worker = uuid::Uuid::new_v4().to_string();
        q.claim(&c.command_id, &worker).unwrap();
        assert!(q.claim(&c.command_id, &worker).is_err());
        assert!(q
            .receipt(
                &c.command_id,
                "other",
                json!({"schema":"elon.win_group_ai_result.v1","ok":true})
            )
            .is_err());
        q.receipt(&c.command_id, &worker, json!({"schema":"elon.win_group_ai_result.v1","ok":true,"cookie":"secret","groups":[{"id":"g","name":"fixture","url":"secret"}]})).unwrap();
        let result = q.status(Path::new("a"), &c.command_id).unwrap();
        assert_eq!(result["status"], "completed");
        assert!(!result.to_string().contains("secret"));
    }

    #[test]
    fn write_receipt_drops_question_but_preserves_retry_fingerprint() {
        let q = GroupAiControl::default();
        let mut c = command("start");
        c.owner_binding = Some("binding".into());
        c.group_id = Some("group".into());
        c.message_ids = vec!["image".into()];
        c.message_revisions.insert("image".into(), 2);
        c.question = Some("Synthetic objects question".into());
        c.confirmed = true;
        q.enqueue(Path::new("a"), c.clone()).unwrap();
        let worker = uuid::Uuid::new_v4().to_string();
        q.claim(&c.command_id, &worker).unwrap();
        q.receipt(
            &c.command_id,
            &worker,
            json!({"schema":"elon.win_group_ai_result.v1","ok":true,"phase":"preparing"}),
        )
        .unwrap();
        assert!(lock(&q.0)[0].command.question.is_none());
        assert_eq!(
            q.enqueue(Path::new("a"), c.clone()).unwrap()["status"],
            "completed"
        );
        c.question = Some("Different question".into());
        assert!(q.enqueue(Path::new("a"), c).is_err());
        assert!(q.pending().is_empty());
    }

    #[test]
    fn expired_claims_are_not_replayed() {
        let q = GroupAiControl::default();
        let c = command("groups");
        q.enqueue(Path::new("a"), c.clone()).unwrap();
        lock(&q.0)[0].expires = 0;
        assert!(q.pending().is_empty());
        assert_eq!(
            q.status(Path::new("a"), &c.command_id).unwrap()["status"],
            "expired"
        );
        assert!(q
            .claim(&c.command_id, &uuid::Uuid::new_v4().to_string())
            .is_err());
        assert_eq!(q.enqueue(Path::new("a"), c).unwrap()["status"], "expired");
    }
}
