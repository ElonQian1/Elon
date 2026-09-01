//! Canonical q16 child header for unread truncate and cleanup-release receipts.

pub(in super::super) const REPORT_VERSION: &str = "a2lockq16";
pub(in super::super) const REPORT_VALUE_COUNT: usize = 164;

pub(in super::super) fn classify_header(
    version: &str,
    selected: &str,
) -> Result<Option<usize>, &'static str> {
    if version != REPORT_VERSION {
        return Ok(None);
    }
    for action_tag in 1..=2 {
        for first in 0..8 {
            for count in 1..=8 - first {
                for completion_tag in 1..=2 {
                    if selector(action_tag, first, count, completion_tag).as_deref() == Ok(selected)
                    {
                        return Ok(Some(REPORT_VALUE_COUNT));
                    }
                }
            }
        }
    }
    Err("A2_DYNAMIC_CHILD_ACTUAL_SELECTOR_INVALID")
}

pub(in super::super) fn selector(
    action_tag: u64,
    first: u8,
    count: u8,
    completion_tag: u64,
) -> Result<String, &'static str> {
    let end = first
        .checked_add(count)
        .ok_or("A2_DYNAMIC_CHILD_ACTUAL_SELECTOR_INVALID")?;
    if count == 0 || first >= 8 || end > 8 {
        return Err("A2_DYNAMIC_CHILD_ACTUAL_SELECTOR_INVALID");
    }
    let action = match action_tag {
        1 if count == 1 => "lock-shared",
        2 => "lock-exclusive",
        _ => return Err("A2_DYNAMIC_CHILD_ACTUAL_SELECTOR_INVALID"),
    };
    let completion = match completion_tag {
        1 => "retention-succeeded",
        2 => "retention-route-unknown-prior-quarantine",
        _ => return Err("A2_DYNAMIC_CHILD_ACTUAL_SELECTOR_INVALID"),
    };
    Ok(format!(
        "initialization-{action}-first-{first}-count-{count}-created-first-truncate-error-release-failed-{completion}-terminal-route-unknown"
    ))
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    #[test]
    fn eighty_eight_q16_selectors_are_unique_and_fail_closed() {
        let mut selectors = BTreeSet::new();
        for first in 0..8 {
            for count in 1..=8 - first {
                for completion in 1..=2 {
                    selectors
                        .insert(selector(2, first, count, completion).expect("exclusive selector"));
                }
            }
            for completion in 1..=2 {
                selectors.insert(selector(1, first, 1, completion).expect("shared selector"));
            }
        }
        assert_eq!(selectors.len(), 88);
        assert!(selector(1, 0, 2, 1).is_err());
        assert!(selector(3, 0, 1, 1).is_err());
        assert!(selector(2, 7, 2, 1).is_err());
        assert!(selector(2, 0, 1, 3).is_err());
    }
}
