//! Fail-closed payload header contract for completed Lock lifecycle evidence.

pub(in super::super) const REPORT_VERSION: &str = "a2lockq2";
pub(in super::super) const REPORT_VALUE_COUNT: usize = 103;

pub(in super::super) fn selector(
    action_tag: u64,
    path_tag: u64,
    first: u8,
    count: u8,
) -> Result<String, &'static str> {
    let action = match action_tag {
        1 => "lock-shared",
        2 => "lock-exclusive",
        3 => "unlock-shared",
        4 => "unlock-exclusive",
        _ => return Err("A2_DYNAMIC_CHILD_ACTUAL_SELECTOR_INVALID"),
    };
    let path = match path_tag {
        1 => "native-acquire",
        2 => "native-release",
        3 => "shared-local-acquire",
        4 => "shared-local-release",
        _ => return Err("A2_DYNAMIC_CHILD_ACTUAL_SELECTOR_INVALID"),
    };
    if !valid_range(action_tag, path_tag, first, count) {
        return Err("A2_DYNAMIC_CHILD_ACTUAL_SELECTOR_INVALID");
    }
    Ok(format!(
        "{path}-{action}-first{first}-count{count}-completed"
    ))
}

pub(in super::super) fn classify_header(
    version: &str,
    candidate: &str,
) -> Result<Option<usize>, &'static str> {
    if version != REPORT_VERSION {
        return Ok(None);
    }
    for (action_tag, path_tag) in [(1, 1), (2, 1), (3, 2), (4, 2), (1, 3), (3, 4)] {
        for count in 1..=8u8 {
            for first in 0..=8 - count {
                if selector(action_tag, path_tag, first, count).as_deref() == Ok(candidate) {
                    return Ok(Some(REPORT_VALUE_COUNT));
                }
            }
        }
    }
    Err("A2_DYNAMIC_CHILD_ACTUAL_SELECTOR_INVALID")
}

const fn valid_range(action_tag: u64, path_tag: u64, first: u8, count: u8) -> bool {
    if count == 0 || first >= 8 || count > 8 - first {
        return false;
    }
    match (action_tag, path_tag) {
        (1, 1) | (3, 2) => count == 1,
        (2, 1) | (4, 2) => true,
        (1, 3) | (3, 4) => count == 1,
        _ => false,
    }
}
