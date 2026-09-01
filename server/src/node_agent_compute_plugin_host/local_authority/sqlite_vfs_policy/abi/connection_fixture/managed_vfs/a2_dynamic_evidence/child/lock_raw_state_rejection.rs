//! Canonical q11 child header for controlled, memory-safe raw-state rejection.

pub(in super::super) const REPORT_VERSION: &str = "a2lockq11";
pub(in super::super) const REPORT_VALUE_COUNT: usize = 98;

pub(in super::super) fn classify_header(
    version: &str,
    selected: &str,
) -> Result<Option<usize>, &'static str> {
    if version != REPORT_VERSION {
        return Ok(None);
    }
    if RAW_COMPLETION_TAGS
        .iter()
        .any(|&(raw_state, completion)| selector(raw_state, completion) == Ok(selected))
    {
        return Ok(Some(REPORT_VALUE_COUNT));
    }
    Err("A2_DYNAMIC_CHILD_ACTUAL_SELECTOR_INVALID")
}

pub(in super::super) const fn selector(
    raw_state_tag: u64,
    completion_tag: u64,
) -> Result<&'static str, &'static str> {
    match (raw_state_tag, completion_tag) {
        (1, 1) => Ok("raw-null-file-direct"),
        (2, 1) => Ok("raw-uninstalled-direct"),
        (3, 1) => Ok("raw-methods-null-state-present-direct"),
        (4, 1) => Ok("raw-foreign-methods-state-null-direct"),
        (5, 1) => Ok("raw-foreign-methods-state-present-direct"),
        (6, 1) => Ok("raw-exact-methods-state-null-direct"),
        (7, 6) => Ok("raw-other-type-payload-missing-drop-completed"),
        (8, 6) => Ok("raw-other-type-payload-present-drop-completed"),
        (8, 7) => Ok("raw-other-type-payload-present-drop-unwind-caught"),
        (9, 6) => Ok("raw-expected-type-payload-missing-drop-completed"),
        (10, 1) => Ok("raw-handle-bound-file-missing-direct"),
        _ => Err("A2_DYNAMIC_CHILD_ACTUAL_SELECTOR_INVALID"),
    }
}

const RAW_COMPLETION_TAGS: [(u64, u64); 11] = [
    (1, 1),
    (2, 1),
    (3, 1),
    (4, 1),
    (5, 1),
    (6, 1),
    (7, 6),
    (8, 6),
    (8, 7),
    (9, 6),
    (10, 1),
];

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    #[test]
    fn eleven_raw_state_selectors_are_unique_and_fail_closed() {
        let selectors = RAW_COMPLETION_TAGS
            .iter()
            .map(|&(raw_state, completion)| selector(raw_state, completion).expect("q11 selector"))
            .collect::<BTreeSet<_>>();
        assert_eq!(selectors.len(), 11);
        for unknown in [0, 11, u64::MAX] {
            assert!(selector(unknown, 1).is_err());
            assert!(selector(1, unknown).is_err());
        }
    }
}
