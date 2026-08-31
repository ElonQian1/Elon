//! Fail-closed q4 header contract for the route-unknown stored-poison Lock sibling.

use super::{profile_name, valid_range};

pub(in super::super::super) const REPORT_VERSION: &str = "a2lockq4";
pub(in super::super::super) const REPORT_VALUE_COUNT: usize = 140;

pub(in super::super::super) fn selector(
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
    if !valid_range(action_tag, first, count) {
        return Err("A2_DYNAMIC_CHILD_ACTUAL_SELECTOR_INVALID");
    }
    let profile = profile_name(profile_tag)?;
    Ok(format!(
        "{action}-first{first}-count{count}-{profile}-retention-route-unknown"
    ))
}

pub(in super::super::super) fn classify_header(
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

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    #[test]
    fn all_1320_route_unknown_selectors_are_unique_and_exact_width() {
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
