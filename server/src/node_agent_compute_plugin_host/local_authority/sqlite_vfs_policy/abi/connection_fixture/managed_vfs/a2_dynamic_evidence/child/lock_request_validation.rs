//! Fail-closed payload header allow-list for Lock managed-request dynamic evidence.

pub(super) const REPORT_VERSION: &str = "a2lockq1";
pub(super) const REPORT_VALUE_COUNT: usize = 51;

const SELECTORS: &[&str] = &[
    "range-overflow-lock-shared-completed",
    "range-overflow-lock-exclusive-completed",
    "range-overflow-unlock-shared-completed",
    "range-overflow-unlock-exclusive-completed",
    "end-past-eight-lock-shared-completed",
    "end-past-eight-lock-exclusive-completed",
    "end-past-eight-unlock-shared-completed",
    "end-past-eight-unlock-exclusive-completed",
    "shared-multi-slot-lock-shared-completed",
    "shared-multi-slot-unlock-shared-completed",
];

/// Returns this family's fixed scalar width, rejects its unknown selectors, and leaves other
/// versions to the caller's closed version dispatcher.
pub(super) fn classify_header(
    version: &str,
    selector: &str,
) -> Result<Option<usize>, &'static str> {
    if version != REPORT_VERSION {
        return Ok(None);
    }
    if !SELECTORS.contains(&selector) {
        return Err("A2_DYNAMIC_CHILD_ACTUAL_SELECTOR_INVALID");
    }
    Ok(Some(REPORT_VALUE_COUNT))
}
