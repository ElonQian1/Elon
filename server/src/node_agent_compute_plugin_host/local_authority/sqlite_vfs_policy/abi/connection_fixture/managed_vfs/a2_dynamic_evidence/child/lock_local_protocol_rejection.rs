//! Canonical q8 child header for real local Lock protocol rejection evidence.

pub(in super::super) const REPORT_VERSION: &str = "a2lockq8";
pub(in super::super) const REPORT_VALUE_COUNT: usize = 134;

pub(in super::super) fn classify_header(
    version: &str,
    selected: &str,
) -> Result<Option<usize>, &'static str> {
    if version != REPORT_VERSION {
        return Ok(None);
    }
    for path_tag in 1..=2 {
        for action_tag in 1..=4 {
            for first in 0..8 {
                for count in 1..=8 - first {
                    if selector(path_tag, action_tag, first, count).as_deref() == Ok(selected) {
                        return Ok(Some(REPORT_VALUE_COUNT));
                    }
                }
            }
        }
    }
    Err("A2_DYNAMIC_CHILD_ACTUAL_SELECTOR_INVALID")
}

pub(in super::super) fn selector(
    path_tag: u64,
    action_tag: u64,
    first: u8,
    count: u8,
) -> Result<String, &'static str> {
    let end = first
        .checked_add(count)
        .ok_or("A2_DYNAMIC_CHILD_ACTUAL_SELECTOR_INVALID")?;
    if count == 0 || first >= 8 || end > 8 {
        return Err("A2_DYNAMIC_CHILD_ACTUAL_SELECTOR_INVALID");
    }
    let (path, action, shared) = match (path_tag, action_tag) {
        (1, 1) => ("own-overlap", "lock-shared", true),
        (1, 2) => ("own-overlap", "lock-exclusive", false),
        (2, 3) => ("not-held", "unlock-shared", true),
        (2, 4) => ("not-held", "unlock-exclusive", false),
        _ => return Err("A2_DYNAMIC_CHILD_ACTUAL_SELECTOR_INVALID"),
    };
    if shared && count != 1 {
        return Err("A2_DYNAMIC_CHILD_ACTUAL_SELECTOR_INVALID");
    }
    Ok(format!(
        "{action}-first{first}-count{count}-{path}-terminal-completed"
    ))
}
