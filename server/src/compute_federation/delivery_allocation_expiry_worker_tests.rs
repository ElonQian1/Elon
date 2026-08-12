use super::{interval_secs, run_cycle, DEFAULT_INTERVAL_SECS, PAGE_LIMIT};
use crate::store::Store;

#[test]
fn worker_configuration_is_bounded_and_fail_closed() {
    assert_eq!(PAGE_LIMIT, 100);
    assert_eq!(interval_secs(None), DEFAULT_INTERVAL_SECS);
    assert_eq!(interval_secs(Some("invalid")), DEFAULT_INTERVAL_SECS);
    assert_eq!(interval_secs(Some("9")), DEFAULT_INTERVAL_SECS);
    assert_eq!(interval_secs(Some(" 10 ")), 10);
}

#[test]
fn one_cycle_on_an_empty_store_returns_an_empty_aggregate() {
    let path = std::env::temp_dir().join(format!(
        "elon-delivery-allocation-expiry-worker-{}.sqlite",
        uuid::Uuid::new_v4().simple()
    ));
    let store = Store::open(&path).unwrap();

    let report = run_cycle(&store).unwrap();

    assert_eq!(report.selected_count, 0);
    assert_eq!(report.expired_count, 0);
    assert_eq!(report.replayed_count, 0);
    assert_eq!(report.blocked_count, 0);
    assert_eq!(report.failed_count, 0);
    assert!(report.sweep_completed);
    assert_eq!(report.checkpoint_effect, "cleared");

    drop(store);
    let _ = std::fs::remove_file(path);
}
