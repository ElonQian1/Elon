use std::{
    collections::BTreeMap,
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::Mutex,
    time::Duration,
};

use serde::{Deserialize, Serialize};
use serde_json::Value;

const ACTION_NETWORK: &str = "research_network_observation";
const ACTION_VOICE: &str = "research_voice_observation";
const DIRECTORY_NAME: &str = "private-observations";
const FILE_NAME: &str = "summary-v1.json";
const SCHEMA: &str = "yilong.web-ai.private-observation.v1";
const MAX_DETAIL_BYTES: usize = 160;
const MAX_KINDS: usize = 96;
const MAX_AGE: Duration = Duration::from_secs(30 * 24 * 60 * 60);
static STORE_LOCK: Mutex<()> = Mutex::new(());

const VOICE_CHANNELS: &[&str] = &[
    "observer-ready",
    "media-request",
    "media-granted",
    "media-error",
    "peer-created",
    "peer-create-offer",
    "peer-create-answer",
    "peer-local-description",
    "peer-remote-description",
    "peer-connection",
    "peer-ice",
    "peer-signaling",
    "peer-track",
    "peer-data-channel",
    "network-start",
    "network-end",
    "network-error",
    "network-shape",
    "socket-start",
    "socket-open",
    "socket-close",
    "socket-error",
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct PrivateObservation {
    action: &'static str,
    channel: String,
    detail: String,
}

#[derive(Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct PrivateObservationStatus {
    pub network_observation_count: u64,
    pub voice_observation_count: u64,
    pub latest_observed_at_ms: u64,
    pub voice_channels: Vec<String>,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct ObservationDocument {
    schema: String,
    provider_id: String,
    updated_at_ms: u64,
    total_count: u64,
    observations: BTreeMap<String, ObservationAggregate>,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct ObservationAggregate {
    action: String,
    channel: String,
    count: u64,
    last_seen_at_ms: u64,
    last_detail: String,
}

impl ObservationDocument {
    fn empty(provider_id: &str) -> Self {
        Self {
            schema: SCHEMA.to_string(),
            provider_id: provider_id.to_string(),
            updated_at_ms: 0,
            total_count: 0,
            observations: BTreeMap::new(),
        }
    }
}

pub(super) fn parse(provider_id: &str, kind: &str, payload: &Value) -> Option<PrivateObservation> {
    if kind != "command_result" || !payload.get("ok").and_then(Value::as_bool)? {
        return None;
    }
    let action = match payload.get("action").and_then(Value::as_str)? {
        ACTION_NETWORK => ACTION_NETWORK,
        ACTION_VOICE if provider_id == "chatgpt" => ACTION_VOICE,
        _ => return None,
    };
    let detail = payload.get("detail").and_then(Value::as_str)?;
    if !valid_detail(detail) {
        return None;
    }
    let parts = detail.split('|').collect::<Vec<_>>();
    if parts.len() < 2 || parts.len() > 12 || parts[0] != "v1" {
        return None;
    }
    let channel = parts[1];
    if !valid_channel(channel)
        || (action == ACTION_VOICE && !VOICE_CHANNELS.contains(&channel))
        || (action == ACTION_VOICE && contains_sensitive_marker(detail))
    {
        return None;
    }
    Some(PrivateObservation {
        action,
        channel: channel.to_string(),
        detail: detail.to_string(),
    })
}

pub(super) fn store(
    root: &Path,
    provider_id: &str,
    observation: PrivateObservation,
    observed_at_ms: u64,
) -> Result<(), String> {
    let _guard = STORE_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let directory = root.join(DIRECTORY_NAME);
    fs::create_dir_all(&directory).map_err(display_error)?;
    let path = directory.join(FILE_NAME);
    let mut document = read_document(&path, provider_id);
    prune(&mut document, observed_at_ms);
    let key = format!("{}:{}", observation.action, observation.channel);
    if !document.observations.contains_key(&key) && document.observations.len() >= MAX_KINDS {
        return Ok(());
    }
    let entry = document
        .observations
        .entry(key)
        .or_insert_with(|| ObservationAggregate {
            action: observation.action.to_string(),
            channel: observation.channel.clone(),
            count: 0,
            last_seen_at_ms: 0,
            last_detail: String::new(),
        });
    entry.count = entry.count.saturating_add(1);
    entry.last_seen_at_ms = observed_at_ms;
    entry.last_detail = observation.detail;
    document.schema = SCHEMA.to_string();
    document.provider_id = provider_id.to_string();
    document.updated_at_ms = observed_at_ms;
    document.total_count = document.total_count.saturating_add(1);
    write_document(&path, &document)
}

pub(super) fn read_status(root: &Path, provider_id: &str) -> PrivateObservationStatus {
    let document = read_document(&root.join(DIRECTORY_NAME).join(FILE_NAME), provider_id);
    let mut status = PrivateObservationStatus {
        latest_observed_at_ms: document.updated_at_ms,
        ..PrivateObservationStatus::default()
    };
    for observation in document.observations.values() {
        match observation.action.as_str() {
            ACTION_NETWORK => {
                status.network_observation_count = status
                    .network_observation_count
                    .saturating_add(observation.count);
            }
            ACTION_VOICE => {
                status.voice_observation_count = status
                    .voice_observation_count
                    .saturating_add(observation.count);
                status.voice_channels.push(observation.channel.clone());
            }
            _ => {}
        }
    }
    status.voice_channels.sort();
    status.voice_channels.dedup();
    status
}

fn read_document(path: &Path, provider_id: &str) -> ObservationDocument {
    fs::read(path)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<ObservationDocument>(&bytes).ok())
        .filter(|document| document.schema == SCHEMA && document.provider_id == provider_id)
        .unwrap_or_else(|| ObservationDocument::empty(provider_id))
}

fn write_document(path: &Path, document: &ObservationDocument) -> Result<(), String> {
    let encoded = serde_json::to_vec_pretty(document).map_err(display_error)?;
    let mut file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(path)
        .map_err(display_error)?;
    file.write_all(&encoded).map_err(display_error)?;
    file.sync_all().map_err(display_error)
}

fn prune(document: &mut ObservationDocument, observed_at_ms: u64) {
    let maximum_age_ms = MAX_AGE.as_millis() as u64;
    document.observations.retain(|_, observation| {
        observed_at_ms.saturating_sub(observation.last_seen_at_ms) <= maximum_age_ms
    });
}

fn valid_detail(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_DETAIL_BYTES
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric()
                || matches!(byte, b'.' | b'_' | b':' | b'/' | b'|' | b'{' | b'}' | b'-')
        })
}

fn valid_channel(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 48
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
}

fn contains_sensitive_marker(value: &str) -> bool {
    value.split(['|', '.', '_', '/', '-']).any(|part| {
        matches!(
            part,
            "authorization"
                | "bearer"
                | "cookie"
                | "credential"
                | "proof"
                | "sdp"
                | "candidate"
                | "secret"
                | "token"
        )
    })
}

fn display_error(error: impl std::fmt::Display) -> String {
    format!("无法保存本机网页 AI 私有结构观察：{error}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn accepts_redacted_voice_and_network_observations() {
        let voice = parse(
            "chatgpt",
            "command_result",
            &json!({"action": ACTION_VOICE, "ok": true, "detail": "v1|peer-created|unified-plan"}),
        )
        .unwrap();
        assert_eq!(voice.channel, "peer-created");
        let network = parse(
            "google-ai-mode",
            "command_result",
            &json!({"action": ACTION_NETWORK, "ok": true, "detail": "v1|reply|rpc:complete"}),
        )
        .unwrap();
        assert_eq!(network.channel, "reply");
    }

    #[test]
    fn rejects_voice_secrets_unknown_channels_and_non_chatgpt_voice() {
        for detail in [
            "v1|peer-created|bearer",
            "v1|peer-remote-description|sdp",
            "v1|unknown-channel|safe",
        ] {
            assert!(parse(
                "chatgpt",
                "command_result",
                &json!({"action": ACTION_VOICE, "ok": true, "detail": detail}),
            )
            .is_none());
        }
        assert!(parse(
            "google-ai-mode",
            "command_result",
            &json!({"action": ACTION_VOICE, "ok": true, "detail": "v1|peer-created|safe"}),
        )
        .is_none());
    }

    #[test]
    fn aggregates_bounded_observations_without_raw_credentials() {
        let root = temporary_root("aggregate");
        let first = parse(
            "chatgpt",
            "command_result",
            &json!({"action": ACTION_VOICE, "ok": true, "detail": "v1|socket-open|chatgpt-subdomain"}),
        )
        .unwrap();
        store(&root, "chatgpt", first.clone(), 100).unwrap();
        store(&root, "chatgpt", first, 200).unwrap();
        let network = parse(
            "chatgpt",
            "command_result",
            &json!({"action": ACTION_NETWORK, "ok": true, "detail": "v1|private_stream|first|1|20"}),
        )
        .unwrap();
        store(&root, "chatgpt", network, 300).unwrap();

        let status = read_status(&root, "chatgpt");
        assert_eq!(status.voice_observation_count, 2);
        assert_eq!(status.network_observation_count, 1);
        assert_eq!(status.voice_channels, vec!["socket-open"]);
        let persisted = fs::read_to_string(root.join(DIRECTORY_NAME).join(FILE_NAME)).unwrap();
        assert!(!persisted.contains("authorization"));
        assert!(!persisted.contains("cookie"));
        assert!(!persisted.contains("candidate"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn expired_channels_are_pruned_on_next_observation() {
        let root = temporary_root("expiry");
        let old = parse(
            "chatgpt",
            "command_result",
            &json!({"action": ACTION_VOICE, "ok": true, "detail": "v1|socket-open|safe"}),
        )
        .unwrap();
        store(&root, "chatgpt", old, 1).unwrap();
        let recent = parse(
            "chatgpt",
            "command_result",
            &json!({"action": ACTION_NETWORK, "ok": true, "detail": "v1|private_stream|success|3|40"}),
        )
        .unwrap();
        store(&root, "chatgpt", recent, MAX_AGE.as_millis() as u64 + 2).unwrap();
        let status = read_status(&root, "chatgpt");
        assert_eq!(status.voice_observation_count, 0);
        assert_eq!(status.network_observation_count, 1);
        fs::remove_dir_all(root).unwrap();
    }

    fn temporary_root(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "elon-private-observation-{label}-{}-{}",
            std::process::id(),
            crate::local_ai_browser::research_capture::now_ms(),
        ))
    }
}
