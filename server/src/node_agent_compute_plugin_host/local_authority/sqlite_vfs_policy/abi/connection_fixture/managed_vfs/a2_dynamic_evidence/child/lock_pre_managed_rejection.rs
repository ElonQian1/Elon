//! Canonical q9 child header for Lock callbacks rejected before managed/native dispatch.

pub(in super::super) const REPORT_VERSION: &str = "a2lockq9";
pub(in super::super) const REPORT_VALUE_COUNT: usize = 115;

pub(in super::super) fn classify_header(
    version: &str,
    selected: &str,
) -> Result<Option<usize>, &'static str> {
    if version != REPORT_VERSION {
        return Ok(None);
    }
    for rejection in 1..=4 {
        for completion in 1..=3 {
            for action in 1..=4 {
                for first in 0..8 {
                    for count in 1..=8 - first {
                        if selector(rejection, completion, action, first, count).as_deref()
                            == Ok(selected)
                        {
                            return Ok(Some(REPORT_VALUE_COUNT));
                        }
                    }
                }
            }
        }
    }
    Err("A2_DYNAMIC_CHILD_ACTUAL_SELECTOR_INVALID")
}

pub(in super::super) fn selector(
    rejection_tag: u64,
    completion_tag: u64,
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
    let profile = match (rejection_tag, completion_tag) {
        (1, 1) => "admission-route-unknown-direct",
        (2, 1) => "admission-counter-overflow-direct",
        (3, 2) => "unsupported-file-role-completed",
        (3, 3) => "unsupported-file-role-route-unknown",
        (4, 2) => "shm-detached-completed",
        (4, 3) => "shm-detached-route-unknown",
        _ => return Err("A2_DYNAMIC_CHILD_ACTUAL_SELECTOR_INVALID"),
    };
    let (action, shared) = match action_tag {
        1 => ("lock-shared", true),
        2 => ("lock-exclusive", false),
        3 => ("unlock-shared", true),
        4 => ("unlock-exclusive", false),
        _ => return Err("A2_DYNAMIC_CHILD_ACTUAL_SELECTOR_INVALID"),
    };
    if shared && count != 1 {
        return Err("A2_DYNAMIC_CHILD_ACTUAL_SELECTOR_INVALID");
    }
    Ok(format!("{profile}-{action}-first-{first}-count-{count}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn six_profiles_each_admit_exactly_eighty_eight_lock_ranges() {
        let total = (1..=4)
            .flat_map(|rejection| (1..=3).map(move |completion| (rejection, completion)))
            .flat_map(|(rejection, completion)| {
                (1..=4).flat_map(move |action| {
                    (0..8).flat_map(move |first| {
                        (1..=8 - first).map(move |count| {
                            selector(rejection, completion, action, first, count).is_ok()
                        })
                    })
                })
            })
            .filter(|valid| *valid)
            .count();
        assert_eq!(total, 6 * 88);
    }
}
