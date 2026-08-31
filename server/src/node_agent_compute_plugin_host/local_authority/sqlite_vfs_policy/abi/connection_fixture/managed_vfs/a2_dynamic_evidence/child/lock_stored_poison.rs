//! Fail-closed q3 header contract for stored-poison Lock quarantine evidence.

pub(in super::super) mod route_unknown;

pub(in super::super) const REPORT_VERSION: &str = "a2lockq3";
pub(in super::super) const REPORT_VALUE_COUNT: usize = 135;

pub(in super::super) fn selector(
    action_tag: u64,
    profile_tag: u64,
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
    let profile = profile_name(profile_tag)?;
    if !valid_range(action_tag, first, count) {
        return Err("A2_DYNAMIC_CHILD_ACTUAL_SELECTOR_INVALID");
    }
    Ok(format!(
        "{action}-first{first}-count{count}-{profile}-retention-succeeded"
    ))
}

pub(in super::super) fn classify_header(
    version: &str,
    candidate: &str,
) -> Result<Option<usize>, &'static str> {
    if version != REPORT_VERSION {
        return Ok(None);
    }
    for action_tag in 1..=4 {
        for count in 1..=8_u8 {
            for first in 0..=8 - count {
                for profile_tag in 1..=15 {
                    if selector(action_tag, profile_tag, first, count).as_deref() == Ok(candidate) {
                        return Ok(Some(REPORT_VALUE_COUNT));
                    }
                }
            }
        }
    }
    Err("A2_DYNAMIC_CHILD_ACTUAL_SELECTOR_INVALID")
}

pub(super) fn profile_name(tag: u64) -> Result<&'static str, &'static str> {
    match tag {
        1 => Ok("gate-none-lock-certain"),
        2 => Ok("file-close-none-lock-certain"),
        3 => Ok("exact-sibling-delete-none-lock-certain"),
        4 => Ok("exact-sibling-open-uncertain-lock-certain"),
        5 => Ok("dms-truncate-uncertain-lock-certain"),
        6 => Ok("file-close-uncertain-lock-certain"),
        7 => Ok("exact-sibling-delete-uncertain-lock-certain"),
        8 => Ok("file-grow-uncertain-lock-certain"),
        9 => Ok("mapping-close-uncertain-lock-certain"),
        10 => Ok("view-unmap-uncertain-lock-certain"),
        11 => Ok("lock-release-none-lock-uncertain"),
        12 => Ok("connection-detach-none-lock-uncertain"),
        13 => Ok("delete-authorization-none-lock-uncertain"),
        14 => Ok("dms-exclusive-release-uncertain-lock-uncertain"),
        15 => Ok("dms-shared-release-uncertain-lock-uncertain"),
        _ => Err("A2_DYNAMIC_CHILD_ACTUAL_SELECTOR_INVALID"),
    }
}

pub(super) const fn valid_range(action_tag: u64, first: u8, count: u8) -> bool {
    if count == 0 || first >= 8 || count > 8 - first {
        return false;
    }
    match action_tag {
        1 | 3 => count == 1,
        2 | 4 => true,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    #[test]
    fn all_1320_stored_poison_selectors_are_unique_and_exact_width() {
        let mut selectors = BTreeSet::new();
        for action_tag in 1..=4 {
            for count in 1..=8_u8 {
                for first in 0..=8 - count {
                    if !valid_range(action_tag, first, count) {
                        continue;
                    }
                    for profile_tag in 1..=15 {
                        let value = selector(action_tag, profile_tag, first, count).unwrap();
                        assert!(selectors.insert(value.clone()));
                        assert_eq!(
                            classify_header(REPORT_VERSION, &value),
                            Ok(Some(REPORT_VALUE_COUNT))
                        );
                    }
                }
            }
        }
        assert_eq!(selectors.len(), 1_320);
    }
}
