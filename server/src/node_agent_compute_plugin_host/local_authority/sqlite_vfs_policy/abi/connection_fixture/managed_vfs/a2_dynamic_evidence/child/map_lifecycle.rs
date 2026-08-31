//! Fail-closed q3 payload header for six completed single-region Map lifecycles.

pub(in super::super) const REPORT_VERSION: &str = "a2mapq3";
pub(in super::super) const REPORT_VALUE_COUNT: usize = 111;

pub(in super::super) fn selector(path_tag: u64) -> Result<&'static str, &'static str> {
    match path_tag {
        1 => Ok("empty-observe-not-present-completed"),
        2 => Ok("empty-extend-mapped-completed"),
        3 => Ok("reuse-observe-mapped-completed"),
        4 => Ok("reuse-extend-mapped-completed"),
        5 => Ok("target-missing-observe-not-present-completed"),
        6 => Ok("target-missing-extend-mapped-completed"),
        _ => Err("A2_DYNAMIC_CHILD_ACTUAL_SELECTOR_INVALID"),
    }
}

pub(in super::super) fn classify_header(
    version: &str,
    candidate: &str,
) -> Result<Option<usize>, &'static str> {
    if version != REPORT_VERSION {
        return Ok(None);
    }
    if (1..=6).any(|tag| selector(tag) == Ok(candidate)) {
        Ok(Some(REPORT_VALUE_COUNT))
    } else {
        Err("A2_DYNAMIC_CHILD_ACTUAL_SELECTOR_INVALID")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn q3_header_accepts_exactly_six_selectors_at_fixed_width() {
        let selectors = (1..=6)
            .map(|tag| selector(tag).unwrap())
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(selectors.len(), 6);
        for selector in selectors {
            assert_eq!(
                classify_header(REPORT_VERSION, selector),
                Ok(Some(REPORT_VALUE_COUNT))
            );
        }
    }

    #[test]
    fn q3_header_rejects_wrong_version_and_selector() {
        assert_eq!(classify_header("a2mapq2", selector(1).unwrap()), Ok(None));
        assert_eq!(
            classify_header(REPORT_VERSION, "empty-observe-mapped-completed"),
            Err("A2_DYNAMIC_CHILD_ACTUAL_SELECTOR_INVALID")
        );
    }
}
