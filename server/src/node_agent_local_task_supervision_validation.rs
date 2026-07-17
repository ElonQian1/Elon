const MAX_ID_CHARS: usize = 200;

pub(super) fn clean_enum(value: &str, field: &str, allowed: &[&str]) -> Result<String, String> {
    let value = value.trim();
    allowed
        .contains(&value)
        .then(|| value.to_string())
        .ok_or_else(|| format!("{field} 只能是 {}。", allowed.join("、")))
}

pub(super) fn clean_id(value: &str, field: &str) -> Result<String, String> {
    let value = value.trim();
    if value.is_empty()
        || value.chars().count() > MAX_ID_CHARS
        || value.chars().any(char::is_control)
    {
        return Err(format!(
            "{field} 不能为空、不能包含控制字符，且最多 {MAX_ID_CHARS} 个字符。"
        ));
    }
    Ok(value.to_string())
}

pub(super) fn clean_optional_id(
    value: Option<&str>,
    field: &str,
) -> Result<Option<String>, String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| clean_id(value, field))
        .transpose()
}

pub(super) fn clean_list(
    values: Vec<String>,
    max_items: usize,
    max_chars: usize,
    field: &str,
) -> Result<Vec<String>, String> {
    if values.len() > max_items {
        return Err(format!("{field} 最多 {max_items} 项。"));
    }
    let mut cleaned = Vec::new();
    for value in values {
        let value = value.trim();
        if value.is_empty() {
            continue;
        }
        if value.chars().count() > max_chars {
            return Err(format!("{field} 单项不能超过 {max_chars} 个字符。"));
        }
        cleaned.push(value.to_string());
    }
    Ok(cleaned)
}
