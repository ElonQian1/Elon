use serde_json::{Map, Number, Value};

const RICH_SCHEMA: &str = "yilong.rich-content.v1";

pub(super) fn sanitize_rich_card(part: &Map<String, Value>) -> Option<Value> {
    let rich = part.get("richContent")?.as_object()?;
    if rich.get("schema").and_then(Value::as_str) != Some(RICH_SCHEMA) {
        return None;
    }
    let kind = rich.get("kind").and_then(Value::as_str)?;
    let source = rich.get("source").and_then(Value::as_str)?;
    if !matches!(source, "official_dom" | "private_response" | "cache") {
        return None;
    }
    let payload_source = rich.get("payload")?.as_object()?;
    let payload = match kind {
        "finance" => sanitize_finance_payload(payload_source),
        "weather" => sanitize_weather_payload(payload_source),
        _ => None,
    }?;
    let text = super::clean_string(part.get("text"), 180);
    let title = payload
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let mut sanitized_rich = Map::new();
    sanitized_rich.insert("schema".into(), Value::String(RICH_SCHEMA.into()));
    sanitized_rich.insert("kind".into(), Value::String(kind.into()));
    sanitized_rich.insert("source".into(), Value::String(source.into()));
    sanitized_rich.insert("payload".into(), Value::Object(payload));
    Some(Value::Object(Map::from_iter([
        ("type".into(), Value::String("rich_card".into())),
        (
            "text".into(),
            Value::String(if text.is_empty() { title } else { text }),
        ),
        ("kind".into(), Value::String(kind.into())),
        ("richContent".into(), Value::Object(sanitized_rich)),
    ])))
}

fn sanitize_weather_payload(source: &Map<String, Value>) -> Option<Map<String, Value>> {
    let title = super::clean_string(source.get("title"), 120);
    if title.is_empty() {
        return None;
    }
    let rows = source
        .get("rows")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .take(24)
        .filter_map(|value| {
            let row = value.as_object()?;
            let period = super::clean_string(row.get("period"), 48);
            let condition = super::clean_string(row.get("condition"), 64);
            let temperature = super::clean_string(row.get("temperature"), 32);
            if period.is_empty() || condition.is_empty() || temperature.is_empty() {
                return None;
            }
            let mut sanitized = Map::from_iter([
                ("period".into(), Value::String(period)),
                ("condition".into(), Value::String(condition)),
                ("temperature".into(), Value::String(temperature)),
            ]);
            insert_optional_text(&mut sanitized, row, "precipitation", 32);
            insert_optional_text(&mut sanitized, row, "wind", 48);
            Some(Value::Object(sanitized))
        })
        .collect::<Vec<_>>();
    if rows.is_empty() {
        return None;
    }
    let mut payload = Map::from_iter([
        ("title".into(), Value::String(title)),
        ("rows".into(), Value::Array(rows)),
    ]);
    insert_optional_text(&mut payload, source, "summary", 240);
    Some(payload)
}

fn sanitize_finance_payload(source: &Map<String, Value>) -> Option<Map<String, Value>> {
    let title = super::clean_string(source.get("title"), 120);
    let primary_value = super::clean_string(source.get("primaryValue"), 64);
    if title.is_empty() || primary_value.is_empty() {
        return None;
    }
    let mut payload = Map::new();
    payload.insert("title".into(), Value::String(title));
    payload.insert("primaryValue".into(), Value::String(primary_value));
    insert_optional_text(&mut payload, source, "symbol", 24);
    insert_optional_text(&mut payload, source, "secondaryValue", 96);
    let trend = source
        .get("trend")
        .and_then(Value::as_str)
        .filter(|value| matches!(*value, "positive" | "negative" | "neutral"))
        .unwrap_or("neutral");
    payload.insert("trend".into(), Value::String(trend.into()));
    insert_periods(&mut payload, source.get("periods"));
    insert_metrics(&mut payload, source.get("metrics"));
    insert_chart(&mut payload, source.get("chart"));
    Some(payload)
}

fn insert_optional_text(
    target: &mut Map<String, Value>,
    source: &Map<String, Value>,
    key: &str,
    max: usize,
) {
    let value = super::clean_string(source.get(key), max);
    if !value.is_empty() {
        target.insert(key.into(), Value::String(value));
    }
}

fn insert_periods(target: &mut Map<String, Value>, value: Option<&Value>) {
    let periods = value
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .take(12)
        .filter_map(|value| {
            let item = value.as_object()?;
            let id = super::clean_string(item.get("id"), 16);
            let label = super::clean_string(item.get("label"), 16);
            if id.is_empty()
                || label.is_empty()
                || !id
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
            {
                return None;
            }
            Some(Value::Object(Map::from_iter([
                ("id".into(), Value::String(id)),
                ("label".into(), Value::String(label)),
                (
                    "selected".into(),
                    Value::Bool(
                        item.get("selected")
                            .and_then(Value::as_bool)
                            .unwrap_or(false),
                    ),
                ),
            ])))
        })
        .collect::<Vec<_>>();
    if !periods.is_empty() {
        target.insert("periods".into(), Value::Array(periods));
    }
}

fn insert_metrics(target: &mut Map<String, Value>, value: Option<&Value>) {
    let metrics = value
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .take(16)
        .filter_map(|value| {
            let item = value.as_object()?;
            let label = super::clean_string(item.get("label"), 64);
            let value = super::clean_string(item.get("value"), 96);
            (!label.is_empty() && !value.is_empty()).then(|| {
                Value::Object(Map::from_iter([
                    ("label".into(), Value::String(label)),
                    ("value".into(), Value::String(value)),
                ]))
            })
        })
        .collect::<Vec<_>>();
    if !metrics.is_empty() {
        target.insert("metrics".into(), Value::Array(metrics));
    }
}

fn insert_chart(target: &mut Map<String, Value>, value: Option<&Value>) {
    let Some(chart) = value.and_then(Value::as_object) else {
        return;
    };
    if chart.get("kind").and_then(Value::as_str) != Some("line") {
        return;
    }
    let points = chart
        .get("points")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .take(512)
        .filter_map(|value| {
            let point = value.as_object()?;
            let x = super::clean_string(point.get("x"), 64);
            let y = point.get("y").and_then(Value::as_f64)?;
            let y = Number::from_f64(y)?;
            (!x.is_empty()).then(|| {
                Value::Object(Map::from_iter([
                    ("x".into(), Value::String(x)),
                    ("y".into(), Value::Number(y)),
                ]))
            })
        })
        .collect::<Vec<_>>();
    if points.len() > 1 {
        target.insert(
            "chart".into(),
            Value::Object(Map::from_iter([
                ("kind".into(), Value::String("line".into())),
                ("points".into(), Value::Array(points)),
            ])),
        );
    }
}
