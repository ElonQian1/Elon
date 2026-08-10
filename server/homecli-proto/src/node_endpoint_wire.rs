pub(crate) const MAX_IJSON_SAFE_INTEGER: u64 = 9_007_199_254_740_991;

pub(crate) fn bounded_identifier(value: &str, max_bytes: usize) -> bool {
    !value.is_empty()
        && value == value.trim()
        && value.len() <= max_bytes
        && !value.chars().any(char::is_control)
}

pub(crate) fn bounded_timestamp(value: &str) -> bool {
    (20..=64).contains(&value.len())
        && value == value.trim()
        && !value.chars().any(char::is_control)
}

pub(crate) fn safe_integer(value: u64) -> bool {
    value <= MAX_IJSON_SAFE_INTEGER
}

pub(crate) fn positive_safe_integer(value: u64) -> bool {
    value > 0 && safe_integer(value)
}

pub(crate) fn sha256_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

pub(crate) fn bounded_string_list(values: &[String], max_items: usize) -> bool {
    values.len() <= max_items && values.iter().all(|value| bounded_identifier(value, 256))
}
