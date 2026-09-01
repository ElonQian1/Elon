//! Small selector-only bridge kept out of the process launcher.

use super::{CHILD_ROOT_ENV, STORED_POISON_SELECTOR_ENV};

pub(in super::super::super) fn selected_lock_stored_poison_selector_for_test() -> Option<String> {
    std::env::var_os(CHILD_ROOT_ENV)?;
    std::env::var(STORED_POISON_SELECTOR_ENV).ok()
}

pub(in super::super::super) fn lock_stored_poison_selector_for_test(
    action_tag: u64,
    profile_tag: u64,
    first: u8,
    count: u8,
    completion_tag: u64,
) -> Result<String, &'static str> {
    match completion_tag {
        3 => super::super::child::lock_stored_poison::selector(
            action_tag,
            profile_tag,
            first,
            count,
        ),
        4 => super::super::child::lock_stored_poison::route_unknown::selector(
            action_tag,
            profile_tag,
            first,
            count,
        ),
        _ => Err("A2_DYNAMIC_CHILD_ACTUAL_SELECTOR_INVALID"),
    }
}
