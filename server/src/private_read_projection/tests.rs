use super::*;
use storage::{migrate, put, Binding};
fn sample(at: u64) -> Projection {
    let mut value = Projection {
        schema: "yilong.private_read_projection.v1".into(),
        source: SOURCE.into(),
        connection_id: "synthetic".into(),
        generation: 1,
        revision: String::new(),
        observed_at_ms: at,
        fresh_until_ms: at + 60_000,
        status: "fresh".into(),
        payload: json!({"schema":"yilong.quant.binance_grid_snapshot.v1","bots":[]}),
    };
    value.revision = value.expected_revision().unwrap();
    value
}
#[test]
fn rejects_duplicate_credentials_future_and_revision_mutation() {
    let at = 1_780_000_000_000;
    let value = sample(at);
    let body = serde_json::to_vec(&value).unwrap();
    assert!(parse(&body, at).is_ok());
    assert!(parse(
        format!(
            "{{\"generation\":2,{}",
            &String::from_utf8(body).unwrap()[1..]
        )
        .as_bytes(),
        at
    )
    .is_err());
    let mut altered = value.clone();
    altered.payload["password"] = json!("synthetic");
    altered.revision = altered.expected_revision().unwrap();
    assert!(altered.validate(at).is_err());
    let mut future = sample(at + 30_001);
    assert!(future.validate(at).is_err());
    future.observed_at_ms = at;
    assert!(future.validate(at).is_err());
}
#[test]
fn durable_outbox_is_idempotent_ordered_and_bound_to_credentials() {
    let mut conn = rusqlite::Connection::open_in_memory().unwrap();
    migrate(&conn).unwrap();
    let binding = Binding {
        owner: "a",
        node: "node",
        install: "install",
        credential_hash: "credential1",
    };
    let value = sample(1_780_000_000_000);
    let tx = conn.transaction().unwrap();
    assert!(put(&tx, &binding, &value, value.observed_at_ms).unwrap());
    assert!(!put(&tx, &binding, &value, value.observed_at_ms + 1).unwrap());
    tx.commit().unwrap();
    assert_eq!(storage::pending(&conn, &binding).unwrap().len(), 1);
    let other = Binding {
        credential_hash: "credential2",
        ..binding
    };
    assert!(storage::pending(&conn, &other).unwrap().is_empty());
    let older = sample(value.observed_at_ms - 1);
    assert!(put(&conn, &binding, &older, older.observed_at_ms).is_err());
    let mut newer = sample(value.observed_at_ms + 1);
    newer.generation = 2;
    newer.revision = newer.expected_revision().unwrap();
    put(&conn, &binding, &newer, newer.observed_at_ms).unwrap();
    storage::settle(
        &conn,
        &binding,
        &value,
        newer.observed_at_ms,
        true,
        false,
        None,
    )
    .unwrap();
    assert_eq!(storage::pending(&conn, &binding).unwrap().len(), 1);
    storage::settle(
        &conn,
        &binding,
        &newer,
        newer.observed_at_ms,
        true,
        false,
        None,
    )
    .unwrap();
    assert!(storage::pending(&conn, &binding).unwrap().is_empty());
}
#[test]
fn stale_read_never_refreshes_evidence_timestamp_or_revision() {
    let value = sample(1_780_000_000_000);
    let projected = value.projected("opaque", value.fresh_until_ms);
    assert_eq!(projected["status"], "fresh"); // Consumer derives stale from the original expiry.
    assert_eq!(projected["revision"], value.revision);
    assert_eq!(projected["observed_at_ms"], value.observed_at_ms);
}

#[test]
fn owner_and_node_sources_are_isolated_and_rolled_back_before_commit() {
    let mut conn = rusqlite::Connection::open_in_memory().unwrap();
    migrate(&conn).unwrap();
    let value = sample(1_780_000_000_000);
    let a = Binding {
        owner: "alice",
        node: "node-a",
        install: "install-a",
        credential_hash: "hash-a",
    };
    let b = Binding {
        owner: "bob",
        node: "node-b",
        install: "install-b",
        credential_hash: "hash-b",
    };
    {
        let tx = conn.transaction().unwrap();
        put(&tx, &a, &value, value.observed_at_ms).unwrap();
    }
    assert!(storage::pending(&conn, &a).unwrap().is_empty());
    put(&conn, &a, &value, value.observed_at_ms).unwrap();
    assert!(storage::pending(&conn, &b).unwrap().is_empty());
    put(&conn, &b, &value, value.observed_at_ms).unwrap();
    assert_ne!(a.projection_id(&value), b.projection_id(&value));
    storage::settle(&conn, &b, &value, value.observed_at_ms, true, false, None).unwrap();
    assert_eq!(storage::pending(&conn, &a).unwrap().len(), 1);
}

#[test]
fn source_limit_and_equal_timestamp_conflicts_never_silently_truncate() {
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    migrate(&conn).unwrap();
    let binding = Binding {
        owner: "alice",
        node: "node",
        install: "install",
        credential_hash: "hash",
    };
    let at = 1_780_000_000_000;
    for index in 0..MAX_SOURCES {
        let mut value = sample(at);
        value.connection_id = format!("source-{index}");
        value.revision = value.expected_revision().unwrap();
        put(&conn, &binding, &value, at).unwrap();
    }
    assert!(put(&conn, &binding, &sample(at), at).is_err());
    let mut conflict = sample(at);
    conflict.connection_id = "source-0".into();
    conflict.status = "stale".into();
    conflict.revision = conflict.expected_revision().unwrap();
    assert!(put(&conn, &binding, &conflict, at).is_err());
    assert_eq!(
        storage::pending(&conn, &binding).unwrap().len(),
        MAX_SOURCES
    );
}

#[test]
fn nested_duplicate_metadata_and_corrupt_persistence_are_rejected() {
    let at = 1_780_000_000_000;
    let mut value = sample(at);
    value.payload["bots"] = json!([{"bot":{"symbol":"SYNTHETIC","authorization":"synthetic"},"provider_status":"working","detail_available":false}]);
    value.revision = value.expected_revision().unwrap();
    assert_eq!(value.validate(at), Err("projection_credentials_forbidden"));
    let mut clean = sample(at);
    clean.payload["bots"] = json!([{"bot":{"symbol":"SYNTHETIC"},"provider_status":"working","detail_available":false}]);
    clean.revision = clean.expected_revision().unwrap();
    let text = serde_json::to_string(&clean).unwrap().replace(
        "\"symbol\":\"SYNTHETIC\"",
        "\"symbol\":\"SYNTHETIC\",\"symbol\":\"other\"",
    );
    assert_eq!(
        parse(text.as_bytes(), at).err(),
        Some("projection_invalid_json")
    );
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    migrate(&conn).unwrap();
    let binding = Binding {
        owner: "alice",
        node: "node",
        install: "install",
        credential_hash: "hash",
    };
    put(&conn, &binding, &clean, at).unwrap();
    conn.execute(
        "UPDATE private_read_projection_heads SET revision='corrupt'",
        [],
    )
    .unwrap();
    assert!(storage::pending(&conn, &binding).is_err());
}

#[test]
fn acknowledgments_cannot_settle_a_different_connection_or_revision() {
    let value = sample(1_780_000_000_000);
    let ack = json!({"schema":"yilong.private_read_projection.ack.v1","revision":value.revision,
        "connection_id":value.connection_id,"status":"accepted"});
    assert!(transport::validate_ack(&serde_json::to_vec(&ack).unwrap(), &value).is_ok());
    let mut wrong = ack.clone();
    wrong["connection_id"] = json!("other");
    assert!(transport::validate_ack(&serde_json::to_vec(&wrong).unwrap(), &value).is_err());
    wrong = ack.clone();
    wrong["revision"] = json!("0".repeat(64));
    assert!(transport::validate_ack(&serde_json::to_vec(&wrong).unwrap(), &value).is_err());
    wrong = ack;
    wrong["status"] = json!("queued");
    assert!(transport::validate_ack(&serde_json::to_vec(&wrong).unwrap(), &value).is_err());
}

#[test]
fn newer_detail_batch_can_keep_list_time_but_never_extend_its_freshness() {
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    migrate(&conn).unwrap();
    let binding = Binding {
        owner: "alice",
        node: "node",
        install: "install",
        credential_hash: "hash",
    };
    let mut original = sample(1_780_000_000_000);
    original.payload["bots"] =
        json!([{"bot":{"id":"synthetic"},"provider_status":"WORKING","detail_available":false}]);
    original.revision = original.expected_revision().unwrap();
    put(&conn, &binding, &original, original.observed_at_ms).unwrap();
    let mut detailed = original.clone();
    detailed.generation += 1;
    detailed.payload["bots"][0]["detail_available"] = json!(true);
    detailed.revision = detailed.expected_revision().unwrap();
    assert!(put(&conn, &binding, &detailed, detailed.observed_at_ms + 1000).unwrap());
    assert_eq!(detailed.fresh_until_ms, original.fresh_until_ms);
    let mut extended = detailed.clone();
    extended.generation += 1;
    extended.fresh_until_ms += 1;
    extended.revision = extended.expected_revision().unwrap();
    assert!(put(&conn, &binding, &extended, extended.observed_at_ms + 1000).is_err());
    assert!(put(&conn, &binding, &original, original.observed_at_ms + 2000).is_err());
    let mut old_generation = detailed.clone();
    old_generation.generation = 1;
    old_generation.observed_at_ms += 2000;
    old_generation.revision = old_generation.expected_revision().unwrap();
    assert!(put(
        &conn,
        &binding,
        &old_generation,
        old_generation.observed_at_ms
    )
    .is_err());
}

#[test]
fn real_kotlin_synthetic_export_matches_rust_canonical_digest_and_projection() {
    let exported: Value = serde_json::from_str(include_str!(
        "../../tests/private-read-projection-harness/fixtures/android-grid-read-wire.json"
    ))
    .unwrap();
    let expected = exported["snapshots"][0].clone();
    let mut wire = expected.clone();
    wire.as_object_mut().unwrap().remove("projection_id");
    let projection = parse(&serde_json::to_vec(&wire).unwrap(), 1_780_000_000_000).unwrap();
    assert_eq!(
        projection.revision,
        "e9a1bd2c5dfd79b1e5e38fe7902ba5550439a93d060ad95178f75314308f9b31"
    );
    assert_eq!(
        projection.projected(
            expected["projection_id"].as_str().unwrap(),
            1_780_000_000_000
        ),
        expected
    );
    assert_eq!(
        projection.payload["bots"][0]["bot"]["metrics"]["exchange_reported_profit"]["value"],
        "0.000000000000000001"
    );
}
