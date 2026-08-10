use super::*;
use std::{sync::mpsc, time::Duration};
use uuid::Uuid;

#[test]
fn platform_summary_releases_connection_before_nested_aggregates() {
    let path =
        std::env::temp_dir().join(format!("elon_admin_stats_{}.db", Uuid::new_v4().simple()));
    let store = Store::open(&path).expect("store should open");
    let (result_tx, result_rx) = mpsc::channel();

    std::thread::spawn(move || {
        let result = store.admin_platform_summary(7);
        let _ = result_tx.send(result);
    });

    let summary = result_rx
        .recv_timeout(Duration::from_secs(2))
        .expect("platform summary must not deadlock on the Store connection")
        .expect("platform summary should load");
    assert_eq!(summary.period_days, 7);
    assert_eq!(summary.total_tokens_period, 0);

    let _ = std::fs::remove_file(path);
}
