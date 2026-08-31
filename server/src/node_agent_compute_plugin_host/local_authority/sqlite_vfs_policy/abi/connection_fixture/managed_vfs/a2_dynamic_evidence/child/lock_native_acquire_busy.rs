//! Fail-closed q5 header contract for real native-acquire contention evidence.

pub(in super::super) const REPORT_VERSION: &str = "a2lockq5";
pub(in super::super) const REPORT_VALUE_COUNT: usize = 115;

pub(in super::super) fn selector(
    action_tag: u64,
    first: u8,
    count: u8,
) -> Result<String, &'static str> {
    let action = match action_tag {
        1 => "lock-shared",
        2 => "lock-exclusive",
        _ => return Err("A2_DYNAMIC_CHILD_ACTUAL_SELECTOR_INVALID"),
    };
    if !valid_range(action_tag, first, count) {
        return Err("A2_DYNAMIC_CHILD_ACTUAL_SELECTOR_INVALID");
    }
    Ok(format!(
        "native-acquire-{action}-first{first}-count{count}-native-busy-completed"
    ))
}

pub(in super::super) fn classify_header(
    version: &str,
    candidate: &str,
) -> Result<Option<usize>, &'static str> {
    if version != REPORT_VERSION {
        return Ok(None);
    }
    for action_tag in 1..=2 {
        for count in 1..=8_u8 {
            for first in 0..=8 - count {
                if selector(action_tag, first, count).as_deref() == Ok(candidate) {
                    return Ok(Some(REPORT_VALUE_COUNT));
                }
            }
        }
    }
    Err("A2_DYNAMIC_CHILD_ACTUAL_SELECTOR_INVALID")
}

const fn valid_range(action_tag: u64, first: u8, count: u8) -> bool {
    if count == 0 || first >= 8 || count > 8 - first {
        return false;
    }
    match action_tag {
        1 => count == 1,
        2 => true,
        _ => false,
    }
}
