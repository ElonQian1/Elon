//! Additive q4 runner for the exact route-already-unknown stored-poison Lock sibling.

mod payload;

use std::path::Path;

use anyhow::anyhow;

use super::{stored_poison, LockRunnerStoredPoisonBindingV1, LockRunnerStoredPoisonCompletionV1};

pub(super) use payload::{upgrade_payload, validate_payload};

pub(super) fn validate_binding(binding: LockRunnerStoredPoisonBindingV1) -> anyhow::Result<()> {
    if binding.completion != LockRunnerStoredPoisonCompletionV1::RetentionRouteUnknown {
        return Err(anyhow!("q4 Lock stored-poison completion mismatch"));
    }
    stored_poison::validate_common_binding(binding)
}

pub(super) fn exact_selector(binding: LockRunnerStoredPoisonBindingV1) -> String {
    super::super::child::lock_stored_poison::route_unknown::selector(
        stored_poison::action_tag(binding.action),
        binding.profile.tag(),
        binding.first,
        binding.count,
    )
    .expect("validated q4 Lock stored-poison selector")
}

pub(super) fn exercise_child(
    root: &Path,
    binding: LockRunnerStoredPoisonBindingV1,
) -> anyhow::Result<()> {
    validate_binding(binding)?;
    stored_poison::exercise_child_inner(root, binding)
}
