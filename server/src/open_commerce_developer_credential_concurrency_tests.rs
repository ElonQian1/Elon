use std::{
    collections::HashSet,
    sync::{Arc, Barrier},
    thread,
};

use chrono::{Duration, Utc};

use crate::{
    open_commerce_developer_production_test_support::approved_developer_fixture_for, store::Store,
};

#[test]
fn concurrent_production_credential_rotations_linearize_across_connections() {
    let fixture = approved_developer_fixture_for(
        "consumer.production.concurrent",
        &["menu.preview", "order.commit"],
    );
    let stores = (0..4)
        .map(|_| Store::open(&fixture.database_path).expect("race connection should open"))
        .collect::<Vec<_>>();
    let barrier = Arc::new(Barrier::new(stores.len() + 1));
    let expires_at = (Utc::now() + Duration::days(30)).to_rfc3339();

    let writes = stores
        .into_iter()
        .enumerate()
        .map(|(index, store)| {
            let barrier = Arc::clone(&barrier);
            let app = fixture.app.clone();
            let admission_id = fixture.admission_id.clone();
            let expires_at = expires_at.clone();
            thread::spawn(move || {
                barrier.wait();
                store.issue_open_commerce_developer_production_credential(
                    &app,
                    &admission_id,
                    &["menu.preview".to_string()],
                    &format!("concurrent-reviewer-{index}"),
                    &expires_at,
                )
            })
        })
        .collect::<Vec<_>>();

    barrier.wait();
    let secrets = writes
        .into_iter()
        .map(|write| {
            write
                .join()
                .expect("credential race thread should finish")
                .expect("credential rotations should serialize without lock failures")
        })
        .collect::<Vec<_>>();
    assert_eq!(secrets.len(), 4);
    assert_eq!(
        secrets
            .iter()
            .map(|secret| secret.live_token.as_str())
            .collect::<HashSet<_>>()
            .len(),
        4
    );

    let credentials = fixture
        .store
        .list_open_commerce_developer_production_credentials(
            &fixture.project_id,
            &fixture.app.id,
            20,
        )
        .expect("concurrent credential history should be readable");
    assert_eq!(credentials.len(), 4);
    assert_eq!(
        credentials
            .iter()
            .filter(|credential| credential.status == "active")
            .count(),
        1
    );
    let revoked = credentials
        .iter()
        .filter(|credential| credential.status == "revoked")
        .collect::<Vec<_>>();
    assert_eq!(revoked.len(), 3);
    assert!(revoked.iter().all(|credential| {
        credential.revocation_reason.as_deref() == Some("credential_rotated")
            && credential.revoked_at.is_some()
    }));
}
