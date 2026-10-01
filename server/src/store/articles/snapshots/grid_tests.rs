use super::*;
use serde_json::json;

fn doc() -> SnapshotDocument {
    let grid: grid::GridShare = serde_json::from_value(json!({"schema":"yilong.grid_share.v1",
        "observed_at_ms":chrono::Utc::now().timestamp_millis(), "show_amounts":false,
        "fields":{"symbol":"TESTUSDT","direction":"SHORT","leverage":"4","count":"60","lower":"0.1","upper":"0.2"}})).unwrap();
    SnapshotDocument {
        schema: SCHEMA.into(),
        provider: "binance".into(),
        title: grid.title(),
        summary: grid.summary(),
        cover_asset_id: None,
        messages: vec![],
        grid: Some(grid),
    }
}

#[test]
fn grid_share_rejects_private_fields_hidden_amounts_invalid_values_and_stale_reads() {
    assert!(doc().validate().is_ok());
    for (key, value) in [
        ("account", "42"),
        ("id", "333"),
        ("Cookie", "secret"),
        ("positionQty", "42"),
        ("profit", "5"),
        ("lower", "NaN"),
        ("count", "1.5"),
    ] {
        let mut d = doc();
        d.grid
            .as_mut()
            .unwrap()
            .fields
            .insert(key.into(), value.into());
        assert!(d.validate().is_err(), "{key}");
    }
    let mut d = doc();
    d.grid.as_mut().unwrap().observed_at_ms -= 301_000;
    assert!(tests::fixture()
        .create_ai_snapshot("author", "g1", "stale-grid", d)
        .is_err());
    let mut d = doc();
    d.grid.as_mut().unwrap().show_amounts = true;
    d.grid
        .as_mut()
        .unwrap()
        .fields
        .insert("profit".into(), "-5.5".into());
    assert!(d.validate().is_ok());
    let mut d = doc();
    d.messages = tests::document().messages;
    assert!(d.validate().is_err());
}

#[test]
fn public_grid_snapshot_has_member_acl_idempotence_and_authoritative_ai_context() {
    let store = tests::fixture();
    let document = doc();
    let created = store
        .create_ai_snapshot("author", "g1", "grid-key-1", document.clone())
        .unwrap();
    let again = store
        .create_ai_snapshot("author", "g1", "grid-key-1", document)
        .unwrap();
    assert!(again.replayed);
    assert_eq!(again.message.id, created.message.id);
    assert!(store
        .read_ai_snapshot("other", "g1", &created.snapshot_id)
        .is_err());
    assert!(store
        .read_ai_snapshot("author", "g2", &created.snapshot_id)
        .is_err());
    let view = store
        .read_ai_snapshot("reader", "g1", &created.snapshot_id)
        .unwrap();
    let serialized = serde_json::to_string(&view).unwrap();
    assert!(!serialized.contains("positionQty"));
    assert!(!serialized.contains("investment"));
    {
        let conn = store.conn().unwrap();
        let value = grid_versions::ai_context(
            &conn,
            "reader",
            "g1",
            &created.message.id,
            &created.message.content,
        )
        .unwrap()
        .unwrap();
        assert_eq!(value["grid"]["fields"]["symbol"], "TESTUSDT");
        assert!(grid_versions::ai_context(
            &conn,
            "other",
            "g1",
            &created.message.id,
            &created.message.content
        )
        .is_err());
        assert!(grid_versions::ai_context(
            &conn,
            "reader",
            "g1",
            "not-a-message",
            &created.message.content
        )
        .is_err());
    }
    store
        .revoke_ai_snapshot("author", "g1", &created.snapshot_id)
        .unwrap();
    assert!(store
        .read_ai_snapshot("reader", "g1", &created.snapshot_id)
        .is_err());
    let conn = store.conn().unwrap();
    assert!(grid_versions::ai_context(
        &conn,
        "reader",
        "g1",
        &created.message.id,
        &created.message.content
    )
    .is_err());
}

#[test]
fn updates_are_immutable_same_owner_same_group_and_have_no_forks() {
    let store = tests::fixture();
    let first = store
        .create_ai_snapshot("author", "g1", "grid-key-1", doc())
        .unwrap();
    let mut next = doc();
    next.grid.as_mut().unwrap().previous_snapshot_id = Some(first.snapshot_id.clone());
    assert!(store
        .create_ai_snapshot("reader", "g1", "grid-key-2", next.clone())
        .is_err());
    assert!(store
        .create_ai_snapshot("author", "g2", "grid-key-2", next.clone())
        .is_err());
    let second = store
        .create_ai_snapshot("author", "g1", "grid-key-2", next.clone())
        .unwrap();
    assert!(store
        .create_ai_snapshot("author", "g1", "grid-key-3", next.clone())
        .is_err());
    assert!(
        store
            .create_ai_snapshot("author", "g1", "grid-key-2", next)
            .unwrap()
            .replayed
    );
    let original = store
        .read_ai_snapshot("reader", "g1", &first.snapshot_id)
        .unwrap();
    assert_eq!(original.latest_snapshot_id, Some(second.snapshot_id));
    assert!(original
        .document
        .grid
        .unwrap()
        .previous_snapshot_id
        .is_none());
}
