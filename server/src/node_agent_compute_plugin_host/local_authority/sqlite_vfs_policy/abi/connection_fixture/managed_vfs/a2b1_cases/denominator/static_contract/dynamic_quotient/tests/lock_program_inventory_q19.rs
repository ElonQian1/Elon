//! Q19 inventory boundary kept separate from the aggregate inventory test.

use std::collections::BTreeSet;

use super::lock_native_acquire_existing_first_shared_busy_close_succeeded_cases::{
    lock_existing_first_shared_busy_close_succeeded_expected_groups_v1,
    LOCK_EXISTING_FIRST_SHARED_BUSY_CLOSE_SUCCEEDED_MEMBER_COUNT,
};
use super::{DynamicClassKeyV1, StaticMemberSealV1};

pub(super) fn lock_q19_expected_groups_after_v1(
    prior_keys: &BTreeSet<DynamicClassKeyV1>,
) -> BTreeSet<(DynamicClassKeyV1, StaticMemberSealV1)> {
    let groups = lock_existing_first_shared_busy_close_succeeded_expected_groups_v1();
    assert_eq!(
        groups.len(),
        LOCK_EXISTING_FIRST_SHARED_BUSY_CLOSE_SUCCEEDED_MEMBER_COUNT
    );
    assert!(groups
        .iter()
        .all(|(key, _)| !prior_keys.contains(key)));
    assert_eq!(
        groups
            .iter()
            .map(|(_, member)| *member)
            .collect::<BTreeSet<_>>()
            .len(),
        LOCK_EXISTING_FIRST_SHARED_BUSY_CLOSE_SUCCEEDED_MEMBER_COUNT
    );
    groups
}
