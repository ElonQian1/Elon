//! Fail-closed q4 payload header for successful multi-region Map loops.

pub(in super::super) const REPORT_VERSION: &str = "a2mapq4";
pub(in super::super) const REPORT_VALUE_COUNT: usize = 114;

const CREATED_FIRST_PREFIX: &str = "created-first-empty-extend-mapped-region-";
const NODE_LIVE_PREFIX: &str = "node-live-missing-extend-mapped-region-";
const CREATE_SEPARATOR: &str = "-create-";
const COMPLETED_SUFFIX: &str = "-completed";

pub(in super::super) fn selector(
    family_tag: u64,
    target_region: u32,
    regions_to_create: u16,
) -> Result<String, &'static str> {
    let prefix = match family_tag {
        1 if target_region <= 255 && regions_to_create == target_region as u16 + 1 => {
            CREATED_FIRST_PREFIX
        }
        2 if (1..=255).contains(&target_region) && regions_to_create == target_region as u16 => {
            NODE_LIVE_PREFIX
        }
        _ => return Err("A2_DYNAMIC_CHILD_ACTUAL_SELECTOR_INVALID"),
    };
    Ok(format!(
        "{prefix}{target_region}{CREATE_SEPARATOR}{regions_to_create}{COMPLETED_SUFFIX}"
    ))
}

pub(in super::super) fn classify_header(
    version: &str,
    candidate: &str,
) -> Result<Option<usize>, &'static str> {
    if version != REPORT_VERSION {
        return Ok(None);
    }
    for (family_tag, prefix) in [(1, CREATED_FIRST_PREFIX), (2, NODE_LIVE_PREFIX)] {
        let Some(axes) = candidate
            .strip_prefix(prefix)
            .and_then(|value| value.strip_suffix(COMPLETED_SUFFIX))
        else {
            continue;
        };
        let Some((region, count)) = axes.split_once(CREATE_SEPARATOR) else {
            continue;
        };
        let Some(target_region) = canonical_u32(region) else {
            continue;
        };
        let Some(regions_to_create) = canonical_u16(count) else {
            continue;
        };
        if selector(family_tag, target_region, regions_to_create).as_deref() == Ok(candidate) {
            return Ok(Some(REPORT_VALUE_COUNT));
        }
    }
    Err("A2_DYNAMIC_CHILD_ACTUAL_SELECTOR_INVALID")
}

fn canonical_u32(value: &str) -> Option<u32> {
    value
        .parse::<u32>()
        .ok()
        .filter(|parsed| parsed.to_string() == value)
}

fn canonical_u16(value: &str) -> Option<u16> {
    value
        .parse::<u16>()
        .ok()
        .filter(|parsed| parsed.to_string() == value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn q4_header_accepts_all_511_exact_members_at_fixed_width() {
        let mut selectors = std::collections::BTreeSet::new();
        for target_region in 0..=255 {
            selectors.insert(selector(1, target_region, target_region as u16 + 1).unwrap());
        }
        for target_region in 1..=255 {
            selectors.insert(selector(2, target_region, target_region as u16).unwrap());
        }
        assert_eq!(selectors.len(), 511);
        for selector in selectors {
            assert_eq!(
                classify_header(REPORT_VERSION, &selector),
                Ok(Some(REPORT_VALUE_COUNT))
            );
        }
    }

    #[test]
    fn q4_header_rejects_noncanonical_or_divergent_axes() {
        for candidate in [
            "created-first-empty-extend-mapped-region-00-create-1-completed",
            "created-first-empty-extend-mapped-region-0-create-2-completed",
            "created-first-empty-extend-mapped-region-256-create-257-completed",
            "node-live-missing-extend-mapped-region-0-create-0-completed",
            "node-live-missing-extend-mapped-region-2-create-1-completed",
            "node-live-missing-extend-mapped-region-1-create-01-completed",
        ] {
            assert_eq!(
                classify_header(REPORT_VERSION, candidate),
                Err("A2_DYNAMIC_CHILD_ACTUAL_SELECTOR_INVALID")
            );
        }
        assert_eq!(
            classify_header("a2mapq3", &selector(1, 0, 1).unwrap()),
            Ok(None)
        );
    }
}
