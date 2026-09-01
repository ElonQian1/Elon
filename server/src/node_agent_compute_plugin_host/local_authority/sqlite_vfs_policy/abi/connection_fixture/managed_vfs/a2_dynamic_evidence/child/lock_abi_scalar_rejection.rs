//! Canonical q10 child header for installed xShmLock ABI-scalar rejection.

pub(in super::super) const REPORT_VERSION: &str = "a2lockq10";
pub(in super::super) const REPORT_VALUE_COUNT: usize = 89;

pub(in super::super) fn classify_header(
    version: &str,
    selected: &str,
) -> Result<Option<usize>, &'static str> {
    if version != REPORT_VERSION {
        return Ok(None);
    }
    for offset in 1..=2 {
        for count in 1..=2 {
            for flags in 1..=2 {
                if selector(offset, count, flags).as_deref() == Ok(selected) {
                    return Ok(Some(REPORT_VALUE_COUNT));
                }
            }
        }
    }
    Err("A2_DYNAMIC_CHILD_ACTUAL_SELECTOR_INVALID")
}

pub(in super::super) fn selector(
    offset_tag: u64,
    count_tag: u64,
    flags_tag: u64,
) -> Result<String, &'static str> {
    if (offset_tag, count_tag, flags_tag) == (2, 2, 2) {
        return Err("A2_DYNAMIC_CHILD_ACTUAL_SELECTOR_INVALID");
    }
    Ok(format!(
        "abi-offset-{}-count-{}-flags-{}-direct",
        validity_label(offset_tag)?,
        validity_label(count_tag)?,
        validity_label(flags_tag)?,
    ))
}

fn validity_label(tag: u64) -> Result<&'static str, &'static str> {
    match tag {
        1 => Ok("invalid"),
        2 => Ok("valid"),
        _ => Err("A2_DYNAMIC_CHILD_ACTUAL_SELECTOR_INVALID"),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    #[test]
    fn seven_scalar_selectors_are_unique_and_fail_closed() {
        let mut selectors = BTreeSet::new();
        for offset in 1..=2 {
            for count in 1..=2 {
                for flags in 1..=2 {
                    match selector(offset, count, flags) {
                        Ok(selected) => assert!(selectors.insert(selected)),
                        Err(_) => assert_eq!((offset, count, flags), (2, 2, 2)),
                    }
                }
            }
        }
        assert_eq!(selectors.len(), 7);
        assert!(selector(2, 2, 2).is_err());
        for unknown in [0, 3, u64::MAX] {
            assert!(selector(unknown, 1, 1).is_err());
            assert!(selector(1, unknown, 1).is_err());
            assert!(selector(1, 1, unknown).is_err());
        }
    }
}
