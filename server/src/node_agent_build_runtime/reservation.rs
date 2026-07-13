use super::{admission::BuildCachePolicy, lease, paths::BuildRunPaths};
use anyhow::Result;

/// Capacity already promised to live build runs. The prepare lock serializes
/// admission, so two tasks cannot both observe the same unreserved free space.
pub(crate) fn active_reserved_bytes(paths: &BuildRunPaths) -> Result<u64> {
    lease::active_reserved_bytes(&paths.lease_root)
}

pub(crate) fn reservation_for_new_run(policy: &BuildCachePolicy) -> u64 {
    policy.build_headroom_bytes
}

/// Admission keeps the hard floor, every live task reservation, and this
/// task's own reservation free at the instant it is accepted.
pub(crate) fn admission_required_free(
    policy: &BuildCachePolicy,
    active_reserved_bytes: u64,
) -> u64 {
    policy
        .min_free_bytes
        .saturating_add(active_reserved_bytes)
        .saturating_add(reservation_for_new_run(policy))
}

/// A completed task restores enough capacity for existing tasks and one next
/// build. This preserves the previous "ready for the next task" behavior.
pub(crate) fn cleanup_required_free(
    policy: &BuildCachePolicy,
    active_reserved_bytes: u64,
) -> u64 {
    admission_required_free(policy, active_reserved_bytes)
}
