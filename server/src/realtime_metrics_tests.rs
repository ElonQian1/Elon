use super::*;
use rusqlite::params;
use std::sync::{Mutex, MutexGuard, OnceLock};
use uuid::Uuid;

static COUNTER_TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

fn counter_test_lock() -> MutexGuard<'static, ()> {
    COUNTER_TEST_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn temp_store() -> (crate::store::Store, std::path::PathBuf) {
    let path = std::env::temp_dir().join(format!(
        "elon_realtime_metrics_{}.db",
        Uuid::new_v4().simple()
    ));
    (
        crate::store::Store::open(&path).expect("store should open"),
        path,
    )
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
        entry["id"] == "homecli_agent" && entry["close_reason_source"] == "AgentSessionCloseReason"
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
        serde_json::from_str(include_str!("realtime_diagnostics_catalog.snapshot.json")).unwrap();

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
                && row.get("close_reason").and_then(|value| value.as_str()) == Some(close_reason)
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
