use super::*;

pub(super) fn context_pack_has_fb2_external_context(context_pack: &str) -> bool {
    contains_any(
        context_pack,
        &[
            "fb2.answer_policy.v1",
            "<fb2_context_pack",
            "\"app_id\": \"fb2\"",
            "\"app_id\":\"fb2\"",
        ],
    )
}

pub(super) fn contains_any(text: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| text.contains(needle))
}

pub(super) fn ensure_fb2_summary_context_audit_source(summary: &str, context_pack: &str) -> String {
    let Some(context_audit_id) = extract_context_audit_id(context_pack) else {
        return summary.to_string();
    };
    if summary.contains(&context_audit_id) {
        return summary.to_string();
    }
    format!("{summary}\n\n## 来源审计\n- context_audit_id {context_audit_id}")
}

pub(super) fn extract_context_audit_id(context_pack: &str) -> Option<String> {
    let parsed: Value = serde_json::from_str(context_pack).ok()?;
    find_context_audit_id(&parsed)
}

pub(super) fn find_context_audit_id(value: &Value) -> Option<String> {
    if let Some(id) = value
        .get("context_audit_id")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|id| !id.is_empty())
    {
        return Some(id.to_string());
    }
    match value {
        Value::Array(items) => items.iter().find_map(find_context_audit_id),
        Value::Object(map) => map.values().find_map(find_context_audit_id),
        _ => None,
    }
}

pub(super) fn sanitize_unmatched_fb2_source_ids(
    summary: &str,
    context_pack: &str,
    external_context: Option<&Value>,
) -> String {
    let allowed_ids = fb2_summary_allowed_source_ids(context_pack, external_context);
    let chars = summary.chars().collect::<Vec<_>>();
    let mut out = String::with_capacity(summary.len());
    let mut index = 0;

    while index < chars.len() {
        if starts_ext_source_token_at(&chars, index) {
            let (token, next_index) = read_ext_source_token(&chars, index);
            push_allowed_or_redacted_source_token(&mut out, &allowed_ids, &token);
            index = next_index;
            continue;
        }
        if starts_uuid_source_token_at(&chars, index) {
            let token = chars[index..index + 36].iter().collect::<String>();
            push_allowed_or_redacted_source_token(&mut out, &allowed_ids, &token);
            index += 36;
            continue;
        }
        out.push(chars[index]);
        index += 1;
    }

    out
}

pub(super) fn read_ext_source_token(chars: &[char], mut index: usize) -> (String, usize) {
    let start = index;
    index += 4;
    while index < chars.len() && is_source_token_char(chars[index]) {
        index += 1;
    }
    (chars[start..index].iter().collect::<String>(), index)
}

pub(super) fn push_allowed_or_redacted_source_token(
    out: &mut String,
    allowed_ids: &HashSet<String>,
    token: &str,
) {
    if allowed_ids.contains(&token.to_ascii_lowercase()) {
        out.push_str(token);
    } else {
        out.push_str("未核验来源编号");
    }
}

pub(super) fn fb2_summary_allowed_source_ids(
    context_pack: &str,
    external_context: Option<&Value>,
) -> HashSet<String> {
    let mut out = HashSet::new();
    let parsed_context;
    let context = if let Some(context) = external_context {
        Some(context)
    } else {
        parsed_context = serde_json::from_str::<Value>(context_pack).ok();
        parsed_context
            .as_ref()
            .and_then(|pack| pack.get("external_app_context"))
    };
    let Some(context) = context else {
        return out;
    };

    if let Some(context_audit_id) = context
        .get("context_audit_id")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        out.insert(context_audit_id.to_ascii_lowercase());
    }
    for source in context
        .get("citation_sources")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        for field in ["id", "message_id", "source_message_id", "context_audit_id"] {
            if let Some(value) = source
                .get(field)
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
            {
                out.insert(value.to_ascii_lowercase());
            }
        }
    }
    out
}

pub(super) fn starts_ext_source_token_at(chars: &[char], index: usize) -> bool {
    if index + 4 > chars.len() {
        return false;
    }
    if index > 0 && is_source_token_char(chars[index - 1]) {
        return false;
    }
    chars[index].eq_ignore_ascii_case(&'e')
        && chars[index + 1].eq_ignore_ascii_case(&'x')
        && chars[index + 2].eq_ignore_ascii_case(&'t')
        && chars[index + 3] == '-'
}

pub(super) fn starts_uuid_source_token_at(chars: &[char], index: usize) -> bool {
    const UUID_LEN: usize = 36;
    if index + UUID_LEN > chars.len() {
        return false;
    }
    if index > 0 && is_source_token_char(chars[index - 1]) {
        return false;
    }
    for offset in 0..UUID_LEN {
        let ch = chars[index + offset];
        let valid = match offset {
            8 | 13 | 18 | 23 => ch == '-',
            _ => ch.is_ascii_hexdigit(),
        };
        if !valid {
            return false;
        }
    }
    if index + UUID_LEN < chars.len() && is_source_token_char(chars[index + UUID_LEN]) {
        return false;
    }
    true
}

pub(super) fn is_source_token_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | ':')
}


#[cfg(test)]
#[path = "group_summary_context_pack_tests.rs"]
mod tests;
