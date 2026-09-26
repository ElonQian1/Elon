use super::*;
use serde_json::json;

fn command(detail: bool) -> ResearchCommand {
    serde_json::from_value(if detail {
        json!({"kind":"binance_grid_detail","request_id":"fixture_read_1","resource_id":"123"})
    } else {
        json!({"kind":"binance_grid_list","request_id":"fixture_read_1","limit":1})
    })
    .unwrap()
}
fn row(detail: bool) -> Value {
    let mut value = serde_json::Map::new();
    for field in if detail { DETAIL_FIELDS } else { LIST_FIELDS } {
        value.insert((*field).into(), Value::Null);
    }
    value.insert("id".into(), json!("123"));
    value.insert("symbol".into(), json!("龙虾USDT"));
    value.insert("lower".into(), json!("0.103280000000000001"));
    Value::Object(value)
}
fn receipt(detail: bool) -> Value {
    let mut reader = json!({"schema":"yilong.binance-grid-read.v1","request_id":"fixture_read_1",
        "status":"ready","observed_at_ms":1900000000000u64,"expires_at_ms":1900000300000u64});
    if detail {
        reader["row"] = row(true);
    } else {
        reader["items"] = json!([row(false)]);
        reader["total"] = json!(2);
        reader["offset"] = json!(0);
        reader["next_offset"] = json!(1);
    }
    json!({"schema":"yilong.browser-research.result.v1","kind":command(detail).kind,"reader":reader})
}
#[test]
fn grid_commands_accept_only_fixed_read_contract() {
    for detail in [false, true] {
        let c = command(detail);
        assert!(c.validate().is_ok());
        let mut start = c.clone();
        start.query = Some("start".into());
        assert!(start.validate().is_ok());
        for (key, value) in [
            ("query", json!("fetch('/trade')")),
            ("site_id", json!("other")),
            ("session_id", json!("another")),
            ("request_id", json!("tiny")),
            ("limit", json!(51)),
        ] {
            let mut raw = serde_json::to_value(&c).unwrap();
            raw[key] = value;
            assert!(
                serde_json::from_value::<ResearchCommand>(raw)
                    .unwrap()
                    .validate()
                    .is_err(),
                "{key}"
            );
        }
    }
    for id in [
        "0",
        "0123",
        "1e3",
        "https://example.org",
        "123456789012345678901",
    ] {
        let mut c = command(true);
        c.resource_id = Some(id.into());
        assert!(c.validate().is_err());
    }
}
#[test]
fn grid_receipts_bind_request_strategy_page_and_exact_fields() {
    for detail in [false, true] {
        let c = command(detail);
        let good = receipt(detail);
        assert_eq!(validate_response(&c, &good), Ok(()));
        for (pointer, value) in [
            ("/kind", json!("sites")),
            ("/reader/request_id", json!("other_read_1")),
            ("/reader/expires_at_ms", json!(1900000300001u64)),
            ("/reader/observed_at_ms", json!(0)),
        ] {
            let mut bad = good.clone();
            *bad.pointer_mut(pointer).unwrap() = value;
            assert_eq!(validate_response(&c, &bad), Err("invalid_result"));
        }
        let mut bad = good.clone();
        bad["reader"]["cookie"] = json!("private");
        assert!(validate_response(&c, &bad).is_err());
        let mut bad = good.clone();
        let selected = if detail {
            &mut bad["reader"]["row"]
        } else {
            &mut bad["reader"]["items"][0]
        };
        selected["account"] = json!("100");
        assert!(validate_response(&c, &bad).is_err());
        let mut bad = good.clone();
        let selected = if detail {
            &mut bad["reader"]["row"]
        } else {
            &mut bad["reader"]["items"][0]
        };
        selected["lower"] = json!(0.10328);
        assert!(validate_response(&c, &bad).is_err());
    }
    let mut bad = receipt(true);
    bad["reader"]["row"]["id"] = json!("456");
    assert!(validate_response(&command(true), &bad).is_err());
    for (key, value) in [
        ("offset", json!(1)),
        ("next_offset", Value::Null),
        ("total", json!(501)),
    ] {
        let mut bad = receipt(false);
        bad["reader"][key] = value;
        assert!(validate_response(&command(false), &bad).is_err());
    }
    let mut c = command(false);
    c.limit = Some(2);
    let mut bad = receipt(false);
    bad["reader"]["items"] = json!([row(false), row(false)]);
    bad["reader"]["next_offset"] = Value::Null;
    assert!(validate_response(&c, &bad).is_err());
}
#[test]
fn grid_pending_and_failures_cannot_smuggle_private_body() {
    let c = command(false);
    let mut value = receipt(false);
    value["reader"] = json!({"schema":"yilong.binance-grid-read.v1","request_id":"fixture_read_1","status":"pending"});
    assert!(validate_response(&c, &value).is_ok());
    value["reader"]["body"] = json!("private");
    assert!(validate_response(&c, &value).is_err());
    value["reader"].as_object_mut().unwrap().remove("body");
    value["reader"]["status"] = json!("failed");
    value["reader"]["error"] = json!("context_changed");
    assert!(validate_response(&c, &value).is_ok());
    value["reader"]["error"] = json!("raw web error");
    assert!(validate_response(&c, &value).is_err());
}
