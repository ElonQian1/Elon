use super::*;
use rusqlite::params;
use uuid::Uuid;

fn temp_store() -> (Store, std::path::PathBuf) {
    let path = std::env::temp_dir().join(format!(
        "elon_realtime_close_events_{}.db",
        Uuid::new_v4().simple()
    ));
    (Store::open(&path).expect("store should open"), path)
}

#[test]
fn realtime_close_metrics_support_persistent_windows() {
    let (store, path) = temp_store();
    store
        .record_realtime_close_event("global_app", "peer_closed")
        .unwrap();
    store
        .record_realtime_close_event("global_app", "peer_closed")
        .unwrap();
    store
        .record_realtime_close_event("voice_transcribe", "read_error")
        .unwrap();

    let old_cutoff = chrono::Utc::now().timestamp() - 2 * 60 * 60;
    store
        .conn
        .lock()
        .unwrap()
        .execute(
            "UPDATE realtime_close_events
             SET created_at_unix = ?1, created_at = '2000-01-01T00:00:00Z'
             WHERE channel = 'voice_transcribe'",
            params![old_cutoff],
        )
        .unwrap();

    let all_time = store.admin_realtime_close_metrics_since(None).unwrap();
    assert!(all_time.iter().any(|row| {
        row.channel == "global_app" && row.close_reason == "peer_closed" && row.count == 2
    }));
    assert!(all_time.iter().any(|row| {
        row.channel == "voice_transcribe" && row.close_reason == "read_error" && row.count == 1
    }));

    let last_hour = store
        .admin_realtime_close_metrics_since(Some(chrono::Utc::now().timestamp() - 60 * 60))
        .unwrap();
    assert_eq!(last_hour.len(), 1);
    assert_eq!(last_hour[0].channel, "global_app");
    assert_eq!(last_hour[0].close_reason, "peer_closed");
    assert_eq!(last_hour[0].count, 2);

    let _ = std::fs::remove_file(path);
}

#[test]
fn realtime_close_alerts_are_scoped_to_realtime_view() {
    let (store, path) = temp_store();
    for _ in 0..21 {
        store
            .record_realtime_close_event("global_app", "read_error")
            .unwrap();
    }

    let realtime_alerts = store.refresh_realtime_close_alerts().unwrap();
    assert!(realtime_alerts.iter().any(|alert| {
        alert.fingerprint == "realtime:read-errors-last-hour"
            && alert.severity == "critical"
            && alert.metric_value == 21
    }));

    let billing_alerts = store.billing_list_alerts(false, 100).unwrap();
    assert!(
        billing_alerts
            .iter()
            .all(|alert| alert.fingerprint.starts_with("billing:")),
        "billing alert list must not include realtime alerts"
    );
    let billing_history = store.billing_list_alerts(true, 100).unwrap();
    assert!(
        billing_history
            .iter()
            .all(|alert| alert.fingerprint.starts_with("billing:")),
        "billing alert history must not include realtime alerts"
    );

    let _ = std::fs::remove_file(path);
}

#[test]
fn realtime_close_alert_thresholds_follow_admin_config() {
    let (store, path) = temp_store();
    for _ in 0..3 {
        store
            .record_realtime_close_event("global_app", "read_error")
            .unwrap();
    }

    let default_alerts = store.refresh_realtime_close_alerts().unwrap();
    assert!(
        default_alerts
            .iter()
            .all(|alert| alert.fingerprint != "realtime:read-errors-last-hour"),
        "default threshold should not alert on three read errors"
    );

    store
        .billing_set_config("realtime_close_read_error_alert_threshold_1h", "2")
        .unwrap();
    let configured_alerts = store.refresh_realtime_close_alerts().unwrap();
    let alert = configured_alerts
        .iter()
        .find(|alert| alert.fingerprint == "realtime:read-errors-last-hour")
        .expect("lowered threshold should create a realtime read-error alert");
    assert_eq!(alert.status, "open");
    assert_eq!(alert.severity, "critical");
    assert_eq!(alert.metric_value, 3);

    store
        .billing_set_config("realtime_close_read_error_alert_threshold_1h", "10")
        .unwrap();
    let raised_alerts = store.refresh_realtime_close_alerts().unwrap();
    assert!(
        raised_alerts
            .iter()
            .all(|alert| alert.fingerprint != "realtime:read-errors-last-hour"),
        "raising the threshold should clear the open realtime read-error alert"
    );
    let all_realtime_alerts = store.realtime_list_alerts(true, 100).unwrap();
    let resolved = all_realtime_alerts
        .iter()
        .find(|alert| alert.fingerprint == "realtime:read-errors-last-hour")
        .expect("resolved realtime read-error alert should remain in history");
    assert_eq!(resolved.status, "resolved");

    let _ = std::fs::remove_file(path);
}

#[test]
fn realtime_close_alert_details_include_diagnostics_first_check() {
    let (store, path) = temp_store();
    store
        .billing_set_config("realtime_close_read_error_alert_threshold_1h", "1")
        .unwrap();
    store
        .billing_set_config("realtime_close_write_failure_alert_threshold_1h", "1")
        .unwrap();
    store
        .billing_set_config("realtime_close_timeout_alert_threshold_1h", "1")
        .unwrap();

    for _ in 0..2 {
        store
            .record_realtime_close_event("global_app", "read_error")
            .unwrap();
        store
            .record_realtime_close_event("project_ws", "write_failed")
            .unwrap();
        store
            .record_realtime_close_event("homecli_agent", "reader_timeout")
            .unwrap();
    }

    let alerts = store.refresh_realtime_close_alerts().unwrap();
    let read_alert = find_realtime_alert(&alerts, "realtime:read-errors-last-hour");
    assert!(read_alert.detail.contains("首查建议："));
    assert!(read_alert.detail.contains("Check client network quality"));

    let write_alert = find_realtime_alert(&alerts, "realtime:write-failures-last-hour");
    assert!(write_alert.detail.contains("首查建议："));
    assert!(write_alert.detail.contains("Check half-open clients"));

    let timeout_alert = find_realtime_alert(&alerts, "realtime:timeouts-last-hour");
    assert!(timeout_alert.detail.contains("首查建议："));
    assert!(timeout_alert.detail.contains("Check PC sleep"));

    let _ = std::fs::remove_file(path);
}

#[test]
fn realtime_close_alert_counts_follow_diagnostics_buckets() {
    let (store, path) = temp_store();
    store
        .billing_set_config("realtime_close_read_error_alert_threshold_1h", "2")
        .unwrap();
    store
        .billing_set_config("realtime_close_write_failure_alert_threshold_1h", "2")
        .unwrap();
    store
        .billing_set_config("realtime_close_timeout_alert_threshold_1h", "1")
        .unwrap();

    store
        .record_realtime_close_event("homecli_agent", "reader_error")
        .unwrap();
    store
        .record_realtime_close_event("peer_relay", "peer_read_error")
        .unwrap();
    store
        .record_realtime_close_event("global_app", "read_error")
        .unwrap();
    store
        .record_realtime_close_event("homecli_agent", "writer_closed")
        .unwrap();
    store
        .record_realtime_close_event("peer_relay", "peer_write_error")
        .unwrap();
    store
        .record_realtime_close_event("global_app", "write_failed")
        .unwrap();
    store
        .record_realtime_close_event("homecli_agent", "reader_timeout")
        .unwrap();
    store
        .record_realtime_close_event("global_app", "peer_closed")
        .unwrap();

    let alerts = store.refresh_realtime_close_alerts().unwrap();
    assert_eq!(
        find_realtime_alert(&alerts, "realtime:read-errors-last-hour").metric_value,
        3,
        "read-error alert should count every diagnostics bucket reason"
    );
    assert_eq!(
        find_realtime_alert(&alerts, "realtime:write-failures-last-hour").metric_value,
        3,
        "write-failure alert should count every diagnostics bucket reason"
    );
    assert!(
        alerts
            .iter()
            .all(|alert| alert.fingerprint != "realtime:timeouts-last-hour"),
        "timeout threshold uses a strict greater-than comparison"
    );

    let _ = std::fs::remove_file(path);
}

fn find_realtime_alert<'a>(
    alerts: &'a [BillingAlertRow],
    fingerprint: &str,
) -> &'a BillingAlertRow {
    alerts
        .iter()
        .find(|alert| alert.fingerprint == fingerprint)
        .expect("expected realtime alert")
}
