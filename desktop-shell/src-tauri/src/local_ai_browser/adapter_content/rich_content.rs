use serde_json::{Map, Number, Value};

const RICH_SCHEMA: &str = "yilong.rich-content.v1";

pub(super) fn sanitize_rich_card(provider_id: &str, part: &Map<String, Value>) -> Option<Value> {
    let rich = part.get("richContent")?.as_object()?;
    if rich.get("schema").and_then(Value::as_str) != Some(RICH_SCHEMA) {
        return None;
    }
    let kind = rich.get("kind").and_then(Value::as_str)?;
    let source = rich.get("source").and_then(Value::as_str)?;
    if !matches!(source, "official_dom" | "private_response" | "cache")
        || (source == "private_response"
            && !crate::local_ai_browser::private_response_authorization::allows_rich_kind(
                provider_id,
                kind,
            ))
    {
        return None;
    }
    let payload_source = rich.get("payload")?.as_object()?;
    let payload = match kind {
        "finance" => sanitize_finance_payload(payload_source),
        "chart" => sanitize_chart_payload(payload_source),
        "weather" => sanitize_weather_payload(payload_source),
        "media_gallery" => sanitize_media_gallery_payload(payload_source),
        "map" => sanitize_map_payload(payload_source),
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

fn sanitize_chart_payload(source: &Map<String, Value>) -> Option<Map<String, Value>> {
    let title = super::clean_string(source.get("title"), 120);
    if title.is_empty() || source.get("chartType").and_then(Value::as_str) != Some("line") {
        return None;
    }
    let series = source
        .get("series")
        .and_then(Value::as_array)?
        .iter()
        .take(4)
        .filter_map(|value| {
            let item = value.as_object()?;
            let key = super::clean_string(item.get("key"), 48);
            let label = super::clean_string(item.get("label"), 64);
            if key.is_empty()
                || label.is_empty()
                || !key.bytes().all(|byte| {
                    byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-')
                })
            {
                return None;
            }
            let mut sanitized = Map::from_iter([
                ("key".into(), Value::String(key)),
                ("label".into(), Value::String(label)),
            ]);
            insert_optional_text(&mut sanitized, item, "valuePrefix", 16);
            insert_optional_text(&mut sanitized, item, "valueSuffix", 16);
            Some(Value::Object(sanitized))
        })
        .collect::<Vec<_>>();
    if series.is_empty() {
        return None;
    }
    let series_count = series.len();
    let points = source
        .get("points")
        .and_then(Value::as_array)?
        .iter()
        .take(256)
        .filter_map(|value| {
            let point = value.as_object()?;
            let x = super::clean_string(point.get("x"), 64);
            let values = point.get("values").and_then(Value::as_array)?;
            if x.is_empty() || values.len() != series_count {
                return None;
            }
            let values = values
                .iter()
                .map(|value| Number::from_f64(value.as_f64()?))
                .collect::<Option<Vec<_>>>()?;
            Some(Value::Object(Map::from_iter([
                ("x".into(), Value::String(x)),
                (
                    "values".into(),
                    Value::Array(values.into_iter().map(Value::Number).collect()),
                ),
            ])))
        })
        .collect::<Vec<_>>();
    if points.len() < 2 {
        return None;
    }
    let mut payload = Map::from_iter([
        ("title".into(), Value::String(title)),
        ("chartType".into(), Value::String("line".into())),
        ("series".into(), Value::Array(series)),
        ("points".into(), Value::Array(points)),
    ]);
    insert_optional_text(&mut payload, source, "description", 240);
    Some(payload)
}

fn sanitize_media_gallery_payload(source: &Map<String, Value>) -> Option<Map<String, Value>> {
    let title = super::clean_string(source.get("title"), 120);
    if title.is_empty() {
        return None;
    }
    let mut seen = std::collections::HashSet::new();
    let items = source
        .get("items")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .take(8)
        .filter_map(|value| {
            let item = value.as_object()?;
            let url = super::sanitize_public_url(item.get("url"));
            let alt = super::clean_string(item.get("alt"), 180);
            if url.is_empty() || alt.is_empty() || !seen.insert(url.clone()) {
                return None;
            }
            let mut sanitized = Map::from_iter([
                ("url".into(), Value::String(url)),
                ("alt".into(), Value::String(alt)),
            ]);
            let media_type = super::clean_string(item.get("mediaType"), 48);
            if matches!(
                media_type.as_str(),
                "image/*"
                    | "image/jpeg"
                    | "image/png"
                    | "image/gif"
                    | "image/webp"
                    | "image/avif"
                    | "image/svg+xml"
            ) {
                sanitized.insert("mediaType".into(), Value::String(media_type));
            }
            insert_bounded_dimension(&mut sanitized, item, "width");
            insert_bounded_dimension(&mut sanitized, item, "height");
            let source_url = super::sanitize_public_url(item.get("sourceUrl"));
            if !source_url.is_empty() {
                sanitized.insert("sourceUrl".into(), Value::String(source_url));
            }
            Some(Value::Object(sanitized))
        })
        .collect::<Vec<_>>();
    if items.is_empty() {
        return None;
    }
    Some(Map::from_iter([
        ("title".into(), Value::String(title)),
        ("items".into(), Value::Array(items)),
    ]))
}

fn sanitize_map_payload(source: &Map<String, Value>) -> Option<Map<String, Value>> {
    let title = super::clean_string(source.get("title"), 120);
    if title.is_empty() {
        return None;
    }
    let summary = super::clean_string(source.get("summary"), 500);
    let places = source
        .get("places")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .take(12)
        .map(|value| super::clean_string(Some(value), 120))
        .filter(|value| !value.is_empty())
        .map(Value::String)
        .collect::<Vec<_>>();
    if summary.is_empty() && places.is_empty() {
        return None;
    }
    let mut payload = Map::from_iter([
        ("title".into(), Value::String(title)),
        ("places".into(), Value::Array(places)),
    ]);
    if !summary.is_empty() {
        payload.insert("summary".into(), Value::String(summary));
    }
    Some(payload)
}

fn insert_bounded_dimension(
    target: &mut Map<String, Value>,
    source: &Map<String, Value>,
    key: &str,
) {
    if let Some(value) = source
        .get(key)
        .and_then(Value::as_u64)
        .filter(|value| *value > 0 && *value <= 8_192)
    {
        target.insert(key.into(), Value::Number(value.into()));
    }
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
    let sanitized = match chart.get("kind").and_then(Value::as_str) {
        Some("line") => sanitize_line_chart(chart),
        Some("candlestick") => sanitize_candlestick_chart(chart),
        _ => None,
    };
    if let Some(chart) = sanitized {
        target.insert("chart".into(), Value::Object(chart));
    }
}

fn sanitize_line_chart(chart: &Map<String, Value>) -> Option<Map<String, Value>> {
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
    (points.len() > 1).then(|| {
        Map::from_iter([
            ("kind".into(), Value::String("line".into())),
            ("points".into(), Value::Array(points)),
        ])
    })
}

fn sanitize_candlestick_chart(chart: &Map<String, Value>) -> Option<Map<String, Value>> {
    let candles = chart
        .get("candles")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .take(512)
        .filter_map(|value| {
            let candle = value.as_object()?;
            let x = super::clean_string(candle.get("x"), 64);
            let open = candle.get("open").and_then(Value::as_f64)?;
            let high = candle.get("high").and_then(Value::as_f64)?;
            let low = candle.get("low").and_then(Value::as_f64)?;
            let close = candle.get("close").and_then(Value::as_f64)?;
            if x.is_empty()
                || !open.is_finite()
                || !high.is_finite()
                || !low.is_finite()
                || !close.is_finite()
                || high < open.max(close)
                || low > open.min(close)
            {
                return None;
            }
            Some(Value::Object(Map::from_iter([
                ("x".into(), Value::String(x)),
                ("open".into(), Value::Number(Number::from_f64(open)?)),
                ("high".into(), Value::Number(Number::from_f64(high)?)),
                ("low".into(), Value::Number(Number::from_f64(low)?)),
                ("close".into(), Value::Number(Number::from_f64(close)?)),
            ])))
        })
        .collect::<Vec<_>>();
    (!candles.is_empty()).then(|| {
        Map::from_iter([
            ("kind".into(), Value::String("candlestick".into())),
            ("candles".into(), Value::Array(candles)),
        ])
    })
}
