//! Lightweight metrics facade for realtime connection lifecycle events.
//!
//! The project currently relies on structured tracing rather than a dedicated
//! metrics backend. Keeping this facade small gives realtime modules one stable
//! place to report counters, and leaves a clean adapter point for Prometheus or
//! OpenTelemetry later.

use std::{
    collections::HashMap,
    sync::{Arc, Mutex, OnceLock},
};

use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;

use crate::{
    admin::check_auth,
    project_auth::json_error,
    store::{RealtimeCloseMetricRow, Store},
    types::AppState,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RealtimeChannel {
    AppNotify,
    GlobalApp,
    HomecliAgent,
    PeerRelay,
    ProjectWs,
    VoiceRealtimeChat,
    VoiceTranscribe,
    VoiceVirtualMic,
}

impl RealtimeChannel {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::AppNotify => "app_notify",
            Self::GlobalApp => "global_app",
            Self::HomecliAgent => "homecli_agent",
            Self::PeerRelay => "peer_relay",
            Self::ProjectWs => "project_ws",
            Self::VoiceRealtimeChat => "voice_realtime_chat",
            Self::VoiceTranscribe => "voice_transcribe",
            Self::VoiceVirtualMic => "voice_virtual_mic",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RealtimeCloseMetricSnapshot {
    pub channel: String,
    pub close_reason: String,
    pub count: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct RealtimeDiagnosticChannel {
    pub id: &'static str,
    pub business_boundary: &'static str,
    pub entry_modules: &'static [&'static str],
    pub close_reason_source: &'static str,
    pub metric_variant: &'static str,
    pub sync_targets: &'static [&'static str],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct RealtimeDiagnosticCloseReason {
    pub id: &'static str,
    pub source: &'static str,
    pub category: &'static str,
    pub alert_bucket: Option<&'static str>,
    pub meaning: &'static str,
    pub first_check: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct RealtimeDiagnosticsCatalog {
    pub version: &'static str,
    pub channels: &'static [RealtimeDiagnosticChannel],
    pub close_reasons: &'static [RealtimeDiagnosticCloseReason],
    pub change_rules: &'static [&'static str],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct RealtimeCloseMetricKey {
    channel: &'static str,
    close_reason: &'static str,
}

static CLOSE_COUNTERS: OnceLock<Mutex<HashMap<RealtimeCloseMetricKey, u64>>> = OnceLock::new();

fn close_counters() -> &'static Mutex<HashMap<RealtimeCloseMetricKey, u64>> {
    CLOSE_COUNTERS.get_or_init(|| Mutex::new(HashMap::new()))
}

const REALTIME_DIAGNOSTIC_CHANNELS: &[RealtimeDiagnosticChannel] = &[
    RealtimeDiagnosticChannel {
        id: "app_notify",
        business_boundary: "Legacy mobile notify and upgrade prompt websocket",
        entry_modules: &["server/src/app_update.rs"],
        close_reason_source: "WsCloseReason",
        metric_variant: "RealtimeChannel::AppNotify",
        sync_targets: &[
            "docs/realtime-operations-runbook.md",
            "docs/realtime-channel-ownership.md",
            "scripts/check-realtime-ownership.ps1",
        ],
    },
    RealtimeDiagnosticChannel {
        id: "global_app",
        business_boundary: "Global app websocket for online presence and common pushes",
        entry_modules: &["server/src/global_ws.rs"],
        close_reason_source: "WsCloseReason",
        metric_variant: "RealtimeChannel::GlobalApp",
        sync_targets: &[
            "docs/realtime-operations-runbook.md",
            "docs/realtime-channel-ownership.md",
            "scripts/check-realtime-ownership.ps1",
        ],
    },
    RealtimeDiagnosticChannel {
        id: "project_ws",
        business_boundary: "Project task websocket for execution, approvals, and task status",
        entry_modules: &["server/src/project_ws_session.rs"],
        close_reason_source: "WsCloseReason",
        metric_variant: "RealtimeChannel::ProjectWs",
        sync_targets: &[
            "docs/realtime-operations-runbook.md",
            "docs/realtime-channel-ownership.md",
            "scripts/test-admin-realtime-health.js",
            "scripts/check-realtime-ownership.ps1",
        ],
    },
    RealtimeDiagnosticChannel {
        id: "voice_transcribe",
        business_boundary: "Realtime speech-to-text websocket",
        entry_modules: &[
            "server/src/voice_ws_transcribe.rs",
            "server/src/voice_ws_transcribe_impl.rs",
        ],
        close_reason_source: "WsCloseReason",
        metric_variant: "RealtimeChannel::VoiceTranscribe",
        sync_targets: &[
            "docs/realtime-operations-runbook.md",
            "docs/realtime-channel-ownership.md",
            "scripts/test-admin-realtime-health.js",
            "scripts/check-realtime-ownership.ps1",
        ],
    },
    RealtimeDiagnosticChannel {
        id: "voice_realtime_chat",
        business_boundary: "Realtime AI voice chat websocket",
        entry_modules: &[
            "server/src/voice_ws_realtime_chat.rs",
            "server/src/voice_ws_realtime_chat_impl.rs",
        ],
        close_reason_source: "WsCloseReason",
        metric_variant: "RealtimeChannel::VoiceRealtimeChat",
        sync_targets: &[
            "docs/realtime-operations-runbook.md",
            "docs/realtime-channel-ownership.md",
            "scripts/test-admin-realtime-health.js",
            "scripts/check-realtime-ownership.ps1",
        ],
    },
    RealtimeDiagnosticChannel {
        id: "voice_virtual_mic",
        business_boundary: "Virtual microphone realtime input websocket",
        entry_modules: &["server/src/voice_ws_virtual_mic.rs"],
        close_reason_source: "WsCloseReason",
        metric_variant: "RealtimeChannel::VoiceVirtualMic",
        sync_targets: &[
            "docs/realtime-operations-runbook.md",
            "docs/realtime-channel-ownership.md",
            "scripts/test-admin-realtime-health.js",
            "scripts/check-realtime-ownership.ps1",
        ],
    },
    RealtimeDiagnosticChannel {
        id: "homecli_agent",
        business_boundary: "HomeCLI/PC agent reverse websocket for PC CLI dispatch",
        entry_modules: &["server/src/homecli_agent/agent_session.rs"],
        close_reason_source: "AgentSessionCloseReason",
        metric_variant: "RealtimeChannel::HomecliAgent",
        sync_targets: &[
            "docs/realtime-operations-runbook.md",
            "docs/realtime-channel-ownership.md",
            "scripts/check-realtime-ownership.ps1",
            "server/src/homecli_agent_tests.rs",
        ],
    },
    RealtimeDiagnosticChannel {
        id: "peer_relay",
        business_boundary: "Mobile P2P APK relay and seeder transfer websocket",
        entry_modules: &["server/src/peer_relay.rs"],
        close_reason_source: "PeerWsCloseReason",
        metric_variant: "RealtimeChannel::PeerRelay",
        sync_targets: &[
            "docs/realtime-operations-runbook.md",
            "docs/realtime-channel-ownership.md",
            "scripts/check-realtime-ownership.ps1",
        ],
    },
];

const REALTIME_DIAGNOSTIC_CLOSE_REASONS: &[RealtimeDiagnosticCloseReason] = &[
    RealtimeDiagnosticCloseReason {
        id: "peer_closed",
        source: "WsCloseReason|PeerWsCloseReason",
        category: "normal_close",
        alert_bucket: None,
        meaning: "Peer sent a websocket close frame or closed intentionally.",
        first_check: "Check client navigation, backgrounding, refresh, or intentional stop flow.",
    },
    RealtimeDiagnosticCloseReason {
        id: "client_control_close",
        source: "WsCloseReason",
        category: "normal_close",
        alert_bucket: None,
        meaning: "Client sent an application-level control message that closes the realtime flow.",
        first_check: "Check voice/task stop order and client control message handling.",
    },
    RealtimeDiagnosticCloseReason {
        id: "reader_ended",
        source: "WsCloseReason",
        category: "ended",
        alert_bucket: None,
        meaning: "Websocket read stream ended without a specific read error.",
        first_check: "Check proxy idle timeout, client process exit, and network transitions.",
    },
    RealtimeDiagnosticCloseReason {
        id: "peer_reader_ended",
        source: "PeerWsCloseReason",
        category: "ended",
        alert_bucket: None,
        meaning: "Peer relay seeder read stream ended during registration or transfer.",
        first_check: "Check seeder app lifecycle, local network stability, and relay registry cleanup.",
    },
    RealtimeDiagnosticCloseReason {
        id: "reader_closed",
        source: "AgentSessionCloseReason",
        category: "ended",
        alert_bucket: None,
        meaning: "HomeCLI agent reader closed normally.",
        first_check: "Check PC agent restart, server upgrade, or intentional reconnect.",
    },
    RealtimeDiagnosticCloseReason {
        id: "reader_shutdown",
        source: "AgentSessionCloseReason",
        category: "shutdown",
        alert_bucket: None,
        meaning: "HomeCLI agent session reader was shut down by server-side lifecycle control.",
        first_check: "Check server restart, deployment, or writer-side failure that triggered shutdown.",
    },
    RealtimeDiagnosticCloseReason {
        id: "read_error",
        source: "WsCloseReason",
        category: "read_error",
        alert_bucket: Some("read_error"),
        meaning: "Server failed to read a websocket frame.",
        first_check: "Check client network quality, websocket protocol compatibility, and reverse proxy behavior.",
    },
    RealtimeDiagnosticCloseReason {
        id: "reader_error",
        source: "AgentSessionCloseReason",
        category: "read_error",
        alert_bucket: Some("read_error"),
        meaning: "Server failed to read from the HomeCLI agent websocket.",
        first_check: "Check PC agent connectivity, local runtime restart, and node false-online state.",
    },
    RealtimeDiagnosticCloseReason {
        id: "peer_read_error",
        source: "PeerWsCloseReason",
        category: "read_error",
        alert_bucket: Some("read_error"),
        meaning: "Server failed to read from the peer relay seeder websocket.",
        first_check: "Check seeder network, app lifecycle, and APK transfer interruption.",
    },
    RealtimeDiagnosticCloseReason {
        id: "write_failed",
        source: "WsCloseReason",
        category: "write_failure",
        alert_bucket: Some("write_failure"),
        meaning: "Server failed to write to a websocket peer.",
        first_check: "Check half-open clients, slow consumers, and proxy disconnects.",
    },
    RealtimeDiagnosticCloseReason {
        id: "pong_write_failed",
        source: "WsCloseReason",
        category: "write_failure",
        alert_bucket: Some("write_failure"),
        meaning: "Server failed to write Pong after receiving Ping.",
        first_check: "Check half-open connection, network jitter, and proxy keepalive behavior.",
    },
    RealtimeDiagnosticCloseReason {
        id: "writer_closed",
        source: "AgentSessionCloseReason",
        category: "write_failure",
        alert_bucket: Some("write_failure"),
        meaning: "HomeCLI agent writer side closed while the session was active.",
        first_check: "Check PC agent writer loop, outbound queue closure, and pending CLI failure messages.",
    },
    RealtimeDiagnosticCloseReason {
        id: "peer_write_error",
        source: "PeerWsCloseReason",
        category: "write_failure",
        alert_bucket: Some("write_failure"),
        meaning: "Server failed to write to a peer relay seeder websocket.",
        first_check: "Check relay command delivery, seeder disconnects, and APK transfer backpressure.",
    },
    RealtimeDiagnosticCloseReason {
        id: "reader_timeout",
        source: "AgentSessionCloseReason",
        category: "timeout",
        alert_bucket: Some("timeout"),
        meaning: "HomeCLI agent reader timed out.",
        first_check: "Check PC sleep, node false-online state, heartbeat loss, and local network/proxy issues.",
    },
    RealtimeDiagnosticCloseReason {
        id: "request_channel_closed",
        source: "PeerWsCloseReason",
        category: "relay_control",
        alert_bucket: None,
        meaning: "Peer relay request channel closed before the transfer completed.",
        first_check: "Check relay registry cleanup, downloader cancellation, and seeder request lifecycle.",
    },
];

const REALTIME_DIAGNOSTIC_CHANGE_RULES: &[&str] = &[
    "Add every new RealtimeChannel label to the diagnostics catalog, ownership doc, runbook, and ownership guard.",
    "Add every new close reason to the source enum label test, diagnostics catalog, runbook, and alert bucket classification when applicable.",
    "Realtime entry modules should record closes through realtime_metrics::record_close_with_store instead of implementing local counters.",
    "Admin UI display changes should keep scripts/test-admin-realtime-health.js in sync.",
    "Diagnostics catalog changes must update server/src/realtime_diagnostics_catalog.snapshot.json and the snapshot test.",
];

pub fn realtime_diagnostics_catalog() -> RealtimeDiagnosticsCatalog {
    RealtimeDiagnosticsCatalog {
        version: "2026-07-16",
        channels: REALTIME_DIAGNOSTIC_CHANNELS,
        close_reasons: REALTIME_DIAGNOSTIC_CLOSE_REASONS,
        change_rules: REALTIME_DIAGNOSTIC_CHANGE_RULES,
    }
}

#[cfg(test)]
pub fn record_close(channel: RealtimeChannel, close_reason: &'static str) -> u64 {
    record_close_inner(None, channel, close_reason)
}

pub fn record_close_with_store(
    store: &Store,
    channel: RealtimeChannel,
    close_reason: &'static str,
) -> u64 {
    record_close_inner(Some(store), channel, close_reason)
}

fn record_close_inner(
    store: Option<&Store>,
    channel: RealtimeChannel,
    close_reason: &'static str,
) -> u64 {
    let channel = channel.as_str();
    let key = RealtimeCloseMetricKey {
        channel,
        close_reason,
    };
    let count = {
        let mut counters = close_counters()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let count = counters.entry(key).or_default();
        *count += 1;
        *count
    };

    if let Some(store) = store {
        if let Err(error) = store.record_realtime_close_event(channel, close_reason) {
            tracing::warn!(
                target: "realtime_metrics",
                channel,
                close_reason,
                error = %error,
                "failed to persist realtime close event"
            );
        }
    }

    tracing::info!(
        target: "realtime_metrics",
        transport = "websocket",
        channel,
        close_reason,
        count,
        "realtime connection closed"
    );

    count
}

pub fn close_metric_snapshot() -> Vec<RealtimeCloseMetricSnapshot> {
    let counters = close_counters()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let mut rows: Vec<_> = counters
        .iter()
        .map(|(key, count)| RealtimeCloseMetricSnapshot {
            channel: key.channel.to_string(),
            close_reason: key.close_reason.to_string(),
            count: *count,
        })
        .collect();
    rows.sort_by(|a, b| {
        a.channel
            .cmp(&b.channel)
            .then_with(|| a.close_reason.cmp(&b.close_reason))
    });
    rows
}

pub async fn admin_close_metrics(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Response {
    if !check_auth(&headers, &state.admin_token) {
        return json_error(StatusCode::UNAUTHORIZED, "无效的管理员令牌");
    }

    Json(admin_close_metrics_payload(
        &state.store,
        chrono::Utc::now().timestamp(),
    ))
    .into_response()
}

pub async fn admin_diagnostics(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    if !check_auth(&headers, &state.admin_token) {
        return json_error(StatusCode::UNAUTHORIZED, "无效的管理员令牌");
    }

    Json(realtime_diagnostics_catalog()).into_response()
}

fn admin_close_metrics_payload(store: &Store, now_unix: i64) -> serde_json::Value {
    let all_time = query_store_window(store, None, "all_time");
    let last_1h = query_store_window(store, Some(now_unix - 60 * 60), "last_1h");
    let last_24h = query_store_window(store, Some(now_unix - 24 * 60 * 60), "last_24h");
    let process = close_metric_snapshot();
    let alerts = match store.refresh_realtime_close_alerts() {
        Ok(alerts) => alerts,
        Err(error) => {
            tracing::warn!(
                target: "realtime_metrics",
                error = %error,
                "failed to refresh realtime close alerts"
            );
            Vec::new()
        }
    };

    serde_json::json!({
        "metrics": all_time,
        "alerts": alerts,
        "windows": {
            "all_time": all_time,
            "last_1h": last_1h,
            "last_24h": last_24h,
            "process": process,
        }
    })
}

fn query_store_window(
    store: &Store,
    since_unix: Option<i64>,
    window_name: &str,
) -> Vec<RealtimeCloseMetricRow> {
    match store.admin_realtime_close_metrics_since(since_unix) {
        Ok(rows) => rows,
        Err(error) => {
            tracing::warn!(
                target: "realtime_metrics",
                window = window_name,
                error = %error,
                "failed to load realtime close metrics"
            );
            Vec::new()
        }
    }
}

#[cfg(test)]
pub fn reset_for_tests() {
    close_counters()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clear();
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::params;
    use uuid::Uuid;

    static COUNTER_TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

    fn counter_test_lock() -> std::sync::MutexGuard<'static, ()> {
        COUNTER_TEST_LOCK
            .get_or_init(|| Mutex::new(()))
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn temp_store() -> (Store, std::path::PathBuf) {
        let path = std::env::temp_dir().join(format!(
            "elon_realtime_metrics_{}.db",
            Uuid::new_v4().simple()
        ));
        (Store::open(&path).expect("store should open"), path)
    }

    #[test]
    fn record_close_increments_by_channel_and_reason() {
        let _guard = counter_test_lock();
        reset_for_tests();

        assert_eq!(
            record_close(RealtimeChannel::HomecliAgent, "reader_closed"),
            1
        );
        assert_eq!(
            record_close(RealtimeChannel::HomecliAgent, "reader_closed"),
            2
        );
        assert_eq!(record_close(RealtimeChannel::PeerRelay, "peer_closed"), 1);

        assert_eq!(
            close_metric_snapshot(),
            vec![
                RealtimeCloseMetricSnapshot {
                    channel: "homecli_agent".to_string(),
                    close_reason: "reader_closed".to_string(),
                    count: 2,
                },
                RealtimeCloseMetricSnapshot {
                    channel: "peer_relay".to_string(),
                    close_reason: "peer_closed".to_string(),
                    count: 1,
                },
            ]
        );
    }

    #[test]
    fn realtime_channel_labels_are_stable() {
        let cases = [
            (RealtimeChannel::AppNotify, "app_notify"),
            (RealtimeChannel::GlobalApp, "global_app"),
            (RealtimeChannel::HomecliAgent, "homecli_agent"),
            (RealtimeChannel::PeerRelay, "peer_relay"),
            (RealtimeChannel::ProjectWs, "project_ws"),
            (RealtimeChannel::VoiceRealtimeChat, "voice_realtime_chat"),
            (RealtimeChannel::VoiceTranscribe, "voice_transcribe"),
            (RealtimeChannel::VoiceVirtualMic, "voice_virtual_mic"),
        ];

        for (channel, expected) in cases {
            assert_eq!(channel.as_str(), expected);
        }
    }

    #[test]
    fn realtime_diagnostics_catalog_covers_channels_and_close_reasons() {
        let catalog = realtime_diagnostics_catalog();
        assert_eq!(catalog.version, "2026-07-16");

        let expected_channels = [
            RealtimeChannel::AppNotify.as_str(),
            RealtimeChannel::GlobalApp.as_str(),
            RealtimeChannel::HomecliAgent.as_str(),
            RealtimeChannel::PeerRelay.as_str(),
            RealtimeChannel::ProjectWs.as_str(),
            RealtimeChannel::VoiceRealtimeChat.as_str(),
            RealtimeChannel::VoiceTranscribe.as_str(),
            RealtimeChannel::VoiceVirtualMic.as_str(),
        ];
        for channel in expected_channels {
            let entry = catalog
                .channels
                .iter()
                .find(|entry| entry.id == channel)
                .expect("channel should exist in diagnostics catalog");
            assert!(
                !entry.entry_modules.is_empty(),
                "channel entry should name owner modules"
            );
            assert!(
                entry.metric_variant.starts_with("RealtimeChannel::"),
                "channel entry should name the metric variant"
            );
        }

        let expected_reasons = [
            "peer_closed",
            "client_control_close",
            "reader_ended",
            "peer_reader_ended",
            "reader_closed",
            "reader_shutdown",
            "read_error",
            "reader_error",
            "peer_read_error",
            "write_failed",
            "pong_write_failed",
            "writer_closed",
            "peer_write_error",
            "reader_timeout",
            "request_channel_closed",
        ];
        for reason in expected_reasons {
            let entry = catalog
                .close_reasons
                .iter()
                .find(|entry| entry.id == reason)
                .expect("close reason should exist in diagnostics catalog");
            assert!(!entry.meaning.is_empty());
            assert!(!entry.first_check.is_empty());
        }

        assert_eq!(
            close_reason_entry("read_error").alert_bucket,
            Some("read_error")
        );
        assert_eq!(
            close_reason_entry("writer_closed").alert_bucket,
            Some("write_failure")
        );
        assert_eq!(
            close_reason_entry("reader_timeout").alert_bucket,
            Some("timeout")
        );
        assert_eq!(close_reason_entry("peer_closed").alert_bucket, None);
    }

    #[test]
    fn realtime_diagnostics_catalog_serializes_for_admin_api() {
        let json = serde_json::to_value(realtime_diagnostics_catalog()).unwrap();
        assert_eq!(json["version"], "2026-07-16");
        assert!(json["channels"].as_array().unwrap().iter().any(|entry| {
            entry["id"] == "homecli_agent"
                && entry["close_reason_source"] == "AgentSessionCloseReason"
        }));
        assert!(json["close_reasons"]
            .as_array()
            .unwrap()
            .iter()
            .any(|entry| {
                entry["id"] == "peer_write_error" && entry["alert_bucket"] == "write_failure"
            }));
        assert!(json["change_rules"]
            .as_array()
            .unwrap()
            .iter()
            .any(|entry| {
                entry
                    .as_str()
                    .unwrap_or_default()
                    .contains("record_close_with_store")
            }));
    }

    #[test]
    fn realtime_diagnostics_catalog_matches_snapshot() {
        let actual = serde_json::to_value(realtime_diagnostics_catalog()).unwrap();
        let expected: serde_json::Value =
            serde_json::from_str(include_str!("realtime_diagnostics_catalog.snapshot.json"))
                .unwrap();

        assert_eq!(actual, expected);
    }

    #[test]
    fn admin_close_metrics_payload_keeps_persistent_windows_separate() {
        let _guard = counter_test_lock();
        reset_for_tests();
        let (store, path) = temp_store();
        let now = chrono::Utc::now().timestamp();

        store
            .record_realtime_close_event("global_app", "peer_closed")
            .unwrap();
        store
            .record_realtime_close_event("voice_transcribe", "read_error")
            .unwrap();
        store
            .conn()
            .unwrap()
            .execute(
                "UPDATE realtime_close_events
                 SET created_at_unix = ?1, created_at = '2000-01-01T00:00:00Z'
                 WHERE channel = 'voice_transcribe'",
                params![now - 2 * 60 * 60],
            )
            .unwrap();

        let payload = admin_close_metrics_payload(&store, now);
        assert_metric_count(&payload["metrics"], "global_app", "peer_closed", 1);
        assert_metric_count(&payload["metrics"], "voice_transcribe", "read_error", 1);
        assert_metric_count(
            &payload["windows"]["all_time"],
            "voice_transcribe",
            "read_error",
            1,
        );
        assert_metric_count(
            &payload["windows"]["last_1h"],
            "global_app",
            "peer_closed",
            1,
        );
        assert_no_metric(
            &payload["windows"]["last_1h"],
            "voice_transcribe",
            "read_error",
        );
        assert!(payload["windows"]["process"].is_array());
        assert!(payload["alerts"].is_array());

        let _ = std::fs::remove_file(path);
        reset_for_tests();
    }

    fn assert_metric_count(
        value: &serde_json::Value,
        channel: &str,
        close_reason: &str,
        expected_count: i64,
    ) {
        let rows = value.as_array().expect("metrics should be an array");
        let row = rows
            .iter()
            .find(|row| {
                row.get("channel").and_then(|value| value.as_str()) == Some(channel)
                    && row.get("close_reason").and_then(|value| value.as_str())
                        == Some(close_reason)
            })
            .expect("expected metric row");
        assert_eq!(
            row.get("count").and_then(|value| value.as_i64()),
            Some(expected_count)
        );
    }

    fn assert_no_metric(value: &serde_json::Value, channel: &str, close_reason: &str) {
        let rows = value.as_array().expect("metrics should be an array");
        assert!(!rows.iter().any(|row| {
            row.get("channel").and_then(|value| value.as_str()) == Some(channel)
                && row.get("close_reason").and_then(|value| value.as_str()) == Some(close_reason)
        }));
    }

    fn close_reason_entry(id: &str) -> RealtimeDiagnosticCloseReason {
        realtime_diagnostics_catalog()
            .close_reasons
            .iter()
            .find(|entry| entry.id == id)
            .copied()
            .expect("close reason should exist")
    }
}
