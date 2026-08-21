use serde_json::{json, Map, Value};

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

pub(super) fn sanitize_parts(value: Option<&Value>) -> Vec<Value> {
    value
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .take(MAX_CONTENT_PARTS)
        .filter_map(sanitize_part)
        .collect()
}

fn sanitize_part(value: &Value) -> Option<Value> {
    let part = value.as_object()?;
    let part_type = part.get("type")?.as_str()?;
    if matches!(part_type, "text" | "markdown") {
        let text = clean_string(part.get("text"), MAX_MESSAGE_CHARS);
        return (!text.is_empty()).then(|| json!({"type": part_type, "text": text}));
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

fn sanitize_public_url(value: Option<&Value>) -> String {
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
            {"type":"citation","text":"Docs","url":"https://example.com/docs?token=secret","iconUrl":"https://cdn.example.com/icons/docs.png?signature=secret","targetHost":"example.com"},
            {"type":"credential","text":"secret"}
        ]);
        let sanitized = sanitize_parts(Some(&parts));
        assert_eq!(sanitized.len(), 3);
        assert_eq!(sanitized[0]["type"], "markdown");
        assert_eq!(sanitized[1]["lineCount"], 12);
        assert_eq!(sanitized[2]["url"], "https://example.com/docs");
        assert_eq!(sanitized[2]["iconUrl"], "https://cdn.example.com/icons/docs.png");
        assert!(!Value::Array(sanitized).to_string().contains("token"));
    }
}
