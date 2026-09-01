//! Canonical q7 child header for ordinary Lock callback-completion RouteUnknown evidence.

pub(in super::super) const REPORT_VERSION: &str = "a2lockq7";
pub(in super::super) const REPORT_VALUE_COUNT: usize = 141;

pub(in super::super) fn classify_header(
    version: &str,
    selected: &str,
) -> Result<Option<usize>, &'static str> {
    if version != REPORT_VERSION {
        return Ok(None);
    }
    for path_tag in 1..=6 {
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
        (1, 1) => ("native-acquire-acquired", "lock-shared", true),
        (1, 2) => ("native-acquire-acquired", "lock-exclusive", false),
        (2, 1) => ("native-acquire-busy", "lock-shared", true),
        (2, 2) => ("native-acquire-busy", "lock-exclusive", false),
        (3, 3) => ("native-release", "unlock-shared", true),
        (3, 4) => ("native-release", "unlock-exclusive", false),
        (4, 1) => ("shared-local-acquire", "lock-shared", true),
        (5, 3) => ("shared-local-release", "unlock-shared", true),
        (6, 1) => ("local-sibling-contention", "lock-shared", true),
        (6, 2) => ("local-sibling-contention", "lock-exclusive", false),
        _ => return Err("A2_DYNAMIC_CHILD_ACTUAL_SELECTOR_INVALID"),
    };
    if shared && count != 1 {
        return Err("A2_DYNAMIC_CHILD_ACTUAL_SELECTOR_INVALID");
    }
    Ok(format!(
        "{path}-{action}-first-{first}-count-{count}-terminal-route-unknown"
    ))
}
