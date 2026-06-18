use serde::{Deserialize, Serialize};
use std::{fs, io, path::PathBuf};

use super::{normalize_text, now_ms};

const MAX_SESSIONS: usize = 80;
const MAX_MESSAGES_PER_SESSION: usize = 120;
const MAX_CONTEXT_MESSAGES: usize = 14;
const MAX_TITLE_CHARS: usize = 42;
const MAX_MESSAGE_CHARS: usize = 8_000;
const MAX_CONTEXT_MESSAGE_CHARS: usize = 1_200;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct DoctorSession {
    pub(super) id: String,
    pub(super) title: String,
    pub(super) messages: Vec<DoctorSessionMessage>,
    pub(super) created_at_ms: u64,
    pub(super) updated_at_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct DoctorSessionMessage {
    pub(super) id: String,
    pub(super) role: String,
    pub(super) content: String,
    pub(super) kind: Option<String>,
    pub(super) created_at_ms: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct DoctorSessionSummary {
    pub(super) id: String,
    pub(super) title: String,
    pub(super) last_message: String,
    pub(super) message_count: usize,
    pub(super) created_at_ms: u64,
    pub(super) updated_at_ms: u64,
}

pub(super) fn sessions_path() -> PathBuf {
    if cfg!(windows) {
        if let Ok(appdata) = std::env::var("APPDATA") {
            return PathBuf::from(appdata)
                .join("elon-node-agent")
                .join("doctor_sessions.json");
        }
    }

    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
        .join(".config")
        .join("elon-node-agent")
        .join("doctor_sessions.json")
}

pub(super) fn list_session_summaries() -> io::Result<Vec<DoctorSessionSummary>> {
    Ok(read_sessions()?
        .into_iter()
        .map(|session| DoctorSessionSummary {
            id: session.id,
            title: session.title,
            last_message: session
                .messages
                .last()
                .map(|message| preview_text(&message.content, 90))
                .unwrap_or_default(),
            message_count: session.messages.len(),
            created_at_ms: session.created_at_ms,
            updated_at_ms: session.updated_at_ms,
        })
        .collect())
}

pub(super) fn create_session(title: Option<&str>) -> io::Result<DoctorSession> {
    let now = now_ms();
    let session = DoctorSession {
        id: format!("doctor-session-{now}"),
        title: session_title(title.unwrap_or("新的电脑诊断")),
        messages: Vec::new(),
        created_at_ms: now,
        updated_at_ms: now,
    };
    save_session(&session)?;
    Ok(session)
}

pub(super) fn read_session(session_id: &str) -> io::Result<Option<DoctorSession>> {
    Ok(read_sessions()?
        .into_iter()
        .find(|session| session.id == session_id))
}

pub(super) fn delete_session(session_id: &str) -> io::Result<bool> {
    let mut sessions = read_sessions()?;
    let before = sessions.len();
    sessions.retain(|session| session.id != session_id);
    if sessions.len() == before {
        return Ok(false);
    }
    write_sessions(&sessions)?;
    Ok(true)
}

pub(super) fn load_or_create(
    session_id: Option<&str>,
    title_hint: &str,
) -> io::Result<DoctorSession> {
    if let Some(session_id) = session_id.and_then(|value| {
        let clean = value.trim();
        (!clean.is_empty()).then_some(clean)
    }) {
        if let Some(session) = read_session(session_id)? {
            return Ok(session);
        }
    }

    let now = now_ms();
    Ok(DoctorSession {
        id: format!("doctor-session-{now}"),
        title: session_title(title_hint),
        messages: Vec::new(),
        created_at_ms: now,
        updated_at_ms: now,
    })
}

pub(super) fn push_message(
    session: &mut DoctorSession,
    role: &str,
    content: &str,
    kind: Option<&str>,
) -> DoctorSessionMessage {
    let now = now_ms();
    if session.messages.is_empty() && role == "user" {
        session.title = session_title(content);
    }
    let message = DoctorSessionMessage {
        id: format!("doctor-message-{now}-{}", session.messages.len() + 1),
        role: normalize_role(role),
        content: normalize_text(content, MAX_MESSAGE_CHARS),
        kind: kind.map(|value| normalize_text(value, 24)),
        created_at_ms: now,
    };
    session.messages.push(message.clone());
    if session.messages.len() > MAX_MESSAGES_PER_SESSION {
        let excess = session.messages.len() - MAX_MESSAGES_PER_SESSION;
        session.messages.drain(0..excess);
    }
    session.updated_at_ms = now;
    message
}

pub(super) fn save_session(session: &DoctorSession) -> io::Result<()> {
    let mut sessions = read_sessions()?;
    sessions.retain(|item| item.id != session.id);
    sessions.insert(0, session.clone());
    sessions.sort_by(|left, right| right.updated_at_ms.cmp(&left.updated_at_ms));
    sessions.truncate(MAX_SESSIONS);
    write_sessions(&sessions)
}

pub(super) fn context_messages(session: &DoctorSession) -> Vec<DoctorSessionMessage> {
    let start = session.messages.len().saturating_sub(MAX_CONTEXT_MESSAGES);
    session.messages[start..]
        .iter()
        .cloned()
        .map(|mut message| {
            message.content = normalize_text(&message.content, MAX_CONTEXT_MESSAGE_CHARS);
            message
        })
        .collect()
}

fn read_sessions() -> io::Result<Vec<DoctorSession>> {
    let path = sessions_path();
    if !path.exists() {
        return Ok(Vec::new());
    }
    let text = fs::read_to_string(path)?;
    let mut sessions = serde_json::from_str::<Vec<DoctorSession>>(&text).unwrap_or_default();
    sessions.sort_by(|left, right| right.updated_at_ms.cmp(&left.updated_at_ms));
    Ok(sessions)
}

fn write_sessions(sessions: &[DoctorSession]) -> io::Result<()> {
    let path = sessions_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let text = serde_json::to_string_pretty(sessions).unwrap_or_else(|_| "[]".to_string());
    fs::write(path, text)
}

fn session_title(value: &str) -> String {
    let clean = normalize_text(value, MAX_TITLE_CHARS);
    if clean.is_empty() {
        "新的电脑诊断".to_string()
    } else {
        clean
    }
}

fn preview_text(value: &str, max_chars: usize) -> String {
    normalize_text(value, max_chars).replace('\n', " ")
}

fn normalize_role(role: &str) -> String {
    match role {
        "user" => "user".to_string(),
        "assistant" => "assistant".to_string(),
        _ => "system".to_string(),
    }
}
