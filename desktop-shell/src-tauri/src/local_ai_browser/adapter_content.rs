use serde_json::{json, Map, Value};

#[path = "adapter_content/rich_content.rs"]
mod rich_content;

const MAX_CONTENT_PARTS: usize = 24;
const MAX_MESSAGE_CHARS: usize = 40_000;
const MAX_STRUCTURED_LABEL_CHARS: usize = 180;
const STRUCTURED_TYPES: &[&str] = &[
    "image",
    "file",
    "citation",
    "code",
    "table",
    "artifact",
    "audio",
    "video",
    "math",
    "chart",
    "map",
    "interactive",
];

pub(super) fn sanitize_parts(provider_id: &str, value: Option<&Value>) -> Vec<Value> {
    value
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .take(MAX_CONTENT_PARTS)
        .filter_map(|value| sanitize_part(provider_id, value))
        .collect()
}

fn sanitize_part(provider_id: &str, value: &Value) -> Option<Value> {
    let part = value.as_object()?;
    let part_type = part.get("type")?.as_str()?;
    if matches!(part_type, "text" | "markdown") {
        let text = clean_string(part.get("text"), MAX_MESSAGE_CHARS);
        return (!text.is_empty()).then(|| json!({"type": part_type, "text": text}));
    }
    if part_type == "rich_card" {
        return rich_content::sanitize_rich_card(provider_id, part).or_else(|| {
            let text = clean_string(part.get("text"), MAX_STRUCTURED_LABEL_CHARS);
            (!text.is_empty()).then(|| {
                json!({
                    "type": "interactive",
                    "text": text,
                    "kind": "renderer_upgrade_required",
                })
            })
        });
    }
    if !STRUCTURED_TYPES.contains(&part_type) {
        return None;
    }
    let mut text = clean_string(part.get("text"), MAX_STRUCTURED_LABEL_CHARS);
    if text.is_empty() && part_type == "citation" {
        text = clean_string(part.get("title"), MAX_STRUCTURED_LABEL_CHARS);
    }
    if text.is_empty() {
        return None;
    }
    let mut sanitized = Map::new();
    sanitized.insert("type".into(), Value::String(part_type.to_string()));
    sanitized.insert("text".into(), Value::String(text));
    insert_token(&mut sanitized, part, "kind", 32, valid_lower_token);
    insert_token(&mut sanitized, part, "language", 32, valid_language);
    insert_token(&mut sanitized, part, "mediaType", 96, valid_media_type);
    insert_token(&mut sanitized, part, "targetKind", 16, |value| {
        matches!(value, "same_origin" | "external")
    });
    insert_token(&mut sanitized, part, "targetHost", 253, valid_host);
    insert_count(&mut sanitized, part, "lineCount", 1_000_000);
    insert_count(&mut sanitized, part, "rowCount", 10_000);
    insert_count(&mut sanitized, part, "columnCount", 10_000);
    let public_url = sanitize_public_url(part.get("url"));
    if !public_url.is_empty() {
        sanitized.insert("url".into(), Value::String(public_url));
    }
    let icon_url = sanitize_public_url(part.get("iconUrl"));
    if part_type == "citation" && !icon_url.is_empty() {
        sanitized.insert("iconUrl".into(), Value::String(icon_url));
    }
    if part_type == "citation" {
        let marker_text = clean_string(part.get("markerText"), 80);
        if !marker_text.is_empty() {
            sanitized.insert("markerText".into(), Value::String(marker_text));
        }
        insert_token(&mut sanitized, part, "citationId", 64, valid_lower_token);
        insert_count(&mut sanitized, part, "groupSize", 32);
        let snippet = clean_string(part.get("snippet"), 320);
        if !snippet.is_empty() {
            sanitized.insert("snippet".into(), Value::String(snippet));
        }
        let thumbnail_url = sanitize_public_url(part.get("thumbnailUrl"));
        if !thumbnail_url.is_empty() {
            sanitized.insert("thumbnailUrl".into(), Value::String(thumbnail_url));
        }
    }
    Some(Value::Object(sanitized))
}

fn insert_token(
    target: &mut Map<String, Value>,
    source: &Map<String, Value>,
    key: &str,
    max: usize,
    validate: impl Fn(&str) -> bool,
) {
    let value = clean_string(source.get(key), max);
    if validate(&value) {
        target.insert(key.to_string(), Value::String(value));
    }
}

fn insert_count(target: &mut Map<String, Value>, source: &Map<String, Value>, key: &str, max: u64) {
    if let Some(value) = source
        .get(key)
        .and_then(Value::as_u64)
        .filter(|value| *value > 0 && *value <= max)
    {
        target.insert(key.to_string(), Value::Number(value.into()));
    }
}

fn valid_lower_token(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
}

fn valid_language(value: &str) -> bool {
    !value.is_empty()
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'+' | b'.' | b'#' | b'-')
        })
}

fn valid_media_type(value: &str) -> bool {
    value.split_once('/').is_some()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'+' | b'-' | b'/'))
}

fn valid_host(value: &str) -> bool {
    !value.is_empty()
        && !value.contains("..")
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-'))
}

pub(super) fn sanitize_public_url(value: Option<&Value>) -> String {
    let Some(raw) = value.and_then(Value::as_str) else {
        return String::new();
    };
    let Ok(url) = raw.parse::<tauri::Url>() else {
        return String::new();
    };
    if url.scheme() != "https"
        || !url.username().is_empty()
        || url.password().is_some()
        || url.port().is_some_and(|port| port != 443)
    {
        return String::new();
    }
    let Some(host) = url.host_str().filter(|host| valid_host(host)) else {
        return String::new();
    };
    format!("https://{host}{}", url.path())
}

fn clean_string(value: Option<&Value>, max: usize) -> String {
    value
        .and_then(Value::as_str)
        .unwrap_or_default()
        .chars()
        .filter(|character| !character.is_control() || matches!(character, '\n' | '\t'))
        .take(max)
        .collect::<String>()
        .trim()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rich_parts_keep_visible_metadata_and_drop_secrets() {
        let parts = json!([
            {"type":"markdown","text":"**answer**"},
            {"type":"code","text":"Rust code","kind":"code_block","language":"rust","lineCount":12},
            {"type":"citation","text":"Docs","url":"https://example.com/docs?token=secret","iconUrl":"https://cdn.example.com/icons/docs.png?signature=secret","targetHost":"example.com","markerText":"Docs +2","citationId":"citation_control_1","groupSize":3,"snippet":"Visible public summary","thumbnailUrl":"https://cdn.example.com/thumbs/docs.jpg?signature=secret","privateField":"secret"},
            {"type":"credential","text":"secret"}
        ]);
        let sanitized = sanitize_parts("chatgpt", Some(&parts));
        assert_eq!(sanitized.len(), 3);
        assert_eq!(sanitized[0]["type"], "markdown");
        assert_eq!(sanitized[1]["lineCount"], 12);
        assert_eq!(sanitized[2]["url"], "https://example.com/docs");
        assert_eq!(
            sanitized[2]["iconUrl"],
            "https://cdn.example.com/icons/docs.png"
        );
        assert_eq!(sanitized[2]["markerText"], "Docs +2");
        assert_eq!(sanitized[2]["citationId"], "citation_control_1");
        assert_eq!(sanitized[2]["groupSize"], 3);
        assert_eq!(sanitized[2]["snippet"], "Visible public summary");
        assert_eq!(
            sanitized[2]["thumbnailUrl"],
            "https://cdn.example.com/thumbs/docs.jpg"
        );
        assert!(sanitized[2].get("privateField").is_none());
        assert!(!Value::Array(sanitized).to_string().contains("token"));
    }

    #[test]
    fn finance_rich_card_keeps_versioned_visible_data_and_bounds_chart_points() {
        let parts = json!([{
            "type":"rich_card",
            "text":"Bitcoin (BTC)",
            "kind":"finance",
            "richContent":{
                "schema":"yilong.rich-content.v1",
                "kind":"finance",
                "source":"official_dom",
                "payload":{
                    "title":"Bitcoin (BTC)",
                    "symbol":"BTC",
                    "primaryValue":"US$77,274.00",
                    "secondaryValue":"+US$123.00 (0.16%)",
                    "trend":"positive",
                    "periods":[{"id":"1d","label":"1D","selected":true}],
                    "periodViews":[
                        {
                            "id":"1d","label":"1D","selected":true,
                            "primaryValue":"US$77,274.00",
                            "secondaryValue":"+US$123.00 (0.16%)","trend":"positive",
                            "chart":{"kind":"line","points":[{"x":"12:00","y":77274.0},{"x":"13:00","y":77301.5}]}
                        },
                        {
                            "id":"5d","label":"5D","selected":false,"trend":"negative",
                            "chart":{"kind":"line","points":[{"x":"8/20","y":79000.0},{"x":"8/25","y":77274.0}]}
                        }
                    ],
                    "metrics":[{"label":"当日最低价","value":"76,601"}],
                    "chart":{"kind":"line","points":[{"x":"12:00","y":77274.0},{"x":"13:00","y":77301.5}]}
                },
                "privateDebugField":"secret"
            }
        }]);
        let sanitized = sanitize_parts("chatgpt", Some(&parts));
        assert_eq!(sanitized.len(), 1);
        assert_eq!(sanitized[0]["type"], "rich_card");
        assert_eq!(
            sanitized[0]["richContent"]["schema"],
            "yilong.rich-content.v1"
        );
        assert_eq!(
            sanitized[0]["richContent"]["payload"]["chart"]["points"][1]["y"],
            77301.5
        );
        assert_eq!(
            sanitized[0]["richContent"]["payload"]["periodViews"][1]["chart"]["points"][0]["y"],
            79000.0
        );
        assert_eq!(
            sanitized[0]["richContent"]["payload"]["periodViews"][1]["trend"],
            "negative"
        );
        assert!(!sanitized[0].to_string().contains("secret"));
    }

    #[test]
    fn authorized_client_chart_keeps_bounded_series_and_numeric_points() {
        let parts = json!([{
            "type":"rich_card",
            "text":"比特币近期走势",
            "kind":"chart",
            "richContent":{
                "schema":"yilong.rich-content.v1",
                "kind":"chart",
                "source":"private_response",
                "payload":{
                    "title":"比特币近期走势",
                    "description":"BTC/USD 日内收盘价参考。",
                    "chartType":"line",
                    "series":[{"key":"price","label":"BTC/USD","valuePrefix":"$","private":"secret"}],
                    "points":[
                        {"x":"8/20","values":[69268],"private":"secret"},
                        {"x":"8/21","values":[73031]},
                        {"x":"8/24","values":[77728]}
                    ]
                }
            }
        }]);
        let sanitized = sanitize_parts("chatgpt", Some(&parts));
        assert_eq!(sanitized.len(), 1);
        assert_eq!(sanitized[0]["kind"], "chart");
        assert_eq!(
            sanitized[0]["richContent"]["payload"]["points"][2]["values"][0],
            77728.0
        );
        assert!(!sanitized[0].to_string().contains("secret"));
    }

    #[test]
    fn finance_rich_card_keeps_valid_candlesticks_and_drops_invalid_values() {
        let parts = json!([{
            "type":"rich_card",
            "kind":"finance",
            "richContent":{
                "schema":"yilong.rich-content.v1",
                "kind":"finance",
                "source":"official_dom",
                "payload":{
                    "title":"Apple Inc. (AAPL)",
                    "primaryValue":"US$231.59",
                    "trend":"positive",
                    "chart":{"kind":"candlestick","candles":[
                        {"x":"2026-08-20","open":228.1,"high":232.4,"low":227.7,"close":231.2},
                        {"x":"invalid","open":231.2,"high":229.0,"low":228.0,"close":230.4},
                        {"x":"2026-08-21","open":231.2,"high":233.1,"low":229.8,"close":230.4}
                    ]}
                }
            }
        }]);
        let sanitized = sanitize_parts("chatgpt", Some(&parts));
        let candles = sanitized[0]["richContent"]["payload"]["chart"]["candles"]
            .as_array()
            .expect("valid OHLC data should survive the sanitizer");
        assert_eq!(candles.len(), 2);
        assert_eq!(candles[1]["close"], 230.4);

        let single_day = json!([{
            "type":"rich_card",
            "kind":"finance",
            "richContent":{
                "schema":"yilong.rich-content.v1",
                "kind":"finance",
                "source":"official_dom",
                "payload":{
                    "title":"Apple Inc. (AAPL)",
                    "primaryValue":"309.35 美元",
                    "trend":"negative",
                    "chart":{"kind":"candlestick","candles":[
                        {"x":"最新交易日","open":312.15,"high":312.6,"low":307.03,"close":309.35}
                    ]}
                }
            }
        }]);
        let sanitized_single_day = sanitize_parts("chatgpt", Some(&single_day));
        assert_eq!(
            sanitized_single_day[0]["richContent"]["payload"]["chart"]["candles"]
                .as_array()
                .expect("one visible OHLC table row should produce one candle")
                .len(),
            1
        );
    }

    #[test]
    fn weather_rich_card_preserves_rows_and_discards_unknown_fields() {
        let parts = json!([{
            "type":"rich_card",
            "text":"彰化县今晚天气",
            "kind":"weather",
            "richContent":{
                "schema":"yilong.rich-content.v1",
                "kind":"weather",
                "source":"official_dom",
                "payload":{
                    "title":"彰化县今晚天气",
                    "rows":[
                        {"period":"17:00","condition":"多云时阴","temperature":"33°C","precipitation":"20%","private":"secret"}
                    ]
                }
            }
        }]);
        let sanitized = sanitize_parts("google-ai-mode", Some(&parts));
        assert_eq!(sanitized[0]["kind"], "weather");
        assert_eq!(
            sanitized[0]["richContent"]["payload"]["rows"][0]["temperature"],
            "33°C"
        );
        assert!(!sanitized[0].to_string().contains("secret"));
    }

    #[test]
    fn media_and_map_rich_cards_keep_only_safe_visible_content() {
        let parts = json!([
            {
                "type":"rich_card",
                "text":"回答图片",
                "kind":"media_gallery",
                "richContent":{
                    "schema":"yilong.rich-content.v1",
                    "kind":"media_gallery",
                    "source":"official_dom",
                    "payload":{
                        "title":"回答图片",
                        "items":[
                            {"url":"https://images.example.com/chart.png?signature=secret","alt":"行情图","mediaType":"image/png","width":640,"height":360,"sourceUrl":"https://example.com/report?tracking=secret"},
                            {"url":"http://unsafe.example.com/image.png","alt":"不安全图片"}
                        ]
                    }
                }
            },
            {
                "type":"rich_card",
                "text":"附近地点",
                "kind":"map",
                "richContent":{
                    "schema":"yilong.rich-content.v1",
                    "kind":"map",
                    "source":"official_dom",
                    "payload":{"title":"附近地点","summary":"官网地图中的可见摘要","places":["人民广场","外滩"],"coordinates":"secret"}
                }
            }
        ]);
        let sanitized = sanitize_parts("chatgpt", Some(&parts));
        assert_eq!(sanitized.len(), 2);
        assert_eq!(
            sanitized[0]["richContent"]["payload"]["items"][0]["url"],
            "https://images.example.com/chart.png"
        );
        assert_eq!(
            sanitized[0]["richContent"]["payload"]["items"][0]["sourceUrl"],
            "https://example.com/report"
        );
        assert_eq!(sanitized[1]["richContent"]["payload"]["places"][1], "外滩");
        assert!(!Value::Array(sanitized).to_string().contains("secret"));
    }

    #[test]
    fn private_response_rich_cards_fail_closed_and_signal_renderer_upgrade() {
        let parts = json!([{
            "type":"rich_card",
            "text":"回答图片",
            "kind":"media_gallery",
            "richContent":{
                "schema":"yilong.rich-content.v1",
                "kind":"media_gallery",
                "source":"private_response",
                "payload":{"title":"回答图片","items":[{"url":"https://images.example.com/chart.png","alt":"行情图"}]}
            }
        }]);
        let sanitized = sanitize_parts("chatgpt", Some(&parts));
        assert_eq!(
            sanitized,
            vec![json!({
                "type":"interactive",
                "text":"回答图片",
                "kind":"renderer_upgrade_required"
            })]
        );
    }
}
